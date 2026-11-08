use super::install::{Failure, Resolved, Staging, download_error};
use super::layout;
use super::manager::{LiveState, runtime_label};
use super::verification::{HealthFailure, health};
use super::*;
use std::fs;
use std::sync::Arc;
use uuid::Uuid;

/// Retains one selected pair for executing managed-tool work. Drop releases files without
/// reporting a Download outcome; only finish_download changes runtime probation.
/// Acquire at execution, not enqueue time; update activation and acquisition share one lock.
pub struct VideoToolsUse {
    tools: Arc<VideoTools>,
    pair: InstalledPair,
    pub invocation: VideoToolsInvocation,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct CompatibilityHold {
    pub yt_dlp: String,
    pub runtime: u64,
    pub opul_version: String,
}

#[derive(Clone, Copy)]
pub(super) enum UpdateMode {
    Check,
    Manual,
    Automatic,
}

struct UpdatePlan {
    old: InstalledPair,
    runtime_update: bool,
    script_update: bool,
}

impl VideoToolsUse {
    pub fn finish_download(self, success: bool) -> Result<(), String> {
        let result = self.tools.download_outcome(&self.pair, success);
        if let Err(error) = &result {
            self.tools.log_failure(error);
        }
        result
    }
}

impl Drop for VideoToolsUse {
    fn drop(&mut self) {
        let mut live = self.tools.lock();
        live.active_users -= 1;
        if live.active_users == 0 {
            if let Err(error) = super::recovery::prune(&self.tools.config.root, &live.stored) {
                self.tools.log_failure(&error);
            }
            self.tools.idle.notify_waiters();
        }
    }
}

impl VideoTools {
    pub fn acquire(self: &Arc<Self>) -> Option<VideoToolsUse> {
        let mut live = self.lock();
        let VideoToolsState::Ready { installed, .. } = &live.snapshot.state else {
            return None;
        };
        let pair = installed.clone();
        live.active_users += 1;
        Some(VideoToolsUse {
            tools: Arc::clone(self),
            invocation: layout::selected(&self.config, &pair),
            pair,
        })
    }

    /// Run `act` under the activation lock once no Download or probe holds the tools.
    /// Returns None when the job was cancelled or replaced while it waited.
    pub(super) async fn when_idle<T>(
        &self,
        id: Uuid,
        mut act: impl FnMut(&mut LiveState) -> Result<T, Failure>,
    ) -> Result<Option<T>, Failure> {
        loop {
            let notified = self.idle.notified();
            {
                let mut live = self.lock();
                if !live.is_current(id) {
                    return Ok(None);
                }
                if live.active_users == 0 {
                    return act(&mut live).map(Some);
                }
            }
            notified.await;
        }
    }

    fn download_outcome(&self, pair: &InstalledPair, success: bool) -> Result<(), String> {
        let mut live = self.lock();
        if live.stored.rollback_pair.is_none()
            || live.stored.installed.as_ref().map(|p| p.runtime) != Some(pair.runtime)
        {
            return Ok(());
        }
        live.commit(&self.config, |stored| {
            if success {
                stored.rollback_pair = None;
            } else {
                stored.roll_back();
            }
        })?;
        if !success && let Some(previous) = live.stored.installed.clone() {
            if let VideoToolsState::Ready { installed, .. } = &mut live.snapshot.state {
                *installed = previous;
            }
            live.set_update_flag(UpdateFlag::Failed {
                version: runtime_label(pair.runtime),
            });
            self.log_failure(&format!(
                "{} failed its first video Download; rolled back.",
                runtime_label(pair.runtime)
            ));
        }
        self.publish(&mut live);
        Ok(())
    }

    pub(super) async fn run_update(self: Arc<Self>, id: Uuid, mode: UpdateMode) {
        let result = self.prepare_update(id, mode).await;
        let mut live = self.lock();
        if !live.is_current(id) {
            return;
        }
        live.job = None;
        if let Err(failure) = result {
            self.log_failure(&failure.details);
            // Metadata checks fail silently; an install failure stays Ready with the old pair.
            if let VideoToolsState::Ready {
                update_flag: UpdateFlag::Updating { version, .. },
                ..
            } = &live.snapshot.state
            {
                let version = version.clone();
                live.set_update_flag(UpdateFlag::Failed { version });
            }
            self.publish(&mut live);
        }
    }

