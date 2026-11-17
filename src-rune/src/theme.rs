//! Owned semantic tokens, starting with #101's reviewed Soft Edge treatment.
use rune::{Color, TextStyle, oklch};

#[derive(Clone, Copy, Default)]
pub enum Look {
    #[default]
    SoftEdge,
    QuietLine,
    InsetPanel,
}
impl Look {
    pub fn next(self) -> Self {
        match self {
            Self::SoftEdge => Self::QuietLine,
            Self::QuietLine => Self::InsetPanel,
            Self::InsetPanel => Self::SoftEdge,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::SoftEdge => "Soft Edge",
            Self::QuietLine => "Quiet line",
            Self::InsetPanel => "Inset panel",
        }
    }
    pub fn row_height(self) -> f32 {
        match self {
            Self::SoftEdge => 76.0,
            Self::QuietLine => 56.0,
            Self::InsetPanel => 80.0,
        }
    }
    pub fn radius(self) -> f32 {
        match self {
            Self::SoftEdge => 10.0,
            Self::QuietLine => 4.0,
            Self::InsetPanel => 12.0,
        }
    }
    pub fn gap(self) -> f32 {
        if matches!(self, Self::SoftEdge) {
            8.0
        } else {
            0.0
        }
    }
}

pub struct Theme {
    pub page: Color,
    pub surface: Color,
    pub control: Color,
    pub text: Color,
    pub muted: Color,
    pub border: Color,
    pub focus: Color,
    pub danger: Color,
    pub dark: bool,
    pub look: Look,
}
impl Theme {
    pub fn new(dark: bool, look: Look) -> Self {
        let text = oklch(if dark { 0.985 } else { 0.145 }, 0.0, 0.0);
        Self {
            page: oklch(if dark { 0.145 } else { 1.0 }, 0.0, 0.0),
            surface: oklch(if dark { 0.205 } else { 0.985 }, 0.0, 0.0),
            control: oklch(if dark { 0.269 } else { 0.97 }, 0.0, 0.0),
            text,
            muted: oklch(if dark { 0.708 } else { 0.5 }, 0.0, 0.0),
            border: text.alpha(if dark { 0.12 } else { 0.10 }),
            focus: oklch(if dark { 0.75 } else { 0.5 }, 0.15, 256.0),
            danger: oklch(if dark { 0.75 } else { 0.5 }, 0.18, 25.0),
            dark,
            look,
        }
    }
}
pub const BODY: TextStyle = TextStyle::new(13.0, 400.0);
pub const CAPTION: TextStyle = TextStyle::new(12.0, 400.0);
pub const NAME: TextStyle = TextStyle::new(14.0, 500.0);
