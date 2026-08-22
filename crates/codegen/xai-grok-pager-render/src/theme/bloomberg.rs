//! Bloomberg Terminal theme — black canvas with amber chrome.
//!
//! Amber (`#ff9900`) is the primary accent; ticker green and alert red
//! mark success/error. Neutral enough to survive 256-color quantization.

use ratatui::style::{Color, Modifier};

use super::tokyonight::Theme;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

mod palette {
    use super::*;

    pub const BG: Color = rgb(8, 8, 8); // #080808
    pub const BG_PANEL: Color = rgb(14, 12, 8); // #0e0c08
    pub const BG_RAISED: Color = rgb(28, 22, 10); // #1c160a
    pub const BG_HOVER: Color = rgb(40, 30, 12); // #281e0c
    pub const AMBER: Color = rgb(255, 153, 0); // #ff9900
    pub const AMBER_BRIGHT: Color = rgb(255, 170, 0); // #ffaa00
    pub const AMBER_GOLD: Color = rgb(255, 204, 0); // #ffcc00
    pub const AMBER_DIM: Color = rgb(153, 96, 0); // #996000
    pub const AMBER_MUTED: Color = rgb(180, 130, 48); // #b48230
    pub const FG: Color = rgb(255, 184, 64); // #ffb840
    pub const FG_DIM: Color = rgb(168, 120, 40); // #a87828
    pub const COMMENT: Color = rgb(110, 88, 40); // #6e5828
    pub const GREEN: Color = rgb(0, 255, 102); // #00ff66
    pub const GREEN_DARK: Color = rgb(0, 56, 22); // #003816
    pub const RED: Color = rgb(255, 51, 51); // #ff3333
    pub const RED_DARK: Color = rgb(66, 12, 12); // #420c0c
    pub const CYAN: Color = rgb(0, 220, 180); // #00dcb4
}

use palette::*;

impl Theme {
    /// Bloomberg Terminal amber-on-black.
    pub const fn bloomberg() -> Self {
        Self {
            bg_base: BG_PANEL,
            bg_light: BG_RAISED,
            bg_dark: rgb(18, 14, 8),
            bg_highlight: BG_RAISED,
            bg_hover: BG_HOVER,
            bg_terminal: BG,

            accent_user: AMBER,
            accent_assistant: AMBER_BRIGHT,
            accent_thinking: AMBER_DIM,
            accent_tool: AMBER_MUTED,
            accent_system: AMBER_GOLD,
            accent_error: RED,
            accent_success: GREEN,
            accent_running: AMBER_GOLD,
            accent_skill: CYAN,

            text_primary: FG,
            text_secondary: FG_DIM,

            gray_dim: rgb(72, 56, 24),
            gray: COMMENT,
            gray_bright: AMBER_MUTED,

            command: AMBER_GOLD,
            path: AMBER,
            running: CYAN,
            warning: AMBER_GOLD,

            fuzzy_accent: AMBER_BRIGHT,

            accent_plan: AMBER_GOLD,
            accent_verify: AMBER_MUTED,
            accent_remember: GREEN,

            selection_border: AMBER_DIM,
            prompt_border: rgb(80, 52, 8),
            prompt_border_active: AMBER,
            hover_border: rgb(48, 32, 8),

            accent_model: AMBER_GOLD,

            scrollbar_bg: rgb(17, 14, 8),
            scrollbar_fg: AMBER_DIM,

            diff_delete_bg: RED_DARK,
            diff_delete_fg: RED,
            diff_insert_bg: GREEN_DARK,
            diff_insert_fg: GREEN,
            diff_equal_fg: COMMENT,
            diff_gutter_fg: COMMENT,

            bg_visual: BG_HOVER,

            paste_bg: BG,
            paste_fg: FG_DIM,
            paste_dim: COMMENT,

            md_heading_h1: AMBER_GOLD,
            md_heading_h1_mod: Modifier::BOLD,
            md_heading_h2: AMBER,
            md_heading_h2_mod: Modifier::BOLD,
            md_heading_h3: AMBER_BRIGHT,
            md_heading_h3_mod: Modifier::BOLD,
            md_heading_h4: AMBER_MUTED,
            md_heading_h4_mod: Modifier::BOLD,
            md_heading_h5: COMMENT,
            md_heading_h5_mod: Modifier::BOLD,
            md_heading_h6: COMMENT,
            md_heading_h6_mod: Modifier::empty(),
            md_code: CYAN,
            md_task_checked: GREEN,
            md_task_unchecked: FG_DIM,
            md_muted: COMMENT,
            md_code_bg: rgb(18, 14, 8),
            md_text: FG,
            link_fg: AMBER_BRIGHT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bloomberg_is_dark_amber() {
        let theme = Theme::bloomberg();
        assert!(theme.is_dark());
        assert!(matches!(theme.accent_user, Color::Rgb(255, 153, 0)));
    }
}
