//! Selection set for batch entity operations.
//!
//! A [`SelectionSet`] is a lightweight collection of entity handles that
//! can be built up interactively or programmatically and then used to
//! apply bulk property changes through a [`CadDocument`].
//!
//! # Example
//!
//! ```rust
//! use acadrust::CadDocument;
//! use acadrust::api::selection::SelectionSet;
//! use acadrust::entities::{EntityType, Line, Circle};
//! use acadrust::types::{Color, Vector3};
//!
//! let mut doc = CadDocument::new();
//! let h1 = doc.add_entity(EntityType::Line(Line::from_coords(0.0,0.0,0.0, 1.0,0.0,0.0))).unwrap();
//! let h2 = doc.add_entity(EntityType::Circle(Circle::from_center_radius(Vector3::ZERO, 5.0))).unwrap();
//! let h3 = doc.add_entity(EntityType::Line(Line::from_coords(2.0,0.0,0.0, 3.0,0.0,0.0))).unwrap();
//!
//! let mut sel = SelectionSet::new();
//! sel.add(h1);
//! sel.add(h2);
//!
//! // Batch change color
//! sel.set_color(&mut doc, Color::RED);
//! assert_eq!(doc.get_entity(h1).unwrap().common().color, Color::RED);
//! assert_eq!(doc.get_entity(h2).unwrap().common().color, Color::RED);
//! // h3 unchanged
//! assert_ne!(doc.get_entity(h3).unwrap().common().color, Color::RED);
//! ```

use std::collections::HashSet;

use crate::document::CadDocument;
use crate::types::{Color, Handle, LineWeight, Transparency};

/// A set of entity handles for batch operations.
#[derive(Debug, Clone, Default)]
pub struct SelectionSet {
    handles: HashSet<Handle>,
}

impl SelectionSet {
    /// Create an empty selection set.
    pub fn new() -> Self {
        Self {
            handles: HashSet::new(),
        }
    }

    /// Create a selection set from an iterator of handles.
    pub fn from_handles(handles: impl IntoIterator<Item = Handle>) -> Self {
        Self {
            handles: handles.into_iter().collect(),
        }
    }

    /// Add a handle to the selection.  Returns `true` if it was newly inserted.
    pub fn add(&mut self, handle: Handle) -> bool {
        self.handles.insert(handle)
    }

    /// Remove a handle from the selection.  Returns `true` if it was present.
    pub fn remove(&mut self, handle: Handle) -> bool {
        self.handles.remove(&handle)
    }

    /// Toggle a handle in/out of the selection.
    pub fn toggle(&mut self, handle: Handle) {
        if !self.handles.remove(&handle) {
            self.handles.insert(handle);
        }
    }

    /// Remove all handles from the selection.
    pub fn clear(&mut self) {
        self.handles.clear();
    }

    /// Return the number of selected handles.
    pub fn len(&self) -> usize {
        self.handles.len()
    }

    /// Return `true` if the selection is empty.
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }

    /// Check whether a handle is in the selection.
    pub fn contains(&self, handle: Handle) -> bool {
        self.handles.contains(&handle)
    }

    /// Iterate over the selected handles.
    pub fn handles(&self) -> impl Iterator<Item = Handle> + '_ {
        self.handles.iter().copied()
    }

    /// Collect the handles into a `Vec`.
    pub fn to_vec(&self) -> Vec<Handle> {
        self.handles.iter().copied().collect()
    }

    // ── Batch property setters ──────────────────────────────────────

    /// Apply an arbitrary mutation to every selected entity.
    pub fn modify<F>(&self, doc: &mut CadDocument, mut f: F)
    where
        F: FnMut(&mut crate::entities::EntityType),
    {
        let handles: Vec<Handle> = self.handles.iter().copied().collect();
        doc.modify_entities(&handles, |e| f(e));
    }

    /// Set the layer of every selected entity.
    pub fn set_layer(&self, doc: &mut CadDocument, layer: &str) {
        let layer = layer.to_string();
        self.modify(doc, |e| e.common_mut().layer = layer.clone());
    }

    /// Set the color of every selected entity.
    pub fn set_color(&self, doc: &mut CadDocument, color: Color) {
        self.modify(doc, |e| e.common_mut().color = color);
    }

    /// Set the lineweight of every selected entity.
    pub fn set_line_weight(&self, doc: &mut CadDocument, lw: LineWeight) {
        self.modify(doc, |e| e.common_mut().line_weight = lw);
    }

    /// Set the linetype of every selected entity.
    pub fn set_linetype(&self, doc: &mut CadDocument, lt: &str) {
        let lt = lt.to_string();
        self.modify(doc, |e| e.common_mut().linetype = lt.clone());
    }

    /// Set the linetype scale of every selected entity.
    pub fn set_linetype_scale(&self, doc: &mut CadDocument, scale: f64) {
        self.modify(doc, |e| e.common_mut().linetype_scale = scale);
    }

    /// Set the transparency of every selected entity.
    pub fn set_transparency(&self, doc: &mut CadDocument, t: Transparency) {
        self.modify(doc, |e| e.common_mut().transparency = t);
    }

    /// Set the visibility of every selected entity.
    pub fn set_invisible(&self, doc: &mut CadDocument, invisible: bool) {
        self.modify(doc, |e| e.common_mut().invisible = invisible);
    }

    /// Translate every selected entity by an offset.
    pub fn translate(&self, doc: &mut CadDocument, offset: crate::types::Vector3) {
        self.modify(doc, |e| {
            e.as_entity_mut().translate(offset);
        });
    }

    /// Delete every selected entity from the document.
    ///
    /// After this call the selection set is cleared.
    pub fn delete_all(&mut self, doc: &mut CadDocument) {
        for h in self.handles.drain() {
            let _ = doc.remove_entity(h);
        }
    }
}

