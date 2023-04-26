use eframe::{
    egui,
    egui::{epaint, style, Color32},
};
/// Apply the given theme to a [`Context`](egui::Context).
pub fn set_theme(ctx: &egui::Context, theme: Theme) {
    let old = ctx.style().visuals.clone();
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
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Theme {
    pub primary: Color32,
    pub secondary: Color32,
    pub accent: Color32,
    pub neutral: Color32,
    pub base_100: Color32,
    pub base_200: Color32,
    pub text_base: Color32,
    pub text_accent: Color32,
}
#[macro_export]
macro_rules! hex_color {
    ($s:literal) => {{
        let array = color_hex::color_from_hex!($s);
        $crate::Color32::from_rgb(array[0], array[1], array[2])
    }};
}

pub const THEME: Theme = Theme {
    primary: hex_color!("#38BDF8"),
    secondary: hex_color!("#818CF8"),
    accent: hex_color!("#F471B5"),
    neutral: hex_color!("#1E293B"),
    base_100: hex_color!("#0F172A"),
    base_200: hex_color!("#141f38"),
    text_base: hex_color!("#B3CCF6"),
    text_accent: hex_color!("#002B3D"),
};

// pub const LATTE: Theme = Theme {
//     rosewater: Color32::from_rgb(220, 138, 120),
//     flamingo: Color32::from_rgb(221, 120, 120),
//     pink: Color32::from_rgb(234, 118, 203),
//     mauve: Color32::from_rgb(136, 57, 239),
//     red: Color32::from_rgb(210, 15, 57),
//     maroon: Color32::from_rgb(230, 69, 83),
//     peach: Color32::from_rgb(254, 100, 11),
//     yellow: Color32::from_rgb(223, 142, 29),
//     green: Color32::from_rgb(64, 160, 43),
//     teal: Color32::from_rgb(23, 146, 153),
//     sky: Color32::from_rgb(4, 165, 229),
//     sapphire: Color32::from_rgb(32, 159, 181),
//     blue: Color32::from_rgb(30, 102, 245),
//     lavender: Color32::from_rgb(114, 135, 253),
//     text: Color32::from_rgb(76, 79, 105),
//     subtext1: Color32::from_rgb(92, 95, 119),
//     subtext0: Color32::from_rgb(108, 111, 133),
//     overlay2: Color32::from_rgb(124, 127, 147),
//     overlay1: Color32::from_rgb(140, 143, 161),
//     overlay0: Color32::from_rgb(156, 160, 176),
//     surface2: Color32::from_rgb(172, 176, 190),
//     surface1: Color32::from_rgb(188, 192, 204),
//     surface0: Color32::from_rgb(204, 208, 218),
//     base: Color32::from_rgb(239, 241, 245),
//     mantle: Color32::from_rgb(230, 233, 239),
//     crust: Color32::from_rgb(220, 224, 232),
// };

// pub const FRAPPE: Theme = Theme {
//     rosewater: Color32::from_rgb(242, 213, 207),
//     flamingo: Color32::from_rgb(238, 190, 190),
//     pink: Color32::from_rgb(244, 184, 228),
//     mauve: Color32::from_rgb(202, 158, 230),
//     red: Color32::from_rgb(231, 130, 132),
//     maroon: Color32::from_rgb(234, 153, 156),
//     peach: Color32::from_rgb(239, 159, 118),
//     yellow: Color32::from_rgb(229, 200, 144),
//     green: Color32::from_rgb(166, 209, 137),
//     teal: Color32::from_rgb(129, 200, 190),
//     sky: Color32::from_rgb(153, 209, 219),
//     sapphire: Color32::from_rgb(133, 193, 220),
//     blue: Color32::from_rgb(140, 170, 238),
//     lavender: Color32::from_rgb(186, 187, 241),
//     text: Color32::from_rgb(198, 208, 245),
//     subtext1: Color32::from_rgb(181, 191, 226),
//     subtext0: Color32::from_rgb(165, 173, 206),
//     overlay2: Color32::from_rgb(148, 156, 187),
//     overlay1: Color32::from_rgb(131, 139, 167),
//     overlay0: Color32::from_rgb(115, 121, 148),
//     surface2: Color32::from_rgb(98, 104, 128),
//     surface1: Color32::from_rgb(81, 87, 109),
//     surface0: Color32::from_rgb(65, 69, 89),
//     base: Color32::from_rgb(48, 52, 70),
//     mantle: Color32::from_rgb(41, 44, 60),
//     crust: Color32::from_rgb(35, 38, 52),
// };

// pub const MACCHIATO: Theme = Theme {
//     rosewater: Color32::from_rgb(244, 219, 214),
//     flamingo: Color32::from_rgb(240, 198, 198),
//     pink: Color32::from_rgb(245, 189, 230),
//     mauve: Color32::from_rgb(198, 160, 246),
//     red: Color32::from_rgb(237, 135, 150),
//     maroon: Color32::from_rgb(238, 153, 160),
//     peach: Color32::from_rgb(245, 169, 127),
//     yellow: Color32::from_rgb(238, 212, 159),
//     green: Color32::from_rgb(166, 218, 149),
//     teal: Color32::from_rgb(139, 213, 202),
//     sky: Color32::from_rgb(145, 215, 227),
//     sapphire: Color32::from_rgb(125, 196, 228),
//     blue: Color32::from_rgb(138, 173, 244),
//     lavender: Color32::from_rgb(183, 189, 248),
//     text: Color32::from_rgb(202, 211, 245),
//     subtext1: Color32::from_rgb(184, 192, 224),
//     subtext0: Color32::from_rgb(165, 173, 203),
//     overlay2: Color32::from_rgb(147, 154, 183),
//     overlay1: Color32::from_rgb(128, 135, 162),
//     overlay0: Color32::from_rgb(110, 115, 141),
//     surface2: Color32::from_rgb(91, 96, 120),
//     surface1: Color32::from_rgb(73, 77, 100),
//     surface0: Color32::from_rgb(54, 58, 79),
//     base: Color32::from_rgb(36, 39, 58),
//     mantle: Color32::from_rgb(30, 32, 48),
//     crust: Color32::from_rgb(24, 25, 38),
// };

// pub const MOCHA: Theme = Theme {
//     rosewater: Color32::from_rgb(245, 224, 220),
//     flamingo: Color32::from_rgb(242, 205, 205),
//     pink: Color32::from_rgb(245, 194, 231),
//     mauve: Color32::from_rgb(203, 166, 247),
//     red: Color32::from_rgb(243, 139, 168),
//     maroon: Color32::from_rgb(235, 160, 172),
//     peach: Color32::from_rgb(250, 179, 135),
//     yellow: Color32::from_rgb(249, 226, 175),
//     green: Color32::from_rgb(166, 227, 161),
//     teal: Color32::from_rgb(148, 226, 213),
//     sky: Color32::from_rgb(137, 220, 235),
//     sapphire: Color32::from_rgb(116, 199, 236),
//     blue: Color32::from_rgb(137, 180, 250),
//     lavender: Color32::from_rgb(180, 190, 254),
//     text: Color32::from_rgb(205, 214, 244),
//     subtext1: Color32::from_rgb(186, 194, 222),
//     subtext0: Color32::from_rgb(166, 173, 200),
//     overlay2: Color32::from_rgb(147, 153, 178),
//     overlay1: Color32::from_rgb(127, 132, 156),
//     overlay0: Color32::from_rgb(108, 112, 134),
//     surface2: Color32::from_rgb(88, 91, 112),
//     surface1: Color32::from_rgb(69, 71, 90),
//     surface0: Color32::from_rgb(49, 50, 68),
//     base: Color32::from_rgb(30, 30, 46),
//     mantle: Color32::from_rgb(24, 24, 37),
//     crust: Color32::from_rgb(17, 17, 27),
// };
