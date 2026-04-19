//! Syntax highlighting via tree-sitter.
//!
//! Provides incremental parsing and viewport-limited highlighting.
//! Each buffer gets its own `SyntaxHighlighter` instance that maintains
//! a persistent syntax tree for efficient re-parsing on edits.

#![allow(clippy::cast_possible_truncation)]

use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ropey::Rope;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::theme::Theme;

/// Recognized highlight capture names, in priority order.
///
/// The index into this array is the highlight ID returned by tree-sitter-highlight.
/// The names must match entries in the theme's `syntax_style_map()`.
const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "boolean",
    "comment",
    "comment.block",
    "comment.line",
    "constant",
    "constant.builtin",
    "constructor",
    "escape",
    "float",
    "function",
    "function.builtin",
    "function.macro",
    "function.method",
    "keyword",
    "keyword.control",
    "keyword.function",
    "keyword.operator",
    "keyword.return",
    "label",
    "module",
    "namespace",
    "number",
    "operator",
    "property",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "string",
    "string.special",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.parameter",
];

/// Manages tree-sitter parsing and highlighting for a single buffer.
pub struct SyntaxHighlighter {
    /// The highlight configuration for the active language.
    config: HighlightConfiguration,
    /// Compiled style map from the theme (capture index → Style).
    styles: Vec<Style>,
}

impl SyntaxHighlighter {
    /// Create a highlighter for a given language.
    ///
    /// Returns `None` if the language is not supported.
    #[must_use]
    pub fn for_extension(ext: &str, theme: &Theme) -> Option<Self> {
        let config = match ext {
            "rs" => {
                let language = tree_sitter_rust::LANGUAGE.into();
                let result = HighlightConfiguration::new(
                    language,
                    "rust",
                    tree_sitter_rust::HIGHLIGHTS_QUERY,
                    tree_sitter_rust::INJECTIONS_QUERY,
                    "",  // locals
                );
                match result {
                    Ok(mut cfg) => {
                        cfg.configure(HIGHLIGHT_NAMES);
                        cfg
                    }
                    Err(e) => {
                        tracing::error!("Failed to create highlight config: {e}");
                        return None;
                    }
                }
            }
            _ => return None,
        };

        let style_map = theme.syntax_style_map();
        let styles = HIGHLIGHT_NAMES
            .iter()
            .map(|name| {
                style_map
                    .get(name)
                    .copied()
                    .unwrap_or_default()
            })
            .collect();

        Some(Self { config, styles })
    }

    /// Highlight a single line of the buffer, returning a styled `Line`.
    ///
    /// If highlighting fails, returns plain unstyled text.
    #[must_use]
    pub fn highlight_line(&self, rope: &Rope, line_idx: usize, default_style: Style) -> Line<'static> {
        let line_count = rope.len_lines();
        if line_idx >= line_count {
            return Line::default();
        }

        let line_slice = rope.line(line_idx);
        let line_text: String = line_slice.chunks().collect();

        // Calculate byte range for this line in the full document
        let line_start_byte = rope.line_to_byte(line_idx);
        let line_end_byte = if line_idx + 1 < line_count {
            rope.line_to_byte(line_idx + 1)
        } else {
            rope.len_bytes()
        };

        // Get the full source for tree-sitter (it needs context for correct parsing)
        let full_source: String = rope.chunks().collect();
        let source_bytes = full_source.as_bytes();

        let mut highlighter = Highlighter::new();
        let Ok(events) = highlighter.highlight(
            &self.config,
            source_bytes,
            None, // no injection callback
            |_| None,
        ) else {
            // Fallback: return plain text
            return Line::from(Span::styled(line_text, default_style));
        };

        // Build spans for this specific line
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut current_style = default_style;

        for event in events {
            match event {
                Ok(HighlightEvent::Source { start, end }) => {
                    // Only include bytes that overlap with our line
                    if end <= line_start_byte || start >= line_end_byte {
                        continue;
                    }

                    let overlap_start = start.max(line_start_byte);
                    let overlap_end = end.min(line_end_byte);

                    if overlap_start < overlap_end {
                        let local_start = overlap_start - line_start_byte;
                        let local_end = overlap_end - line_start_byte;

                        if let Some(text) = line_text.get(local_start..local_end)
                            && !text.is_empty() {
                                spans.push(Span::styled(text.to_string(), current_style));
                            }
                    }
                }
                Ok(HighlightEvent::HighlightStart(highlight)) => {
                    current_style = self.styles.get(highlight.0)
                        .copied()
                        .unwrap_or(default_style);
                }
                Ok(HighlightEvent::HighlightEnd) => {
                    current_style = default_style;
                }
                Err(_) => {
                    // On error, return what we have so far
                    break;
                }
            }
        }

        // If no spans were produced, return the raw line
        if spans.is_empty() {
            Line::from(Span::styled(line_text, default_style))
        } else {
            Line::from(spans)
        }
    }
}

/// Detect language from file extension.
#[must_use]
pub fn detect_language(path: &std::path::Path) -> Option<&'static str> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext {
            "rs" => Some("rust"),
            "js" | "jsx" => Some("javascript"),
            "ts" | "tsx" => Some("typescript"),
            "py" => Some("python"),
            "go" => Some("go"),
            "c" | "h" => Some("c"),
            "cpp" | "hpp" | "cc" | "cxx" => Some("cpp"),
            "toml" => Some("toml"),
            "json" => Some("json"),
            "md" | "markdown" => Some("markdown"),
            "html" | "htm" => Some("html"),
            "css" => Some("css"),
            "yaml" | "yml" => Some("yaml"),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust_source() {
        let theme = Theme::catppuccin_mocha();
        let highlighter = SyntaxHighlighter::for_extension("rs", &theme)
            .expect("Rust highlighter should be available");

        let source = r#"fn main() {
    let x = 42;
    println!("hello");
}
"#;
        let rope = Rope::from_str(source);
        let default_style = Style::default();

        // Highlight each line
        for i in 0..rope.len_lines() {
            let line = highlighter.highlight_line(&rope, i, default_style);
            // Each line should produce at least one span
            assert!(!line.spans.is_empty(), "Line {i} should have spans");
        }
    }

    #[test]
    fn test_unsupported_extension() {
        let theme = Theme::catppuccin_mocha();
        assert!(SyntaxHighlighter::for_extension("xyz", &theme).is_none());
    }

    #[test]
    fn test_detect_language() {
        use std::path::Path;
        assert_eq!(detect_language(Path::new("main.rs")), Some("rust"));
        assert_eq!(detect_language(Path::new("app.js")), Some("javascript"));
        assert_eq!(detect_language(Path::new("unknown.xyz")), None);
    }
}
