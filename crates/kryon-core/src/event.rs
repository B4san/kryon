//! Editor events and actions.
//!
//! Defines the action types that flow through the Elm-architecture
//! update cycle: User Input → Action → State Update → Render.

/// Actions that the editor can perform.
///
/// These are dispatched from input handlers and processed by the
/// application update function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorAction {
    // -- Text editing --
    /// Insert a character at the current cursor position.
    InsertChar(char),
    /// Insert a string at the current cursor position.
    InsertText(String),
    /// Delete the character before the cursor (backspace).
    DeleteBackward,
    /// Delete the character at the cursor (delete key).
    DeleteForward,
    /// Delete the current line.
    DeleteLine,

    // -- Cursor movement --
    /// Move cursor left by one character.
    MoveLeft,
    /// Move cursor right by one character.
    MoveRight,
    /// Move cursor up by one line.
    MoveUp,
    /// Move cursor down by one line.
    MoveDown,
    /// Move cursor to the beginning of the line.
    MoveLineStart,
    /// Move cursor to the end of the line.
    MoveLineEnd,
    /// Move cursor to the beginning of the buffer.
    MoveBufferStart,
    /// Move cursor to the end of the buffer.
    MoveBufferEnd,
    /// Move cursor one word left.
    MoveWordLeft,
    /// Move cursor one word right.
    MoveWordRight,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,

    // -- Selection --
    /// Select left.
    SelectLeft,
    /// Select right.
    SelectRight,
    /// Select up.
    SelectUp,
    /// Select down.
    SelectDown,
    /// Select to line start.
    SelectLineStart,
    /// Select to line end.
    SelectLineEnd,
    /// Select one word left.
    SelectWordLeft,
    /// Select one word right.
    SelectWordRight,
    /// Select all text.
    SelectAll,

    // -- Clipboard --
    /// Copy selected text to clipboard.
    Copy,
    /// Cut selected text to clipboard.
    Cut,
    /// Paste from clipboard.
    Paste(String),

    // -- History --
    /// Undo the last operation.
    Undo,
    /// Redo the last undone operation.
    Redo,

    // -- File operations --
    /// Save the current file.
    Save,
    /// Save the current file with a new name.
    SaveAs(String),
    /// Open a file.
    OpenFile(String),
    /// Create a new empty buffer.
    NewFile,
    /// Close the current tab.
    CloseTab,

    // -- UI --
    /// Toggle the file explorer sidebar.
    ToggleSidebar,
    /// Open the command palette.
    OpenCommandPalette,
    /// Open the search panel.
    OpenSearch,
    /// Open the go-to-line prompt.
    OpenGoToLine,
    /// Close the current modal/panel.
    CloseOverlay,
    /// Switch to a specific tab by index.
    SwitchTab(usize),
    /// Go to a specific line number.
    GoToLine(usize),

    // -- Search --
    /// Navigate to the next search match.
    SearchNext,
    /// Navigate to the previous search match.
    SearchPrev,
    /// Submit the search query from the input.
    SubmitSearchQuery(String),
    /// Submit the go-to-line input.
    SubmitGoToLine(String),

    // -- Mouse --
    /// Mouse click at a buffer-relative position.
    MouseClick {
        /// Line index in the buffer.
        line: usize,
        /// Column index in the buffer.
        col: usize,
    },
    /// Mouse drag to extend selection.
    MouseDrag {
        /// Line index in the buffer.
        line: usize,
        /// Column index in the buffer.
        col: usize,
    },
    /// Mouse scroll.
    MouseScroll {
        /// Scroll direction.
        direction: ScrollDirection,
        /// Number of lines to scroll.
        amount: usize,
    },

    // -- Buffer/Tab navigation --
    /// Switch to the next buffer tab.
    NextBuffer,
    /// Switch to the previous buffer tab.
    PrevBuffer,
    /// Create a new empty buffer.
    NewBuffer,
    /// Close the current buffer.
    CloseBuffer,

    // -- Preview --
    /// Toggle markdown preview mode.
    TogglePreview,

    // -- Terminal --
    /// Toggle the integrated terminal panel.
    ToggleTerminal,
    /// Run the current file.
    RunCurrentFile,
    /// Stop the running process.
    StopProcess,

    // -- Application --
    /// Quit the editor.
    Quit,
    /// Force quit without saving.
    ForceQuit,
    /// No operation (used for unmapped keys).
    Noop,
}

/// Scroll direction for mouse wheel events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    /// Scroll up (toward beginning of file).
    Up,
    /// Scroll down (toward end of file).
    Down,
}

/// Events emitted by the editor for the UI to handle.
#[derive(Debug, Clone)]
pub enum EditorEvent {
    /// Buffer content changed.
    BufferChanged {
        /// Index of the buffer that changed.
        buffer_index: usize,
    },
    /// A file was saved.
    FileSaved {
        /// Path of the saved file.
        path: String,
    },
    /// An error occurred.
    Error {
        /// Human-readable error message.
        message: String,
    },
    /// Status message to display.
    StatusMessage {
        /// The message text.
        text: String,
    },
}
