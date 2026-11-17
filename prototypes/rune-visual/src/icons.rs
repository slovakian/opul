//! Small Lucide-style 24 point stroke glyphs for this disposable study.
use rune::Icon;

macro_rules! glyph {
    ($name:ident, $body:literal) => {
        pub static $name: Icon = Icon { name: stringify!($name), view: 24.0,
            body: concat!(r#"<g fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">"#, $body, "</g>") };
    };
}

glyph!(
    INBOX,
    r#"<path d="M4 4h16l2 9v7H2v-7Z"/><path d="M2 13h6l2 3h4l2-3h6"/>"#
);
glyph!(DOWN, r#"<path d="M12 3v18m-7-7 7 7 7-7"/>"#);
glyph!(UP, r#"<path d="M12 21V3m-7 7 7-7 7 7"/>"#);
glyph!(CHECK, r#"<path d="m5 12 4 4L19 6"/>"#);
glyph!(PLUS, r#"<path d="M12 5v14M5 12h14"/>"#);
glyph!(CHEVRON, r#"<path d="m8 5 7 7-7 7"/>"#);
glyph!(CHEVRON_DOWN, r#"<path d="m6 9 6 6 6-6"/>"#);
glyph!(
    SEARCH,
    r#"<circle cx="10.5" cy="10.5" r="7"/><path d="m16 16 5 5"/>"#
);
glyph!(
    PANEL,
    r#"<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18"/>"#
);
glyph!(
    LIST,
    r#"<path d="M8 6h13M8 12h13M8 18h13M3 6h.1M3 12h.1M3 18h.1"/>"#
);
glyph!(FILTER, r#"<path d="M3 6h18M6 12h12M10 18h4"/>"#);
glyph!(
    SORT,
    r#"<path d="M7 3v18m-4-4 4 4 4-4M17 21V3m-4 4 4-4 4 4"/>"#
);
glyph!(
    MORE,
    r#"<circle cx="5" cy="12" r="1"/><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/>"#
);
glyph!(
    PAUSE,
    r#"<rect x="6" y="4" width="4" height="16" rx="1"/><rect x="14" y="4" width="4" height="16" rx="1"/>"#
);
glyph!(PLAY, r#"<path d="m7 4 13 8-13 8Z"/>"#);
glyph!(
    FOLDER,
    r#"<path d="M3 7V5a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7Z"/><path d="M3 9h18"/>"#
);
glyph!(
    SETTINGS,
    r#"<path d="M3 6h4m4 0h10M3 18h10m4 0h4"/><circle cx="9" cy="6" r="2"/><circle cx="15" cy="18" r="2"/>"#
);
glyph!(CLOSE, r#"<path d="m6 6 12 12M6 18 18 6"/>"#);
glyph!(
    LINK,
    r#"<path d="m9 15 6-6M11 6l1-1a5 5 0 0 1 7 7l-1 1M13 18l-1 1a5 5 0 0 1-7-7l1-1"/>"#
);