    async fn prepare_update(self: &Arc<Self>, id: Uuid, mode: UpdateMode) -> Result<(), Failure> {
        let staging = Staging(layout::staging_dir(&self.config.root, id));
        fs::create_dir(&staging.0).map_err(download_error)?;
        let client = self.sources.client()?;
        let transfer = super::transport::Transfer {
            tools: self,
            client: &client,
            step: None,
        };
        let resolved = self.resolve(&transfer).await?;
        let Some(plan) = self.update_plan(id, &resolved, mode)? else {
            return Ok(());
        };
        let UpdatePlan {
            old,
            runtime_update,
            script_update,
        } = &plan;
        let runtime_dir = if *runtime_update {
            match self.fetch_runtime(id, &staging.0, &client, &resolved).await {
                Ok(path) => Some(path),
                Err(failure) => {
                    self.skip_integrity(id, &resolved, Component::Python, &failure)?;
                    return Err(failure);
                }
            }
        } else {
            None
        };
        let script_dir = if *script_update {
            match self.fetch_script(id, &staging.0, &client, &resolved).await {
                Ok(path) => Some(path),
                Err(failure) => {
                    self.skip_integrity(id, &resolved, Component::YtDlp, &failure)?;
                    return Err(failure);
                }
            }
        } else {
            None
        };
        let usage = self
            .acquire()
            .ok_or_else(|| download_error("The selected pair is unavailable."))?;
        if usage.pair != *old {
            return Err(download_error(
                "The selected pair changed while preparing an update.",
            ));
        }
        let selected_runtime = layout::runtime_dir(&self.config.root, old.runtime);
        let selected_script =
            layout::script_dir(&self.config.root, old.yt_dlp_channel, &old.yt_dlp);
        let script = script_dir.as_deref().unwrap_or(&selected_script);
        let version = if *script_update {
            &resolved.yt_dlp
        } else {
            &old.yt_dlp
        };
        // Every candidate script is checked on the installed runtime first, even with a newer runtime available.
        let existing_health = health(&layout::invocation(&selected_runtime, script), version).await;
        let python_version = match &runtime_dir {
            Some(runtime) => health(&layout::invocation(runtime, script), version).await,
            None => existing_health,
        };
        drop(usage);
        let python_version = match python_version {
            Ok(version) => version,
            Err(HealthFailure::Execution(details)) => {
                return Err(Failure {
                    reason: FailureReason::HealthCheck,
                    details,
                    exhausted_integrity: false,
                });
            }
            Err(HealthFailure::Incompatible(details)) => {
                return self.hold_for_runtime(id, old, &resolved, &details);
            }
        };
        let installed = InstalledPair {
            runtime: if *runtime_update {
                resolved.runtime
            } else {
                old.runtime
            },
            yt_dlp: version.clone(),
            yt_dlp_channel: if *script_update {
                resolved.yt_dlp_channel
            } else {
                old.yt_dlp_channel
            },
            python_version,
        };
        self.activate_update(
            id,
            &plan,
            installed,
            runtime_dir.as_deref(),
            script_dir.as_deref(),
        )
        .await
    }

    /// The new yt-dlp needs a newer runtime than is published. Keep the pair and wait.
    fn hold_for_runtime(
        &self,
        id: Uuid,
        old: &InstalledPair,
        resolved: &Resolved,
        details: &str,
    ) -> Result<(), Failure> {
        let mut live = self.lock();
        if !live.is_current(id) {
            return Ok(());
        }
        if live.stored.installed.as_ref() != Some(old) {
            return Err(download_error(
                "The selected pair changed during its health check.",
            ));
        }
        live.commit(&self.config, |stored| {
            stored.available_update = None;
            stored.compatibility_hold = Some(CompatibilityHold {
                yt_dlp: resolved.yt_dlp.clone(),
                runtime: resolved.runtime,
                opul_version: self.config.opul_version.clone(),
            });
        })
        .map_err(download_error)?;
        let flag = live.stored.update_flag();
        live.set_update_flag(flag);
        self.log_failure(details);
        self.publish(&mut live);
        Ok(())
    }

