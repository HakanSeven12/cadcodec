//! Builder for block definitions.
//!
//! A *block definition* in DXF/DWG is the combination of a [`BlockRecord`]
//! table entry, a structural [`Block`] entity, zero or more child entities,
//! and a terminating [`BlockEnd`] entity.  Wiring those together manually
//! is tedious and error-prone — [`BlockBuilder`] handles the bookkeeping.
//!
//! # Example
//!
//! ```rust
//! use acadrust::CadDocument;
//! use acadrust::api::block::BlockBuilder;
//! use acadrust::entities::{EntityType, Line, Circle};
//! use acadrust::types::Vector3;
//!
//! let mut doc = CadDocument::new();
//!
//! let br_handle = BlockBuilder::new("BOLT")
//!     .base_point(Vector3::new(0.0, 0.0, 0.0))
//!     .description("Standard bolt symbol")
//!     .entity(EntityType::Circle(Circle::from_center_radius(Vector3::ZERO, 5.0)))
//!     .entity(EntityType::Line(Line::from_coords(-5.0, 0.0, 0.0, 5.0, 0.0, 0.0)))
//!     .entity(EntityType::Line(Line::from_coords(0.0, -5.0, 0.0, 0.0, 5.0, 0.0)))
//!     .build(&mut doc)
//!     .unwrap();
//!
//! // Now insert the block into model space
//! doc.insert_block("BOLT", Vector3::new(50.0, 50.0, 0.0)).unwrap();
//! assert!(doc.block_records.get("BOLT").is_some());
//! ```

use crate::document::CadDocument;
use crate::entities::{EntityType, Block, BlockEnd};
use crate::tables::{BlockRecord, TableEntry};
use crate::types::{Handle, Vector3};
use crate::Result;

/// Fluent builder for creating a block definition in a [`CadDocument`].
///
/// Call [`build`](Self::build) to commit the block record, structural
/// entities, and child entities to the document.
#[derive(Debug, Clone)]
pub struct BlockBuilder {
    name: String,
    base_point: Vector3,
    description: String,
    entities: Vec<EntityType>,
}

impl BlockBuilder {
    /// Start building a block with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_point: Vector3::default(),
            description: String::new(),
            entities: Vec::new(),
        }
    }

    /// Set the block's base (insertion) point.
    pub fn base_point(mut self, point: Vector3) -> Self {
        self.base_point = point;
        self
    }

    /// Set the block description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a child entity to the block definition.
    pub fn entity(mut self, entity: EntityType) -> Self {
        self.entities.push(entity);
        self
    }

    /// Add multiple child entities.
    pub fn entities(mut self, entities: impl IntoIterator<Item = EntityType>) -> Self {
        self.entities.extend(entities);
        self
    }

    /// Commit the block definition to the document.
    ///
    /// Creates a [`BlockRecord`], a structural [`Block`] entity, the child
    /// entities, and a [`BlockEnd`] entity — all wired together with correct
    /// handles and owner references.
    ///
    /// Returns the block-record handle.  Use
    /// [`CadDocument::insert_block`] to place instances.
    pub fn build(self, doc: &mut CadDocument) -> Result<Handle> {
        if doc.block_records.get(&self.name).is_some() {
            return Err(crate::error::DxfError::Custom(format!(
                "Block '{}' already exists",
                self.name
            )));
        }

        // Create BlockRecord
        let mut br = BlockRecord::new(&self.name);
        br.set_handle(doc.allocate_handle());
        br.block_entity_handle = doc.allocate_handle();
        br.block_end_handle = doc.allocate_handle();
        br.description = self.description.clone();
        let br_handle = br.handle;
        let block_entity_handle = br.block_entity_handle;
        let block_end_handle = br.block_end_handle;

        // Store Block entity
        let mut block = Block::new(&self.name, self.base_point);
        block.description = self.description;
        block.common.handle = block_entity_handle;
        block.common.owner_handle = br_handle;
        let idx = doc.entities.len();
        doc.entities.push(EntityType::Block(block));
        doc.entity_index.insert(block_entity_handle, idx);

        // Store child entities — each owned by the block record
        for mut child in self.entities {
            let h = doc.allocate_handle();
            child.common_mut().handle = h;
            child.common_mut().owner_handle = br_handle;
            br.entity_handles.push(h);
            let idx = doc.entities.len();
            doc.entities.push(child);
            doc.entity_index.insert(h, idx);
        }

        // Store BlockEnd entity
        let mut block_end = BlockEnd::new();
        block_end.common.handle = block_end_handle;
        block_end.common.owner_handle = br_handle;
        let idx = doc.entities.len();
        doc.entities.push(EntityType::BlockEnd(block_end));
        doc.entity_index.insert(block_end_handle, idx);

        // Add the block record to the table
        doc.block_records
            .add(br)
            .map_err(|e| crate::error::DxfError::Custom(e))?;

        Ok(br_handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::{Circle, Line};

    #[test]
    fn build_block_and_insert() {
        let mut doc = CadDocument::new();

        let br_handle = BlockBuilder::new("TestBlock")
            .base_point(Vector3::new(1.0, 2.0, 0.0))
            .description("A test block")
            .entity(EntityType::Line(Line::from_coords(
                0.0, 0.0, 0.0, 10.0, 10.0, 0.0,
            )))
            .entity(EntityType::Circle(Circle::from_center_radius(Vector3::ZERO, 5.0)))
            .build(&mut doc)
            .unwrap();

        assert!(!br_handle.is_null());
        let br = doc.block_records.get("TestBlock").unwrap();
        assert_eq!(br.description, "A test block");
        assert_eq!(br.entity_handles.len(), 2); // 2 child entities
    }

    #[test]
    fn duplicate_block_name_errors() {
        let mut doc = CadDocument::new();
        BlockBuilder::new("Dup").build(&mut doc).unwrap();
        assert!(BlockBuilder::new("Dup").build(&mut doc).is_err());
    }

    #[test]
    fn insert_block_helper() {
        let mut doc = CadDocument::new();
        BlockBuilder::new("Bolt")
            .entity(EntityType::Circle(Circle::from_center_radius(Vector3::ZERO, 3.0)))
            .build(&mut doc)
            .unwrap();

        let h = doc
            .insert_block("Bolt", Vector3::new(50.0, 50.0, 0.0))
            .unwrap();
        assert!(!h.is_null());

        // Verify the Insert entity exists
        let entity = doc.get_entity(h).unwrap();
        match entity {
            EntityType::Insert(ins) => {
                assert_eq!(ins.block_name, "Bolt");
                assert_eq!(ins.insert_point.x, 50.0);
            }
            _ => panic!("Expected Insert entity"),
        }
    }
}
