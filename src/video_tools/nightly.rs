use super::manager::{LiveState, cancel, persist};
use super::updates::UpdateMode;
use super::*;
use std::sync::Arc;

impl VideoTools {
    pub(super) fn set_use_nightly(
        self: &Arc<Self>,
        live: &mut LiveState,
        enabled: bool,
    ) -> Result<bool, String> {
        if !self.config.nightly_available {
            return Err("yt-dlp nightly builds are available only in Opul Nightly.".into());
        }
        let channel = if enabled {
            YtDlpChannel::Nightly
        } else {
            YtDlpChannel::Stable
        };
        if live.stored.yt_dlp_channel == channel {
            return Ok(false);
        }
        let mut stored = live.stored.clone();
        stored.yt_dlp_channel = channel;
        stored.compatibility_hold = None;
        stored.skipped_yt_dlp = None;
        stored.available_update = None;
        stored.last_checked = None;
        persist(&self.config, &stored)?;
        cancel(live);
        live.stored = stored;
        live.snapshot.use_nightly = enabled;
        live.snapshot.published_size = None;
        live.candidate = None;
        if let VideoToolsState::Ready {
            update_flag,
            last_checked,
            ..
        } = &mut live.snapshot.state
        {
            *update_flag = UpdateFlag::None;
            *last_checked = None;
            // The switch is explicit consent even with automatic updates disabled.
            self.start_update(live, UpdateMode::Manual);
        } else if matches!(live.snapshot.state, VideoToolsState::SettingUp { .. }) {
            live.snapshot.state = VideoToolsState::NotSetUp;
            return Ok(true);
        }
        Ok(false)
    }
}
