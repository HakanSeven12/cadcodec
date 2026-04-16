//! Geometry kernel for command-style modify operations.
//!
//! This module contains low-level geometric building blocks used by
//! document-level wrappers such as trim/extend/offset/fillet/chamfer.

pub mod intersections;
pub mod modify;
pub mod offsets;
