# AutoCAD Command Gap Analysis for acadrust SDK

Date: 2026-04-16
Repository: acadrust

## Executive Summary

This report analyzes acadrust against the AutoCAD command universe and identifies SDK gaps.

Key findings:

- acadrust is strong in file-format interoperability (DXF/DWG read/write, broad entity model), but it is not yet a full command-execution CAD SDK.
- The command universe is very large (about 1590 commands in common public indexes). Most UI, visualization, and interactive-editing commands are outside current scope.
- Core data-model command families are in place (entity creation, transforms, blocks, layouts, layers, xrefs metadata), but computational geometry and parametric/constraint workflows are major gaps.
- There are a few high-impact roundtrip gaps in DWG writing paths for specific entity/object types.

---

## Scope and Sources

### AutoCAD command universe sources

- Autodesk AutoCAD 2026 official command index (A-Z):
  - https://help.autodesk.com/view/ACD/2026/ENU/?page=commands
  - https://help.autodesk.com/view/ACD/2026/ENU/?page=commands&q=*
  - Letter filters validated: `?q=A*`, `?q=B*`, `?q=C*`
- CADForum command dictionary:
  - https://www.cadforum.cz/en/command.asp?ver=2026
  - Indicates overview of about 1590 AutoCAD commands.

### Repository deep-search scope

Reviewed with targeted scans and source reads across:

- `src/document.rs`
- `src/api/*.rs`
- `src/entities/mod.rs` and entity files
- `src/objects/mod.rs` and object files
- `src/tables/mod.rs` and table files
- `src/io/**` read/write pipelines
- `Cargo.toml` feature gating

Evidence-focused reads for unsupported/stub behavior:

- `src/io/dwg/dwg_stream_writers/object_writer/entities.rs`
- `src/io/dwg/dwg_stream_writers/object_writer/objects.rs`
- `src/io/dxf/reader/section_reader.rs`
- `src/io/mod.rs`

---

## Capability Inventory (Current SDK Surface)

### 1. Core document and operation API

Document-level capabilities are broad for batch and programmatic editing:

- Entity CRUD and queries
  - `add_entity`, `remove_entity`, `get_entity`, `get_entity_mut`, `copy_entity`
- Transform operations
  - `move_entity`, `rotate_entity`, `scale_entity`
- Explode
  - `explode_entity`
- Layout operations
  - `add_layout`, `rename_layout`, `remove_layout`, `add_entity_to_layout`
- Layer and style ensure helpers
  - `ensure_layer`, `ensure_linetype`, `ensure_text_style`, `ensure_dim_style`, `ensure_app_id`
- Group operations
  - `create_group`, `remove_group`, group query APIs
- Xref metadata-level operations
  - `attach_xref`, `detach_xref`, `xrefs`, `xref_info`
- IO convenience APIs
  - `from_file`, `save`, `from_bytes`, `to_bytes`, `to_dxf_bytes`
- New event and distance utilities
  - `events`, `drain_events`, `clear_events`
  - `distance_between_handles`, `nearest_distance_from`

### 2. High-level helper modules

- `src/api/command.rs`
  - Snapshot-based undo/redo (`CommandHistory`)
- `src/api/selection.rs`
  - Selection sets and batch property changes (`SelectionSet`)
- `src/api/block.rs`
  - Fluent block definition builder (`BlockBuilder`)

### 3. Entity model breadth

`EntityType` currently includes 42 variants in source (`src/entities/mod.rs`), including:

- 2D/3D primitives and annotation: line, arc, circle, ellipse, text, mtext, dimension, hatch, leader, multileader
- Polyline families: lwpolyline, polyline2d/3d, polyface mesh, polygon mesh
- Structural/block entities: insert, block, blockend, attrib/attdef, seqend
- Advanced/model entities: solid3d, region, body, mesh, mline, raster image, underlay, table, wipeout, ole2frame
- Unknown fallback entity

### 4. Object and table support

- `ObjectType` includes 23 object variants (tuple + unknown struct variant), including dictionaries, layouts, xrecords, groups, mline style, multileader style, table style, scale, sortents, visual/material/geodata/spatialfilter stubs, and unknown fallback.
- Tables include 9 table domains:
  - layer, linetype, text style, block record, dim style, app id, view, vport, ucs.

### 5. File format and import scope

- DXF read/write: broad support
- DWG read/write: broad support, but with targeted gaps (see Gap Register)
- Feature-gated import module (`import` feature in `Cargo.toml`): STL, COLLADA, OBJ, glTF/GLB, FBX

---

## Command-Family Coverage Map

This section maps the command universe by family, not only by individual command names. This is the only scalable way to cover "all commands" at AutoCAD scale.

Status legend:

