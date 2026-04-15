Based on a thorough review of the codebase, here is an analysis of what exists and what is needed for a complete CAD API/SDK.

Current State
The library already has a solid foundation:

Core Data Model (document.rs)

CadDocument with full tables (layers, linetypes, text styles, block records, dim styles, viewports, UCS, views)
41 entity types + EntityCommon metadata on every entity
Full objects map (Layout, Dictionary, MLineStyle, ImageDefinition, Group, PlotSettings, etc.)
add_entity(), remove_entity(), get_entity(), get_entity_mut(), entities() iterator, add_entity_to_layout()
Entity Operations (entities)

apply_transform() — matrix transformation for all entity types
translate() — offset all entity types
mirror() — mirror all entity types
explode_entity() — decompose complex entities to primitives
File I/O — full DXF (R12–R2018) and DWG (R13–R2018) read/write

Gaps for a CAD Application SDK
1. Layer & Table Management API
No ergonomic helpers for the common pattern of "ensure layer X exists, then assign entity to it." Currently requires direct doc.layers.add(Layer::new(...)) with manual handle allocation.

2. Entity Querying / Filtering
No queries like:

entities_on_layer("Walls")
entities_in_bounding_box(min, max)
entities_of_type::<Line>()
3. Block Definition & Instance API
No high-level create_block_definition(name) → BlockDef builder that wires up BlockRecord, Block, BlockEnd, and child entities together, or an insert_block(name, position, scale, rotation) helper.

4. Layout Management
No API to add/remove/rename paper space layouts or switch the active layout.

5. Spatial / Geometry Utilities
No bounding_box() per entity for spatial queries or extent computation. The BoundingBox3D type exists in types but per-entity computation is not exposed.

6. Selection Set / Batch Operations
No concept of a selection set for batch property changes (e.g., move all selected to layer "X", change color for all selected).

7. Table Entry Existence Helpers
No ensure_layer() / ensure_linetype() / ensure_text_style() that get-or-create without duplicate errors — callers must check layers.get() before layers.add().

8. Change Tracking / Undo-Redo
No observable model or command stack for application-level undo.

9. Xref API
Block records have xref_path (recently added) but no higher-level attach_xref(), detach_xref(), or reload_xref() operations.

10. Entity Copying Between Layouts/Documents
No copy_entity_to_layout() or cross-document entity copy that deep-clones handles and re-maps owner references.

Recommended API Surface to Build
Priority	Feature	Where
P1	doc.ensure_layer(name, color, ltype) → Handle	document.rs
P1	doc.entities_on_layer(name) / entities_of_type::<T>()	document.rs
P1	doc.create_block(name) → builder	new src/api/block.rs
P1	EntityType::bounding_box()	mod.rs
P2	SelectionSet + batch property ops	new src/api/selection.rs
P2	doc.add_layout(name) / doc.layouts()	document.rs
P2	doc.copy_entity(handle, target_layout)	document.rs
P3	Command/undo stack	new src/api/command.rs
P3	Xref attach/detach	new src/api/xref.rs
The largest immediate value-add for a CAD app would be P1 items — layer/table helpers, entity filtering, block creation, and per-entity bounding box — since those are needed by virtually every application-level operation.