    fn update_plan(
        &self,
        id: Uuid,
        resolved: &Resolved,
        mode: UpdateMode,
    ) -> Result<Option<UpdatePlan>, Failure> {
        let mut live = self.lock();
        if !live.is_current(id) {
            return Ok(None);
        }
        let install = matches!(mode, UpdateMode::Manual)
            || (matches!(mode, UpdateMode::Automatic) && live.stored.keep_up_to_date);
        let old = live
            .stored
            .installed
            .clone()
            .ok_or_else(|| download_error("No video tools are installed."))?;
        let runtime_update = resolved.runtime_compatible
            && resolved.runtime > old.runtime
            && live.stored.skipped_runtime != Some(resolved.runtime);
        let script_update = (resolved.yt_dlp_channel != old.yt_dlp_channel
            || resolved.yt_dlp > old.yt_dlp)
            && live.stored.skipped_yt_dlp.as_ref() != Some(&resolved.yt_dlp);
        let any_update = runtime_update || script_update;
        let version = if script_update {
            resolved.yt_dlp.clone()
        } else {
            runtime_label(resolved.runtime)
        };
        live.candidate = Some((resolved.runtime, resolved.yt_dlp.clone()));
        let held = live.stored.compatibility_hold.as_ref().is_some_and(|hold| {
            hold.yt_dlp == resolved.yt_dlp
                && hold.runtime == resolved.runtime
                && hold.opul_version == self.config.opul_version
        });
        let checked = self.sources.environment.now().as_secs();
        live.commit(&self.config, |stored| {
            stored.last_checked = Some(checked);
            stored.available_update = (!held && any_update).then(|| UpdateNotice {
                identity: format!(
                    "{}:runtime-{}:yt-dlp-{}",
                    match resolved.yt_dlp_channel {
                        YtDlpChannel::Stable => "Stable",
                        YtDlpChannel::Nightly => "Nightly",
                    },
                    if runtime_update {
                        resolved.runtime
                    } else {
                        old.runtime
                    },
                    if script_update {
                        &resolved.yt_dlp
                    } else {
                        &old.yt_dlp
                    }
                ),
                version: version.clone(),
            });
            if !held {
                stored.compatibility_hold = None;
            }
        })
        .map_err(download_error)?;
        if let VideoToolsState::Ready { last_checked, .. } = &mut live.snapshot.state {
            *last_checked = Some(checked);
        }
        let flag = if held {
            live.stored.update_flag()
        } else if !any_update {
            UpdateFlag::None
        } else if install {
            UpdateFlag::Updating {
                version,
                progress: 0,
            }
        } else {
            UpdateFlag::Available { version }
        };
        live.set_update_flag(flag);
        self.publish(&mut live);
        if !install || held || !any_update {
            return Ok(None);
        }
        Ok(Some(UpdatePlan {
            old,
            runtime_update,
            script_update,
        }))
    }

    async fn activate_update(
        &self,
        id: Uuid,
        plan: &UpdatePlan,
        installed: InstalledPair,
        runtime_dir: Option<&std::path::Path>,
        script_dir: Option<&std::path::Path>,
    ) -> Result<(), Failure> {
        self.when_idle(id, |live| {
            if live.stored.installed.as_ref() != Some(&plan.old) {
                return Err(download_error(
                    "The selected pair changed while preparing an update.",
                ));
            }
            super::install::place_pair(&self.config, &installed, runtime_dir, script_dir)
                .map_err(download_error)?;
            live.commit(&self.config, |stored| {
                if plan.runtime_update && stored.rollback_pair.is_none() {
                    stored.rollback_pair = Some(plan.old.clone());
                }
                if plan.script_update {
                    stored.previous_yt_dlp = Some(layout::script_name(
                        plan.old.yt_dlp_channel,
                        &plan.old.yt_dlp,
                    ));
                }
                stored.installed = Some(installed.clone());
                stored.compatibility_hold = None;
                stored.available_update = None;
            })
            .map_err(download_error)?;
            if let VideoToolsState::Ready {
                installed: current, ..
            } = &mut live.snapshot.state
            {
                *current = installed.clone();
            }
            live.set_update_flag(UpdateFlag::None);
            self.publish(live);
            super::recovery::prune(&self.config.root, &live.stored).map_err(download_error)
        })
        .await
        .map(|_| ())
    }

    fn skip_integrity(
        &self,
        id: Uuid,
        resolved: &Resolved,
        component: Component,
        failure: &Failure,
    ) -> Result<(), Failure> {
        if !failure.exhausted_integrity {
            return Ok(());
        }
        let mut live = self.lock();
        if !live.is_current(id) {
            return Ok(());
        }
        live.commit(&self.config, |stored| {
            stored.available_update = None;
            match component {
                Component::Python => stored.skipped_runtime = Some(resolved.runtime),
                Component::YtDlp => stored.skipped_yt_dlp = Some(resolved.yt_dlp.clone()),
            }
        })
        .map_err(download_error)
    }
}
