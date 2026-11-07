use super::manager::{StoredState, cancel, persist};
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
            let stored = StoredState {
                turned_off: turn_off || live.stored.turned_off,
                keep_up_to_date: live.stored.keep_up_to_date,
                notified_updates: live.stored.notified_updates.clone(),
                ..StoredState::default()
            };
            persist(&self.config, &stored)?;
            cancel(&mut live);
            live.candidate = None;
            live.stored = stored;
            live.removing = true;
            live.snapshot.state = if live.stored.turned_off {
                VideoToolsState::TurnedOff
            } else {
                VideoToolsState::NotSetUp
            };
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
