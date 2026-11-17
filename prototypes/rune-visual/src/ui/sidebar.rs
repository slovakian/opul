use super::*;

fn navigation(
    s: &Preview,
    label: &'static str,
    key: &str,
    glyph: &'static Icon,
    count: &str,
    k: &Kit,
) -> El {
    let selected = s.selected == label;
    row()
        .key(key)
        .h(34.0)
        .w_full()
        .px(10.0)
        .gap(10.0)
        .center_y()
        .role("button")
        .label(label)
        .focusable(true)
        .focus_ring(2.0, 2.0, k.c.border_focus)
        .radius(7.0)
        .fill(if selected {
            k.c.layer_03
        } else {
            Color::TRANSPARENT
        })
        .hover(k.c.hover)
        .on_press(move |s: &mut Preview| {
            s.selected = label;
            s.sidebar = false;
        })
        .child(icon(glyph, 17.0).color(k.c.text_base).a11y_hidden())
        .child(
            text(label)
                .font(if selected { k.ty.ui_medium } else { k.ty.ui })
                .color(k.c.text_base)
                .grow(1.0),
        )
        .child(caption(count, k))
}

fn workspace(s: &Preview, label: &'static str, key: &str, k: &Kit) -> El {
    let selected = s.selected == label;
    row()
        .key(key)
        .h(34.0)
        .w_full()
        .px(10.0)
        .gap(10.0)
        .center_y()
        .role("button")
        .label(label)
        .focusable(true)
        .focus_ring(2.0, 2.0, k.c.border_focus)
        .radius(7.0)
        .fill(if selected {
            k.c.layer_03
        } else {
            Color::TRANSPARENT
        })
        .hover(k.c.hover)
        .on_press(move |s: &mut Preview| {
            s.selected = label;
            s.sidebar = false;
        })
        .child(
            icon(&i::FOLDER, 17.0)
                .color(rune::oklch(
                    if k.dark { 0.76 } else { 0.55 },
                    0.13,
                    if label == "Personal" { 65.0 } else { 245.0 },
                ))
                .a11y_hidden(),
        )
        .child(text(label).font(k.ty.ui).color(k.c.text_base).grow(1.0))
}

pub(super) fn sidebar(s: &Preview, k: &Kit) -> El {
    let mut studio = row()
        .w_full()
        .center_y()
        .child(workspace(s, "Studio", "workspace-studio", k).grow(1.0));
    if s.multiple {
        studio = studio.child(
            icon_button(
                if s.expanded {
                    &i::CHEVRON_DOWN
                } else {
                    &i::CHEVRON
                },
                "Expand or collapse Studio folders",
                "disclosure",
                k,
                |s| s.expanded = !s.expanded,
            )
            .expanded(s.expanded),
        );
    }
    let mut folders = col().gap(2.0).child(studio);
    if s.multiple && s.expanded {
        folders = folders.child(
            col()
                .pl(27.0)
                .gap(2.0)
                .child(
                    row()
                        .center_y()
                        .child(workspace(s, "Footage", "folder-footage", k).grow(1.0))
                        .child(caption("Default", k).pr(10.0)),
                )
                .child(workspace(s, "Audio", "folder-audio", k)),
        );
    }
    col()
        .key("sidebar")
        .w(232.0)
        .h_full()
        .fill(k.c.bg_base)
        .border_sides([0.0, 1.0, 0.0, 0.0], k.c.border_base)
        .child(
            row()
                .key("brand")
                .h(60.0)
                .pl(94.0)
                .gap(8.0)
                .center_y()
                .window_drag()
                .child(
                    el().size(23.0, 23.0)
                        .pixels(s.artwork[0].pixels())
                        .a11y_hidden(),
                )
                .child(
                    text("Opul")
                        .font(TextStyle::new(15.0, 550.0))
                        .color(k.c.text_base),
                ),
        )
        .child(
            col()
                .padding(10.0)
                .gap(3.0)
                .child(navigation(s, "All Downloads", "nav-all", &i::INBOX, "5", k))
                .child(navigation(
                    s,
                    "In progress",
                    "nav-progress",
                    &i::DOWN,
                    "3",
                    k,
                ))
                .child(navigation(
                    s,
                    "Completed",
                    "nav-completed",
                    &i::CHECK,
                    "1",
                    k,
                )),
        )
        .child(el().w_full().px(16.0).child(rule(k)).mt(10.0).mb(18.0))
        .child(
            row()
                .px(20.0)
                .center_y()
                .justify_between()
                .child(caption("Workspaces", k))
                .child(icon(&i::PLUS, 14.0).color(k.c.text_muted).a11y_hidden()),
        )
        .child(
            col()
                .padding(10.0)
                .gap(3.0)
                .child(workspace(s, "Personal", "workspace-personal", k))
                .child(folders)
                .child(
                    row()
                        .h(34.0)
                        .px(10.0)
                        .gap(10.0)
                        .center_y()
                        .child(icon(&i::PLUS, 16.0).color(k.c.text_muted))
                        .child(caption("New workspace", k))
                        .a11y_hidden(),
                ),
        )
        .child(el().grow(1.0))
        .child(el().w_full().px(16.0).child(rule(k)))
        .child(
            row()
                .padding(20.0)
                .gap(10.0)
                .center_y()
                .child(icon(&i::SETTINGS, 17.0).color(k.c.text_base))
                .child(text("Settings").font(k.ty.ui).color(k.c.text_base))
                .a11y_hidden(),
        )
        .child(
            row()
                .px(20.0)
                .pb(16.0)
                .gap(6.0)
                .center_y()
                .child(icon(&i::DOWN, 12.0).color(k.c.text_muted))
                .child(caption("2.4 MB/s", k))
                .child(el().grow(1.0))
                .child(el().size(5.0, 5.0).radius(2.5).fill(k.c.success.icon))
                .child(caption("Ready", k)),
        )
}
