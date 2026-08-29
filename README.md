# acadrust

[![Crates.io](https://img.shields.io/crates/v/acadrust.svg)](https://crates.io/crates/acadrust)
[![Documentation](https://docs.rs/acadrust/badge.svg)](https://docs.rs/acadrust)
[![License: MPL 2.0](https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg)](https://opensource.org/licenses/MPL-2.0)

**A pure Rust library for reading and writing CAD files (DXF and DWG).**

Supports DXF (ASCII & Binary) and DWG (R13â€“R2018) files.

## Quick Start

```toml
[dependencies]
acadrust = "0.5.1"
```

```rust
use acadrust::{CadDocument, DxfReader, DxfWriter};

fn main() -> acadrust::Result<()> {
    // Read
    let doc = DxfReader::from_file("input.dxf")?.read()?;
    println!("{} entities", doc.entities().count());

    // Write
    let writer = DxfWriter::new(&doc);
    writer.write_to_file("output.dxf")?;
    Ok(())
}
```

## Features

- **DXF Read/Write** â€” ASCII and Binary formats, R12â€“R2018+
- **DWG Read/Write** â€” Native binary, R13â€“R2018 (208/208 roundtrip-perfect)
- **41 Entity Types** â€” Lines, arcs, polylines, hatches, dimensions, 3D solids, viewports, and more
- **Tables & Objects** â€” Layers, linetypes, styles, dictionaries, layouts, materials
- **Serde Support** â€” Optional `Serialize`/`Deserialize` for all types (`features = ["serde"]`)
- **Failsafe Mode** â€” Error-tolerant parsing with structured diagnostics
- **Encoding Support** â€” ~40 code pages for pre-2007 files

## File Version Support

| Version | AutoCAD | DXF | DWG |
|---------|---------|-----|-----|
| AC1009 | R12 | âœ… | â€” |
| AC1012â€“AC1014 | R13â€“R14 | âœ… | âœ… |
| AC1015â€“AC1032 | 2000â€“2018+ | âœ… | âœ… |

## Examples

<details>
<summary>DWG Read/Write</summary>

```rust
use acadrust::{CadDocument, DwgWriter};
use acadrust::io::dwg::DwgReader;
use acadrust::entities::*;
use acadrust::types::{Color, Vector3};

fn main() -> acadrust::Result<()> {
    // Read DWG
    let mut reader = DwgReader::from_file("drawing.dwg")?;
    let doc = reader.read()?;

    // Iterate entities
    for entity in doc.entities() {
        println!("{:?}", entity);
    }

    // Create & Write DWG
    let mut doc = CadDocument::new();
    let mut line = Line::from_coords(0.0, 0.0, 0.0, 100.0, 50.0, 0.0);
    line.common.color = Color::RED;
    doc.add_entity(EntityType::Line(line))?;
    DwgWriter::write_to_file("output.dwg", &doc)?;
    Ok(())
}
```
</details>

<details>
<summary>Paper Space Layouts & Viewports</summary>

```rust
use acadrust::{CadDocument, DxfVersion, DxfWriter};
use acadrust::entities::{EntityType, Viewport};
use acadrust::types::Vector3;

fn main() -> acadrust::Result<()> {
    let mut doc = CadDocument::with_version(DxfVersion::AC1027);

    // Add geometry to model space
    let line = acadrust::entities::Line::from_coords(0.0, 0.0, 0.0, 100.0, 100.0, 0.0);
    doc.add_entity(EntityType::Line(line))?;

    // Overall viewport (ID=1) for default Layout1
    let mut overall_vp = Viewport::new();
    overall_vp.id = 1;
    overall_vp.center = Vector3::new(148.5, 105.0, 0.0);
    doc.add_paper_space_entity(EntityType::Viewport(overall_vp))?;

    // Detail viewport using builder pattern
    let vp1 = Viewport::new()
        .with_center(Vector3::new(148.5, 105.0, 0.0))
        .with_view_target(Vector3::new(50.0, 50.0, 0.0))
        .with_scale(1.0)
        .with_locked();
    doc.add_paper_space_entity(EntityType::Viewport(vp1))?;

    // Create a second layout with its own viewport
    doc.add_layout("Layout2")?;
    let mut vp2 = Viewport::with_size(Vector3::new(200.0, 150.0, 0.0), 400.0, 300.0);
    vp2.id = 1;
    doc.add_entity_to_layout(EntityType::Viewport(vp2), "Layout2")?;

    DxfWriter::new(&doc).write_to_file("layouts.dxf")?;
    Ok(())
}
```
</details>

<details>
<summary>Serde / JSON</summary>

```rust
use acadrust::{CadDocument, DxfReader};

fn main() -> acadrust::Result<()> {
    let doc = DxfReader::from_file("drawing.dxf")?.read()?;
    let json = serde_json::to_string_pretty(&doc).unwrap();
    let doc2: CadDocument = serde_json::from_str(&json).unwrap();
    println!("Entities: {}", doc2.entities().count());
    Ok(())
}
```
</details>

## Documentation

Full API docs: [docs.rs/acadrust](https://docs.rs/acadrust)

---

## Changelog


### 0.5.1

- **Docs cleanup** - Removed the stale "IFCCAD foundation" feature bullet from the README (the feature never shipped in the published crate).

### 0.5.0

- **BricsCAD / AutoCAD 2026 compatibility** â€” Round-tripped DXF files now open without the recovery prompt: EED handles are decoded/encoded big-endian, `$PSTYLEMODE` is written as a boolean (290) so layer plot-style references validate, the ACAD RegApp record is emitted first, built-in materials use the minimal form, unrestorable associative-framework objects and empty map file names are dropped, dictionary-with-default records match BricsCAD's export, and `$CELWEIGHT` is sanitized to a valid lineweight.

- **DXF read/write cycle stability** â€” Handle-less records (R12-era files) receive handles, colliding defaults are re-handled, `DictionaryWithDefault::default_handle` follows remapped objects, MLINESTYLE angles round-trip in degrees/radians symmetrically, and NOD entry ownership matches what the writer emits. Duplicate handles and dangling references that made CAD applications discard drawings are gone.

- **DWG decoding fixes** â€” AC15 (R13â€“R2000) files whose AuxHeader sits behind the Handles section now read fully (AcDbObjects inferred from the Classes-to-Handles gap); code-page strings and MIF `\U+XXXX` escapes decode correctly, with unmappable characters written as MIF escapes.

- **AC1032 writer fixes (issue #45)** â€” `$ACADMAINTVER` uses group code 90, manually added layers get real non-zero handles below `$HANDSEED`, and LAYER records carry the required 390 plot-style pointer.

- **LWPOLYLINE down-save** â€” Plain 2D polylines are written as LWPOLYLINE for R2000+ output instead of the legacy POLYLINE/VERTEX/SEQEND form, matching what CAD applications expect (issue #63 follow-up).

- **DXF hard-owner alignment (issue #63)** â€” `ACAD_FIELD` is written as a hard owner (360) so applications no longer erase it; remaining NOD entries match BricsCAD's own export.

- **Community contributions** â€” Solid history graph management, exact ACIS spline topology, lump/shell chain traversal, block record base points, viewport shadow layer normalization, case-insensitive space records, entity extension-dictionary remapping, large mesh face stream preservation, and dimension definition point fixes.


### 0.4.1

- **MTEXT formatting** â€” Added a structured MTEXT format parser with richer control-code handling, including escaped semicolons, caret codes, legacy `%%u`/`%%o`/`%%nnn` text codes, line-spacing style, and relative-vs-absolute height scalars.

- **Expanded entity coverage** â€” Added read/write and round-trip support for `HELIX`, `ACAD_TABLE` cell content, PDF/DWF/DGN underlay references and definitions, ACAD surface entities, SPATIAL_FILTER clip boundaries, complex linetype shapes/text, and additional surface/body/history fields.

- **ACIS and 3D solid reliability** â€” Improved planar B-rep and NURBS spline-surface output, transformed ACIS body geometry correctly, linked R2013+ 3DSOLID/REGION/BODY geometry to AcDs SAB blobs, and fixed several AcDs record pairing/search/index layouts.

- **DWG/DXF interoperability fixes** â€” Tightened DWG writer conformance for AutoCAD round-trips, R2018 MLEADER/MTEXT column handling, viewport and plot settings persistence, xref block preservation, ENC color/transparency decoding, spline scenario detection, dimension angles/group codes, TEXT thickness/generation flags, and POLYLINE routing by flags.

- **Performance and security** â€” Removed quadratic AcDs SAB scans, made `SatDocument::record()` O(1), bounded SAB end-marker searches, and added a JSON recursion depth limit to prevent stack-overflow denial of service in glTF import.


### 0.4.0

- **Annotative styles** â€” `TextStyle`, `DimStyle`, and `TableStyle` now carry an `annotative` flag, persisted the standard way via `AcadAnnotative` XDATA/EED in both DXF and DWG.

- **AcDbGeoData decode** â€” DWG reader now decodes the `AcDbGeoData` coordinate-system definition.

- **CANNOSCALE header vars** â€” Read/write support for the `CANNOSCALE` and `CANNOSCALEVALUE` header variables in DXF.

- **VPORT visual style** â€” Render mode / visual style is persisted through both DXF and DWG; duplicate and tiled `*Active` VPORT entries are preserved instead of being collapsed.

- **Layout paper dimensions** â€” Paper size and plot rotation are exposed on `Layout`.

- **DWG reader robustness** â€” Hatch boundary-handle counts capped with `safe_count`; raster-image / wipeout clip-boundary vertices retained; 3DFACE corners 2â€“4 always decode Z with BD-default; invalid page offsets from gap entries no longer computed.

- **DXF reader fixes** â€” Improved 3D-point header parsing, null entity-handle allocation, BlockRecord initialization ordering, and configurable default encoding. Mirrored explode now produces correct arc/ellipse handedness and OCS centers.

- **DWG roundtrip** â€” Roundtrip workflows across supported versions with newline sanitization and improved reader alignment handling.


### 0.3.4

- **DWG roundtrip expanded** â€” Roundtrip workflows now cover supported DWG versions end-to-end, with additional byte-level diagnostics and compatibility fixes in the writer pipeline.

- **DXF output compatibility** â€” ASCII and Binary DXF roundtrip support tightened across multiple versions, including symbol name sanitization, corrected subclass marker emission, and newline-to-`\P` paragraph marker conversion in Binary DXF strings.

- **ACIS downgrade support** â€” ACIS SAT/SAB handling now downgrades incompatible record layouts for older consumers, improving 3DSOLID interoperability.

- **AC1021 encoding fix** â€” Corrected RS encoding behavior for AutoCAD 2007-class DWG files.


### 0.3.2

- **Entity explode** â€” `EntityType::explode()` decomposes complex entities (polylines, hatches, meshes, dimensions, etc.) into simpler primitives (lines, arcs, faces); `CadDocument::explode_entity()` allocates handles automatically

- **Centralized transform/mirror/translate** â€” Transformation logic extracted from 38 entity files into `translate.rs`, `transform.rs`, and `mirror.rs` modules; all Entity trait implementations delegate to these centralized functions. Direct `EntityType` dispatch methods added (`entity.translate()`, `entity.apply_transform()`, `entity.mirror_x()`, etc.) alongside the existing trait-based API.

- **DWG parser/writer fixes**

### 0.3.0

- **ACI color support** â€” Full 256-entry AutoCAD Color Index (ACI) to RGB lookup table, `Color::rgb()` resolves index colors, `Color::approximate_index()` finds nearest ACI match for true colors

- **Hatch edge fix** â€” Corrected hatch edge reading/writing issues

- **LwPolyline bulge fix** â€” Fixed bulge value handling in parser and writer

- **Performance optimizations** â€” Zero-allocation number formatting with `itoa`/`ryu`, buffered I/O, reduced memory allocations throughout DXF read/write pipeline. Parsing/writing speed are dramatically increased.

- **Table entry deduplication** â€” `add_or_replace` for table entries eliminates handle collisions during read

#### Breaking API change

- **`BlockRecord` entity storage** â€” `BlockRecord` now stores `entity_handles: Vec<Handle>` instead of owning entities directly. All entities live in flat storage inside `CadDocument` with O(1) handle-based lookup. If you accessed block entities directly, use `doc.get_entity(handle)` instead:
  ```rust
  // Before (0.2.x): iterating block entities directly
  // for entity in &block_record.entities { ... }

  // After (0.3.0): resolve handles through the document
  for &handle in &block_record.entity_handles {
      if let Some(entity) = doc.get_entity(handle) {
          // use entity
      }
  }
  ```
  The `CadDocument` public API (`add_entity()`, `entities()`, `get_entity()`, `get_entity_mut()`) is unchanged.

### 0.2.10

- **Paper space & layout support** â€” `add_paper_space_entity()`, `add_entity_to_layout()`, `add_layout()` API for creating viewports in multiple paper space layouts
- **Correct DXF paper space structure** â€” Active layout (`*Paper_Space`) entities in ENTITIES section with code 67; non-active layouts (`*Paper_Space0`, `*Paper_Space1`, â€¦) entities inside BLOCK definitions
- **AutoCAD AUDIT compatibility** â€” Fixed code 67 paper space flag, MLineStyle angle conversion (radiansâ†’degrees), AcDbPlotSettings flag, viewport owner handles
- **DXF reader** â€” Proper handling of code 67 (paper space flag) in common entity parsing

### 0.2.9

- **ACIS 3DSOLID write support** â€” SAT text builder (R2000â€“R2007) and SAB binary (R2013+) with primitives: box, wedge, pyramid, cylinder, cone, sphere, torus
- **`SatDocument` builder API** â€” `add_plane_surface`, `add_cone_surface`, `add_sphere_surface`, `add_torus_surface`, `add_straight_curve`, `add_ellipse_curve`
- **208/208 DWG roundtrip integrity** â€” Zero field drift across 26 entity types Ã— 8 versions

### 0.2.8

- **DWG binary read** â€” Full DWG reader for R13 through R2018
- **DWG binary write** â€” Full DWG writer for R13 through R2018
- **Handle resolution** â€” Automatic owner handle assignment after read

### 0.2.7

- **Optional serde support** â€” `Serialize`/`Deserialize` for all document types with `features = ["serde"]`
- **JSON/YAML round-trip** â€” Full document serialization and deserialization

### 0.2.6

- **41 entity types** â€” Added MultiLeader, Table, MLine, Mesh, Underlay, Ole2Frame, Wipeout, Shape, and more
- **Objects** â€” Dictionaries, Groups, Layouts, MLineStyle, MultiLeaderStyle, TableStyle, PlotSettings, Scale, Materials, VisualStyle, GeoData
- **CLASSES section** â€” Full read/write support
- **Extended data (XData)** â€” Full support for application-specific extended data
- **Reactors & extension dictionaries** â€” Read/write for all entity and object types

### 0.2.0â€“0.2.5

- ASCII and Binary DXF read/write
- Core entity types (Point, Line, Circle, Arc, Ellipse, Polyline, LwPolyline, Text, MText, Spline, Dimension, Hatch, Solid, Face3D, Insert, Viewport)
- Table system (Layer, LineType, TextStyle, DimStyle, BlockRecord, AppId, View, VPort, UCS)
- Encoding support (~40 code pages)
- Failsafe reading mode
- Unknown entity preservation

---


## Used By
-[Open CAD Studio](https://github.com/HakanSeven12/OpenCADStudio) An open-source (GPLv3) CAD application that uses acadrust as its core native DWG/DXF engine for read/write operations and 3D modeling.

## License

MPL-2.0 â€” see [LICENSE](LICENSE).

## Acknowledgments

- [ACadSharp](https://github.com/DomCR/ACadSharp) â€” the C# library that inspired this project

