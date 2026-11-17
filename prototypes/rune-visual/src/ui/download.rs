use super::*;

pub(super) fn download(s: &Preview, index: usize, k: &Kit, narrow: bool) -> El {
    let description = match index {
        0 => match s.transfer {
            Transfer::Downloading => "256 MB of 610 MB  ·  14s remaining",
            Transfer::Paused => "256 MB of 610 MB  ·  Paused",
            Transfer::Completed => "vimeo.com  ·  610 MB",
            Transfer::Failed => "Connection lost  ·  Try again",
        },
        1 => "Looking for peers  ·  Size unknown",
        2 => "8.2 GB  ·  12 peers connected",
        3 => "example.com  ·  2.4 MB",
        _ => "Connection lost  ·  Check your network",
    };
    let mut item = row()
        .key(format!("row-{index}"))
        .w_full()
        .h(if s.minimal && !narrow {
            58.0
        } else {
            s.treatment.row_height()
        })
        .px(16.0)
        .gap(14.0)
        .center_y()
        .child(
            el().size(42.0, 42.0)
                .pixels(s.artwork[1].pixels())
                .a11y_hidden(),
        )
        .child(
            col()
                .grow(1.0)
                .min_w(0.0)
                .gap(5.0)
                .child(
                    Button::new(FILES[index])
                        .variant(ButtonVariant::Ghost)
                        .label_with(move |_, ink| {
                            text(FILES[index])
                                .font(TextStyle::new(14.0, 500.0))
                                .color(ink)
                                .truncate()
                                .grow(1.0)
                                .min_w(0.0)
                        })
                        .on_press(move |s: &mut Preview| s.sheet = Some(index))
                        .build(k)
                        .key(format!("download-{index}"))
                        .h(20.0)
                        .px(0.0)
                        .w_full()
                        .min_w(0.0)
                        .justify_start(),
                )
                .child(
                    caption(description, k)
                        .key(format!("status-{index}"))
                        .role("status")
                        .label(status(s, index))
                        .truncate(),
                )
                .children((narrow && index < 2).then(|| {
                    effects::progress(
                        &format!("progress-{index}"),
                        if index == 0 {
                            Some(if matches!(s.transfer, Transfer::Completed) {
                                1.0
                            } else {
                                0.42
                            })
                        } else {
                            None
                        },
                        index == 1 || matches!(s.transfer, Transfer::Downloading),
                        s.reduced,
                        k,
                    )
                })),
        )
        .children((narrow && index == 2).then(|| effects::seeding(s.reduced, k)))
        .children((!narrow).then(|| {
            let mut state = col()
                .key(format!("indicator-{index}"))
                .w(144.0)
                .gap(6.0)
                .justify_center();
            match index {
                0 if matches!(s.transfer, Transfer::Downloading | Transfer::Paused) => {
                    if matches!(s.treatment, Treatment::List) {
                        state = state.child(
                            row()
                                .gap(8.0)
                                .center_y()
                                .child(
                                    ProgressCircle::new(0.42)
                                        .size(22.0)
                                        .label("Coast download progress")
                                        .build(k),
                                )
                                .child(caption("42%", k)),
                        );
                    } else {
                        state = state.child(effects::progress(
                            "progress-0",
                            Some(0.42),
                            matches!(s.transfer, Transfer::Downloading),
                            s.reduced,
                            k,
                        ));
                    }
                }
                0 => state = state.child(caption(s.transfer.name(), k)),
                1 => state = state.child(effects::progress("progress-1", None, true, s.reduced, k)),
                2 => state = state.child(effects::seeding(s.reduced, k)),
                3 => {
                    state = state.child(
                        row()
                            .gap(5.0)
                            .center_y()
                            .child(icon(&i::CHECK, 13.0).color(k.c.success.fg))
                            .child(caption("Completed", k)),
                    )
                }
                _ => state = state.child(caption("Retry available", k).color(k.c.danger.fg)),
            }
            state
        }))
        .children((index == 0).then(|| {
            icon_button(
                if matches!(s.transfer, Transfer::Downloading) {
                    &i::PAUSE
                } else {
                    &i::PLAY
                },
                if matches!(s.transfer, Transfer::Downloading) {
                    "Pause"
                } else {
                    "Resume"
                },
                "pause",
                k,
                toggle_transfer,
            )
        }))
        .children((index != 0).then(|| el().w(28.0)))
        .child(icon_button(
            &i::MORE,
            "Download details",
            &format!("more-{index}"),
            k,
            move |s| s.sheet = Some(index),
        ));
    if s.minimal || matches!(s.treatment, Treatment::List | Treatment::Panels) {
        item = item.border_sides([0.0, 0.0, 1.0, 0.0], k.c.border_base);
    } else {
        item = item
            .radius(s.treatment.radius())
            .fill(k.c.layer_01)
            .border(1.0, k.c.border_base)
            .shadow(
                1.0,
                2.0,
                Color::BLACK.alpha(if k.dark { 0.10 } else { 0.025 }),
            );
    }
    item
}

pub(super) fn toggle_transfer(s: &mut Preview) {
    s.transfer = if matches!(s.transfer, Transfer::Downloading) {
        Transfer::Paused
    } else {
        Transfer::Downloading
    };
}
