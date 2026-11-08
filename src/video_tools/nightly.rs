use super::manager::LiveState;
use super::updates::UpdateMode;
use super::*;
use std::sync::Arc;

impl VideoTools {
    pub(super) fn set_use_nightly(
        self: &Arc<Self>,
        live: &mut LiveState,
        enabled: bool,
    ) -> Result<(), String> {
        if !self.config.nightly_available {
            return Err("yt-dlp nightly builds are available only in Opul Nightly.".into());
        }
        let channel = if enabled {
            YtDlpChannel::Nightly
        } else {
            YtDlpChannel::Stable
        };
        if live.stored.yt_dlp_channel == channel {
            return Ok(());
        }
        live.commit(&self.config, |stored| {
            stored.yt_dlp_channel = channel;
            stored.compatibility_hold = None;
            stored.skipped_yt_dlp = None;
            stored.available_update = None;
            stored.last_checked = None;
        })?;
        live.stop_job();
        live.snapshot.use_nightly = enabled;
        live.snapshot.published_size = None;
        live.candidate = None;
        match &mut live.snapshot.state {
            VideoToolsState::Ready {
                update_flag,
                last_checked,
                ..
            } => {
                *update_flag = UpdateFlag::None;
                *last_checked = None;
                // The switch is explicit consent even with automatic updates disabled.
                self.start_update(live, UpdateMode::Manual);
            }
            VideoToolsState::SettingUp { .. } => {
                live.snapshot.state = live.stored.resting_state();
                self.start_setup(live)?;
            }
            _ => {}
        }
        Ok(())
    }
}
