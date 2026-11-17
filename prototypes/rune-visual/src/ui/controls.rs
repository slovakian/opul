use super::*;

pub(super) fn switcher(s: &Preview, k: &Kit, narrow: bool) -> El {
    let appearance = match s.appearance {
        Appearance::System => "System",
        Appearance::Light => "Light",
        Appearance::Dark => "Dark",
    };
    let variants = row()
        .center_y()
        .gap(10.0)
        .child(caption("VISUAL STUDY", k).font(TextStyle {
            tracking: 1.0,
            ..TextStyle::new(10.0, 600.0)
        }))
        .child(
            icon_button(&i::CHEVRON, "Previous treatment", "previous", k, |s| {
                s.treatment = s.treatment.cycle(false)
            })
            .rotate(std::f32::consts::PI),
        )
        .child(
            text(s.treatment.name())
                .key("variant")
                .font(k.ty.ui_medium)
                .color(k.c.text_base),
        )
        .child(icon_button(&i::CHEVRON, "Next treatment", "next", k, |s| {
            s.treatment = s.treatment.cycle(true)
        }));
    let controls = row()
        .center_y()
        .gap(4.0)
        .child(control(appearance, "appearance", k, |s| {
            s.appearance = match s.appearance {
                Appearance::System => Appearance::Light,
                Appearance::Light => Appearance::Dark,
                Appearance::Dark => Appearance::System,
            }
        }))
        .child(control(
            if s.reduced { "Reduced" } else { "Motion" },
            "motion",
            k,
            |s| s.reduced = !s.reduced,
        ))
        .child(control(
            if s.minimal { "Minimal" } else { "Cards" },
            "presentation",
            k,
            |s| s.minimal = !s.minimal,
        ))
        .child(control(
            if s.multiple { "2 folders" } else { "1 folder" },
            "folders",
            k,
            |s| s.multiple = !s.multiple,
        ))
        .child(control("State", "transfer-state", k, |s| {
            s.transfer = match s.transfer {
                Transfer::Downloading => Transfer::Paused,
                Transfer::Paused => Transfer::Completed,
                Transfer::Completed => Transfer::Failed,
                Transfer::Failed => Transfer::Downloading,
            }
        }));
    let top = if narrow {
        col()
            .gap(6.0)
            .items_center()
            .child(variants)
            .child(controls)
    } else {
        row()
            .center_y()
            .justify_between()
            .child(variants)
            .child(controls)
    };
    col()
        .key("prototype-controls")
        .w_full()
        .padding(12.0)
        .px(22.0)
        .gap(6.0)
        .fill(k.c.bg_base)
        .border_sides([1.0, 0.0, 0.0, 0.0], k.c.border_base)
        .child(top)
        .child(
            caption(
                format!(
                    "{}  ·  In-memory prototype #101  ·  {}",
                    s.treatment.description(),
                    s.transfer.name()
                ),
                k,
            )
            .key("fixture-state")
            .truncate(),
        )
}
