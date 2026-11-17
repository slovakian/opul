use crate::backend::DownloadAction;
use crate::components::{button, folder_icon, input, progress};
use crate::model::State;
use crate::theme::{BODY, CAPTION, Look, NAME, Theme};
use opul_core::{MachineDownload, MachineDownloadState, MachineSessionState};
use rune::{Cx, El, ScrollSpec, col, row, text};

pub fn view(state: &State, cx: &mut Cx<State>) -> El {
    cx.reduced_motion(state.reduced_motion || cx.system_reduced_motion());
    let theme = Theme::new(cx.env().system_dark, state.look);
    cx.foreground(theme.text);
    cx.focus_ring(2.0, 2.0, theme.focus);
    let busy = state.busy();
    let workspace = state.workspace();
    let mut sidebar = col()
        .key("workspace")
        .w(232.0)
        .h_full()
        .padding(20.0)
        .pt(38.0)
        .gap(12.0)
        .fill(theme.surface)
        .child(
            text("Opul")
                .font(rune::TextStyle::new(15.0, 500.0))
                .color(theme.text),
        )
        .child(
            text(workspace.map_or("Opening library…", |w| &w.name))
                .font(NAME)
                .color(theme.text),
        );
    if let Some(workspace) = workspace {
        for folder in &workspace.folders {
            let id = folder.id.clone();
            let selected = state.selected_folder.as_ref() == Some(&id);
            sidebar = sidebar.child(
                button(
                    &format!("folder-{id}"),
                    &folder.name,
                    !busy,
                    &theme,
                    move |s: &mut State, _| s.selected_folder = Some(id.clone()),
                )
                .w_full()
                .fill(if selected {
                    theme.control
                } else {
                    theme.surface
                }),
            );
        }
    }
    sidebar = sidebar.child(button(
        "choose-folder",
        "Add Saved Folder…",
        !busy,
        &theme,
        |s: &mut State, tx| s.choose_folder(tx),
    ));
    let destination = workspace.and_then(|w| {
        w.folders
            .iter()
            .find(|f| state.selected_folder.as_ref() == Some(&f.id))
    });
    let mut content = col()
        .key("library")
        .grow(1.0)
        .min_w(0.0)
        .padding(32.0)
        .gap(16.0)
        .child(
            text("Downloads")
                .font(rune::TextStyle::new(22.0, 500.0))
                .color(theme.text),
        )
        .child(
            row().gap(8.0).center_y().child(folder_icon(&theme)).child(
                text(destination.map_or("Choose a destination", |f| &f.path))
                    .key("destination")
                    .font(CAPTION)
                    .color(theme.muted)
                    .truncate(),
            ),
        )
        .child(
            row()
                .w_full()
                .gap(8.0)
                .center_y()
                .child(input(&state.url, !busy, &theme))
                .child(button(
                    "submit-download",
                    "Download",
                    !busy && !state.url.text.trim().is_empty(),
                    &theme,
                    |s: &mut State, _| s.submit(),
                )),
        );
    if let Some(error) = &state.error {
        content = content.child(
            text(error)
                .key("command-error")
                .role("alert")
                .font(BODY)
                .color(theme.danger),
        );
    }
    if let Some(snapshot) = &state.snapshot {
        content = content.child(
            text(match snapshot.session {
                MachineSessionState::Booting => "Opening library…",
                MachineSessionState::Reconciling => "Restoring downloads…",
                MachineSessionState::Running => "Ready",
                MachineSessionState::Quiescing => "Stopping downloads…",
                MachineSessionState::Stopped => "Downloads stopped",
                MachineSessionState::RecoveryRequired => "Library recovery required",
                MachineSessionState::ReadOnly => "Library is read-only",
            })
            .key("session-status")
            .role("status")
            .font(CAPTION)
            .color(theme.muted),
        );
        for notice in &snapshot.recovery_notices {
            content = content.child(text(notice).font(CAPTION).color(theme.muted));
        }
        if snapshot.session == MachineSessionState::RecoveryRequired {
            content = content.child(button(
                "recover",
                "Recover library",
                !state.quitting,
                &theme,
                |s: &mut State, _| s.recover(),
            ));
        }
    }
    let mut downloads = col().key("downloads").w_full().gap(state.look.gap());
    if let Some(workspace) = workspace {
        for download in &workspace.downloads {
            downloads = downloads.child(download_row(download, busy, &theme));
        }
        if workspace.downloads.is_empty() {
            downloads = downloads.child(
                text("Paste a direct file link to start a Download.")
                    .key("empty-library")
                    .font(BODY)
                    .color(theme.muted),
            );
        }
    }
    content = content.child(
        col()
            .grow(1.0)
            .min_h(0.0)
            .w_full()
            .scroll(ScrollSpec {
                stick_bottom: false,
            })
            .child(downloads),
    );
    content = content.child(
        row()
            .w_full()
            .gap(8.0)
            .child(button(
                "look",
                state.look.name(),
                true,
                &theme,
                |s: &mut State, _| s.look = s.look.next(),
            ))
            .child(button(
                "reduced-motion",
                if state.reduced_motion {
                    "Reduced motion: on"
                } else {
                    "Reduced motion: off"
                },
                true,
                &theme,
                |s: &mut State, _| s.reduced_motion = !s.reduced_motion,
            )),
    );
    row()
        .key("app")
        .w_full()
        .h_full()
        .fill(theme.page)
        .child(sidebar)
        .child(content)
}