- Supported: direct and practical SDK support exists
- Partial: represented in data model and/or IO, but no robust operation layer
- Gap: missing or only fallback/stub behavior

## Family Matrix

| Family | Typical AutoCAD Commands | SDK Status | Notes |
|---|---|---|---|
| Basic draw primitives | LINE, CIRCLE, ARC, ELLIPSE, POINT, PLINE, SPLINE, XLINE, RAY | Supported | Entity model + DXF/DWG IO are strong. |
| Annotation draw | TEXT, MTEXT, DIM*, LEADER, MLEADER, TOLERANCE | Supported | Core entity support present; advanced associativity workflows are limited. |
| Block creation/insertion | BLOCK, INSERT, ATTDEF, ATTEDIT-like data paths | Supported | BlockBuilder + insert/block entities are present. |
| Layer/style authoring | LAYER, LINETYPE, STYLE, DIMSTYLE | Supported | Table entries and ensure helpers are present. |
| Layout/viewport data operations | LAYOUT, VPORT, PSPACE/MSPACE data management | Partial | Layout data ops exist; interactive viewport tooling is not SDK-level. |
| Selection and batch property edits | QSELECT-like, CHPROP-like workflows | Supported | SelectionSet batch mutation patterns are available. |
| Transform family | MOVE, ROTATE, SCALE, MIRROR | Partial | Move/rotate/scale document APIs exist; mirror is entity-level, not full document command workflow. |
| Copy/erase/explode | COPY, ERASE, EXPLODE | Supported | `copy_entity`, `remove_entity`, `explode_entity` exist. |
| Computational modify geometry | TRIM, EXTEND, FILLET, CHAMFER, OFFSET, JOIN, BREAK, STRETCH | Gap | No complete computational geometry command layer. |
| Arrays and patterning | ARRAY, ARRAYPATH, ARRAYPOLAR, ARRAYRECT | Gap | No dedicated array command APIs. |
| Constraint/parametric | AUTOCONSTRAIN, CONSTRAINTSETTINGS, PARAMETER families | Gap | No constraint solver / parametric command subsystem. |
| 3D procedural modeling | EXTRUDE, REVOLVE, SWEEP, LOFT, PRESSPULL, UNION, SUBTRACT, INTERSECT | Gap | 3D entity storage exists, but operation commands are not implemented as modeling algorithms. |
| Underlay workflows | PDFATTACH, DGNATTACH, DWFATTACH, CLIP underlays | Partial | Underlay entity exists; write-path gaps remain in DWG path (see Gap Register). |
| Plot/publish workflows | PLOT, PAGESETUP, PUBLISH, EXPORT variants | Partial | Plot-related objects/fields exist; no end-to-end command workflow engine. |
| Compare/audit/recovery style flows | COMPARE, AUDIT, RECOVER-like workflows | Partial | Validation exists, but not full command parity with UI-driven AutoCAD behavior. |
| UI/palette/macros | CUI, TOOLBAR, palettes, action recorder commands | Gap | Out of scope for current SDK architecture. |
| Automation app ecosystem | APPLOAD, ARX, VBA, command macros | Gap | No equivalent extension runtime; SDK is a data/IO library. |

---

## Deep Gap Register (Evidence-backed)

## P0 - High impact gaps (roundtrip and interoperability risk)

1. DWG writer silently skips some entity types.

- Evidence: `src/io/dwg/dwg_stream_writers/object_writer/entities.rs`
- Current behavior includes skip paths for types such as table/underlay in writer dispatch.
- Risk: entity loss on DWG output.

2. DWG writer skips or cannot serialize several object variants.

- Evidence: `src/io/dwg/dwg_stream_writers/object_writer/objects.rs`
- Stub/unsupported object variants are skipped.
- Risk: object metadata loss (visual/material/geodata/spatial filter/table style related domains).

3. Unknown/non-supported parse fallback paths are intentionally lossy in operation semantics.

- Evidence: `src/io/dxf/reader/section_reader.rs`
- Unsupported entities/objects are read into unknown/fallback representations.
- Risk: geometry and rich semantics unavailable for command-style mutation despite raw preservation possibilities.

## P1 - Core command-parity gaps (major feature work)

4. No computational geometry command layer.

- Missing command-equivalent operations:
  - trim/extend/fillet/chamfer/offset/join/break/stretch
- Impact:
  - cannot provide core CAD modify workflows expected from command-driven applications.

5. No procedural 3D modeling command layer.

- Missing operation-equivalent commands:
  - extrude/revolve/sweep/loft/booleans (union/subtract/intersect), presspull
- Impact:
  - 3D entities can be represented, but not generated/edited by command-style modeling ops.

6. Constraint and parametric command family absent.

- Missing:
  - geometric constraints, dimensional constraints solver workflows.

## P2 - Ecosystem and UX command parity gaps

