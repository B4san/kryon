//! Main application struct and event loop.
//!
//! Implements the Elm-architecture cycle:
//! Poll Event → Update State → Draw Frame

#![allow(
    clippy::cast_possible_truncation, // Terminal coordinates are always small enough for u16
    clippy::collapsible_if,           // Nested ifs improve readability in match arms
    clippy::format_push_string,       // Acceptable for line number gutter rendering
    clippy::range_plus_one,           // head..head+1 is clearer than head..=head for rope slicing
    clippy::struct_excessive_bools,   // App struct legitimately needs multiple bool flags
    clippy::too_many_lines,           // update() is necessarily large due to action matching
)]

use crossterm::event::{
    KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use futures::StreamExt;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use std::path::PathBuf;

use kryon_core::{
    buffer::TextBuffer,
    command::{CommandHistory, EditCommand},
    cursor::CursorSet,
    event::{EditorAction, ScrollDirection},
};

use crate::file_tree::FileTree;
use crate::highlight::SyntaxHighlighter;
use crate::markdown::MarkdownRenderer;
use crate::search::{GoToLineState, SearchState};
use crate::tab_bar::{TabBar, TabEntry};
use crate::terminal_panel::{self, TerminalPanel};
use crate::theme::Theme;

/// The editor mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    /// Normal editing mode.
    Normal,
    /// Command palette is open.
    CommandPalette,
    /// Search panel is open.
    Search,
}

/// Per-buffer state: bundles buffer, cursors, undo history, file path, and highlighter.
pub struct BufferState {
    /// The text buffer.
    pub buffer: TextBuffer,
    /// Cursors for this buffer.
    pub cursors: CursorSet,
    /// Undo/redo history for this buffer.
    pub history: CommandHistory,
    /// File path on disk (None for untitled).
    pub file_path: Option<PathBuf>,
    /// Syntax highlighter (None if language not detected).
    pub highlighter: Option<SyntaxHighlighter>,
    /// Vertical scroll offset.
    pub scroll_offset: usize,
    /// Whether markdown preview is active.
    pub preview_mode: bool,
}

impl BufferState {
    /// Create a new empty buffer state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: TextBuffer::new(),
            cursors: CursorSet::new(),
            history: CommandHistory::new(),
            file_path: None,
            highlighter: None,
            scroll_offset: 0,
            preview_mode: false,
        }
    }

    /// Create a buffer state from file content.
    #[must_use]
    pub fn from_file(text: &str, path: PathBuf, theme: &Theme) -> Self {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let highlighter = SyntaxHighlighter::for_extension(ext, theme);
        Self {
            buffer: TextBuffer::from_text(text),
            cursors: CursorSet::new(),
            history: CommandHistory::new(),
            file_path: Some(path),
            highlighter,
            scroll_offset: 0,
            preview_mode: false,
        }
    }

    /// Get display name for this buffer.
    #[must_use]
    pub fn display_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map_or_else(
                || "Untitled".to_string(),
                |n| n.to_string_lossy().to_string(),
            )
    }
}

impl Default for BufferState {
    fn default() -> Self {
        Self::new()
    }
}

/// Main application state.
pub struct App {
    /// Open buffer states (one per tab).
    pub buffers: Vec<BufferState>,
    /// Index of the active buffer.
    pub active_buffer: usize,
    /// Current editor mode.
    pub mode: EditorMode,
    /// Whether the sidebar is visible.
    pub sidebar_visible: bool,
    /// Whether the sidebar has keyboard focus.
    pub sidebar_focused: bool,
    /// Whether the app should quit.
    pub should_quit: bool,
    /// Whether a quit was requested (for double-Ctrl+Q force quit).
    pub quit_requested: bool,
    /// Status bar message.
    pub status_message: String,
    /// Editor color theme.
    pub theme: Theme,
    /// Incremental search state.
    pub search: SearchState,
    /// Go-to-line prompt state.
    pub goto_line: GoToLineState,
    /// Workspace root directory.
    pub workspace_root: Option<PathBuf>,
    /// File tree for the sidebar explorer.
    pub file_tree: Option<FileTree>,
    /// Sidebar scroll offset.
    pub sidebar_scroll: usize,
    /// Cached layout info: gutter width in the last render.
    gutter_width: u16,
    /// Cached layout info: editor area x-offset.
    editor_content_x: u16,
    /// Cached layout info: editor area y-offset.
    editor_content_y: u16,
    /// Cached layout info: editor area height for page up/down.
    editor_height: u16,
    /// Cached layout info: sidebar area for mouse mapping.
    sidebar_area: Option<ratatui::layout::Rect>,
    /// Integrated terminal panel.
    pub terminal_panel: TerminalPanel,
    /// Whether a redraw is needed.
    pub dirty: bool,
    /// Cached layout info: terminal panel area for mouse mapping.
    terminal_area: Option<ratatui::layout::Rect>,
}

