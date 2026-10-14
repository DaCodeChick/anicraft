//! The AniCraft dark theme (research/06 §2 palette).
//!
//! Dark mode is the primary interface (the user-approved decision of 2026-10-13). [`Tokens`] is
//! the single source of colours and metrics: never hard-code a colour in a widget.
//! [`apply`] installs the tokens on an egui context — the egui `Visuals`, the shared
//! `craft-ui` widget-state palette, and the spacing and text styles (the VectorCraft
//! `theme.rs` pattern, reduced to the base dark palette for now).

use egui::{Color32, CornerRadius, FontId, Stroke, TextStyle, Visuals};

/// Design tokens: the verified Craft dark palette (research/06 §2, from VectorCraft's dark
/// `Tokens`). Neutral, low-saturation grays; a single blue accent for selection/focus/active; a
/// pasteboard darker than the panels.
///
/// The full token set is the documented palette itself: the shell only consumes a few of the
/// tokens, the panels, stage and timeline that use the rest land in later milestones.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    /// The application bar (menu strip) background.
    pub app_bar: Color32,
    /// Panel (docked area) background.
    pub panel: Color32,
    /// One step darker than `panel` (insets, the faint egui background).
    pub panel_darker: Color32,
    /// Panel tab strips, document tab bar, dock headers.
    pub tab_strip: Color32,
    /// Text-field and input backgrounds.
    pub input: Color32,
    /// Text-field borders.
    pub input_border: Color32,
    /// Strong borders (window, panel edges).
    pub border: Color32,
    /// Thin separators.
    pub divider: Color32,
    /// Default text.
    pub text: Color32,
    /// Primary labels, field values, button labels.
    pub text_strong: Color32,
    /// Secondary (dimmed) text.
    pub text_dim: Color32,
    /// Disabled text.
    pub text_disabled: Color32,
    /// Default icon tint.
    pub icon: Color32,
    /// Hovered controls.
    pub hover: Color32,
    /// The pressed/selected-tool state.
    pub tool_active: Color32,
    /// Filled (primary) buttons.
    pub button: Color32,
    /// Outline of secondary (outlined) buttons.
    pub button_border: Color32,
    /// Selection, focus, active state.
    pub accent: Color32,
    /// Pressed/focused accent.
    pub accent_strong: Color32,
    /// Selected list rows.
    pub row_selected: Color32,
    /// Selection outline.
    pub selection: Color32,
    /// Selected-handle fill (path vertices).
    pub anchor_selected: Color32,
    /// Active item text in tool flyouts.
    pub flyout_active: Color32,
    /// The canvas surround, darker than the panels.
    pub pasteboard: Color32,
    /// Ruler background.
    pub ruler: Color32,
    /// Ruler ticks and numbers.
    pub ruler_tick: Color32,
    /// Object guides (the high-visibility cyan).
    pub guide: Color32,
    /// Smart guides (the high-visibility magenta).
    pub smart_guide: Color32,
    /// Error marks.
    pub error: Color32,
    /// Warning marks.
    pub warning: Color32,
    /// Control corner radius.
    pub radius: u8,
    /// Base UI font size (documented at 13 to match the vector/creative apps, research/06 §3).
    pub font_size: f32,
}

impl Tokens {
    /// The dark palette (research/06 §2) — the primary theme.
    pub const DARK: Tokens = Tokens {
        app_bar: Color32::from_rgb(0x26, 0x26, 0x26),
        panel: Color32::from_rgb(0x32, 0x32, 0x32),
        panel_darker: Color32::from_rgb(0x28, 0x28, 0x28),
        tab_strip: Color32::from_rgb(0x28, 0x28, 0x28),
        input: Color32::from_rgb(0x1f, 0x1f, 0x1f),
        input_border: Color32::from_rgb(0x46, 0x46, 0x46),
        border: Color32::from_rgb(0x1b, 0x1b, 0x1b),
        divider: Color32::from_rgb(0x3f, 0x3f, 0x3f),
        text: Color32::from_rgb(0xd8, 0xd8, 0xd8),
        text_strong: Color32::from_rgb(0xf5, 0xf5, 0xf5),
        text_dim: Color32::from_rgb(0xa8, 0xa8, 0xa8),
        text_disabled: Color32::from_rgb(0x6e, 0x6e, 0x6e),
        icon: Color32::from_rgb(0xcf, 0xcf, 0xcf),
        hover: Color32::from_rgb(0x44, 0x44, 0x44),
        tool_active: Color32::from_rgb(0x1d, 0x1d, 0x1d),
        button: Color32::from_rgb(0x44, 0x44, 0x44),
        button_border: Color32::from_rgb(0x6a, 0x6a, 0x6a),
        accent: Color32::from_rgb(0x37, 0x8e, 0xf0),
        accent_strong: Color32::from_rgb(0x14, 0x73, 0xe6),
        row_selected: Color32::from_rgb(0x48, 0x4b, 0x50),
        selection: Color32::from_rgb(0x4f, 0x80, 0xff),
        anchor_selected: Color32::from_rgb(0x3d, 0x82, 0xff),
        flyout_active: Color32::from_rgb(0x54, 0xb2, 0xf7),
        pasteboard: Color32::from_rgb(0x1e, 0x1e, 0x1e),
        ruler: Color32::from_rgb(0x2f, 0x2f, 0x2f),
        ruler_tick: Color32::from_rgb(0x8a, 0x8a, 0x8a),
        guide: Color32::from_rgb(0x4a, 0xff, 0xff),
        smart_guide: Color32::from_rgb(0xff, 0x3d, 0xfc),
        error: Color32::from_rgb(0xe3, 0x48, 0x50),
        warning: Color32::from_rgb(0xf0, 0xa3, 0x30),
        radius: 2,
        font_size: 13.0,
    };

