//! High-level SDK helpers for CAD application development.
//!
//! This module provides application-level abstractions on top of the core
//! [`CadDocument`](crate::CadDocument) model: an undo/redo command history,
//! a block definition builder, and (in the future) additional workflow
//! utilities.

pub mod block;
pub mod command;
pub mod selection;
