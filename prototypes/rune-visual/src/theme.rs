//! Disposable treatments derived from Rune-kit's neutral kit, not production tokens.
use rune::{Color, TextStyle, oklch};
use rune_kit::Kit;

#[derive(Clone, Copy, Default)]
pub enum Treatment {
    #[default]
    Cards,
    List,
    Panels,
}

impl Treatment {
    pub fn cycle(self, forward: bool) -> Self {
        match (self, forward) {
            (Self::Cards, true) | (Self::Panels, false) => Self::List,
            (Self::List, true) | (Self::Cards, false) => Self::Panels,
            _ => Self::Cards,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Cards => "A · Soft edge",
            Self::List => "B · Quiet line",
            Self::Panels => "C · Inset panel",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Cards => "Separate cards · subtle elevation",
            Self::List => "Compact rows · progress rings",
            Self::Panels => "Shared surface · inset details",
        }
    }

    pub fn row_height(self) -> f32 {
        match self {
            Self::Cards => 76.0,
            Self::List => 68.0,
            Self::Panels => 80.0,
        }
    }

    pub fn radius(self) -> f32 {
        match self {
            Self::Cards => 10.0,
            Self::List => 4.0,
            Self::Panels => 12.0,
        }
    }

    pub fn kit(self, dark: bool) -> Kit {
        let mut k = Kit::neutral(dark);
        // Opul's achromatic semantic surfaces. Workspace orange is a local identity accent.
        k.c.page = oklch(if dark { 0.145 } else { 1.0 }, 0.0, 0.0);
        k.c.text_base = oklch(if dark { 0.985 } else { 0.145 }, 0.0, 0.0);
        k.c.text_muted = oklch(if dark { 0.708 } else { 0.5 }, 0.0, 0.0);
        k.c.bg_base = oklch(if dark { 0.205 } else { 0.985 }, 0.0, 0.0);
        k.c.layer_01 = oklch(if dark { 0.205 } else { 1.0 }, 0.0, 0.0);
        k.c.layer_03 = oklch(if dark { 0.269 } else { 0.97 }, 0.0, 0.0);
        k.c.border_base = k.c.text_base.alpha(if dark { 0.12 } else { 0.10 });
        k.c.border_focus = oklch(if dark { 0.75 } else { 0.5 }, 0.15, 256.0);
        k.c.border_active = k.c.text_base;
        k.c.icon_muted = k.c.text_muted;
        k.ty.ui = TextStyle::new(13.0, 400.0).lh(20.0);
        k.ty.ui_medium = TextStyle::new(13.0, 500.0).lh(20.0);
        k.ty.small = TextStyle::new(12.0, 400.0).lh(16.0);
        k.c.bg_button_neutral = k.c.layer_01;
        k.elevation.button_neutral.clear();
        k.radius.md = self.radius().min(8.0);
        k.radius.lg = self.radius();
        k.c.bg_button_neutral = k.c.layer_01;
        k.c.text_contrast = if dark { Color::BLACK } else { Color::WHITE };
        k.c.bg_contrast = k.c.text_base;
        k
    }
}