    pub fn cr(&self) -> CornerRadius {
        CornerRadius::same(self.radius)
    }
}

/// Apply the tokens to an egui context: the global visuals (via the shared
/// `craft-ui::theme::WidgetPalette` for the five widget states), spacing and text styles.
pub fn apply(ctx: &egui::Context, t: Tokens) {
    let mut v = Visuals::dark();
    v.panel_fill = t.panel;
    v.window_fill = t.panel;
    v.extreme_bg_color = t.input;
    v.faint_bg_color = t.panel_darker;
    v.window_stroke = Stroke::new(1.0, t.border);
    v.window_corner_radius = CornerRadius::same(6);
    v.menu_corner_radius = CornerRadius::same(6);
    v.selection.bg_fill = t.accent.gamma_multiply(0.55);
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v.hyperlink_color = t.accent;
    v.override_text_color = Some(t.text);
    v.popup_shadow = egui::epaint::Shadow { offset: [0, 4], blur: 14, spread: 0, color: Color32::from_black_alpha(120) };
    v.window_shadow = egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(140) };
    let w = &mut v.widgets;
    craft_ui::theme::WidgetPalette { noninteractive: t.panel, inactive: t.button, hovered: t.hover, active: t.tool_active, open: t.hover }.apply_to(
        w,
        t.cr(),
        Stroke::new(1.0, t.text),
    );
    w.noninteractive.bg_stroke = Stroke::new(1.0, t.divider);
    w.inactive.bg_stroke = Stroke::NONE;
    w.hovered.bg_stroke = Stroke::new(1.0, t.input_border);
    w.active.bg_stroke = Stroke::new(1.0, t.accent);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 5.0);
        s.spacing.button_padding = egui::vec2(6.0, 2.0);
        s.spacing.interact_size = egui::vec2(24.0, 24.0);
        s.spacing.menu_margin = egui::Margin::same(6);
        s.spacing.slider_width = 120.0;
        s.text_styles = [
            (TextStyle::Small, FontId::proportional(11.0)),
            (TextStyle::Body, FontId::proportional(t.font_size)),
            (TextStyle::Button, FontId::proportional(t.font_size)),
            (TextStyle::Heading, FontId::proportional(15.0)),
            (TextStyle::Monospace, FontId::monospace(11.5)),
        ]
        .into();
        s.animation_time = 0.08;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The dark palette must stay the verified Craft dark reference (research/06 §2): neutral
    /// grays, a single blue accent, a pasteboard darker than the panels.
    #[test]
    fn dark_palette_matches_the_documented_reference() {
        let t = Tokens::DARK;
        assert_eq!(t.app_bar, Color32::from_rgb(0x26, 0x26, 0x26));
        assert_eq!(t.panel, Color32::from_rgb(0x32, 0x32, 0x32));
        assert_eq!(t.pasteboard, Color32::from_rgb(0x1e, 0x1e, 0x1e));
        assert_eq!(t.accent, Color32::from_rgb(0x37, 0x8e, 0xf0));
        assert_eq!(t.accent_strong, Color32::from_rgb(0x14, 0x73, 0xe6));
        assert_eq!(t.selection, Color32::from_rgb(0x4f, 0x80, 0xff));
        assert!(t.pasteboard.to_array()[0] < t.panel.to_array()[0], "the pasteboard is darker than the panels");
        assert_eq!(t.radius, 2);
        assert_eq!(t.font_size, 13.0);
    }

    #[test]
    fn the_widget_palette_colors_the_five_states() {
        let ctx = egui::Context::default();
        let t = Tokens::DARK;
        apply(&ctx, t);
        let style = ctx.style_of(ctx.theme());
        assert_eq!(style.visuals.widgets.noninteractive.bg_fill, t.panel);
        assert_eq!(style.visuals.widgets.inactive.bg_fill, t.button);
        assert_eq!(style.visuals.widgets.hovered.bg_fill, t.hover);
        assert_eq!(style.visuals.widgets.active.bg_fill, t.tool_active);
        assert_eq!(style.visuals.widgets.open.bg_fill, t.hover);
        assert_eq!(style.visuals.panel_fill, t.panel);
        assert_eq!(style.visuals.override_text_color, Some(t.text));
    }
}
