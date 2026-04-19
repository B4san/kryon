//! # Kryon Security
//!
//! Foundational security primitives for the Kryon editor.
//! Provides path validation, subprocess safety, and permission enforcement.
//!
//! All path operations normalize and validate against workspace boundaries.
//! All subprocess execution uses structured argument arrays, never shell strings.

#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![deny(missing_docs)]

pub mod path;
pub mod subprocess;
