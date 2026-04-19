//! Terminal setup and teardown utilities.
//!
//! Handles entering/exiting raw mode, alternate screen,
//! mouse capture, and panic hooks for clean recovery.

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, stdout, Stdout};

/// Type alias for the terminal backend.
pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal for TUI mode.
///
/// Enables raw mode, alternate screen, and mouse capture.
/// Installs a panic hook to restore the terminal on crash.
///
/// # Errors
///
/// Returns an error if terminal initialization fails.
pub fn init() -> io::Result<Tui> {
    // Install panic hook FIRST so crashes always clean up
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore();
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Restore the terminal to normal mode.
///
/// Disables raw mode, leaves alternate screen, and disables mouse capture.
///
/// # Errors
///
/// Returns an error if terminal restoration fails.
pub fn restore() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
