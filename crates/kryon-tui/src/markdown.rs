//! Markdown preview renderer for the editor.
//!
//! Converts raw markdown text into styled `Line`s for ratatui rendering.
//! Supports headers, bold, italic, code spans, code blocks, lists,
//! blockquotes, horizontal rules, and links.

#![allow(clippy::cast_possible_truncation)]

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::Theme;

/// State for multi-line block parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BlockState {
    /// Normal text.
    Normal,
    /// Inside a fenced code block.
    CodeBlock,
}

/// Renders markdown text into styled ratatui Lines.
pub struct MarkdownRenderer<'a> {
    theme: &'a Theme,
}

impl<'a> MarkdownRenderer<'a> {
    /// Create a new renderer with the given theme.
    #[must_use]
    pub fn new(theme: &'a Theme) -> Self {
        Self { theme }
    }

    /// Render the full buffer text into styled Lines for display.
    #[must_use]
    pub fn render(&self, text: &str, scroll: usize, visible_lines: usize) -> Vec<Line<'static>> {
        let all_lines: Vec<&str> = text.lines().collect();
        let end = (scroll + visible_lines).min(all_lines.len());

        if scroll >= all_lines.len() {
            return Vec::new();
        }

        let mut result: Vec<Line<'static>> = Vec::with_capacity(visible_lines);
        let mut state = BlockState::Normal;

        // We need to track state from the start of the file to know if we're
        // in a code block at the scroll position
        for line in all_lines.iter().take(scroll) {
            if line.starts_with("```") {
                state = match state {
                    BlockState::Normal => BlockState::CodeBlock,
                    BlockState::CodeBlock => BlockState::Normal,
                };
            }
        }

        // Now render visible lines
        for line in &all_lines[scroll..end] {
            let rendered = self.render_line(line, &mut state);
            result.push(rendered);
        }