impl App {
    /// Create a new application with an empty buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffers: vec![BufferState::new()],
            active_buffer: 0,
            mode: EditorMode::Normal,
            sidebar_visible: true,
            sidebar_focused: false,
            should_quit: false,
            quit_requested: false,
            status_message: String::from("Welcome to Kryon  │  Ctrl+S save  │  Ctrl+Q quit"),
            theme: Theme::default(),
            search: SearchState::new(),
            goto_line: GoToLineState::new(),
            workspace_root: None,
            file_tree: None,
            sidebar_scroll: 0,
            gutter_width: 4,
            editor_content_x: 0,
            editor_content_y: 0,
            editor_height: 24,
            sidebar_area: None,
            terminal_panel: TerminalPanel::new(),
            dirty: true,
            terminal_area: None,
        }
    }

    /// Create a new application with a workspace directory.
    #[must_use]
    pub fn with_workspace(root: PathBuf) -> Self {
        let file_tree = Some(FileTree::new(&root));
        Self {
            workspace_root: Some(root),
            file_tree,
            ..Self::new()
        }
    }

    /// Create a new application with a file loaded.
    #[must_use]
    pub fn with_file(text: &str, path: PathBuf, workspace: Option<PathBuf>) -> Self {
        let theme = Theme::default();
        let buf_state = BufferState::from_file(text, path, &theme);
        let file_tree = workspace.as_ref().map(|r| FileTree::new(r));
        Self {
            buffers: vec![buf_state],
            status_message: String::from("File loaded"),
            theme,
            workspace_root: workspace,
            file_tree,
            ..Self::new()
        }
    }

    /// Get the active buffer state.
    #[must_use]
    pub fn active_buf(&self) -> &BufferState {
        &self.buffers[self.active_buffer]
    }

    /// Get the active buffer state mutably.
    pub fn active_buf_mut(&mut self) -> &mut BufferState {
        &mut self.buffers[self.active_buffer]
    }

    /// Map a key event to an editor action.
    #[must_use]
    pub fn map_key(&self, key: KeyEvent) -> EditorAction {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('q')) => EditorAction::Quit,
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => EditorAction::Save,
            (KeyModifiers::CONTROL, KeyCode::Char('z')) => EditorAction::Undo,
            (KeyModifiers::CONTROL, KeyCode::Char('y')) => EditorAction::Redo,
            (KeyModifiers::CONTROL, KeyCode::Char('b')) => EditorAction::ToggleSidebar,
            (KeyModifiers::CONTROL, KeyCode::Char('n')) => EditorAction::NewBuffer,
            (KeyModifiers::CONTROL, KeyCode::Char('w')) => EditorAction::CloseBuffer,
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => EditorAction::OpenCommandPalette,
            (KeyModifiers::CONTROL, KeyCode::Char('f')) => EditorAction::OpenSearch,
            (KeyModifiers::CONTROL, KeyCode::Char('g')) => EditorAction::OpenGoToLine,
            (KeyModifiers::CONTROL, KeyCode::Char('a')) => EditorAction::SelectAll,
            (KeyModifiers::CONTROL, KeyCode::Char('m')) => EditorAction::TogglePreview,
            (KeyModifiers::CONTROL, KeyCode::Char('`')) => EditorAction::ToggleTerminal,
            (KeyModifiers::NONE, KeyCode::F(5)) => EditorAction::RunCurrentFile,
            // Word movement
            (KeyModifiers::CONTROL, KeyCode::Left) => EditorAction::MoveWordLeft,
            (KeyModifiers::CONTROL, KeyCode::Right) => EditorAction::MoveWordRight,
            // Selection + word
            (m, KeyCode::Left)
                if m.contains(KeyModifiers::CONTROL) && m.contains(KeyModifiers::SHIFT) =>
            {
                EditorAction::SelectWordLeft
            }
            (m, KeyCode::Right)
                if m.contains(KeyModifiers::CONTROL) && m.contains(KeyModifiers::SHIFT) =>
            {
                EditorAction::SelectWordRight
            }
            // Selection
            (KeyModifiers::SHIFT, KeyCode::Left) => EditorAction::SelectLeft,
            (KeyModifiers::SHIFT, KeyCode::Right) => EditorAction::SelectRight,
            (KeyModifiers::SHIFT, KeyCode::Up) => EditorAction::SelectUp,
            (KeyModifiers::SHIFT, KeyCode::Down) => EditorAction::SelectDown,
            (KeyModifiers::SHIFT, KeyCode::Home) => EditorAction::SelectLineStart,
            (KeyModifiers::SHIFT, KeyCode::End) => EditorAction::SelectLineEnd,
            // Basic movement
            (KeyModifiers::NONE, KeyCode::Left) => EditorAction::MoveLeft,
            (KeyModifiers::NONE, KeyCode::Right) => EditorAction::MoveRight,
            (KeyModifiers::NONE, KeyCode::Up) => EditorAction::MoveUp,
            (KeyModifiers::NONE, KeyCode::Down) => EditorAction::MoveDown,
            (KeyModifiers::NONE, KeyCode::Home) => EditorAction::MoveLineStart,
            (KeyModifiers::NONE, KeyCode::End) => EditorAction::MoveLineEnd,
            (KeyModifiers::NONE, KeyCode::PageUp) => EditorAction::PageUp,
            (KeyModifiers::NONE, KeyCode::PageDown) => EditorAction::PageDown,
            (KeyModifiers::NONE, KeyCode::Backspace) => EditorAction::DeleteBackward,
            (KeyModifiers::NONE, KeyCode::Delete) => EditorAction::DeleteForward,
            (KeyModifiers::NONE, KeyCode::Enter) => EditorAction::InsertChar('\n'),
            (KeyModifiers::NONE, KeyCode::Tab) => EditorAction::InsertText("    ".to_string()),
            (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                EditorAction::InsertChar(c)
            }
            (KeyModifiers::NONE, KeyCode::Esc) => EditorAction::CloseOverlay,
            // Tab switching: Ctrl+PageDown/Up
            (KeyModifiers::CONTROL, KeyCode::PageDown) => EditorAction::NextBuffer,
            (KeyModifiers::CONTROL, KeyCode::PageUp) => EditorAction::PrevBuffer,
            _ => EditorAction::Noop,
        }
    }

    /// Process an editor action, updating state.
    pub fn update(&mut self, action: EditorAction) {
        // Reset quit_requested on any non-quit action
        if !matches!(action, EditorAction::Quit | EditorAction::Noop) {
            self.quit_requested = false;
        }

        let idx = self.active_buffer;
        match action {
            EditorAction::Quit => {
                if self.buffers[idx].buffer.is_modified() {
                    if self.quit_requested {
                        self.should_quit = true;
                    } else {
                        self.quit_requested = true;
                        self.status_message =
                            "Unsaved changes! Press Ctrl+Q again to force quit.".to_string();
                    }
                } else {
                    self.should_quit = true;
                }
            }
            EditorAction::ForceQuit => {
                self.should_quit = true;
            }
            EditorAction::Save => {
                if let Some(ref path) = self.buffers[idx].file_path {
                    let content = self.buffers[idx].buffer.text();
                    match std::fs::write(path, &content) {
                        Ok(()) => {
                            self.buffers[idx].buffer.mark_saved();
                            let name = self.buffers[idx].display_name();
                            self.status_message = format!("Saved: {name}");
                        }
                        Err(e) => {
                            self.status_message = format!("Save failed: {e}");
                        }
                    }
                } else {
                    self.status_message = "No file path — use :saveas <path>".to_string();
                }
            }
            EditorAction::InsertChar(c) => {
                let bs = &mut self.buffers[idx];
                let offset = bs.cursors.primary().head;
                let text = c.to_string();
                let cmd = EditCommand::Insert { offset, text };
                if cmd.execute(&mut bs.buffer, &mut bs.cursors).is_ok() {
                    bs.history.push(cmd);
                }
            }
            EditorAction::InsertText(text) => {
                let bs = &mut self.buffers[idx];
                let offset = bs.cursors.primary().head;
                let cmd = EditCommand::Insert { offset, text };
                if cmd.execute(&mut bs.buffer, &mut bs.cursors).is_ok() {
                    bs.history.push(cmd);
                }
            }
            EditorAction::DeleteBackward => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                if head > 0 {
                    let deleted = bs.buffer.rope().slice(head - 1..head).to_string();
                    let cmd = EditCommand::Delete {
                        range: (head - 1)..head,
                        deleted_text: deleted,
                    };
                    if cmd.execute(&mut bs.buffer, &mut bs.cursors).is_ok() {
                        bs.history.push(cmd);
                    }
                }
            }
            EditorAction::DeleteForward => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                let len = bs.buffer.len_chars();
                if head < len {
                    let deleted = bs.buffer.rope().slice(head..head + 1).to_string();
                    let cmd = EditCommand::Delete {
                        range: head..head + 1,
                        deleted_text: deleted,
                    };
                    if cmd.execute(&mut bs.buffer, &mut bs.cursors).is_ok() {
                        bs.history.push(cmd);
                    }
                }
            }
            EditorAction::MoveLeft => {
                let head = self.buffers[idx].cursors.primary().head;
                if head > 0 {
                    self.buffers[idx].cursors.primary_mut().move_to(head - 1);
                }
            }
            EditorAction::MoveRight => {
                let head = self.buffers[idx].cursors.primary().head;
                let len = self.buffers[idx].buffer.len_chars();
                if head < len {
                    self.buffers[idx].cursors.primary_mut().move_to(head + 1);
                }
            }
            EditorAction::MoveUp => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                if let Ok((line, col)) = bs.buffer.offset_to_line_col(head) {
                    if line > 0 {
                        if let Ok(new_offset) = bs.buffer.line_col_to_offset(line - 1, col) {
                            bs.cursors.primary_mut().move_to(new_offset);
                            if bs.scroll_offset > 0 && line - 1 < bs.scroll_offset {
                                bs.scroll_offset = line - 1;
                            }
                        }
                    }
                }
            }
            EditorAction::MoveDown => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                if let Ok((line, col)) = bs.buffer.offset_to_line_col(head) {
                    if line + 1 < bs.buffer.line_count() {
                        if let Ok(new_offset) = bs.buffer.line_col_to_offset(line + 1, col) {
                            bs.cursors.primary_mut().move_to(new_offset);
                        }
                    }
                }
            }
            EditorAction::MoveLineStart => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                if let Ok((line, _)) = bs.buffer.offset_to_line_col(head) {
                    if let Ok(offset) = bs.buffer.line_col_to_offset(line, 0) {
                        bs.cursors.primary_mut().move_to(offset);
                    }
                }
            }
            EditorAction::MoveLineEnd => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                if let Ok((line, _)) = bs.buffer.offset_to_line_col(head) {
                    let line_text = bs.buffer.line(line).unwrap_or_default();
                    let line_len = line_text.trim_end_matches('\n').chars().count();
                    if let Ok(offset) = bs.buffer.line_col_to_offset(line, line_len) {
                        bs.cursors.primary_mut().move_to(offset);
                    }
                }
            }
            // ── Word movement ────────────────────────────────────
            EditorAction::MoveWordLeft => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                let new_pos = bs.buffer.word_boundary_left(head);
                bs.cursors.primary_mut().move_to(new_pos);
            }
            EditorAction::MoveWordRight => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                let new_pos = bs.buffer.word_boundary_right(head);
                bs.cursors.primary_mut().move_to(new_pos);
            }
            // ── Page navigation ──────────────────────────────────
            EditorAction::PageUp => {
                let bs = &mut self.buffers[idx];
                let page = self.editor_height as usize;
                bs.scroll_offset = bs.scroll_offset.saturating_sub(page);
                let head = bs.cursors.primary().head;
                if let Ok((line, col)) = bs.buffer.offset_to_line_col(head) {
                    let new_line = line.saturating_sub(page);
                    if let Ok(offset) = bs.buffer.line_col_to_offset(new_line, col) {
                        bs.cursors.primary_mut().move_to(offset);
                    }
                }
            }
            EditorAction::PageDown => {
                let bs = &mut self.buffers[idx];
                let page = self.editor_height as usize;
                let max_scroll = bs.buffer.line_count().saturating_sub(1);
                bs.scroll_offset = (bs.scroll_offset + page).min(max_scroll);
                let head = bs.cursors.primary().head;
                if let Ok((line, col)) = bs.buffer.offset_to_line_col(head) {
                    let new_line = (line + page).min(bs.buffer.line_count().saturating_sub(1));
                    if let Ok(offset) = bs.buffer.line_col_to_offset(new_line, col) {
                        bs.cursors.primary_mut().move_to(offset);
                    }
                }
            }
            // ── Selection ────────────────────────────────────────
            EditorAction::SelectLeft => {
                let head = self.buffers[idx].cursors.primary().head;
                if head > 0 {
                    self.buffers[idx].cursors.primary_mut().select_to(head - 1);
                }
            }
            EditorAction::SelectRight => {
                let head = self.buffers[idx].cursors.primary().head;
                let len = self.buffers[idx].buffer.len_chars();
                if head < len {
                    self.buffers[idx].cursors.primary_mut().select_to(head + 1);
                }
            }
            EditorAction::SelectUp => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                if let Ok((line, col)) = bs.buffer.offset_to_line_col(head) {
                    if line > 0 {
                        if let Ok(new_offset) = bs.buffer.line_col_to_offset(line - 1, col) {
                            bs.cursors.primary_mut().select_to(new_offset);
                        }
                    }
                }
            }
            EditorAction::SelectDown => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                if let Ok((line, col)) = bs.buffer.offset_to_line_col(head) {
                    if line + 1 < bs.buffer.line_count() {
                        if let Ok(new_offset) = bs.buffer.line_col_to_offset(line + 1, col) {
                            bs.cursors.primary_mut().select_to(new_offset);
                        }
                    }
                }
            }
            EditorAction::SelectLineStart => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                if let Ok((line, _)) = bs.buffer.offset_to_line_col(head) {
                    if let Ok(offset) = bs.buffer.line_col_to_offset(line, 0) {
                        bs.cursors.primary_mut().select_to(offset);
                    }
                }
            }
            EditorAction::SelectLineEnd => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                if let Ok((line, _)) = bs.buffer.offset_to_line_col(head) {
                    let line_text = bs.buffer.line(line).unwrap_or_default();
                    let line_len = line_text.trim_end_matches('\n').chars().count();
                    if let Ok(offset) = bs.buffer.line_col_to_offset(line, line_len) {
                        bs.cursors.primary_mut().select_to(offset);
                    }
                }
            }
            EditorAction::SelectWordLeft => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                let new_pos = bs.buffer.word_boundary_left(head);
                bs.cursors.primary_mut().select_to(new_pos);
            }
            EditorAction::SelectWordRight => {
                let bs = &mut self.buffers[idx];
                let head = bs.cursors.primary().head;
                let new_pos = bs.buffer.word_boundary_right(head);
                bs.cursors.primary_mut().select_to(new_pos);
            }
            EditorAction::SelectAll => {
                let bs = &mut self.buffers[idx];
                let len = bs.buffer.len_chars();
                bs.cursors.primary_mut().anchor = 0;
                bs.cursors.primary_mut().select_to(len);
            }
            // ── Search ───────────────────────────────────────────
            EditorAction::OpenSearch => {
                self.search.open();
                self.mode = EditorMode::Search;
            }
            EditorAction::SearchNext => {
                self.search.next_match();
                if let Some(offset) = self.search.active_offset() {
                    self.buffers[idx].cursors.primary_mut().move_to(offset);
                    self.scroll_to_cursor();
                }
            }
            EditorAction::SearchPrev => {
                self.search.prev_match();
                if let Some(offset) = self.search.active_offset() {
                    self.buffers[idx].cursors.primary_mut().move_to(offset);
                    self.scroll_to_cursor();
                }
            }
            EditorAction::SubmitSearchQuery(query) => {
                let matches = self.buffers[idx].buffer.find_all(&query);
                self.search.set_query(query, matches);
                if let Some(offset) = self.search.active_offset() {
                    self.buffers[idx].cursors.primary_mut().move_to(offset);
                    self.scroll_to_cursor();
                }
                self.status_message = self.search.match_display();
            }
            // ── Go-to-line ───────────────────────────────────────
            EditorAction::OpenGoToLine => {
                self.goto_line.open();
                self.mode = EditorMode::Search; // reuse Search mode for overlay input
            }
            EditorAction::GoToLine(line_num) => {
                let bs = &mut self.buffers[idx];
                let target = line_num.saturating_sub(1); // 1-based → 0-based
                let max_line = bs.buffer.line_count().saturating_sub(1);
                let clamped = target.min(max_line);
                if let Ok(offset) = bs.buffer.line_col_to_offset(clamped, 0) {
                    bs.cursors.primary_mut().move_to(offset);
                    bs.scroll_offset = clamped.saturating_sub(5); // center-ish
                }
                self.goto_line.close();
                self.mode = EditorMode::Normal;
                self.status_message = format!("Line {}", clamped + 1);
            }
            EditorAction::SubmitGoToLine(input) => {
                if let Ok(n) = input.trim().parse::<usize>() {
                    self.update(EditorAction::GoToLine(n));
                } else {
                    self.status_message = "Invalid line number".to_string();
                    self.goto_line.close();
                    self.mode = EditorMode::Normal;
                }
            }
            // ── Undo/Redo ────────────────────────────────────────
            EditorAction::Undo => {
                let bs = &mut self.buffers[idx];
                if let Some(cmd) = bs.history.undo() {
                    let _ = cmd.undo(&mut bs.buffer, &mut bs.cursors);
                    self.status_message = "Undo".to_string();
                }
            }
            EditorAction::Redo => {
                let bs = &mut self.buffers[idx];
                if let Some(cmd) = bs.history.redo() {
                    let _ = cmd.execute(&mut bs.buffer, &mut bs.cursors);
                    self.status_message = "Redo".to_string();
                }
            }
            EditorAction::ToggleSidebar => {
                self.sidebar_visible = !self.sidebar_visible;
            }
            EditorAction::CloseOverlay => {
                self.search.close();
                self.goto_line.close();
                self.mode = EditorMode::Normal;
            }
            EditorAction::MouseClick { line, col } => {
                let bs = &mut self.buffers[idx];
                let target_line = bs.scroll_offset + line;
                if target_line < bs.buffer.line_count() {
                    if let Ok(offset) = bs.buffer.line_col_to_offset(target_line, col) {
                        bs.cursors.primary_mut().move_to(offset);
                    }
                }
            }
            EditorAction::MouseScroll { direction, amount } => {
                let bs = &mut self.buffers[idx];
                match direction {
                    ScrollDirection::Up => {
                        bs.scroll_offset = bs.scroll_offset.saturating_sub(amount);
                    }
                    ScrollDirection::Down => {
                        let max = bs.buffer.line_count().saturating_sub(1);
                        bs.scroll_offset = (bs.scroll_offset + amount).min(max);
                    }
                }
            }
            EditorAction::NewBuffer => {
                self.buffers.push(BufferState::new());
                self.active_buffer = self.buffers.len() - 1;
                self.status_message = "New buffer".to_string();
            }
            EditorAction::CloseBuffer => {
                if self.buffers.len() > 1 {
                    if self.buffers[idx].buffer.is_modified() {
                        self.status_message =
                            "Unsaved changes! Save first or use force close.".to_string();
                    } else {
                        self.buffers.remove(idx);
                        if self.active_buffer >= self.buffers.len() {
                            self.active_buffer = self.buffers.len() - 1;
                        }
                        self.status_message = "Buffer closed".to_string();
                    }
                } else {
                    self.status_message = "Cannot close last buffer".to_string();
                }
            }
            EditorAction::NextBuffer if !self.buffers.is_empty() => {
                self.active_buffer = (self.active_buffer + 1) % self.buffers.len();
            }
            EditorAction::PrevBuffer if !self.buffers.is_empty() => {
                self.active_buffer = if self.active_buffer == 0 {
                    self.buffers.len() - 1
                } else {
                    self.active_buffer - 1
                };
            }
            EditorAction::TogglePreview => {
                let bs = &mut self.buffers[self.active_buffer];
                let is_md = bs
                    .file_path
                    .as_ref()
                    .and_then(|p| p.extension())
                    .is_some_and(|e| e == "md" || e == "markdown");
                if is_md {
                    bs.preview_mode = !bs.preview_mode;
                    self.status_message = if bs.preview_mode {
                        "Preview ON".to_string()
                    } else {
                        "Preview OFF".to_string()
                    };
                } else {
                    self.status_message = "Preview only available for .md files".to_string();
                }
            }
            EditorAction::ToggleTerminal => {
                self.terminal_panel.toggle();
            }
            EditorAction::RunCurrentFile => {
                let bs = &self.buffers[self.active_buffer];
                if let Some(ref path) = bs.file_path {
                    if let Some(config) = terminal_panel::detect_run_command(path) {
                        let cwd = if config.use_file_dir {
                            path.parent()
                                .map_or_else(|| PathBuf::from("."), std::path::Path::to_path_buf)
                        } else {
                            self.workspace_root
                                .clone()
                                .unwrap_or_else(|| PathBuf::from("."))
                        };
                        let args: Vec<&str> = config.args.iter().map(String::as_str).collect();
                        self.terminal_panel
                            .run_command(&config.command, &args, &cwd);
                    } else {
                        self.status_message = "No runner detected for this file type".to_string();
                        self.terminal_panel.visible = true;
                    }
                } else {
                    self.status_message = "Save the file first to run it".to_string();
                }
            }
            EditorAction::StopProcess => {
                self.status_message = "Process stopped".to_string();
            }
            _ => {}
        }
        self.dirty = true;
    }

    /// Scroll the viewport so the cursor is visible.
    fn scroll_to_cursor(&mut self) {
        let bs = &self.buffers[self.active_buffer];
        let head = bs.cursors.primary().head;
        if let Ok((line, _)) = bs.buffer.offset_to_line_col(head) {
            let height = self.editor_height as usize;
            let bs = &mut self.buffers[self.active_buffer];
            if line < bs.scroll_offset {
                bs.scroll_offset = line;
            } else if line >= bs.scroll_offset + height {
                bs.scroll_offset = line.saturating_sub(height / 2);
            }
        }
    }

    /// Draw the UI.
    pub fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Determine if we need an overlay bar at the bottom
        let has_overlay = self.search.visible || self.goto_line.visible;

        // Top-level layout: tab bar | main content | optional terminal | status/overlay bar
        let mut constraints = vec![
            Constraint::Length(1), // tab bar
        ];

        if self.terminal_panel.visible {
            constraints.push(Constraint::Min(8)); // main content (min 8 lines)
            constraints.push(Constraint::Length(self.terminal_panel.height)); // terminal
        } else {
            constraints.push(Constraint::Min(1)); // main content
        }

        constraints.push(Constraint::Length(1)); // status bar

        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        // Tab bar
        self.draw_tab_bar(frame, outer[0]);

        // Main content: optional sidebar | editor
        let main_area = outer[1];
        let editor_area = if self.sidebar_visible {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(28), Constraint::Min(1)])
                .split(main_area);

            // Store sidebar area for mouse mapping
            self.sidebar_area = Some(chunks[0]);

            // Draw the file tree sidebar
            self.draw_sidebar(frame, chunks[0]);

            chunks[1]
        } else {
            self.sidebar_area = None;
            main_area
        };

        // Cache editor height for page up/down
        self.editor_height = editor_area.height;

        // Editor area
        self.draw_editor(frame, editor_area);

        // Terminal panel (if visible)
        if self.terminal_panel.visible {
            let term_idx = 2;
            self.terminal_area = Some(outer[term_idx]);
            self.terminal_panel
                .draw(frame, outer[term_idx], &self.theme);
        } else {
            self.terminal_area = None;
        }

        // Bottom bar: overlay or status
        let status_idx = if self.terminal_panel.visible { 3 } else { 2 };
        if has_overlay {
            self.draw_overlay_bar(frame, outer[status_idx]);
        } else {
            self.draw_status_bar(frame, outer[status_idx]);
        }
    }

    fn draw_tab_bar(&self, frame: &mut Frame, area: Rect) {
        let entries: Vec<TabEntry> = self
            .buffers
            .iter()
            .enumerate()
            .map(|(i, bs)| TabEntry {
                name: bs.display_name(),
                modified: bs.buffer.is_modified(),
                active: i == self.active_buffer,
            })
            .collect();

        let tab_bar = TabBar {
            tabs: &entries,
            theme: &self.theme,
        };
        frame.render_widget(tab_bar, area);
    }

    fn draw_editor(&mut self, frame: &mut Frame, area: Rect) {
        let bs = &self.buffers[self.active_buffer];

        // ── Markdown preview mode ────────────────────────────────
        if bs.preview_mode {
            let text = bs.buffer.text();
            let scroll = bs.scroll_offset;
            let visible = area.height as usize;
            let renderer = MarkdownRenderer::new(&self.theme);
            let lines = renderer.render(&text, scroll, visible);
            let preview = Paragraph::new(lines)
                .style(Style::default().fg(self.theme.fg).bg(self.theme.bg))
                .block(Block::default().borders(Borders::NONE));
            frame.render_widget(preview, area);
            return;
        }

        // ── Normal editor rendering ──────────────────────────────
        let line_count = bs.buffer.line_count();
        let gutter_width = line_count.to_string().len() as u16 + 2;
        self.gutter_width = gutter_width;

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(gutter_width), Constraint::Min(1)])
            .split(area);

        self.editor_content_x = chunks[1].x;
        self.editor_content_y = chunks[1].y;

        // Line numbers
        let visible_lines = area.height as usize;
        let scroll = bs.scroll_offset;
        let end_line = (scroll + visible_lines).min(line_count);
        let mut line_nums = String::new();
        for i in scroll..end_line {
            line_nums.push_str(&format!(
                "{:>width$} \n",
                i + 1,
                width = gutter_width as usize - 2
            ));
        }
        let gutter = Paragraph::new(line_nums).style(
            Style::default()
                .fg(self.theme.gutter_fg)
                .bg(self.theme.gutter_bg),
        );
        frame.render_widget(gutter, chunks[0]);

        // Code content — BATCH viewport highlighting (performance fix)
        let default_style = Style::default().fg(self.theme.fg).bg(self.theme.bg);
        let rope = bs.buffer.rope();

        if let Some(ref hl) = bs.highlighter {
            // Batch highlight: collects source ONCE, runs tree-sitter ONCE
            let lines = hl.highlight_viewport(rope, scroll, visible_lines, default_style);
            let editor = Paragraph::new(lines)
                .style(default_style)
                .block(Block::default().borders(Borders::NONE));
            frame.render_widget(editor, chunks[1]);
        } else {
            // Plain text fallback
            let mut content = String::new();
            for i in scroll..end_line {
                if let Ok(line) = bs.buffer.line(i) {
                    content.push_str(&line);
                }
            }
            let editor = Paragraph::new(content)
                .style(default_style)
                .block(Block::default().borders(Borders::NONE));
            frame.render_widget(editor, chunks[1]);
        }

        // Cursor position
        if let Ok((cur_line, cur_col)) = bs.buffer.offset_to_line_col(bs.cursors.primary().head) {
            if cur_line >= scroll && cur_line < scroll + visible_lines {
                let y = (cur_line - scroll) as u16 + chunks[1].y;
                let x = cur_col as u16 + chunks[1].x;
                if x < chunks[1].x + chunks[1].width && y < chunks[1].y + chunks[1].height {
                    frame.set_cursor_position((x, y));
                }
            }
        }
    }

    fn draw_status_bar(&self, frame: &mut Frame, area: Rect) {
        let bs = &self.buffers[self.active_buffer];
        let (line, col) = bs
            .buffer
            .offset_to_line_col(bs.cursors.primary().head)
            .unwrap_or((0, 0));

        let modified = if bs.buffer.is_modified() { " [+]" } else { "" };
        let filename = bs.display_name();

        let status = format!(
            " {} │ Ln {}, Col {} │ {}{}",
            self.status_message,
            line + 1,
            col + 1,
            filename,
            modified,
        );

        let bar = Paragraph::new(status).style(
            Style::default()
                .bg(self.theme.statusbar_bg)
                .fg(self.theme.statusbar_fg),
        );
        frame.render_widget(bar, area);
    }

    fn draw_overlay_bar(&self, frame: &mut Frame, area: Rect) {
        let style = Style::default()
            .bg(self.theme.statusbar_bg)
            .fg(self.theme.accent);

        if self.search.visible {
            let display = self.search.match_display();
            let text = format!(" 🔍 Search: {}│ {}", self.search.query, display);
            let bar = Paragraph::new(text).style(style);
            frame.render_widget(bar, area);
            // Place cursor at end of search input
            let cursor_x = area.x + 11 + self.search.query.len() as u16;
            if cursor_x < area.x + area.width {
                frame.set_cursor_position((cursor_x, area.y));
            }
        } else if self.goto_line.visible {
            let text = format!(" Go to Line: {}", self.goto_line.input);
            let bar = Paragraph::new(text).style(style);
            frame.render_widget(bar, area);
            let cursor_x = area.x + 14 + self.goto_line.input.len() as u16;
            if cursor_x < area.x + area.width {
                frame.set_cursor_position((cursor_x, area.y));
            }
        }
    }

    /// Draw the file tree sidebar.
    fn draw_sidebar(&mut self, frame: &mut Frame, area: Rect) {
        let sidebar_block = Block::default()
            .title(" Explorer ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if self.sidebar_focused {
                self.theme.accent
            } else {
                self.theme.sidebar_border
            }))
            .style(
                Style::default()
                    .bg(self.theme.sidebar_bg)
                    .fg(self.theme.sidebar_fg),
            );

        let inner = sidebar_block.inner(area);
        frame.render_widget(sidebar_block, area);

        if let Some(ref file_tree) = self.file_tree {
            let entries = file_tree.visible_entries();
            let visible_height = inner.height as usize;

            // Ensure selected entry is visible (auto-scroll)
            if file_tree.selected < self.sidebar_scroll {
                self.sidebar_scroll = file_tree.selected;
            } else if file_tree.selected >= self.sidebar_scroll + visible_height {
                self.sidebar_scroll = file_tree.selected.saturating_sub(visible_height - 1);
            }

            let end = (self.sidebar_scroll + visible_height).min(entries.len());
            let max_width = inner.width as usize;

            for (i, entry) in entries[self.sidebar_scroll..end].iter().enumerate() {
                let y = inner.y + i as u16;
                if y >= inner.y + inner.height {
                    break;
                }

                // Build display string with indentation and icon
                let indent = "  ".repeat(entry.depth);
                let icon = if entry.is_dir {
                    if entry.expanded { "▾ " } else { "▸ " }
                } else {
                    "  "
                };
                let display = format!("{indent}{icon}{}", entry.name);

                // Truncate to available width
                let truncated: String = display.chars().take(max_width).collect();
                // Pad to full width for highlight background
                let padded = format!("{truncated:<max_width$}");

                let style = if entry.selected {
                    Style::default().bg(self.theme.accent).fg(self.theme.bg)
                } else if entry.is_dir {
                    Style::default().fg(self.theme.accent)
                } else {
                    Style::default().fg(self.theme.sidebar_fg)
                };

                let span = Span::styled(padded, style);
                frame.render_widget(
                    Paragraph::new(Line::from(span)),
                    Rect::new(inner.x, y, inner.width, 1),
                );
            }

            // If tree is empty, show hint
            if entries.is_empty() {
                let hint =
                    Paragraph::new("  (empty)").style(Style::default().fg(self.theme.gutter_fg));
                frame.render_widget(hint, inner);
            }
        } else {
            let hint =
                Paragraph::new("  No workspace").style(Style::default().fg(self.theme.gutter_fg));
            frame.render_widget(hint, inner);
        }
    }

    /// Open a file from the file tree into a new buffer tab.
    pub fn open_file_from_path(&mut self, path: PathBuf) {
        // Check if already open in a tab
        for (i, bs) in self.buffers.iter().enumerate() {
            if bs.file_path.as_ref() == Some(&path) {
                self.active_buffer = i;
                self.status_message = format!("Switched to: {}", bs.display_name());
                return;
            }
        }
        // Open new buffer
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let buf_state = BufferState::from_file(&content, path, &self.theme);
                let name = buf_state.display_name();
                self.buffers.push(buf_state);
                self.active_buffer = self.buffers.len() - 1;
                self.status_message = format!("Opened: {name}");
            }
            Err(e) => {
                self.status_message = format!("Failed to open: {e}");
            }
        }
    }

    /// Map terminal mouse coordinates to buffer-relative coordinates.
    fn map_mouse_to_buffer(&self, column: u16, row: u16) -> Option<(usize, usize)> {
        if column < self.editor_content_x || row < self.editor_content_y {
            return None;
        }
        let col = (column - self.editor_content_x) as usize;
        let line = (row - self.editor_content_y) as usize;
        Some((line, col))
    }

    /// Check if a mouse position is within the sidebar area.
    fn is_in_sidebar(&self, column: u16, row: u16) -> bool {
        if let Some(area) = self.sidebar_area {
            column >= area.x
                && column < area.x + area.width
                && row >= area.y
                && row < area.y + area.height
        } else {
            false
        }
    }

    /// Check if a mouse position is within the terminal panel area.
    fn is_in_terminal(&self, column: u16, row: u16) -> bool {
        if let Some(area) = self.terminal_area {
            column >= area.x
                && column < area.x + area.width
                && row >= area.y
                && row < area.y + area.height
        } else {
            false
        }
    }

    /// Map a mouse click in the sidebar to a tree row index.
    fn sidebar_click_row(&self, row: u16) -> Option<usize> {
        if let Some(area) = self.sidebar_area {
            // Account for the Block border (1 row for top border)
            let inner_y = area.y + 1;
            if row >= inner_y && row < area.y + area.height - 1 {
                return Some((row - inner_y) as usize);
            }
        }
        None
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the main event loop (async for subprocess support).
///
/// # Errors
///
/// Returns an error if terminal I/O fails.
pub async fn run(terminal: &mut crate::terminal::Tui, app: &mut App) -> std::io::Result<()> {
    use crossterm::event::{Event, EventStream};
    let mut event_stream = EventStream::new();

    while !app.should_quit {
        // Drain pending subprocess output
        if app.terminal_panel.drain_output() {
            app.dirty = true;
        }
        // Only redraw when dirty
        if app.dirty {
            terminal.draw(|frame| app.draw(frame))?;
            app.dirty = false;
        }
        // Wait for next event with timeout for subprocess polling
        let event = tokio::select! {
            ev = event_stream.next() => ev,
            () = tokio::time::sleep(std::time::Duration::from_millis(32)) => { continue; }
        };
        let Some(event_result) = event else {
            break;
        };
        let Ok(event) = event_result else {
            continue;
        };

        match event {
            Event::Key(key) => {
                // ── BUG FIX: Only process Press events ──────────
                // On Windows, crossterm fires both Press and Release
                // events for each keystroke. Without this filter,
                // every character gets inserted twice.
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // In Search/GoToLine mode, route typing to the overlay input
                if app.search.visible {
                    match (key.modifiers, key.code) {
                        (KeyModifiers::NONE, KeyCode::Esc) => {
                            app.update(EditorAction::CloseOverlay);
                        }
                        (KeyModifiers::NONE, KeyCode::Enter) => {
                            let query = app.search.query.clone();
                            app.update(EditorAction::SubmitSearchQuery(query));
                        }
                        (KeyModifiers::NONE, KeyCode::Backspace) => {
                            app.search.query.pop();
                            // Live search: update matches as user types
                            let query = app.search.query.clone();
                            let matches = app.buffers[app.active_buffer].buffer.find_all(&query);
                            app.search.set_query(query, matches);
                        }
                        (KeyModifiers::NONE, KeyCode::F(3)) => {
                            app.update(EditorAction::SearchNext);
                        }
                        (KeyModifiers::SHIFT, KeyCode::F(3)) => {
                            app.update(EditorAction::SearchPrev);
                        }
                        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                            app.search.query.push(c);
                            // Live search
                            let query = app.search.query.clone();
                            let matches = app.buffers[app.active_buffer].buffer.find_all(&query);
                            app.search.set_query(query, matches);
                            if let Some(offset) = app.search.active_offset() {
                                app.buffers[app.active_buffer]
                                    .cursors
                                    .primary_mut()
                                    .move_to(offset);
                                app.scroll_to_cursor();
                            }
                        }
                        _ => {}
                    }
                } else if app.goto_line.visible {
                    match (key.modifiers, key.code) {
                        (KeyModifiers::NONE, KeyCode::Esc) => {
                            app.update(EditorAction::CloseOverlay);
                        }
                        (KeyModifiers::NONE, KeyCode::Enter) => {
                            let input = app.goto_line.input.clone();
                            app.update(EditorAction::SubmitGoToLine(input));
                        }
                        (KeyModifiers::NONE, KeyCode::Backspace) => {
                            app.goto_line.input.pop();
                        }
                        (KeyModifiers::NONE, KeyCode::Char(c)) if c.is_ascii_digit() => {
                            app.goto_line.input.push(c);
                        }
                        _ => {}
                    }
                } else if app.sidebar_focused {
                    // ── Sidebar keyboard navigation ────────────
                    match (key.modifiers, key.code) {
                        (KeyModifiers::NONE, KeyCode::Up) => {
                            if let Some(ref mut tree) = app.file_tree {
                                tree.move_up();
                            }
                        }
                        (KeyModifiers::NONE, KeyCode::Down) => {
                            if let Some(ref mut tree) = app.file_tree {
                                tree.move_down();
                            }
                        }
                        (KeyModifiers::NONE, KeyCode::Enter | KeyCode::Right) => {
                            let path = app.file_tree.as_mut().and_then(FileTree::activate);
                            if let Some(p) = path {
                                app.open_file_from_path(p);
                                app.sidebar_focused = false;
                            }
                        }
                        (KeyModifiers::NONE, KeyCode::Left) => {
                            // Collapse current directory or move to parent
                            if let Some(ref mut tree) = app.file_tree {
                                if tree.is_dir_at(tree.selected) {
                                    tree.toggle(tree.selected);
                                }
                            }
                        }
                        (KeyModifiers::NONE, KeyCode::Esc | KeyCode::Tab) => {
                            app.sidebar_focused = false;
                        }
                        (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                            app.update(EditorAction::ToggleSidebar);
                        }
                        (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
                            app.update(EditorAction::Quit);
                        }
                        _ => {}
                    }
                } else if app.terminal_panel.focused {
                    // ── Terminal panel keyboard handling ────────
                    match (key.modifiers, key.code) {
                        (KeyModifiers::NONE, KeyCode::Esc) => {
                            app.terminal_panel.focused = false;
                            app.dirty = true;
                        }
                        (KeyModifiers::CONTROL, KeyCode::Char('`')) => {
                            app.update(EditorAction::ToggleTerminal);
                        }
                        (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
                            app.update(EditorAction::Quit);
                        }
                        (KeyModifiers::NONE, KeyCode::F(5)) => {
                            app.update(EditorAction::RunCurrentFile);
                        }
                        _ => {}
                    }
                } else {
                    // ── Normal editor key handling ─────────────
                    let action = app.map_key(key);
                    app.update(action);
                }
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    if app.sidebar_visible && app.is_in_sidebar(mouse.column, mouse.row) {
                        app.sidebar_focused = true;
                        app.terminal_panel.focused = false;
                        if let Some(row) = app.sidebar_click_row(mouse.row) {
                            let scroll = app.sidebar_scroll;
                            let path = app
                                .file_tree
                                .as_mut()
                                .and_then(|t| t.handle_click(row, scroll));
                            if let Some(p) = path {
                                app.open_file_from_path(p);
                                app.sidebar_focused = false;
                            }
                        }
                        app.dirty = true;
                    } else if app.terminal_panel.visible
                        && app.is_in_terminal(mouse.column, mouse.row)
                    {
                        app.terminal_panel.focused = true;
                        app.sidebar_focused = false;
                        app.dirty = true;
                    } else {
                        app.sidebar_focused = false;
                        app.terminal_panel.focused = false;
                        if let Some((line, col)) = app.map_mouse_to_buffer(mouse.column, mouse.row)
                        {
                            app.update(EditorAction::MouseClick { line, col });
                        }
                    }
                }
                MouseEventKind::ScrollUp => {
                    if app.sidebar_visible && app.is_in_sidebar(mouse.column, mouse.row) {
                        app.sidebar_scroll = app.sidebar_scroll.saturating_sub(3);
                        app.dirty = true;
                    } else if app.terminal_panel.visible
                        && app.is_in_terminal(mouse.column, mouse.row)
                    {
                        app.terminal_panel.scroll_up(3);
                        app.dirty = true;
                    } else {
                        app.update(EditorAction::MouseScroll {
                            direction: ScrollDirection::Up,
                            amount: 3,
                        });
                    }
                }
                MouseEventKind::ScrollDown => {
                    if app.sidebar_visible && app.is_in_sidebar(mouse.column, mouse.row) {
                        let max = app
                            .file_tree
                            .as_ref()
                            .map_or(0, |t| t.visible_count().saturating_sub(1));
                        app.sidebar_scroll = (app.sidebar_scroll + 3).min(max);
                        app.dirty = true;
                    } else if app.terminal_panel.visible
                        && app.is_in_terminal(mouse.column, mouse.row)
                    {
                        app.terminal_panel.scroll_down(3);
                        app.dirty = true;
                    } else {
                        app.update(EditorAction::MouseScroll {
                            direction: ScrollDirection::Down,
                            amount: 3,
                        });
                    }
                }
                _ => {}
            },
            Event::Resize(..) => {
                app.dirty = true;
            }
            _ => {}
        }
    }
    Ok(())
}
