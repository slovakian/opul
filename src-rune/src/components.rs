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
        .child(
            text(label)
                .font(BODY)
                .color(theme.text)
                .truncate()
                .a11y_hidden(),
        );
    if enabled {
        let action = std::rc::Rc::new(action);
        let keyboard = action.clone();
        element
            .hover(theme.text.alpha(0.06))
            .on_press_tx(move |s, tx| action(s, tx))
            .on_key(move |s, key, tx| {
                if key.key != rune::Key::Enter || key.mods != rune::Mods::default() {
                    return false;
                }
                if !tx.key_repeat() {
                    keyboard(s, tx);
                }
                true
            })
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

/// Independent disclosure target: Enter/Space toggle; Left/Right close/open.
pub fn disclosure(key: &str, label: &str, open: bool, theme: &Theme, id: String) -> El {
    let keyboard_id = id.clone();
    button(
        key,
        if open { "⌄" } else { "›" },
        true,
        theme,
        move |s: &mut crate::model::State, _| s.toggle_expansion(id.clone()),
    )
    .label(label)
    .expanded(open)
    .w(24.0)
    .px(0.0)
    .group_opacity(0.0, 1.0)
    .on_key(move |s: &mut crate::model::State, key, tx| {
        if key.mods != rune::Mods::default() {
            return false;
        }
        let expanded = match key.key {
            rune::Key::Left => false,
            rune::Key::Right => true,
            rune::Key::Enter => {
                if !tx.key_repeat() {
                    s.toggle_expansion(keyboard_id.clone());
                }
                return true;
            }
            _ => return false,
        };
        if s.expansion.expanded(&keyboard_id) != expanded {
            s.toggle_expansion(keyboard_id.clone());
        }
        true
    })
}

/// Reduced owned menu surface, adapted from Rune-kit's menu.rs. Rune owns roving
/// keys, focus containment, outside/Escape dismissal and trigger restoration.
pub fn menu(key: &str, label: &str, trigger: &str, theme: &Theme, items: Vec<El>) -> El {
    rune::col()
        .key(key)
        .role("menu")
        .label(label)
        .focus_group()
        .focus_return(trigger)
        .modal(true, |s: &mut crate::model::State| s.workspace_menu = None)
        .anchored_fit(trigger, rune::Side::Bottom, rune::Align::End, 6.0)
        .w(210.0)
        .padding(6.0)
        .gap(4.0)
        .radius(theme.look.radius())
        .fill(theme.surface)
        .border(1.0, theme.border)
        .shadow(4.0, 12.0, rune::Color::BLACK.alpha(0.15))
        .children(items)
}
