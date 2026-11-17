//! Owned, reduced primitives adapted from Rune-kit at cca5df6. See ../NOTICE.md.
use crate::theme::{BODY, Theme};
use rune::{El, Icon, TextEdit, Tx, el, icon, row, text, text_input};

pub fn button<S: 'static>(
    key: &str,
    label: &str,
    enabled: bool,
    theme: &Theme,
    action: impl Fn(&mut S, &mut Tx) + 'static,
) -> El {
    let element = row()
        .key(key)
        .role("button")
        .label(label)
        .h(28.0)
        .px(11.0)
        .center()
        .radius(theme.look.radius().min(8.0))
        .fill(theme.control)
        .border(1.0, theme.border)
        .focus_ring(2.0, 2.0, theme.focus)
        .focusable(enabled)
        .disabled(!enabled)
        .child(text(label).font(BODY).color(theme.text).a11y_hidden());
    if enabled {
        element.hover(theme.text.alpha(0.06)).on_press_tx(action)
    } else {
        element.layer_opacity(0.45)
    }
}

pub fn input(edit: &TextEdit, enabled: bool, theme: &Theme) -> El {
    let element = text_input(edit, BODY)
        .key("direct-url")
        .label("Direct file link")
        .single_line()
        .grow(1.0)
        .min_w(80.0)
        .h(32.0)
        .padding(7.0)
        .placeholder("https://example.com/file.mp4", theme.muted)
        .color(theme.text)
        .fill(theme.control)
        .radius(6.0)
        .border(1.0, theme.border)
        .focus_ring(2.0, 2.0, theme.focus)
        .disabled(!enabled);
    if enabled {
        element
            .on_edit(|s: &mut crate::model::State| &mut s.url)
            .on_submit(|s: &mut crate::model::State, _| s.submit())
    } else {
        element
    }
}

pub fn progress(key: &str, value: Option<f32>, theme: &Theme) -> El {
    let mut track = row()
        .key(key)
        .role("progressbar")
        .label("Download progress")
        .w(120.0)
        .h(4.0)
        .radius(2.0)
        .fill(theme.control)
        .clip_soft(0.0);
    if let Some(value) = value {
        let value = value.clamp(0.0, 1.0);
        track = track.range_value(f64::from(value), 0.0, 1.0).child(
            el().key("fill")
                .w_frac(value)
                .h_full()
                .fill(theme.text)
                .a11y_hidden(),
        );
    }
    // Unknown lengths stay indeterminate with no invented percentage or animated fake progress.
    track
}

static FOLDER: Icon = Icon {
    name: "saved-folder",
    view: 24.0,
    body: r#"<g fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round"><path d="M3 6h7l2 3h9v11H3Z"/></g>"#,
};

pub fn folder_icon(theme: &Theme) -> El {
    icon(&FOLDER, 20.0).color(theme.muted).a11y_hidden()
}