impl FromIterator<Handle> for SelectionSet {
    fn from_iter<I: IntoIterator<Item = Handle>>(iter: I) -> Self {
        Self::from_handles(iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{EntityType, Line};
    use crate::types::Color;

    #[test]
    fn add_remove_toggle() {
        let mut sel = SelectionSet::new();
        let h = Handle::new(1);
        assert!(sel.add(h));
        assert!(!sel.add(h)); // duplicate
        assert_eq!(sel.len(), 1);
        sel.toggle(h);
        assert!(sel.is_empty());
        sel.toggle(h);
        assert!(sel.contains(h));
        assert!(sel.remove(h));
        assert!(!sel.remove(h));
    }

    #[test]
    fn batch_color_change() {
        let mut doc = CadDocument::new();
        let h1 = doc.add_entity(EntityType::Line(Line::from_coords(0.0,0.0,0.0, 1.0,0.0,0.0))).unwrap();
        let h2 = doc.add_entity(EntityType::Line(Line::from_coords(2.0,0.0,0.0, 3.0,0.0,0.0))).unwrap();
        let sel: SelectionSet = [h1, h2].into_iter().collect();
        sel.set_color(&mut doc, Color::BLUE);
        assert_eq!(doc.get_entity(h1).unwrap().common().color, Color::BLUE);
        assert_eq!(doc.get_entity(h2).unwrap().common().color, Color::BLUE);
    }

    #[test]
    fn batch_layer_change() {
        let mut doc = CadDocument::new();
        let h = doc.add_entity(EntityType::Line(Line::new())).unwrap();
        let sel = SelectionSet::from_handles([h]);
        sel.set_layer(&mut doc, "NewLayer");
        assert_eq!(doc.get_entity(h).unwrap().common().layer, "NewLayer");
    }

    #[test]
    fn delete_all() {
        let mut doc = CadDocument::new();
        let h1 = doc.add_entity(EntityType::Line(Line::new())).unwrap();
        let h2 = doc.add_entity(EntityType::Line(Line::new())).unwrap();
        let initial_count = doc.entity_count();
        let mut sel = SelectionSet::from_handles([h1, h2]);
        sel.delete_all(&mut doc);
        assert!(sel.is_empty());
        assert_eq!(doc.entity_count(), initial_count - 2);
    }

    #[test]
    fn translate_selection() {
        use crate::types::Vector3;
        let mut doc = CadDocument::new();
        let h = doc.add_entity(EntityType::Line(
            Line::from_coords(0.0, 0.0, 0.0, 1.0, 0.0, 0.0),
        )).unwrap();
        let sel = SelectionSet::from_handles([h]);
        sel.translate(&mut doc, Vector3::new(10.0, 20.0, 0.0));
        if let EntityType::Line(l) = doc.get_entity(h).unwrap() {
            assert!((l.start.x - 10.0).abs() < 1e-10);
            assert!((l.start.y - 20.0).abs() < 1e-10);
        } else {
            panic!("expected Line");
        }
    }
}