fn download_row(download: &MachineDownload, busy: bool, theme: &Theme) -> El {
    let id = &download.id;
    let status = status(download);
    let mut actions = row().gap(6.0).center_y();
    let capabilities = &download.capabilities;
    for (allowed, action, label) in [
        (capabilities.can_pause, DownloadAction::Pause, "Pause"),
        (capabilities.can_resume, DownloadAction::Resume, "Resume"),
        (capabilities.can_cancel, DownloadAction::Cancel, "Cancel"),
        (capabilities.can_retry, DownloadAction::Retry, "Retry"),
        (
            capabilities.can_remove,
            DownloadAction::Remove,
            "Remove from list",
        ),
    ] {
        if allowed {
            let id = id.clone();
            actions = actions.child(button(
                &format!(
                    "{}-{id}",
                    label.split_whitespace().next().unwrap().to_lowercase()
                ),
                label,
                !busy,
                theme,
                move |s: &mut State, _| s.control(id.clone(), action),
            ));
        }
    }
    let details = col()
        .grow(1.0)
        .min_w(0.0)
        .gap(4.0)
        .child(
            text(&download.name)
                .key(format!("name-{id}"))
                .font(NAME)
                .color(theme.text)
                .truncate(),
        )
        .child(
            text(status)
                .key(format!("status-{id}"))
                .role("status")
                .font(CAPTION)
                .color(theme.muted)
                .truncate(),
        );
    let mut element = row()
        .key(format!("download-{id}"))
        .w_full()
        .min_h(theme.look.row_height())
        .motion(rune::Spring::new(0.25, 0.0))
        .px(16.0)
        .gap(16.0)
        .center_y()
        .radius(theme.look.radius())
        .fill(theme.surface)
        .border(1.0, theme.border)
        .child(details)
        .child(progress(
            &format!("progress-{id}"),
            (download.total_size > 0).then_some(download.progress_millionths as f32 / 1_000_000.0),
            theme,
        ))
        .child(actions);
    if matches!(theme.look, Look::SoftEdge) {
        element = element.shadow(
            1.0,
            2.0,
            rune::Color::BLACK.alpha(if theme.dark { 0.12 } else { 0.04 }),
        );
    }
    element
}

fn status(download: &MachineDownload) -> String {
    use MachineDownloadState::*;
    let label = match download.state {
        WaitingForVideoTools => "Waiting for video tools",
        Queued => "Queued",
        Starting => "Starting",
        Fetching => "Downloading",
        Processing => "Processing",
        Verifying => "Verifying",
        Seeding => "Seeding",
        NotSeeding => "Not seeding",
        CheckingFiles => "Checking files",
        Pausing => "Pausing",
        Paused => "Paused",
        Finished => "Completed",
        Failed => "Failed",
        Cancelled => "Cancelled",
    };
    if let Some(error) = &download.error_message {
        return format!("{label} · {error}");
    }
    if download.state == Finished {
        return format!(
            "{label} · {}",
            download
                .output_path
                .as_deref()
                .unwrap_or("Output unavailable")
        );
    }
    if download.total_size > 0 {
        format!(
            "{label} · {}% · {} / {} bytes · {} B/s",
            download.progress_millionths / 10_000,
            download.bytes_downloaded,
            download.total_size,
            download.download_rate
        )
    } else {
        format!(
            "{label} · {} bytes · {} B/s",
            download.bytes_downloaded, download.download_rate
        )
    }
}
