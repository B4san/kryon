//! Tab bar widget for displaying open buffer tabs.

#![allow(clippy::cast_possible_truncation)]

use ratatui::prelude::*;
use ratatui::widgets::Widget;

use crate::theme::Theme;

/// A single tab entry for the tab bar.
pub struct TabEntry {
    /// Display name (filename or "Untitled").
    pub name: String,
    /// Whether this buffer has unsaved changes.
    pub modified: bool,
    /// Whether this is the active/focused tab.
    pub active: bool,
}

/// Tab bar widget that renders a horizontal strip of buffer tabs.
pub struct TabBar<'a> {
    /// Tab entries to render.
    pub tabs: &'a [TabEntry],
    /// Color theme reference.
    pub theme: &'a Theme,
}

impl Widget for TabBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Fill background
        for x in area.x..area.x + area.width {
            for y in area.y..area.y + area.height {
                buf[(x, y)].set_char(' ').set_bg(self.theme.tab_bg);
            }
        }

        let mut x_offset = area.x;

        for tab in self.tabs {
            // Build tab label
            let modified_indicator = if tab.modified { " ●" } else { "" };
            let label = format!(" {}{modified_indicator} ", tab.name);
            let label_width = label.len() as u16;

            // Check if we have room
            if x_offset + label_width > area.x + area.width {
                break;
            }

            let style = if tab.active {
                Style::default()
                    .fg(self.theme.tab_active_fg)
                    .bg(self.theme.bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(self.theme.tab_inactive_fg)
                    .bg(self.theme.tab_bg)
            };

            // Render each character of the label
            for (i, ch) in label.chars().enumerate() {
                let x = x_offset + i as u16;
                if x < area.x + area.width {
                    buf[(x, area.y)].set_char(ch).set_style(style);
                }
            }

            // Separator between tabs
            x_offset += label_width;
            if x_offset < area.x + area.width {
                buf[(x_offset, area.y)]
                    .set_char('│')
                    .set_fg(self.theme.sidebar_border)
                    .set_bg(self.theme.tab_bg);
                x_offset += 1;
            }
        }
    }
}
