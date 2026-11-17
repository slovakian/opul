use super::*;

pub(super) fn sheet(s: &Preview, index: usize, width: f32, k: &Kit) -> El {
    let inset = if matches!(s.treatment, Treatment::Panels) {
        12.0
    } else {
        0.0
    };
    let workspace = if index == 0 {
        "Studio / Footage"
    } else {
        "Personal / Downloads"
    };
    let mut panel = col()
        .key("details-sheet")
        .role("dialog")
        .label(format!("Details: {}", FILES[index]))
        .pin(None, Some(inset), Some(inset), Some(inset))
        .w(width - inset)
        .radius(if inset > 0.0 { 12.0 } else { 0.0 })
        .fill(k.c.layer_01)
        .border(1.0, k.c.border_base)
        .shadow(0.0, 24.0, Color::BLACK.alpha(0.10))
        .token_motion(k.motion.overlay)
        .transition(Transition::move_edge(rune::Edge::Trailing))
        .modal(true, |s: &mut Preview| s.sheet = None)
        .child(
            row()
                .h(60.0)
                .px(24.0)
                .justify_between()
                .center_y()
                .child(
                    text("Download details")
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                )
                .child(icon_button(
                    &i::CLOSE,
                    "Close details",
                    "close-sheet",
                    k,
                    |s| s.sheet = None,
                )),
        );
    let mut body = col()
        .grow(1.0)
        .min_h(0.0)
        .w_full()
        .scroll(rune::ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .padding(24.0)
                .gap(20.0)
                .child(
                    el().size(64.0, 64.0)
                        .pixels(s.artwork[1].pixels())
                        .a11y_hidden(),
                )
                .child(
                    text(FILES[index])
                        .font(TextStyle::new(20.0, 550.0).lh(27.0))
                        .color(k.c.text_base)
                        .wrap(),
                )
                .child(
                    row()
                        .gap(7.0)
                        .center_y()
                        .child(
                            icon(if index == 2 { &i::UP } else { &i::DOWN }, 14.0)
                                .color(k.c.text_muted),
                        )
                        .child(caption(status(s, index), k).key("sheet-status")),
                )
                .children((index == 0).then(|| {
                    col()
                        .gap(12.0)
                        .child(effects::progress(
                            "sheet-progress",
                            Some(if matches!(s.transfer, Transfer::Completed) {
                                1.0
                            } else {
                                0.42
                            }),
                            matches!(s.transfer, Transfer::Downloading),
                            s.reduced,
                            k,
                        ))
                        .child(caption(
                            if matches!(s.transfer, Transfer::Completed) {
                                "610 MB of 610 MB  ·  100%"
                            } else {
                                "256 MB of 610 MB  ·  42%"
                            },
                            k,
                        ))
                })),
        )
        .child(el().w_full().px(24.0).child(rule(k)));
    let mut metadata = col().padding(24.0).gap(20.0);
    for (label, value) in [
        ("Workspace", workspace),
        ("Source", if index == 2 { "Torrent" } else { "Direct link" }),
        ("Format", "Original file"),
        (
            "Save location",
            if index == 0 { "Footage" } else { "Downloads" },
        ),
        (
            "Download speed",
            if index == 0 && matches!(s.transfer, Transfer::Downloading) {
                "2.4 MB/s"
            } else {
                "—"
            },
        ),
        (
            "Time remaining",
            if index == 0 && matches!(s.transfer, Transfer::Downloading) {
                "14s left"
            } else {
                "—"
            },
        ),
    ] {
        metadata = metadata.child(
            row()
                .justify_between()
                .gap(16.0)
                .child(caption(label, k))
                .child(text(value).font(k.ty.ui).color(k.c.text_base).truncate()),
        );
    }
    body = body.child(metadata).child(el().grow(1.0)).child(
        row()
            .px(24.0)
            .gap(8.0)
            .center_y()
            .child(icon(&i::LINK, 16.0).color(k.c.text_muted))
            .child(caption("https://example.test/download", k).truncate()),
    );
    panel = panel.child(body);
    if index == 0 {
        panel = panel.child(
            Button::new(if matches!(s.transfer, Transfer::Downloading) {
                "Pause download"
            } else {
                "Resume download"
            })
            .on_press(toggle_transfer)
            .build(k)
            .key("sheet-pause")
            .mx(24.0)
            .mt(20.0)
            .mb(24.0),
        );
    }
    panel
}
