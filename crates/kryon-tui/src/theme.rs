//! Color theme system for the Kryon editor.
//!
//! Provides named color palettes. The initial theme is Catppuccin Mocha,
//! a popular dark theme with excellent readability and contrast.

use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;

/// A named set of colors for the entire editor UI.
#[derive(Debug, Clone)]
pub struct Theme {
    // ── UI Colors ──────────────────────────────────────────────
    /// Background color for the editor area.
    pub bg: Color,
    /// Primary foreground text color.
    pub fg: Color,
    /// Muted/dimmed text color (comments, inactive UI).
    pub fg_muted: Color,
    /// Line number gutter text color.
    pub gutter_fg: Color,
    /// Gutter background.
    pub gutter_bg: Color,
    /// Status bar background.
    pub statusbar_bg: Color,
    /// Status bar foreground.
    pub statusbar_fg: Color,
    /// Sidebar background.
    pub sidebar_bg: Color,
    /// Sidebar foreground.
    pub sidebar_fg: Color,
    /// Sidebar border.
    pub sidebar_border: Color,
    /// Active/selected item highlight.
    pub accent: Color,
    /// Warning/modified indicator.
    pub warning: Color,
    /// Error color.
    pub error: Color,
    /// Cursor line highlight background.
    pub cursor_line_bg: Color,
    /// Selection background.
    pub selection_bg: Color,
    /// Tab bar background.
    pub tab_bg: Color,
    /// Active tab foreground.
    pub tab_active_fg: Color,
    /// Inactive tab foreground.
    pub tab_inactive_fg: Color,

    // ── Syntax Highlighting Colors ─────────────────────────────
    /// Language keywords (`fn`, `let`, `if`, `struct`, etc.).
    pub keyword: Color,
    /// String literals.
    pub string: Color,
    /// Comments.
    pub comment: Color,
    /// Function names.
    pub function: Color,
    /// Type names (structs, enums, traits).
    pub type_name: Color,
    /// Variable names.
    pub variable: Color,
    /// Numeric literals.
    pub number: Color,
    /// Operators (`+`, `-`, `=`, etc.).
    pub operator: Color,
    /// Properties / fields.
    pub property: Color,
    /// Punctuation (brackets, semicolons).
    pub punctuation: Color,
    /// Attributes / decorators (`#[derive]`, `@Override`).
    pub attribute: Color,
    /// Constants and boolean literals.
    pub constant: Color,
}