7. UI command families out of scope.

- CUI/toolbars/palettes/action recorder behaviors are not represented in current architecture.

8. Plot/publish workflow commands are only partially represented as data.

- Plot/page setup objects and fields are present, but no full command workflow orchestration.

9. Xref command parity is partial.

- Metadata and some APIs exist, but full attach/reload/bind management semantics remain limited compared to AutoCAD command suite.

---

## Strengths Worth Keeping

1. Breadth of entity and object modeling is already high.

- This is an excellent foundation for command-layer expansion.

2. Strong file IO architecture with fallback behavior.

- Unknown-type handling and notifications are valuable for robust interoperability pipelines.

3. Existing command-adjacent helpers are a good base.

- CommandHistory, SelectionSet, and BlockBuilder can become the backbone for a higher-level command engine.

---

## Prioritized Roadmap for Command-Parity Expansion

## Phase 1 (Immediate)

Goal: close interoperability risk gaps first.

- Implement DWG writer support for currently skipped entity/object cases where practical.
- Improve unknown object/entity preservation write paths where skipping occurs.
- Add regression tests focused on lossless roundtrip for these domains.

## Phase 2 (Core modify command engine)

Goal: support the most expected non-UI AutoCAD modify commands.

- Add geometry kernel modules for:
  - intersections
  - offsets (line/polyline/arc first)
  - trim/extend against boundaries
  - fillet/chamfer for linear and circular primitives
- Add document-level wrappers:
  - `trim_entities`, `extend_entities`, `offset_entity`, `fillet_entities`, `chamfer_entities`, `join_entities`, `break_entity`.

## Phase 3 (3D command layer)

Goal: support practical 3D modeling command parity.

- Add procedural generation ops (extrude/revolve/sweep/loft).
- Add boolean solids pipeline.
- Align resulting entities with existing `Solid3D/Region/Body` representations.

## Phase 4 (constraints and advanced workflows)

Goal: parametric/constraint family coverage.

- Introduce constraint graph and solver infrastructure.
- Bind dimensions/constraints to entity geometry with update propagation.

---

## Detailed Command Family Notes (Representative Coverage)

## Draw and annotate families

Representative commands considered:

- ARC, CIRCLE, ELLIPSE, LINE, PLINE, SPLINE, POINT, XLINE
- TEXT, MTEXT, DIM*, LEADER, MLEADER, HATCH, TABLE, INSERT, BLOCK

Current assessment:

- Data-model coverage is strong.
- Command-execution workflows (interactive prompts, advanced option trees) are intentionally outside current scope.

## Modify families

Representative commands considered:

- MOVE, ROTATE, SCALE, MIRROR, COPY, ERASE, EXPLODE
- TRIM, EXTEND, FILLET, CHAMFER, OFFSET, JOIN, BREAK, STRETCH

Current assessment:

- First group has practical SDK equivalents.
- Second group is mostly missing and should be treated as major feature work.

## Layers/styles/layouts/xrefs

Representative commands considered:

- LAYER, LINETYPE, STYLE, DIMSTYLE
- LAYOUT, VPORTS
- XREF, XBIND-like family

Current assessment:

- Table and layout metadata operations are reasonably mature.
- Full command parity with AutoCAD xref and viewport interaction workflows is partial.

## 3D and constraints

Representative commands considered:

- BOX/CONE/CYLINDER family (entity/data-level)
- EXTRUDE/REVOLVE/SWEEP/LOFT/UNION/SUBTRACT/INTERSECT/PRESSPULL
- AUTOCONSTRAIN and constraint families

Current assessment:

- Storage/interchange exists for many 3D entities.
- Modeling operation commands and constraint solving are not yet implemented.

---

## Practical Gap Checklist (for SDK planning)

Use this as a release checklist for command-family parity:

- [ ] No silent DWG skips for supported entity/object domains.
- [ ] Roundtrip tests for underlay/table/object edge cases.
- [ ] Geometry kernel foundation (intersection/offset/trim/extend).
- [ ] Modify command wrappers with deterministic APIs and tests.
- [ ] 3D procedural op APIs with solid model output.
- [ ] Boolean operation support.
- [ ] Constraint graph + solver integration.
- [ ] Xref lifecycle API parity improvements.
- [ ] Plot/publish orchestration APIs (non-UI).

---

## Final Verdict

acadrust currently provides a strong CAD data/IO SDK and partial command-family coverage for programmatic workflows. It does not yet implement broad AutoCAD command parity, especially in computational geometry, constraints, and interactive/UI-driven command families.

For a command-complete SDK trajectory, prioritize:

1. roundtrip-loss P0 fixes,
2. geometry modify command engine,
3. 3D modeling operations,
4. constraints and advanced workflow layers.
