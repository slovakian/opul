use crate::components::{button, disclosure, folder_icon, menu};
use crate::model::State;
use crate::navigation::Location;
use crate::preferences::Startup;
use crate::theme::{CAPTION, NAME, Theme};
use rune::{El, ScrollSpec, col, row, text};

fn navigation(key: &str, label: &str, location: Location, state: &State, theme: &Theme) -> El {
    let selected = state.location == location;
    button(key, label, true, theme, move |s: &mut State, _| {
        s.navigate(location.clone())
    })
    .w_full()
    .min_w(0.0)
    .fill(if selected {
        theme.control
    } else {
        theme.surface
    })
    .focus_selected(selected)
    .description(if selected { "Current location" } else { "" })
}

pub fn sidebar(state: &State, theme: &Theme) -> El {
    let mut list = col().key("navigation-list").w_full().gap(5.0);
    for (key, label, location) in [
        ("nav-all", "All", Location::All),
        ("nav-progress", "In progress", Location::InProgress),
        ("nav-completed", "Completed", Location::Completed),
    ] {
        list = list.child(navigation(
            key,
            &format!("{label} · {}", state.count(&location)),
            location,
            state,
            theme,
        ));
    }
    list = list.child(text("Workspaces").font(CAPTION).color(theme.muted).mt(16.0));
    if let Some(snapshot) = &state.snapshot {
        for workspace in &snapshot.workspaces {
            let id = &workspace.id;
            let expandable = workspace.folders.len() > 1;
            let expanded = expandable && state.expansion.expanded(id);
            let selected = match &state.location {
                Location::Workspace(selected) => selected == id,
                Location::Folder(selected, _) => selected == id && !expanded,
                _ => false,
            };
            let navigation_id = id.clone();
            let mut heading = row()
                .key(format!("workspace-row-{id}"))
                .focus_return("nav-all")
                .w_full()
                .min_w(0.0)
                .h(30.0)
                .hover_group()
                .radius(6.0)
                .fill(if selected {
                    theme.control
                } else {
                    theme.surface
                });
            // The icon remains in the leading slot; the disclosure replaces it on hover/focus.
            let artwork = if workspace.icon.kind == opul_core::MachineWorkspaceIconKind::Emoji {
                text(&workspace.icon.value).font(NAME).a11y_hidden()
            } else {
                folder_icon(theme)
            };
            let mut leading = row()
                .key(format!("workspace-icon-{id}"))
                .size(24.0, 28.0)
                .center()
                .child(artwork.group_opacity(1.0, if expandable { 0.0 } else { 1.0 }));
            if expandable {
                leading = leading.child(
                    disclosure(
                        &format!("disclosure-{id}"),
                        &format!("Toggle {} folders", workspace.name),
                        expanded,
                        theme,
                        id.clone(),
                    )
                    .absolute(0.0, 0.0),
                );
            } else {
                let id = id.clone();
                leading = leading
                    .on_press(move |s: &mut State| s.navigate(Location::Workspace(id.clone())));
            }
            heading = heading.child(leading).child(
                button(
                    &format!("workspace-{id}"),
                    &format!("{} · {}", workspace.name, workspace.downloads.len()),
                    true,
                    theme,
                    move |s: &mut State, _| s.navigate(Location::Workspace(navigation_id.clone())),
                )
                .label(format!("{} workspace", workspace.name))
                .grow(1.0)
                .min_w(0.0)
                .px(4.0)
                .fill(if selected {
                    theme.control
                } else {
                    theme.surface
                })
                .focus_selected(selected)
                .description(if selected { "Current location" } else { "" }),
            );
            let menu_id = id.clone();
            heading = heading.child(
                button(
                    &format!("workspace-actions-{id}"),
                    "…",
                    true,
                    theme,
                    move |s: &mut State, _| s.workspace_menu = Some(menu_id.clone()),
                )
                .label(format!("{} actions", workspace.name))
                .expanded(state.workspace_menu.as_ref() == Some(id))
                .w(24.0)
                .px(0.0),
            );
            list = list.child(heading);
            if expanded {
                let mut folders = col()
                    .key(format!("folders-{id}"))
                    .focus_return(format!("workspace-{id}"))
                    .w_full()
                    .pl(18.0)
                    .gap(4.0);
                for folder in &workspace.folders {
                    let location = Location::Folder(id.clone(), folder.id.clone());
                    let label = format!(
                        "{}{} · {}",
                        folder.name,
                        if folder.is_default { " (Default)" } else { "" },
                        state.count(&location)
                    );
                    folders = folders.child(
                        navigation(
                            &format!("folder-{}", folder.id),
                            &label,
                            location,
                            state,
                            theme,
                        )
                        .focus_return(format!("workspace-{id}")),
                    );
                }
                list = list.child(folders);
            }
        }
    }
    let mut panel = col()
        .key("workspace")
        .role("navigation")
        .label("Library navigation")
        .w(232.0)
        .h_full()
        .padding(16.0)
        .pt(38.0)
        .gap(10.0)
        .fill(theme.surface)
        .child(text("Opul").font(NAME).color(theme.text))
        .child(
            col()
                .grow(1.0)
                .min_h(0.0)
                .w_full()
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(list),
        )
        .child(button(
            "choose-folder",
            "Add Saved Folder…",
            !state.busy() && state.workspace().is_some(),
            theme,
            |s: &mut State, tx| s.choose_folder(tx),
        ));
    let remember = state.expansion.desired.workspace_expansion_startup == Startup::Remember;
    panel = panel.child(button(
        "expansion-startup",
        if remember {
            "Startup: Remember"
        } else {
            "Startup: Collapsed"
        },
        true,
        theme,
        |s: &mut State, _| {
            s.set_startup(
                if s.expansion.desired.workspace_expansion_startup == Startup::Remember {
                    Startup::Collapsed
                } else {
                    Startup::Remember
                },
            )
        },
    ));
    if let Some(error) = &state.expansion.error {
        panel = panel
            .child(
                text(error)
                    .key("expansion-error")
                    .role("alert")
                    .font(CAPTION)
                    .color(theme.danger),
            )
            .child(button(
                "retry-expansion",
                "Retry navigation preferences",
                true,
                theme,
                |s: &mut State, _| s.retry_expansion(),
            ));
    }
    panel
}

pub fn workspace_menu(state: &State, theme: &Theme) -> Option<El> {
    let id = state.workspace_menu.as_ref()?;
    let workspace = state
        .snapshot
        .as_ref()?
        .workspaces
        .iter()
        .find(|w| &w.id == id)?;
    let workspace_id = id.clone();
    let item = button(
        "menu-add-folder",
        "Add Saved Folder…",
        !state.busy(),
        theme,
        move |s: &mut State, tx| {
            s.workspace_menu = None;
            s.choose_folder_for(workspace_id.clone(), tx);
        },
    )
    .role("menuitem")
    .w_full();
    Some(menu(
        "workspace-menu",
        &format!("{} actions", workspace.name),
        &format!("workspace-actions-{id}"),
        theme,
        vec![item],
    ))
}
