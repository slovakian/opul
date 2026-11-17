//! Disposable renderer/tooling preview for #100, independent of the production application.
use rune::tok::{self, type_};
use rune::{AppSpec, Cx, El, col, el, row, text};
use std::path::PathBuf;

#[derive(Default)]
struct Preview {
    paused: bool,
    dark: bool,
}

fn view(state: &Preview, _: &mut Cx<Preview>) -> El {
    let theme = tok::theme(state.dark);
    let sidebar = col()
        .key("sidebar")
        .w(200.0)
        .h_full()
        .padding(24.0)
        .gap(20.0)
        .fill(theme.card)
        .child(text("Opul").font(type_::TITLE).color(theme.text))
        .child(text("All Downloads").color(theme.text))
        .child(text("In progress").color(theme.text_secondary))
        .child(text("Completed").color(theme.text_secondary))
        .child(text("Workspaces").color(theme.text_secondary))
        .child(text("Downloads").color(theme.text));
    let status = if state.paused {
        "Paused"
    } else {
        "Downloading"
    };
    let action = if state.paused {
        "Resume fixture"
    } else {
        "Pause fixture"
    };
    let toggle = text(action)
        .key("toggle-transfer")
        .role("button")
        .label(action)
        .focusable(true)
        .padding(12.0)
        .radius(8.0)
        .fill(theme.control_off)
        .color(theme.text)
        .on_press(|state: &mut Preview| state.paused = !state.paused);
    let appearance = text("Toggle appearance")
        .key("toggle-appearance")
        .role("button")
        .label("Toggle appearance")
        .focusable(true)
        .padding(12.0)
        .radius(8.0)
        .fill(theme.control_off)
        .color(theme.text)
        .on_press(|state: &mut Preview| state.dark = !state.dark);
    let content = col()
        .grow(1.0)
        .padding(32.0)
        .gap(20.0)
        .child(
            text("Rune baseline preview")
                .font(type_::TITLE)
                .color(theme.text),
        )
        .child(text("Disposable fixtures · #100").color(theme.text_secondary))
        .child(
            col()
                .key("download")
                .padding(20.0)
                .gap(12.0)
                .radius(12.0)
                .fill(theme.card)
                .child(text("Example video.mp4").color(theme.text))
                .child(
                    text(status)
                        .key("transfer-status")
                        .role("status")
                        .color(theme.text_secondary),
                )
                .child(
                    el().w_full()
                        .h(6.0)
                        .radius(3.0)
                        .fill(theme.control_off)
                        .child(
                            el().w(180.0)
                                .h(6.0)
                                .radius(3.0)
                                .fill(tok::accent(0, state.dark)),
                        ),
                )
                .child(toggle),
        )
        .child(appearance)
        .child(
            text("Fixture data and Downloads stay in .build/rune-preview.")
                .color(theme.text_secondary),
        );
    row()
        .w_full()
        .h_full()
        .fill(theme.bg)
        .child(sidebar)
        .child(content)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile-time repository location: never resolve production Application Support or ~/Downloads.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.build/rune-preview");
    for directory in ["data", "Downloads"] {
        std::fs::create_dir_all(root.join(directory))?;
    }
    eprintln!("Preview storage: {}", root.canonicalize()?.display());
    rune::orderly_exit();
    rune::run(AppSpec {
        scenes: &[("dark", |state: &mut Preview| state.dark = true)],
        ..AppSpec::new(
            "Opul Rune preview · #100",
            (900.0, 560.0),
            Preview::default,
            view,
        )
    });
    Ok(())
}
