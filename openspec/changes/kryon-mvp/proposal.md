# Proposal: Kryon Editor Sprint 3 — Syntax Highlighting, File Operations & Mouse Support

**Change ID**: `kryon-mvp`  
**Author**: Architect Agent  
**Date**: 2026-04-19  
**Status**: Proposed  

## Problem Statement

The Kryon editor has a functional core engine (buffer, cursor, command pattern, undo/redo) and a basic TUI shell (editor area, sidebar, status bar, key bindings, theme system). However, it lacks three critical features that prevent it from being a usable daily-driver editor:

1. **No syntax highlighting** — All code appears as plain monochrome text
2. **Incomplete file operations** — No Save As, no New File, no Tab management
3. **No mouse support** — Users cannot click to position cursor or scroll

## Proposed Solution

### A. Tree-sitter Syntax Highlighting

Integrate the `tree-sitter` and `tree-sitter-highlight` crates to provide incremental, error-tolerant syntax highlighting. The implementation will:

- Maintain a persistent syntax tree per buffer
- Use `Tree.edit()` + incremental re-parse on every text change
- Limit highlight queries to the visible viewport (`QueryCursor::set_byte_range`)
- Map tree-sitter capture names to theme colors via semantic tokens
- Support Rust as initial language, with architecture for adding more

### B. Complete File Operations

- **Open from CLI**: Already done (canonicalized path tracking)
- **Save (Ctrl+S)**: Already done (writes buffer to tracked path)
- **Save As**: Prompt-based path entry for untitled buffers
- **New Buffer**: Create empty buffer tab
- **Close Tab**: Remove buffer with unsaved-changes guard
- **Tab Bar**: Visual tab strip for switching between open buffers

### C. Mouse Support

- **Click to position cursor**: Map terminal mouse coords to buffer position
- **Scroll**: Mouse wheel maps to scroll-up/scroll-down
- **Drag select**: Extend selection from click point to drag point

## Impact Analysis

| Component | Files Modified | Risk |
|-----------|---------------|------|
| `kryon-core` | `buffer.rs` (tree-sitter integration), `event.rs` (new actions) | Medium |
| `kryon-tui` | `app.rs`, `theme.rs`, NEW `highlight.rs`, NEW `tab_bar.rs` | High |
| `kryon-cli` | `main.rs` (minor) | Low |

## Success Criteria

1. Opening a `.rs` file shows colored syntax highlighting
2. Ctrl+S saves, status bar confirms
3. Mouse click positions cursor at click location
4. Mouse scroll moves viewport
5. All existing tests continue to pass
6. New tests for highlight pipeline and mouse coordinate mapping

## Research References

- [Deep Research](../references/rust-editor-research.md) — Sections 3, 5, 8
- Helix tree-house architecture for incremental highlighting
- Ratatui StatefulWidget pattern for scrollable editor
