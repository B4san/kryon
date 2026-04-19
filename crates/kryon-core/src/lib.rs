//! # Kryon Core
//!
//! The UI-agnostic editor engine. Contains all business logic for text
//! editing, cursor management, undo/redo, file operations, and syntax
//! integration. This crate is designed to be reusable across terminal,
//! web, and future desktop frontends.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod buffer;
pub mod command;
pub mod cursor;
pub mod event;
