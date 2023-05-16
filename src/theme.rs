// SPDX-License-Identifier: UNLICENSE
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT
// use this code under any of the above license
// do whatever you want with this code go add themes to your application!
// settings.rs&utils.rs is still agpl but you can take some inspiration from that tho

use eframe::{
    egui,
    egui::{epaint, style, Color32, Visuals},
};
/// Apply the given theme to a [`Context`](egui::Context).
/// from my testing this doesn't take more than 3µs so doesnt need to be optimized
pub fn set_theme(ctx: &egui::Context, theme: Theme) {
    let old = if theme.light { Visuals::light() } else { Visuals::dark() };
    ctx.set_visuals(egui::Visuals {
        override_text_color: Some(theme.text_base),
        hyperlink_color: theme.primary,
        faint_bg_color: theme.neutral,
        window_fill: theme.base_100,
        panel_fill: theme.base_100,
        selection: style::Selection {
            bg_fill: theme.primary,
            stroke: egui::Stroke {
                color: theme.accent,
                ..old.selection.stroke
            },
        },
        widgets: style::Widgets {
            noninteractive: make_widget_visual(old.widgets.noninteractive, &theme, theme.base_100.gamma_multiply(2.2)),
            inactive: make_widget_visual(old.widgets.inactive, &theme, theme.base_200.gamma_multiply(2.2)),
            hovered: make_widget_visual(old.widgets.hovered, &theme, theme.neutral.gamma_multiply(2.2)),
            active: make_widget_visual(old.widgets.active, &theme, theme.accent),
            open: make_widget_visual(old.widgets.open, &theme, theme.primary),
        },
        window_shadow: epaint::Shadow {
            color: theme.base_100,
            ..old.window_shadow
        },
        popup_shadow: epaint::Shadow {
            color: theme.base_100,
            ..old.popup_shadow
        },
        ..old
    });
}

fn make_widget_visual(old: style::WidgetVisuals, theme: &Theme, bg_fill: egui::Color32) -> style::WidgetVisuals {
    style::WidgetVisuals {
        bg_fill,
        weak_bg_fill: bg_fill,
        bg_stroke: egui::Stroke {
            color: theme.base_200,
            ..old.bg_stroke
        },
        fg_stroke: egui::Stroke {
            color: theme.text_base,
            ..old.fg_stroke
        },
        ..old
    }
}

// FIXME: Theme should be `Copy` since it isn't big enough to generate a call to `memcpy`,
// do this when egui releases a minor version
/// The colors for a theme variant.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Theme {
    pub primary: Color32,
    pub secondary: Color32,
    pub accent: Color32,
    pub neutral: Color32,
    pub base_100: Color32,
    pub base_200: Color32,
    pub text_base: Color32,
    pub text_accent: Color32,
    pub light: bool,
}
#[macro_export]
macro_rules! hex_color {
    ($s:literal) => {{
        let array = color_hex::color_from_hex!($s);
        $crate::Color32::from_rgb(array[0], array[1], array[2])
    }};
}
pub mod themes {
    use super::Theme;

    pub const DARK_THEME: Theme = Theme {
        primary: hex_color!("#38BDF8"),
        secondary: hex_color!("#818CF8"),
        accent: hex_color!("#F471B5"),
        neutral: hex_color!("#1E293B"),
        base_100: hex_color!("#0F172A"),
        base_200: hex_color!("#141f38"),
        text_base: hex_color!("#B3CCF6"),
        text_accent: hex_color!("#002B3D"),
        light: false,
    };
    pub const LIGHT_THEME: Theme = Theme {
        primary: hex_color!("#818CF8"),
        secondary: hex_color!("#38BDF8"),
        accent: hex_color!("#F471B5"),
        neutral: hex_color!("#E5E7EB"),
        base_100: hex_color!("#F3F4F6"),
        base_200: hex_color!("#E5E7EB"),
        text_base: hex_color!("#1E293B"),
        text_accent: hex_color!("#002B3D"),
        light: true,
    };
    pub const TWILIGHT_THEME: Theme = Theme {
        primary: hex_color!("#F49FBC"),
        secondary: hex_color!("#F8D49F"),
        accent: hex_color!("#B7B2F8"),
        neutral: hex_color!("#474B4F"),
        base_100: hex_color!("#1E1F23"),
        base_200: hex_color!("#2E2F33"),
        text_base: hex_color!("#D9D9F8"),
        text_accent: hex_color!("#1F1F23"),
        light: false,
    };
    pub const SUNRISE_THEME: Theme = Theme {
        primary: hex_color!("#F8A978"),
        secondary: hex_color!("#F8D197"),
        accent: hex_color!("#F5F297"),
        neutral: hex_color!("#FFF0CC"),
        base_100: hex_color!("#FFF8EB"),
        base_200: hex_color!("#FDF0D7"),
        text_base: hex_color!("#8C4B25"),
        text_accent: hex_color!("#5C3D21"),
        light: true,
    };
    pub const OCEANIC_THEME: Theme = Theme {
        primary: hex_color!("#00B4D8"),
        secondary: hex_color!("#90E0EF"),
        accent: hex_color!("#FFD166"),
        neutral: hex_color!("#293241"),
        base_100: hex_color!("#1D3A4D"),
        base_200: hex_color!("#1A2F45"),
        text_base: hex_color!("#FFFFFF"),
        text_accent: hex_color!("#FF9A8B"),
        light: false,
    };
    pub const GALACTIC_THEME: Theme = Theme {
        primary: hex_color!("#E63946"),
        secondary: hex_color!("#F1FAEE"),
        accent: hex_color!("#A8DADC"),
        neutral: hex_color!("#457B9D"),
        base_100: hex_color!("#1D3557"),
        base_200: hex_color!("#0B1A2C"),
        text_base: hex_color!("#FFFFFF"),
        text_accent: hex_color!("#F1FAEE"),
        light: false,
    };
}
