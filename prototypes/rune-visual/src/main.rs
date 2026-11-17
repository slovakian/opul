//! #101: Which Rune-kit treatment suits Opul's existing sidebar/library/sheet layout?
//! Three disposable variants. Fixtures and interactions live only in memory.
mod effects;
mod icons;
mod theme;
mod ui;

use rune::{AppSpec, Color, Cx, El, Key, LoopSpec, TextStyle, Transition, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, ProgressCircle};
use theme::Treatment;
use ui::view;

#[derive(Clone, Copy, Default)]
enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Default)]
enum Transfer {
    #[default]
    Downloading,
    Paused,
    Completed,
    Failed,
}

impl Transfer {
    fn name(self) -> &'static str {
        match self {
            Self::Downloading => "Downloading · 42%",
            Self::Paused => "Paused · 42%",
            Self::Completed => "Completed",
            Self::Failed => "Connection lost",
        }
    }
}

#[derive(Clone)]
struct Preview {
    treatment: Treatment,
    appearance: Appearance,
    reduced: bool,
    minimal: bool,
    multiple: bool,
    expanded: bool,
    sidebar: bool,
    sidebar_hidden: bool,
    selected: &'static str,
    transfer: Transfer,
    sheet: Option<usize>,
    search: rune::TextEdit,
    artwork: [rune::Image; 2],
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            treatment: Treatment::Cards,
            appearance: Appearance::System,
            reduced: false,
            minimal: false,
            multiple: true,
            expanded: true,
            sidebar: false,
            sidebar_hidden: false,
            selected: "All Downloads",
            transfer: Transfer::Downloading,
            sheet: None,
            search: rune::TextEdit::new(""),
            artwork: artwork(),
        }
    }
}

const FILES: [&str; 5] = [
    "Coast at golden hour.mp4",
    "Field recordings.zip",
    "Open cinema collection.torrent",
    "Brand guidelines.pdf",
    "Reference images.zip",
];

fn status(s: &Preview, index: usize) -> &'static str {
    match index {
        0 => s.transfer.name(),
        1 => "Finding sources…",
        2 => "Seeding · ↑ 1.2 MB/s",
        3 => "Completed · 2.4 MB",
        _ => "Connection lost · Retry available",
    }
}

fn artwork() -> [rune::Image; 2] {
    let cache = rune::ImageCache::new(std::sync::Arc::new(|| {}));
    let images = [
        cache.load(include_bytes!("../../../frontend/assets/opul-icon.png").as_slice()),
        cache.load(include_bytes!("../../../assets/download-folder.png").as_slice()),
    ];
    for image in &images {
        image.wait().expect("checked-in prototype artwork");
    }
    images
}

fn main() {
    rune::orderly_exit();
    let spec = AppSpec {
        min_size: Some((560.0, 540.0)),
        scenes: &[
            ("cards", |s: &mut Preview| s.treatment = Treatment::Cards),
            ("list", |s: &mut Preview| s.treatment = Treatment::List),
            ("minimal", |s: &mut Preview| {
                s.treatment = Treatment::List;
                s.minimal = true;
            }),
            ("panels", |s: &mut Preview| s.treatment = Treatment::Panels),
        ],
        on_key: Some(|s, event, tx| {
            if s.sheet.is_some()
                || s.sidebar
                || tx.focused().is_some_and(|key| key.contains("search"))
            {
                return false;
            }
            match event.key {
                Key::Left => s.treatment = s.treatment.cycle(false),
                Key::Right => s.treatment = s.treatment.cycle(true),
                _ => return false,
            }
            true
        }),
        ..AppSpec::new(
            "Opul · disposable visual prototype #101",
            (1180.0, 760.0),
            Preview::default,
            view,
        )
    };
    rune::App::new(spec)
        .chrome(rune::Chrome {
            full_size_content: true,
            // Custom traffic-light reparenting asserts in AppKit on this pinned revision.
            traffic_lights: None,
            drag_regions: true,
        })
        .run();
}
