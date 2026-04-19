//! Syntax highlighting via tree-sitter.
//!
//! Provides incremental parsing and **viewport-batched** highlighting.
//! Each buffer gets its own `SyntaxHighlighter` instance that maintains
//! a persistent syntax tree for efficient re-parsing on edits.
//!
//! ## Performance
//!
//! The key optimisation is `highlight_viewport()`: it collects the full
//! source text **once**, runs tree-sitter **once** for the visible range,
//! and slices the results per-line. This avoids the O(n × visible_lines)
//! allocation that per-line highlighting would cause.

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

    /// Highlight all visible lines in one batch.
    ///
    /// This is the primary rendering method. It collects the source text
    /// **once**, runs tree-sitter **once**, and builds styled `Line`s for
    /// each visible line. This is O(source_len) per call instead of
    /// O(source_len × visible_lines) as the old per-line method was.
    #[must_use]
    pub fn highlight_viewport(
        &self,
        rope: &Rope,
        scroll: usize,
        visible_lines: usize,
        default_style: Style,
    ) -> Vec<Line<'static>> {
        let line_count = rope.len_lines();
        let end_line = (scroll + visible_lines).min(line_count);
        let num_lines = end_line.saturating_sub(scroll);

        if num_lines == 0 {
            return Vec::new();
        }

        // Collect full source ONCE
        let full_source: String = rope.chunks().collect();
        let source_bytes = full_source.as_bytes();

        // Calculate byte ranges for each visible line
        let mut line_byte_ranges: Vec<(usize, usize)> = Vec::with_capacity(num_lines);
        for i in scroll..end_line {
            let start = rope.line_to_byte(i);
            let end = if i + 1 < line_count {
                rope.line_to_byte(i + 1)
            } else {
                rope.len_bytes()
            };
            line_byte_ranges.push((start, end));
        }

        // Extract line texts for fallback and span building
        let line_texts: Vec<String> = (scroll..end_line)
            .map(|i| {
                let line_slice = rope.line(i);
                line_slice.chunks().collect()
            })
            .collect();

        // Run tree-sitter ONCE for the entire source
        let mut highlighter = Highlighter::new();
        let Ok(events) = highlighter.highlight(
            &self.config,
            source_bytes,
            None,
            |_| None,
        ) else {
            // Fallback: return plain text for all lines
            return line_texts
                .into_iter()
                .map(|text| Line::from(Span::styled(text, default_style)))
                .collect();
        };

        // Allocate span buffers for each visible line
        let mut line_spans: Vec<Vec<Span<'static>>> = vec![Vec::new(); num_lines];
        let mut current_style = default_style;

        let viewport_start_byte = line_byte_ranges.first().map_or(0, |r| r.0);
        let viewport_end_byte = line_byte_ranges.last().map_or(0, |r| r.1);

        for event in events {
            match event {
                Ok(HighlightEvent::Source { start, end }) => {
                    // Skip events completely outside our viewport
                    if end <= viewport_start_byte || start >= viewport_end_byte {
                        continue;
                    }

                    // Distribute this source event across visible lines
                    for (line_idx, &(line_start, line_end)) in line_byte_ranges.iter().enumerate() {
                        if end <= line_start || start >= line_end {
                            continue;
                        }

                        let overlap_start = start.max(line_start);
                        let overlap_end = end.min(line_end);

                        if overlap_start < overlap_end {
                            let local_start = overlap_start - line_start;
                            let local_end = overlap_end - line_start;

                            if let Some(text) = line_texts[line_idx].get(local_start..local_end) {
                                if !text.is_empty() {
                                    line_spans[line_idx].push(Span::styled(
                                        text.to_string(),
                                        current_style,
                                    ));
                                }
                            }
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
                Err(_) => break,
            }
        }

        // Build final Lines: use spans if available, otherwise fallback to plain text
        line_spans
            .into_iter()
            .zip(line_texts)
            .map(|(spans, text)| {
                if spans.is_empty() {
                    Line::from(Span::styled(text, default_style))
                } else {
                    Line::from(spans)
                }
            })
            .collect()
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
    fn test_highlight_rust_viewport() {
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

        // Highlight entire viewport at once
        let lines = highlighter.highlight_viewport(&rope, 0, rope.len_lines(), default_style);
        assert_eq!(lines.len(), rope.len_lines());
        for (i, line) in lines.iter().enumerate() {
            assert!(!line.spans.is_empty(), "Line {i} should have spans");
        }
    }

    #[test]
    fn test_highlight_viewport_partial() {
        let theme = Theme::catppuccin_mocha();
        let highlighter = SyntaxHighlighter::for_extension("rs", &theme)
            .expect("Rust highlighter should be available");

        let source = "fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\n";
        let rope = Rope::from_str(source);
        let default_style = Style::default();

        // Only highlight lines 1-2 (scroll=1, visible=2)
        let lines = highlighter.highlight_viewport(&rope, 1, 2, default_style);
        assert_eq!(lines.len(), 2);
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
