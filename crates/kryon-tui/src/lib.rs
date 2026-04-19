//! # Kryon TUI
//!
//! Terminal user interface layer built on Ratatui and Crossterm.
//! Implements the Elm-architecture event loop, layout system,
//! and widget library for the Kryon editor.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod app;
pub mod file_tree;
pub mod highlight;
pub mod markdown;
pub mod search;
pub mod tab_bar;
pub mod terminal;
pub mod terminal_panel;
pub mod theme;
