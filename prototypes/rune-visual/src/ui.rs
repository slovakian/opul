//! Disposable composed Opul shell. The visual study is below the application, not in its rows.
use super::*;
mod controls;
mod download;
mod sheet;
mod sidebar;
use crate::{effects, icons as i};
use controls::switcher;
use download::{download, toggle_transfer};
use rune::{Icon, icon};
use rune_kit::{IconButton, IconButtonVariant, TextInput};
use sheet::sheet;
use sidebar::sidebar;

pub fn caption(value: impl Into<String>, k: &Kit) -> El {
    text(value).font(k.ty.caption()).color(k.c.text_muted)
}

fn icon_button(
    glyph: &'static Icon,
    label: &str,
    key: &str,
    k: &Kit,
    action: impl Fn(&mut Preview) + 'static,
) -> El {
    IconButton::new(glyph)
        .label(label)
        .variant(IconButtonVariant::Ghost)
        .control_size(ControlSize::Large)
        .on_press(action)
        .build(k)
        .key(key)
}

fn control(label: &str, key: &str, k: &Kit, action: impl Fn(&mut Preview) + 'static) -> El {
    Button::new(label)
        .control_size(ControlSize::Regular)
        .variant(ButtonVariant::GhostMuted)
        .on_press(action)
        .build(k)
        .key(key)
}

fn rule(k: &Kit) -> El {
    el().w_full().h(1.0).fill(k.c.border_base).a11y_hidden()
}

fn toolbar(s: &Preview, k: &Kit, narrow: bool) -> El {
    row()
        .key("toolbar")
        .h(60.0)
        .w_full()
        .px(18.0)
        .pl(if narrow || s.sidebar_hidden {
            94.0
        } else {
            18.0
        })
        .gap(12.0)
        .center_y()
        .window_drag()
        .child(icon_button(
            &i::PANEL,
            "Toggle workspaces",
            "show-sidebar",
            k,
            move |s| {
                if narrow {
                    s.sidebar = !s.sidebar;
                } else {
                    s.sidebar_hidden = !s.sidebar_hidden;
                }
            },
        ))
        .children((!narrow).then(|| {
            text(if s.selected == "All Downloads" {
                "All downloads"
            } else {
                s.selected
            })
            .font(k.ty.ui_medium)
            .color(k.c.text_base)
            .max_w(150.0)
            .truncate()
        }))
        .child(
            TextInput::new(&s.search)
                .label("Search fixture downloads")
                .placeholder("Search or paste a link…")
                .leading_icon(&i::SEARCH)
                .width(None)
                .height(34.0)
                .on_edit(|s: &mut Preview| &mut s.search)
                .build(k)
                .key("search")
                .grow(1.0)
                .min_w(100.0),
        )
        .children((!narrow).then(|| {
            row()
                .gap(5.0)
                .center_y()
                .child(icon(&i::FILTER, 18.0).color(k.c.text_muted))
                .child(el().w(10.0))
                .child(icon(&i::SORT, 17.0).color(k.c.text_muted))
                .child(el().w(10.0))
                .child(icon_button(
                    &i::LIST,
                    "Toggle card and minimal presentation",
                    "toolbar-presentation",
                    k,
                    |s| s.minimal = !s.minimal,
                ))
        }))
}

pub fn view(s: &Preview, cx: &mut Cx<Preview>) -> El {
    let s = &Preview {
        reduced: s.reduced || cx.system_reduced_motion(),
        ..s.clone()
    };
    let dark = match s.appearance {
        Appearance::System => cx.system_dark(),
        Appearance::Light => false,
        Appearance::Dark => true,
    };
    let k = s.treatment.kit(dark);
    cx.reduced_motion(s.reduced);
    cx.foreground(k.c.text_base);
    let narrow = cx.layout_with("root", |r| r[2] < 820.0).unwrap_or(false);
    let width = cx.layout_size("root").map_or(1180.0, |r| r[0]);
    let mut list = col().key("download-list").w_full().gap(
        if s.minimal || matches!(s.treatment, Treatment::List | Treatment::Panels) {
            0.0
        } else {
            8.0
        },
    );
    if matches!(s.treatment, Treatment::Panels) && !s.minimal {
        list = list
            .fill(k.c.layer_01)
            .border(1.0, k.c.border_base)
            .radius(12.0)
            .clip_rounded(12.0);
    }
    let query = s.search.text.to_lowercase();
    let mut count = 0;
    for (index, filename) in FILES.iter().enumerate() {
        if filename.to_lowercase().contains(&query) {
            list = list.child(download(s, index, &k, narrow));
            count += 1;
        }
    }
    let content = col()
        .key("content")
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .child(toolbar(s, &k, narrow))
        .child(
            col()
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .padding(if narrow { 20.0 } else { 32.0 })
                .pt(28.0)
                .child(list)
                .child(caption(format!("{count} downloads"), &k).pt(22.0).pl(12.0)),
        );
    let body = row()
        .key("app-layout")
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .fill(k.c.page)
        .children((!narrow && !s.sidebar_hidden).then(|| sidebar(s, &k)))
        .child(content);
    let app = col()
        .w_full()
        .h_full()
        .child(body)
        .child(switcher(s, &k, narrow));
    let mut root = el()
        .key("root")
        .w_full()
        .h_full()
        .fill(k.c.page)
        .clip_soft(0.0)
        .child(if s.sheet.is_some() {
            app.key("application").blur(3.0)
        } else {
            app.key("application")
        });
    if narrow && s.sidebar && s.sheet.is_none() {
        root = root
            .child(
                el().absolute(0.0, 0.0)
                    .w_full()
                    .h_full()
                    .fill(Color::BLACK.alpha(0.12)),
            )
            .child(
                sidebar(s, &k)
                    .key("narrow-sidebar")
                    .absolute(0.0, 0.0)
                    .modal(true, |s: &mut Preview| s.sidebar = false),
            );
    }
    if let Some(index) = s.sheet {
        root = root
            .child(
                el().key("sheet-scrim")
                    .absolute(0.0, 0.0)
                    .w_full()
                    .h_full()
                    .fill(Color::BLACK.alpha(if dark { 0.24 } else { 0.06 }))
                    .a11y_hidden(),
            )
            .child(sheet(s, index, (width - 40.0).min(360.0), &k));
    }
    root
}
