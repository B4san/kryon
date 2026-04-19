# Tasks: Sprint 3 — Syntax Highlighting, File Operations & Mouse Support

## Pre-Implementation
- [x] Deep research on tree-sitter, Ropey, Ratatui, Helix patterns
- [x] Design document with architecture decisions
- [x] Proposal reviewed

---

## Task 1: Extend Theme with Syntax Colors
**Files**: `kryon-tui/src/theme.rs`  
**Complexity**: Low (single file)

- [ ] Add 12 syntax color fields to Theme struct
- [ ] Populate Catppuccin Mocha syntax colors
- [ ] Populate Tokyo Night syntax colors
- [ ] Populate Dracula syntax colors
- [ ] Add `syntax_style_map()` method returning `HashMap<&str, Style>`

---

## Task 2: Create Syntax Highlighter Module
**Files**: `kryon-tui/src/highlight.rs` (NEW), `kryon-tui/Cargo.toml`, `kryon-tui/src/lib.rs`  
**Complexity**: High (tree-sitter integration)

- [ ] Add `tree-sitter`, `tree-sitter-highlight`, `tree-sitter-rust` dependencies
- [ ] Implement `SyntaxHighlighter` struct (parser, tree, config)
- [ ] Implement `SyntaxHighlighter::new(language)` constructor
- [ ] Implement `SyntaxHighlighter::parse_full(rope)` for initial parse
- [ ] Implement `SyntaxHighlighter::update(rope, edit)` for incremental parse
- [ ] Implement `highlight_line(rope, line_idx, theme) -> Line` for rendering
- [ ] Implement language detection from file extension
- [ ] Write unit test: parse Rust source, verify highlight events produced

---

## Task 3: Extend EditorAction with Mouse and Tab Actions
**Files**: `kryon-core/src/event.rs`  
**Complexity**: Low (enum additions)

- [ ] Add `MouseClick { line: usize, col: usize }` variant
- [ ] Add `MouseDrag { line: usize, col: usize }` variant
- [ ] Add `MouseScroll { direction: ScrollDirection, amount: usize }` variant
- [ ] Add `NewBuffer` variant
- [ ] Add `CloseBuffer` variant
- [ ] Add `NextBuffer` / `PrevBuffer` variants
- [ ] Add `ScrollDirection` enum (Up, Down)

---

## Task 4: Refactor App to BufferState
**Files**: `kryon-tui/src/app.rs`  
**Complexity**: High (structural refactor)

- [ ] Create `BufferState` struct (buffer, cursors, history, file_path, highlighter)
- [ ] Replace `Vec<TextBuffer>` + separate cursors/history with `Vec<BufferState>`
- [ ] Update `App::new()` to create `BufferState` instances
- [ ] Update `App::with_file()` to populate `BufferState` with path and highlighter
- [ ] Update all `update()` handlers to use `self.buffers[idx].buffer` etc.
- [ ] Verify: all existing functionality preserved

---

## Task 5: Integrate Syntax Highlighting into Editor Rendering
**Files**: `kryon-tui/src/app.rs`  
**Complexity**: Medium

- [ ] In `draw_editor()`, check if active buffer has a highlighter
- [ ] If highlighting available: render styled `Line` spans instead of plain text
- [ ] If no highlighter: fall back to plain text rendering (current behavior)
- [ ] Ensure highlighting only runs for visible lines (viewport-limited)
- [ ] Update syntax tree after every text edit in `update()`

---

## Task 6: Mouse Event Handling
**Files**: `kryon-tui/src/app.rs`  
**Complexity**: Medium

- [ ] In `run()` event loop, handle `Event::Mouse` events
- [ ] Implement `map_mouse()` to convert mouse event → EditorAction
- [ ] Implement coordinate mapping: terminal (x,y) → buffer (line, col)
- [ ] Handle `MouseClick`: position cursor at mapped location
- [ ] Handle `MouseScroll`: adjust scroll_offset
- [ ] Handle `Event::Resize`: recalculate layout
- [ ] Write unit test: coordinate mapping correctness

---

## Task 7: Tab Bar Widget
**Files**: `kryon-tui/src/tab_bar.rs` (NEW), `kryon-tui/src/app.rs`, `kryon-tui/src/lib.rs`  
**Complexity**: Medium

- [ ] Create `TabBar` widget struct
- [ ] Render tab for each open buffer (filename or "Untitled")
- [ ] Show modified indicator (`●`) on unsaved buffers
- [ ] Highlight active tab with theme accent color
- [ ] Add Ctrl+Tab / Ctrl+Shift+Tab for tab switching
- [ ] Add Ctrl+N for new buffer
- [ ] Add Ctrl+W for close buffer (with unsaved guard)
- [ ] Integrate tab bar into main layout (above editor area)

---

## Task 8: Verification & Testing
**Complexity**: Medium

- [ ] Run `cargo test --workspace` — all tests pass
- [ ] Run `cargo clippy --workspace -- -D warnings` — clean
- [ ] Manual test: open `.rs` file, verify syntax colors
- [ ] Manual test: mouse click positions cursor
- [ ] Manual test: mouse scroll works
- [ ] Manual test: Ctrl+N creates new tab, Ctrl+W closes
- [ ] Manual test: Ctrl+S saves file
- [ ] Update project-index.md with new modules

---

## Execution Order

```
Task 1 (Theme) → Task 3 (Events) → Task 4 (BufferState) → Task 2 (Highlighter)
  → Task 5 (Rendering) → Task 6 (Mouse) → Task 7 (Tab Bar) → Task 8 (Verify)
```