        result
    }

    /// Render a single line of markdown.
    fn render_line(&self, line: &str, state: &mut BlockState) -> Line<'static> {
        // Check for code block fence
        if line.starts_with("```") {
            match *state {
                BlockState::Normal => {
                    *state = BlockState::CodeBlock;
                    let lang = line.trim_start_matches('`').trim();
                    let label = if lang.is_empty() {
                        "── code ──────────────────────".to_string()
                    } else {
                        format!("── {lang} ──────────────────────")
                    };
                    return Line::from(Span::styled(
                        label,
                        Style::default()
                            .fg(self.theme.accent)
                            .add_modifier(Modifier::DIM),
                    ));
                }
                BlockState::CodeBlock => {
                    *state = BlockState::Normal;
                    return Line::from(Span::styled(
                        "──────────────────────────────".to_string(),
                        Style::default()
                            .fg(self.theme.accent)
                            .add_modifier(Modifier::DIM),
                    ));
                }
            }
        }

        // Inside a code block: render as-is with code style
        if *state == BlockState::CodeBlock {
            return Line::from(Span::styled(
                format!("  {line}"),
                Style::default()
                    .fg(self.theme.string)
                    .add_modifier(Modifier::DIM),
            ));
        }

        // Horizontal rule
        let trimmed = line.trim();
        if (trimmed.starts_with("---") || trimmed.starts_with("***") || trimmed.starts_with("___"))
            && trimmed.chars().all(|c| c == '-' || c == '*' || c == '_' || c == ' ')
            && trimmed.len() >= 3
        {
            return Line::from(Span::styled(
                "─".repeat(40),
                Style::default().fg(self.theme.fg_muted),
            ));
        }

        // Headers
        if let Some(rest) = line.strip_prefix("# ") {
            return Line::from(Span::styled(
                format!("  ◆ {rest}"),
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        if let Some(rest) = line.strip_prefix("## ") {
            return Line::from(Span::styled(
                format!("  ◇ {rest}"),
                Style::default()
                    .fg(self.theme.keyword)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        if let Some(rest) = line.strip_prefix("### ") {
            return Line::from(Span::styled(
                format!("  ▸ {rest}"),
                Style::default()
                    .fg(self.theme.function)
                    .add_modifier(Modifier::BOLD),
            ));
        }
        if let Some(rest) = line.strip_prefix("#### ") {
            return Line::from(Span::styled(
                format!("    {rest}"),
                Style::default()
                    .fg(self.theme.type_name)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        // Blockquote
        if let Some(rest) = line.strip_prefix("> ") {
            return Line::from(vec![
                Span::styled("  │ ", Style::default().fg(self.theme.accent)),
                Span::styled(
                    rest.to_string(),
                    Style::default()
                        .fg(self.theme.fg_muted)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]);
        }

        // Unordered list
        if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            let indent = line.len() - line.trim_start().len();
            let prefix = " ".repeat(indent);
            return Line::from(vec![
                Span::styled(format!("{prefix}  • "), Style::default().fg(self.theme.accent)),
                Span::styled(rest.to_string(), Style::default().fg(self.theme.fg)),
            ]);
        }

        // Ordered list (basic: starts with digit + ". ")
        if let Some(dot_pos) = trimmed.find(". ") {
            if dot_pos > 0 && dot_pos <= 3 && trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
                let num = &trimmed[..dot_pos];
                let rest = &trimmed[dot_pos + 2..];
                let indent = line.len() - line.trim_start().len();
                let prefix = " ".repeat(indent);
                return Line::from(vec![
                    Span::styled(
                        format!("{prefix}  {num}. "),
                        Style::default().fg(self.theme.accent),
                    ),
                    Span::styled(rest.to_string(), Style::default().fg(self.theme.fg)),
                ]);
            }
        }

        // Inline formatting: parse bold, italic, code, and links
        let spans = self.parse_inline(line);
        Line::from(spans)
    }

    /// Parse inline markdown formatting into spans.
    fn parse_inline(&self, text: &str) -> Vec<Span<'static>> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut chars = text.char_indices().peekable();
        let mut current = String::new();
        let default_style = Style::default().fg(self.theme.fg);

        while let Some((i, c)) = chars.next() {
            match c {
                '`' => {
                    // Code span
                    if !current.is_empty() {
                        spans.push(Span::styled(current.clone(), default_style));
                        current.clear();
                    }
                    let mut code = String::new();
                    let mut found_end = false;
                    for (_, cc) in chars.by_ref() {
                        if cc == '`' {
                            found_end = true;
                            break;
                        }
                        code.push(cc);
                    }
                    if found_end {
                        spans.push(Span::styled(
                            format!(" {code} "),
                            Style::default().fg(self.theme.string),
                        ));
                    } else {
                        current.push('`');
                        current.push_str(&code);
                    }
                }
                '*' | '_' => {
                    // Check for bold (**) or italic (*)
                    let next = chars.peek().map(|(_, nc)| *nc);
                    if next == Some(c) {
                        // Bold **text**
                        chars.next(); // consume second *
                        if !current.is_empty() {
                            spans.push(Span::styled(current.clone(), default_style));
                            current.clear();
                        }
                        let mut bold_text = String::new();
                        let mut found_end = false;
                        while let Some((_, bc)) = chars.next() {
                            if bc == c {
                                if chars.peek().map(|(_, nc)| *nc) == Some(c) {
                                    chars.next();
                                    found_end = true;
                                    break;
                                }
                            }
                            bold_text.push(bc);
                        }
                        if found_end {
                            spans.push(Span::styled(
                                bold_text,
                                Style::default()
                                    .fg(self.theme.fg)
                                    .add_modifier(Modifier::BOLD),
                            ));
                        } else {
                            current.push(c);
                            current.push(c);
                            current.push_str(&bold_text);
                        }
                    } else {
                        // Italic *text*
                        if !current.is_empty() {
                            spans.push(Span::styled(current.clone(), default_style));
                            current.clear();
                        }
                        let mut italic_text = String::new();
                        let mut found_end = false;
                        for (_, ic) in chars.by_ref() {
                            if ic == c {
                                found_end = true;
                                break;
                            }
                            italic_text.push(ic);
                        }
                        if found_end && !italic_text.is_empty() {
                            spans.push(Span::styled(
                                italic_text,
                                Style::default()
                                    .fg(self.theme.fg)
                                    .add_modifier(Modifier::ITALIC),
                            ));
                        } else {
                            current.push(c);
                            current.push_str(&italic_text);
                        }
                    }
                }
                '[' => {
                    // Link: [text](url)
                    if !current.is_empty() {
                        spans.push(Span::styled(current.clone(), default_style));
                        current.clear();
                    }
                    let mut link_text = String::new();
                    let mut found_bracket = false;
                    for (_, lc) in chars.by_ref() {
                        if lc == ']' {
                            found_bracket = true;
                            break;
                        }
                        link_text.push(lc);
                    }
                    if found_bracket && chars.peek().map(|(_, nc)| *nc) == Some('(') {
                        chars.next(); // consume (
                        let mut url = String::new();
                        let mut found_paren = false;
                        for (_, uc) in chars.by_ref() {
                            if uc == ')' {
                                found_paren = true;
                                break;
                            }
                            url.push(uc);
                        }
                        if found_paren {
                            spans.push(Span::styled(
                                link_text,
                                Style::default()
                                    .fg(self.theme.accent)
                                    .add_modifier(Modifier::UNDERLINED),
                            ));
                        } else {
                            current.push('[');
                            current.push_str(&link_text);
                            current.push_str("](");
                            current.push_str(&url);
                        }
                    } else {
                        current.push('[');
                        current.push_str(&link_text);
                        if found_bracket {
                            current.push(']');
                        }
                    }
                }
                _ => {
                    let _ = i; // suppress unused warning
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            spans.push(Span::styled(current, default_style));
        }

        if spans.is_empty() {
            spans.push(Span::styled(String::new(), default_style));
        }

        spans
    }
}
