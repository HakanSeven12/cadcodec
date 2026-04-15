//! Snapshot-based undo / redo history for [`CadDocument`].
//!
//! [`CommandHistory`] wraps a [`CadDocument`] and keeps a stack of previous
//! states so that any sequence of mutations can be undone (and re-done).
//!
//! # Design
//!
//! Each *checkpoint* is a full clone of the document.  This is simple and
//! correct — every document field is captured — at the cost of memory
//! proportional to `checkpoint_count × document_size`.  For typical
//! interactive CAD sessions (dozens of operations between saves, documents
//! of a few MB) this is perfectly acceptable.  A future delta-based scheme
//! can be added behind the same API without breaking callers.
//!
//! # Example
//!
//! ```rust
//! use acadrust::CadDocument;
//! use acadrust::api::command::CommandHistory;
//! use acadrust::entities::{EntityType, Line};
//!
//! let doc = CadDocument::new();
//! let mut history = CommandHistory::new(doc);
//!
//! // Save a checkpoint, then mutate
//! history.checkpoint();
//! history.doc_mut().add_entity(EntityType::Line(
//!     Line::from_coords(0.0, 0.0, 0.0, 10.0, 10.0, 0.0),
//! )).unwrap();
//! assert_eq!(history.doc().entity_count(), 1);
//!
//! // Undo
//! assert!(history.undo());
//! assert_eq!(history.doc().entity_count(), 0);
//!
//! // Redo
//! assert!(history.redo());
//! assert_eq!(history.doc().entity_count(), 1);
//! ```

use crate::document::CadDocument;

/// Snapshot-based undo / redo history.
///
/// Call [`checkpoint`](Self::checkpoint) **before** each logical operation,
/// then mutate the document via [`doc_mut`](Self::doc_mut).
/// [`undo`](Self::undo) restores the previous checkpoint and pushes the
/// current state onto the redo stack.
#[derive(Debug, Clone)]
pub struct CommandHistory {
    /// The current working document.
    document: CadDocument,
    /// Past states (most-recent last).
    undo_stack: Vec<CadDocument>,
    /// States that were undone (most-recent last).
    redo_stack: Vec<CadDocument>,
    /// Maximum number of undo levels to keep.  `0` means unlimited.
    max_undo: usize,
}

impl CommandHistory {
    /// Wrap a document in a new history with unlimited undo.
    pub fn new(document: CadDocument) -> Self {
        Self {
            document,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo: 0,
        }
    }

    /// Wrap a document with a capped undo depth.
    ///
    /// When the undo stack exceeds `max_undo` entries the oldest snapshot
    /// is dropped.  Pass `0` for unlimited.
    pub fn with_max_undo(document: CadDocument, max_undo: usize) -> Self {
        Self {
            document,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo,
        }
    }

    // ── Accessors ───────────────────────────────────────────────

    /// Immutable access to the current document.
    pub fn doc(&self) -> &CadDocument {
        &self.document
    }

    /// Mutable access to the current document.
    ///
    /// **Important:** call [`checkpoint`](Self::checkpoint) *before*
    /// mutating if you want the previous state to be undoable.
    pub fn doc_mut(&mut self) -> &mut CadDocument {
        &mut self.document
    }

    /// Consume the history and return the inner document.
    pub fn into_inner(self) -> CadDocument {
        self.document
    }

    // ── Checkpoint / Undo / Redo ────────────────────────────────

    /// Save the current document state as an undo checkpoint.
    ///
    /// This clears the redo stack (any undone states are discarded once
    /// a new mutation occurs, matching standard undo semantics).
    pub fn checkpoint(&mut self) {
        self.redo_stack.clear();
        self.undo_stack.push(self.document.clone());
        if self.max_undo > 0 && self.undo_stack.len() > self.max_undo {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last checkpointed operation.
    ///
    /// Returns `true` if a state was restored, `false` if the undo stack
    /// is empty.
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            let current = std::mem::replace(&mut self.document, prev);
            self.redo_stack.push(current);
            true
        } else {
            false
        }
    }

    /// Redo a previously undone operation.
    ///
    /// Returns `true` if a state was restored, `false` if the redo stack
    /// is empty.
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            let current = std::mem::replace(&mut self.document, next);
            self.undo_stack.push(current);
            true
        } else {
            false
        }
    }

    /// Return `true` if [`undo`](Self::undo) would succeed.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Return `true` if [`redo`](Self::redo) would succeed.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Number of available undo levels.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of available redo levels.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Discard all undo and redo history, keeping only the current state.
    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{EntityType, Line};

    #[test]
    fn undo_redo_basic() {
        let doc = CadDocument::new();
        let mut h = CommandHistory::new(doc);

        // checkpoint → add entity
        h.checkpoint();
        h.doc_mut()
            .add_entity(EntityType::Line(Line::from_coords(
                0.0, 0.0, 0.0, 1.0, 1.0, 0.0,
            )))
            .unwrap();
        assert_eq!(h.doc().entity_count(), 1);

        // undo → back to 0
        assert!(h.undo());
        assert_eq!(h.doc().entity_count(), 0);

        // redo → back to 1
        assert!(h.redo());
        assert_eq!(h.doc().entity_count(), 1);
    }

    #[test]
    fn undo_empty_returns_false() {
        let mut h = CommandHistory::new(CadDocument::new());
        assert!(!h.undo());
        assert!(!h.redo());
    }

    #[test]
    fn checkpoint_clears_redo() {
        let mut h = CommandHistory::new(CadDocument::new());
        h.checkpoint();
        h.doc_mut()
            .add_entity(EntityType::Line(Line::from_coords(
                0.0, 0.0, 0.0, 1.0, 0.0, 0.0,
            )))
            .unwrap();
        h.undo();
        assert!(h.can_redo());

        // New checkpoint clears redo
        h.checkpoint();
        assert!(!h.can_redo());
    }

    #[test]
    fn max_undo_limit() {
        let mut h = CommandHistory::with_max_undo(CadDocument::new(), 2);
        h.checkpoint(); // 1
        h.checkpoint(); // 2
        h.checkpoint(); // 3 → oldest dropped
        assert_eq!(h.undo_count(), 2);
    }

    #[test]
    fn clear_history() {
        let mut h = CommandHistory::new(CadDocument::new());
        h.checkpoint();
        h.checkpoint();
        assert_eq!(h.undo_count(), 2);
        h.clear_history();
        assert_eq!(h.undo_count(), 0);
        assert_eq!(h.redo_count(), 0);
    }

    #[test]
    fn into_inner_returns_document() {
        let mut h = CommandHistory::new(CadDocument::new());
        h.doc_mut()
            .add_entity(EntityType::Line(Line::from_coords(
                0.0, 0.0, 0.0, 5.0, 5.0, 0.0,
            )))
            .unwrap();
        let doc = h.into_inner();
        assert_eq!(doc.entity_count(), 1);
    }
}
