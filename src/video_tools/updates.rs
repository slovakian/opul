use super::install::{Failure, Resolved, Staging, download_error, place};
use super::manager::{invocation, is_current, persist};
use super::verification::health;
use super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Holds one selected pair for an executing video Download. Dropping it reports cancellation.
/// Acquire at execution, not enqueue time; update activation and acquisition share one lock.
pub struct VideoDownload {
    tools: Arc<VideoTools>,
    pair: InstalledPair,
    pub invocation: VideoToolsInvocation,
}

impl VideoDownload {
    pub fn finish(self, success: bool) -> Result<(), String> {
        let result = self.tools.download_outcome(&self.pair, success);
        if let Err(error) = &result {
            self.tools.log_failure(error);
        }
        result
    }
}

impl Drop for VideoDownload {
    fn drop(&mut self) {
        let mut live = self.tools.lock();
        live.active_downloads -= 1;
        if live.active_downloads == 0 {
            // Files may still be in use by another Download of the same runtime.
            if let Err(error) = super::recovery::prune(&self.tools.config.root, &live.stored) {
                self.tools.log_failure(&error);
            }
            self.tools.idle.notify_one();
        }
    }
}

impl VideoTools {
    pub fn begin_video_download(self: &Arc<Self>) -> Option<VideoDownload> {
        let mut live = self.lock();
        let VideoToolsState::Ready { installed, .. } = &live.snapshot.state else {
            return None;
        };
        let pair = installed.clone();
        live.active_downloads += 1;
        Some(VideoDownload {
            tools: Arc::clone(self),
            invocation: invocation(&self.config, &pair),
            pair,
        })
    }

    /// Shared by shutdown and Remove. Cancels setup, checks, and updates without changing selection.
    pub fn cancel(self: &Arc<Self>) -> Result<VideoToolsSnapshot, String> {
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

    pub(super) async fn run_update(self: Arc<Self>, id: Uuid, install: bool) {
        let result = self.prepare_update(id, install).await;
        let mut live = self.lock();
        if !is_current(&live, id) {
            return;
        }
        live.job = None;
        if let Err(failure) = result {
            self.log_failure(&failure.details);
            // Metadata checks fail silently; an install failure stays Ready with the old pair.
            if install
                && let Some((_, version)) = live.candidate.clone()
                && let VideoToolsState::Ready { update_flag, .. } = &mut live.snapshot.state
            {
                *update_flag = UpdateFlag::Failed { version };
            }
            self.publish(&mut live);
        }
    }

    async fn prepare_update(&self, id: Uuid, install: bool) -> Result<(), Failure> {
        let staging = Staging(self.config.root.join(format!(".install-{id}")));
        fs::create_dir(&staging.0).map_err(download_error)?;
        let client = self.sources.client()?;
        let transfer = super::transport::Transfer {
            tools: self,
            client: &client,
            step: Some((id, Component::Python)),
        };
        let resolved = self.resolve(&transfer).await?;
        let (old, runtime_update, script_update, held) = {
            let mut live = self.lock();
            if !is_current(&live, id) {
                return Ok(());
            }
            let old = live.stored.installed.clone().unwrap();
            let runtime_update = resolved.runtime > old.runtime
                && live.stored.skipped_runtime != Some(resolved.runtime);
            let script_update = resolved.yt_dlp > old.yt_dlp
                && live.stored.skipped_yt_dlp.as_ref() != Some(&resolved.yt_dlp);
            let version = if script_update {
                resolved.yt_dlp.clone()
            } else {
                format!("Python runtime {}", resolved.runtime)
            };
            live.candidate = Some((resolved.runtime, version.clone()));
            let held = live.stored.compatibility_hold.as_ref()
                == Some(&(resolved.yt_dlp.clone(), resolved.runtime));
            let mut stored = live.stored.clone();
            stored.last_checked = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
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
            (old, runtime_update, script_update, held)
        };
        if !install || held || (!runtime_update && !script_update) {
            return Ok(());
        }
        // Runtime is fetched and installed first when both components have updates.
        let runtime_dir = if runtime_update {
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
        let script_dir = if script_update {
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
        let mut candidate = invocation(&self.config, &old);
        if let Some(path) = &script_dir {
            candidate.script = path.join("yt-dlp");
        }
        let version = if script_update {
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
        let python_version = if runtime_update {
            health(&candidate, version).await
        } else {
            existing_health
        };
        let python_version = match python_version {
            Ok(version) => version,
            Err(details) => {
                let mut live = self.lock();
                if !is_current(&live, id) {
                    return Ok(());
                }
                let mut stored = live.stored.clone();
                stored.compatibility_hold = Some((resolved.yt_dlp.clone(), resolved.runtime));
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
        // Recheck under the acquisition lock after every wake: a new Download may have started.
        loop {
            let notified = self.idle.notified();
            {
                let mut live = self.lock();
                if !is_current(&live, id) {
                    return Ok(());
                }
                if live.stored.installed.as_ref() != Some(&old) {
                    return Err(download_error(
                        "The selected pair changed while preparing an update.",
                    ));
                }
                if live.active_downloads == 0 {
                    let installed = InstalledPair {
                        runtime: if runtime_update {
                            resolved.runtime
                        } else {
                            old.runtime
                        },
                        yt_dlp: version.clone(),
                        python_version,
                    };
                    let selected = invocation(&self.config, &installed);
                    if let Some(path) = &runtime_dir {
                        place(
                            path,
                            selected.interpreter.parent().unwrap().parent().unwrap(),
                        )
                        .map_err(download_error)?;
                    }
                    if let Some(path) = &script_dir {
                        place(path, selected.script.parent().unwrap()).map_err(download_error)?;
                    }
                    let mut stored = live.stored.clone();
                    if runtime_update && stored.rollback_pair.is_none() {
                        stored.rollback_pair = Some(old.clone());
                        stored.previous_runtime = Some(old.runtime);
                    }
                    if script_update {
                        stored.previous_yt_dlp = Some(old.yt_dlp.clone());
                    }
                    stored.installed = Some(installed.clone());
                    stored.compatibility_hold = None;
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
        if failure.reason != FailureReason::Verification {
            return Ok(());
        }
        let mut live = self.lock();
        if !is_current(&live, id) {
            return Ok(());
        }
        let mut stored = live.stored.clone();
        match component {
            Component::Python => stored.skipped_runtime = Some(resolved.runtime),
            Component::YtDlp => stored.skipped_yt_dlp = Some(resolved.yt_dlp.clone()),
        }
        persist(&self.config, &stored).map_err(download_error)?;
        live.stored = stored;
        Ok(())
    }
}
