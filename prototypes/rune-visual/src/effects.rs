use super::*;
use crate::ui::caption;
pub fn progress(key: &str, value: Option<f32>, active: bool, reduced: bool, k: &Kit) -> El {
    let mut track = row()
        .key(key)
        .w_full()
        .h(4.0)
        .radius(2.0)
        .role("progressbar")
        .label("Download progress")
        .fill(k.c.text_base.alpha(0.08))
        .clip_soft(0.0);
    if let Some(value) = value {
        track = track.range_value(f64::from(value), 0.0, 1.0).child(
            el().key(format!("{key}-fill"))
                .w_frac(value)
                .h_full()
                .fill(k.c.text_base),
        );
    }
    let mut empty = el()
        .key(format!("{key}-empty"))
        .grow(1.0)
        .h_full()
        .clip_soft(0.0);
    if active && !reduced {
        let (period, from, to) = if value.is_some() {
            (6.0, [1.0, 0.0], [-1.0, 0.0])
        } else {
            (2.6, [-1.0, 0.0], [1.0, 0.0])
        };
        empty = empty.child(
            el().key(format!("{key}-sheen"))
                .w_full()
                .h_full()
                .linear_gradient(
                    std::f32::consts::FRAC_PI_2,
                    [
                        (0.0, Color::TRANSPARENT),
                        (0.25, k.c.text_base.alpha(0.07)),
                        (0.5, k.c.text_base.alpha(0.14)),
                        (0.75, k.c.text_base.alpha(0.07)),
                        (1.0, Color::TRANSPARENT),
                    ],
                )
                .looping(LoopSpec::slide(period, from, to)),
        );
    }
    track.child(empty.a11y_hidden())
}

pub fn seeding(reduced: bool, k: &Kit) -> El {
    let label = "Seeding · ↑ 1.2 MB/s";
    let mut root = el()
        .key("seeding")
        .role("status")
        .label(label)
        .h(20.0)
        .w(144.0)
        .child(caption(label, k).absolute(0.0, 0.0).a11y_hidden());
    if !reduced {
        // Counter-moving copy stays registered with the base glyphs as its mask rises.
        root = root.child(
            el().key("upload-mask")
                .absolute(0.0, 0.0)
                .w_full()
                .h(12.0)
                .clip_fade(0.0, 6.0, 0.0, 6.0)
                .looping(LoopSpec::slide(4.0, [0.0, 2.0], [0.0, -1.0]).easing(rune::Easing::LINEAR))
                .child(
                    el().key("upload-copy")
                        .w_full()
                        .h(12.0)
                        .looping(
                            LoopSpec::slide(4.0, [0.0, -2.0], [0.0, 1.0])
                                .easing(rune::Easing::LINEAR),
                        )
                        .child(
                            text(label)
                                .font(k.ty.caption())
                                .color(k.c.text_base)
                                .absolute(0.0, 0.0),
                        ),
                ),
        );
    }
    root
}
