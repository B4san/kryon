# Design: Sprint 3 — Syntax Highlighting, File Operations & Mouse Support

## Architecture Decisions

### 1. Syntax Highlighting Pipeline

```
                     ┌─────────────────────┐
  Text Edit ────────►│   tree-sitter       │
  (from buffer)      │   Parser + Tree     │
                     └─────────┬───────────┘
                               │
                     ┌─────────▼───────────┐
                     │  HighlightConfig    │
                     │  (captures.scm)     │
                     └─────────┬───────────┘
                               │
                     ┌─────────▼───────────┐
                     │  Theme Mapping      │
                     │  capture → Style    │
                     └─────────┬───────────┘
                               │
                     ┌─────────▼───────────┐
                     │  Ratatui Spans      │
                     │  (styled text)      │
                     └─────────────────────┘
```

**Module**: `kryon-tui/src/highlight.rs`

```rust
pub struct SyntaxHighlighter {
    parser: tree_sitter::Parser,
    tree: Option<tree_sitter::Tree>,
    config: tree_sitter_highlight::HighlightConfiguration,
}

impl SyntaxHighlighter {
    pub fn new(language: tree_sitter::Language) -> Self;
    pub fn update(&mut self, rope: &Rope, edit: Option<InputEdit>);
    pub fn highlight_line(&self, rope: &Rope, line_idx: usize, theme: &Theme) -> Line<'static>;
}
```

**Key Design Choices**:
- Parser and Tree are **reused** across edits (not recreated)
- `update()` calls `tree.edit()` then `parser.parse()` for incremental updates
- `highlight_line()` returns styled `ratatui::text::Line` for rendering
- Theme mapping is done via a `HashMap<&str, Style>` built from theme colors

### 2. Theme Extension for Syntax Colors

**Module**: `kryon-tui/src/theme.rs` (extended)

Add syntax-specific colors to the Theme struct:
```rust
pub struct Theme {
    // ... existing UI colors ...
    
    // Syntax highlighting colors
    pub keyword: Color,
    pub string: Color,
    pub comment: Color,
    pub function: Color,
    pub type_name: Color,
    pub variable: Color,
    pub number: Color,
    pub operator: Color,
    pub property: Color,
    pub punctuation: Color,
    pub attribute: Color,
    pub constant: Color,
}
```

### 3. Mouse Event Handling

**Flow**:
```
Mouse Event (crossterm) → Map to EditorAction → App::update() → State Change
```

**Coordinate Mapping**:
```
terminal_col → buffer_col (subtract gutter width, sidebar width)
terminal_row → buffer_line (add scroll_offset, subtract editor area top)
```

**New EditorActions**:
```rust
enum EditorAction {
    // ... existing ...
    MouseClick { line: usize, col: usize },
    MouseDrag { line: usize, col: usize },
    MouseScroll { direction: ScrollDirection, amount: usize },
}
```

### 4. Tab Bar Widget

**Module**: `kryon-tui/src/tab_bar.rs`

```
┌──────────┬──────────┬──────────┬────────────────────────┐
│ main.rs  │ lib.rs ● │ + New    │                        │
└──────────┴──────────┴──────────┴────────────────────────┘
```

- Each tab shows filename (or "Untitled")
- `●` dot indicates unsaved changes
- Active tab is highlighted with accent color
- `+` button creates new buffer (Ctrl+N)

### 5. File State Per Buffer

Currently `App` has a single `file_path`. Refactor to per-buffer state:

```rust
pub struct BufferState {
    pub buffer: TextBuffer,
    pub cursors: CursorSet,
    pub history: CommandHistory,
    pub file_path: Option<PathBuf>,
    pub highlighter: Option<SyntaxHighlighter>,
}
```

This bundles buffer + cursor + undo history + file path + syntax tree into one coherent unit.

## Data Flow

```
User Input → crossterm::Event
  ↓
App::map_key() or App::map_mouse()
  ↓
EditorAction enum
  ↓
App::update(action)
  ├─ Modify active BufferState
  ├─ Update syntax tree (highlighter.update())
  └─ Adjust scroll/cursor
  ↓
App::draw(frame)
  ├─ Tab bar
  ├─ Sidebar (explorer)
  ├─ Editor (line numbers + highlighted text)
  └─ Status bar
```

## File Changes

| File | Action | Description |
|------|--------|-------------|
| `kryon-core/src/event.rs` | MODIFY | Add mouse/tab actions to EditorAction |
| `kryon-tui/src/highlight.rs` | NEW | Tree-sitter syntax highlighter |
| `kryon-tui/src/tab_bar.rs` | NEW | Tab bar widget |
| `kryon-tui/src/theme.rs` | MODIFY | Add syntax highlight colors |
| `kryon-tui/src/app.rs` | MODIFY | BufferState, mouse handling, tab switching |
| `kryon-tui/Cargo.toml` | MODIFY | Add tree-sitter dependencies |
| `kryon-tui/src/lib.rs` | MODIFY | Export new modules |

## Testing Strategy

1. **Unit tests for highlight.rs**: Parse Rust source, verify non-empty highlight events
2. **Unit tests for mouse mapping**: Terminal coords → buffer position conversion
3. **Unit tests for BufferState**: Create, switch, close buffers
4. **Integration test**: Open file → edit → save → verify file on disk
