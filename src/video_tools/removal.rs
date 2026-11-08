use super::*;
use std::sync::Arc;

impl VideoTools {
    /// Revoke selection under the activation lock before stopping writers. The caller must
    /// stop its media processes before returning; active leases protect probes still exiting.
    pub fn remove_with(
        self: &Arc<Self>,
        turn_off: bool,
        stop_writers: impl FnOnce() -> Result<(), String>,
    ) -> Result<VideoToolsSnapshot, String> {
        {
            let mut live = self.lock();
            if live.removing {
                return Err("Video tools are being removed.".into());
            }
            live.commit(&self.config, |stored| {
                *stored = stored.preferences_only(turn_off || stored.turned_off);
            })?;
            live.stop_job();
            live.candidate = None;
            live.removing = true;
            live.snapshot.state = live.stored.resting_state();
            self.publish(&mut live);
        }
        let result: Result<(), String> = (|| {
            stop_writers()?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            loop {
                let live = self.lock();
                if live.active_users == 0 {
                    super::recovery::prune(&self.config.root, &live.stored)?;
                    return Ok(());
                }
                if std::time::Instant::now() >= deadline {
                    return Err("Video tools are still in use. Try removing them again.".into());
                }
                drop(live);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        })();
        let mut live = self.lock();
        live.removing = false;
        if let Err(error) = result {
            live.snapshot.state = VideoToolsState::NotWorking {
                reason: FailureReason::StoppedWorking,
                details: error.clone(),
            };
            self.publish(&mut live);
            return Err(error);
        }
        Ok(live.snapshot.clone())
    }
}