impl Theme {
    /// Build a mapping from tree-sitter capture names to ratatui Styles.
    ///
    /// This is used by the syntax highlighter to convert parsed capture names
    /// into concrete terminal styles.
    #[must_use]
    pub fn syntax_style_map(&self) -> HashMap<&'static str, Style> {
        let mut map = HashMap::new();
        map.insert("keyword", Style::default().fg(self.keyword));
        map.insert("keyword.function", Style::default().fg(self.keyword));
        map.insert("keyword.control", Style::default().fg(self.keyword));
        map.insert("keyword.operator", Style::default().fg(self.keyword));
        map.insert("keyword.return", Style::default().fg(self.keyword));
        map.insert("string", Style::default().fg(self.string));
        map.insert("string.special", Style::default().fg(self.string));
        map.insert("comment", Style::default().fg(self.comment).add_modifier(Modifier::ITALIC));
        map.insert("comment.line", Style::default().fg(self.comment).add_modifier(Modifier::ITALIC));
        map.insert("comment.block", Style::default().fg(self.comment).add_modifier(Modifier::ITALIC));
        map.insert("function", Style::default().fg(self.function));
        map.insert("function.method", Style::default().fg(self.function));
        map.insert("function.builtin", Style::default().fg(self.function));
        map.insert("function.macro", Style::default().fg(self.function));
        map.insert("type", Style::default().fg(self.type_name));
        map.insert("type.builtin", Style::default().fg(self.type_name));
        map.insert("constructor", Style::default().fg(self.type_name));
        map.insert("variable", Style::default().fg(self.variable));
        map.insert("variable.parameter", Style::default().fg(self.variable));
        map.insert("variable.builtin", Style::default().fg(self.variable));
        map.insert("number", Style::default().fg(self.number));
        map.insert("float", Style::default().fg(self.number));
        map.insert("operator", Style::default().fg(self.operator));
        map.insert("property", Style::default().fg(self.property));
        map.insert("punctuation", Style::default().fg(self.punctuation));
        map.insert("punctuation.bracket", Style::default().fg(self.punctuation));
        map.insert("punctuation.delimiter", Style::default().fg(self.punctuation));
        map.insert("attribute", Style::default().fg(self.attribute));
        map.insert("constant", Style::default().fg(self.constant));
        map.insert("constant.builtin", Style::default().fg(self.constant));
        map.insert("boolean", Style::default().fg(self.constant));
        map.insert("label", Style::default().fg(self.accent));
        map.insert("namespace", Style::default().fg(self.type_name));
        map.insert("module", Style::default().fg(self.type_name));
        map.insert("escape", Style::default().fg(self.accent));
        map
    }

    /// Catppuccin Mocha — a warm dark theme.
    #[must_use]
    pub fn catppuccin_mocha() -> Self {
        Self {
            // UI
            bg: Color::Rgb(30, 30, 46),           // Base
            fg: Color::Rgb(205, 214, 244),         // Text
            fg_muted: Color::Rgb(147, 153, 178),   // Overlay1
            gutter_fg: Color::Rgb(88, 91, 112),    // Surface2
            gutter_bg: Color::Rgb(30, 30, 46),     // Base
            statusbar_bg: Color::Rgb(24, 24, 37),  // Mantle
            statusbar_fg: Color::Rgb(186, 194, 222), // Subtext1
            sidebar_bg: Color::Rgb(24, 24, 37),    // Mantle
            sidebar_fg: Color::Rgb(166, 173, 200), // Subtext0
            sidebar_border: Color::Rgb(69, 71, 90), // Surface1
            accent: Color::Rgb(137, 180, 250),     // Blue
            warning: Color::Rgb(249, 226, 175),    // Yellow
            error: Color::Rgb(243, 139, 168),      // Red
            cursor_line_bg: Color::Rgb(45, 45, 65), // Surface0+
            selection_bg: Color::Rgb(69, 71, 90),  // Surface1
            tab_bg: Color::Rgb(24, 24, 37),        // Mantle
            tab_active_fg: Color::Rgb(205, 214, 244), // Text
            tab_inactive_fg: Color::Rgb(88, 91, 112), // Surface2
            // Syntax — Catppuccin Mocha palette
            keyword: Color::Rgb(203, 166, 247),    // Mauve
            string: Color::Rgb(166, 227, 161),     // Green
            comment: Color::Rgb(108, 112, 134),    // Overlay0
            function: Color::Rgb(137, 180, 250),   // Blue
            type_name: Color::Rgb(249, 226, 175),  // Yellow
            variable: Color::Rgb(205, 214, 244),   // Text
            number: Color::Rgb(250, 179, 135),     // Peach
            operator: Color::Rgb(148, 226, 213),   // Teal
            property: Color::Rgb(180, 190, 254),   // Lavender
            punctuation: Color::Rgb(147, 153, 178), // Overlay1
            attribute: Color::Rgb(249, 226, 175),  // Yellow
            constant: Color::Rgb(250, 179, 135),   // Peach
        }
    }

    /// Tokyo Night — a cool-toned dark theme.
    #[must_use]
    pub fn tokyo_night() -> Self {
        Self {
            // UI
            bg: Color::Rgb(26, 27, 38),
            fg: Color::Rgb(192, 202, 245),
            fg_muted: Color::Rgb(86, 95, 137),
            gutter_fg: Color::Rgb(59, 66, 97),
            gutter_bg: Color::Rgb(26, 27, 38),
            statusbar_bg: Color::Rgb(22, 22, 30),
            statusbar_fg: Color::Rgb(169, 177, 214),
            sidebar_bg: Color::Rgb(22, 22, 30),
            sidebar_fg: Color::Rgb(169, 177, 214),
            sidebar_border: Color::Rgb(41, 46, 66),
            accent: Color::Rgb(122, 162, 247),
            warning: Color::Rgb(224, 175, 104),
            error: Color::Rgb(247, 118, 142),
            cursor_line_bg: Color::Rgb(41, 46, 66),
            selection_bg: Color::Rgb(41, 46, 66),
            tab_bg: Color::Rgb(22, 22, 30),
            tab_active_fg: Color::Rgb(192, 202, 245),
            tab_inactive_fg: Color::Rgb(59, 66, 97),
            // Syntax — Tokyo Night palette
            keyword: Color::Rgb(187, 154, 247),    // Purple
            string: Color::Rgb(158, 206, 106),     // Green
            comment: Color::Rgb(86, 95, 137),      // Dark blue-gray
            function: Color::Rgb(122, 162, 247),   // Blue
            type_name: Color::Rgb(42, 195, 222),   // Cyan
            variable: Color::Rgb(192, 202, 245),   // Foreground
            number: Color::Rgb(255, 158, 100),     // Orange
            operator: Color::Rgb(137, 221, 255),   // Light cyan
            property: Color::Rgb(115, 218, 202),   // Teal
            punctuation: Color::Rgb(86, 95, 137),  // Dark blue-gray
            attribute: Color::Rgb(224, 175, 104),  // Yellow
            constant: Color::Rgb(255, 158, 100),   // Orange
        }
    }

    /// Dracula — a vibrant dark theme.
    #[must_use]
    pub fn dracula() -> Self {
        Self {
            // UI
            bg: Color::Rgb(40, 42, 54),
            fg: Color::Rgb(248, 248, 242),
            fg_muted: Color::Rgb(98, 114, 164),
            gutter_fg: Color::Rgb(98, 114, 164),
            gutter_bg: Color::Rgb(40, 42, 54),
            statusbar_bg: Color::Rgb(68, 71, 90),
            statusbar_fg: Color::Rgb(248, 248, 242),
            sidebar_bg: Color::Rgb(33, 34, 44),
            sidebar_fg: Color::Rgb(248, 248, 242),
            sidebar_border: Color::Rgb(68, 71, 90),
            accent: Color::Rgb(139, 233, 253),
            warning: Color::Rgb(241, 250, 140),
            error: Color::Rgb(255, 85, 85),
            cursor_line_bg: Color::Rgb(68, 71, 90),
            selection_bg: Color::Rgb(68, 71, 90),
            tab_bg: Color::Rgb(33, 34, 44),
            tab_active_fg: Color::Rgb(248, 248, 242),
            tab_inactive_fg: Color::Rgb(98, 114, 164),
            // Syntax — Dracula palette
            keyword: Color::Rgb(255, 121, 198),    // Pink
            string: Color::Rgb(241, 250, 140),     // Yellow
            comment: Color::Rgb(98, 114, 164),     // Comment blue
            function: Color::Rgb(80, 250, 123),    // Green
            type_name: Color::Rgb(139, 233, 253),  // Cyan
            variable: Color::Rgb(248, 248, 242),   // Foreground
            number: Color::Rgb(189, 147, 249),     // Purple
            operator: Color::Rgb(255, 121, 198),   // Pink
            property: Color::Rgb(139, 233, 253),   // Cyan
            punctuation: Color::Rgb(248, 248, 242), // Foreground
            attribute: Color::Rgb(80, 250, 123),   // Green
            constant: Color::Rgb(189, 147, 249),   // Purple
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::catppuccin_mocha()
    }
}
