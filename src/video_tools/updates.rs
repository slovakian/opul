use super::install::{Failure, Resolved, Staging, download_error};
use super::manager::{invocation, is_current, persist};
use super::verification::{HealthFailure, health};
use super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
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
            // Files may still be in use by another Download or metadata probe.
            if let Err(error) = super::recovery::prune(&self.tools.config.root, &live.stored) {
                self.tools.log_failure(&error);
            }
            self.tools.idle.notify_one();
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
            invocation: invocation(&self.config, &pair),
            pair,
        })
    }

    /// Shared by shutdown and Remove. Cancels setup, checks, and updates without changing selection.
    pub fn cancel(self: &Arc<Self>) -> Result<VideoToolsSnapshot, String> {
        if let Some(schedule) = self.lock().schedule.take() {
            schedule.abort();
        }
        self.execute(VideoToolsOperation::Cancel)
    }

    fn download_outcome(&self, pair: &InstalledPair, success: bool) -> Result<(), String> {
        let mut live = self.lock();
        let Some(previous) = live.stored.rollback_pair.clone() else {
            return Ok(());
        };
        if live.stored.installed.as_ref().map(|p| p.runtime) != Some(pair.runtime) {
            return Ok(());
        }
        let mut stored = live.stored.clone();
        if !success {
            stored.installed = Some(previous.clone());
        }
        stored.previous_runtime = None;
        stored.rollback_pair = None;
        persist(&self.config, &stored)?;
        live.stored = stored;
        if let VideoToolsState::Ready {
            installed,
            update_flag,
            ..
        } = &mut live.snapshot.state
            && !success
        {
            *installed = previous;
            *update_flag = UpdateFlag::Failed {
                version: format!("Python runtime {}", pair.runtime),
            };
            self.log_failure(&format!(
                "Python runtime {} failed its first video Download; rolled back.",
                pair.runtime
            ));
        }
        self.publish(&mut live);
        Ok(())
    }

    pub(super) fn log_failure(&self, details: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.config.root.join("updates.log"))
        {
            let _ = writeln!(file, "{details}");
        }
    }

    pub(super) async fn run_update(self: Arc<Self>, id: Uuid, mode: UpdateMode) {
        let result = self.prepare_update(id, mode).await;
        let mut live = self.lock();
        if !is_current(&live, id) {
            return;
        }
        live.job = None;
        if let Err(failure) = result {
            self.log_failure(&failure.details);
            // Metadata checks fail silently; an install failure stays Ready with the old pair.
            if matches!(
                live.snapshot.state,
                VideoToolsState::Ready {
                    update_flag: UpdateFlag::Updating { .. },
                    ..
                }
            ) && let Some((_, version)) = live.candidate.clone()
                && let VideoToolsState::Ready { update_flag, .. } = &mut live.snapshot.state
            {
                *update_flag = UpdateFlag::Failed { version };
            }
            self.publish(&mut live);
        }
    }

    async fn prepare_update(self: &Arc<Self>, id: Uuid, mode: UpdateMode) -> Result<(), Failure> {
        let staging = Staging(self.config.root.join(format!(".install-{id}")));
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
        // Runtime is fetched and installed first when both components have updates.
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
        let mut candidate = invocation(&self.config, old);
        if let Some(path) = &script_dir {
            candidate.script = path.join("yt-dlp");
        }
        let version = if *script_update {
            &resolved.yt_dlp
        } else {
            &old.yt_dlp
        };
        // Every candidate script is checked on the installed runtime first, even with a newer runtime available.
        let existing_health = health(&candidate, version).await;
        if let Some(path) = &runtime_dir {
            candidate.interpreter = path.join("bin/python3");
            candidate.quickjs = path.join("bin/qjs");
        }
        let python_version = if *runtime_update {
            health(&candidate, version).await
        } else {
            existing_health
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
                let mut live = self.lock();
                if !is_current(&live, id) {
                    return Ok(());
                }
                if live.stored.installed.as_ref() != Some(old) {
                    return Err(download_error(
                        "The selected pair changed during its health check.",
                    ));
                }
                let mut stored = live.stored.clone();
                stored.available_update = None;
                stored.compatibility_hold = Some(CompatibilityHold {
                    yt_dlp: resolved.yt_dlp.clone(),
                    runtime: resolved.runtime,
                    opul_version: self.config.opul_version.clone(),
                });
                persist(&self.config, &stored).map_err(download_error)?;
                live.stored = stored;
                if let VideoToolsState::Ready { update_flag, .. } = &mut live.snapshot.state {
                    *update_flag = UpdateFlag::WaitingForRuntime {
                        version: version.clone(),
                    };
                }
                self.log_failure(&details);
                self.publish(&mut live);
                return Ok(());
            }
        };
        let installed = InstalledPair {
            runtime: if *runtime_update {
                resolved.runtime
            } else {
                old.runtime
            },
            yt_dlp: version.clone(),
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

    fn update_plan(
        &self,
        id: Uuid,
        resolved: &Resolved,
        mode: UpdateMode,
    ) -> Result<Option<UpdatePlan>, Failure> {
        let mut live = self.lock();
        if !is_current(&live, id) {
            return Ok(None);
        }
        let install = matches!(mode, UpdateMode::Manual)
            || (matches!(mode, UpdateMode::Automatic) && live.stored.keep_up_to_date);
        let old = live.stored.installed.clone().unwrap();
        let runtime_update = resolved.runtime_compatible
            && resolved.runtime > old.runtime
            && live.stored.skipped_runtime != Some(resolved.runtime);
        let script_update = resolved.yt_dlp > old.yt_dlp
            && live.stored.skipped_yt_dlp.as_ref() != Some(&resolved.yt_dlp);
        let version = if script_update {
            resolved.yt_dlp.clone()
        } else {
            format!("Python runtime {}", resolved.runtime)
        };
        live.candidate = Some((resolved.runtime, version.clone()));
        let held = live.stored.compatibility_hold.as_ref().is_some_and(|hold| {
            hold.yt_dlp == resolved.yt_dlp
                && hold.runtime == resolved.runtime
                && hold.opul_version == self.config.opul_version
        });
        let mut stored = live.stored.clone();
        stored.last_checked = Some(self.sources.environment.now().as_secs());
        stored.available_update = if !held && (runtime_update || script_update) {
            Some(UpdateNotice {
                identity: format!(
                    "runtime-{}:yt-dlp-{}",
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
            })
        } else {
            None
        };
        if !held {
            stored.compatibility_hold = None;
        }
        persist(&self.config, &stored).map_err(download_error)?;
        live.stored = stored;
        let checked = live.stored.last_checked;
        if let VideoToolsState::Ready {
            last_checked,
            update_flag,
            ..
        } = &mut live.snapshot.state
        {
            *last_checked = checked;
            *update_flag = if held {
                UpdateFlag::WaitingForRuntime { version }
            } else if !runtime_update && !script_update {
                UpdateFlag::None
            } else if install {
                UpdateFlag::Updating {
                    version,
                    progress: 0,
                }
            } else {
                UpdateFlag::Available { version }
            };
        }
        self.publish(&mut live);
        if !install || held || (!runtime_update && !script_update) {
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
        // Recheck under the acquisition lock after every wake: a new Download may have started.
        loop {
            let notified = self.idle.notified();
            {
                let mut live = self.lock();
                if !is_current(&live, id) {
                    return Ok(());
                }
                if live.stored.installed.as_ref() != Some(&plan.old) {
                    return Err(download_error(
                        "The selected pair changed while preparing an update.",
                    ));
                }
                if live.active_users == 0 {
                    super::install::place_pair(&self.config, &installed, runtime_dir, script_dir)
                        .map_err(download_error)?;
                    let mut stored = live.stored.clone();
                    if plan.runtime_update && stored.rollback_pair.is_none() {
                        stored.rollback_pair = Some(plan.old.clone());
                        stored.previous_runtime = Some(plan.old.runtime);
                    }
                    if plan.script_update {
                        stored.previous_yt_dlp = Some(plan.old.yt_dlp.clone());
                    }
                    stored.installed = Some(installed.clone());
                    stored.compatibility_hold = None;
                    stored.available_update = None;
                    persist(&self.config, &stored).map_err(download_error)?;
                    live.stored = stored;
                    if let VideoToolsState::Ready {
                        installed: current,
                        update_flag,
                        ..
                    } = &mut live.snapshot.state
                    {
                        *current = installed;
                        *update_flag = UpdateFlag::None;
                    }
                    self.publish(&mut live);
                    super::recovery::prune(&self.config.root, &live.stored)
                        .map_err(download_error)?;
                    return Ok(());
                }
            }
            notified.await;
        }
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
        if !is_current(&live, id) {
            return Ok(());
        }
        let mut stored = live.stored.clone();
        stored.available_update = None;
        match component {
            Component::Python => stored.skipped_runtime = Some(resolved.runtime),
            Component::YtDlp => stored.skipped_yt_dlp = Some(resolved.yt_dlp.clone()),
        }
        persist(&self.config, &stored).map_err(download_error)?;
        live.stored = stored;
        Ok(())
    }
}
