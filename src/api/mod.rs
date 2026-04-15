//! High-level SDK helpers for CAD application development.
//!
//! This module provides application-level abstractions on top of the core
//! [`CadDocument`](crate::CadDocument) model: an undo/redo command history
//! and (in the future) additional workflow utilities.

pub mod command;
