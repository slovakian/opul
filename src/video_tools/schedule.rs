use super::manager::{LiveState, runtime};
use super::updates::UpdateMode;
use super::*;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

impl VideoTools {
    pub(super) fn set_keep_up_to_date(
        self: &Arc<Self>,
        live: &mut LiveState,
        enabled: bool,
    ) -> Result<(), String> {
        live.commit(&self.config, |stored| stored.keep_up_to_date = enabled)?;
        live.snapshot.keep_up_to_date = enabled;
        if !enabled
            && live.automatic_update
            && matches!(
                live.snapshot.state,
                VideoToolsState::Ready {
                    update_flag: UpdateFlag::Updating { .. },
                    ..
                }
            )
        {
            live.stop_job();
            let flag = live.stored.update_flag();
            live.set_update_flag(flag);
        }
        if enabled && live.stored.available_update.is_some() {
            self.start_update(live, UpdateMode::Automatic);
        }
        Ok(())
    }

    pub(super) fn start_schedule(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        let mut snapshots = self.subscribe();
        let environment = Arc::clone(&self.sources.environment);
        let mut due = environment.now() + Duration::from_secs(60);
        let task = runtime().spawn(async move {
            loop {
                let ready = matches!(
                    snapshots.borrow_and_update().state,
                    VideoToolsState::Ready { .. }
                );
                if !ready {
                    if snapshots.changed().await.is_err() {
                        break;
                    }
                    continue;
                }
                tokio::select! {
                    result = snapshots.changed() => { if result.is_err() { break; } }
                    () = environment.sleep(due.saturating_sub(environment.now())) => {
                        due = environment.now() + Duration::from_secs(86_400);
                        let Some(tools) = weak.upgrade() else { break; };
                        let mut live = tools.lock();
                        tools.start_update(&mut live, UpdateMode::Automatic);
                    }
                }
            }
        });
        self.lock().schedule = Some(task.abort_handle());
    }

    pub(super) fn start_update(self: &Arc<Self>, live: &mut LiveState, mode: UpdateMode) {
        if live.removing
            || live.job.is_some()
            || !matches!(live.snapshot.state, VideoToolsState::Ready { .. })
        {
            return;
        }
        live.candidate = None;
        live.automatic_update = matches!(mode, UpdateMode::Automatic);
        let id = Uuid::new_v4();
        let tools = Arc::clone(self);
        let task = runtime().spawn(async move {
            tools.run_update(id, mode).await;
        });
        live.job = Some((id, task.abort_handle()));
    }

    /// Atomically claim a consent notification before showing it. Concurrent consumers and
    /// restarts cannot claim it twice. A failed write leaves it eligible for a later attempt.
    pub fn take_update_notice(&self) -> Result<Option<UpdateNotice>, String> {
        let mut live = self.lock();
        if live.stored.keep_up_to_date
            || !matches!(
                live.snapshot.state,
                VideoToolsState::Ready {
                    update_flag: UpdateFlag::Available { .. },
                    ..
                }
            )
        {
            return Ok(None);
        }
        let Some(notice) = live.stored.available_update.clone() else {
            return Ok(None);
        };
        if live.stored.notified_updates.contains(&notice.identity) {
            return Ok(None);
        }
        live.commit(&self.config, |stored| {
            stored.notified_updates.push(notice.identity.clone());
        })?;
        Ok(Some(notice))
    }
}
