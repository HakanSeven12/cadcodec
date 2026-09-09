# Audit Continuation

## Current Tracking

[Entity status matrix](entity_status_matrix.md) catalogs 71 fixture cases, every public `EntityType` variant, indirect structural records, and excluded extended/dynamic families across AC1012 through AC1032. Separate matrices cover AutoCAD 2027 and BricsCAD, each for DWG, ASCII DXF, and binary DXF. The JSON sibling contains evidence paths and hashes.

Statuses distinguish native presence, proxy fallback, changed type, missing records, audit errors, blocked drawing opens, stale evidence, unavailable fixtures, and untested cases. A successful audit alone is not marked as native compatibility. Surface subtypes are checked explicitly. BricsCAD's PDFREFERENCE/DWFREFERENCE/DGNREFERENCE are accepted native underlay aliases.

## Confirmed Fixes

- Removed two abstract base classes from fresh default registration: `AcDbAssocActionParam` and `AcDbAssocPointRefActionParam`. Each declaration independently makes AutoCAD 2027 reject an otherwise valid AC1032 LINE drawing. Other action-parameter subclasses remain registered. Imported declarations are retained.
- AutoCAD now directly opens LINE controls for all eight DWG versions with zero audit errors. AC1027 and AC1032 previously failed. BricsCAD's AC1027 control also passes.
- Embedded LINE profiles now use the same lossless verification policy as embedded REGION: if decoding and re-encoding do not reproduce the original meaningful bits, preserve the opaque profile. This avoids corrupting unrecognized native construction payloads; it is not a claim that the complete EXTRUDEDSURFACE round-trip is fixed.

## Evidence

`target/class-single/validation/ACAD2027/` contains the isolated declaration probes. Classes 56 (`ACDBASSOCACTIONPARAM`) and 59 (`ACDBASSOCPOINTREFACTIONPARAM`) failed, while the other declarations in that probe range passed. `target/class-modes/` confirms that omitting both permits the full remaining class table to open; merely changing DWG metadata or DXF names does not fix it. This replaces the earlier unconfirmed oversized-class-table hypothesis.

`target/entity-version-controls/` contains the regenerated eight-version LINE controls. The optional AutoCAD integration test also covers AC1027/AC1032 alongside the existing empty, mixed, preview and multipage AC1021 controls.

`target/entity-matrix-isolated/` holds the fresh per-entity AC1021 DWG probes. Do not infer that their results extend to other untested versions.

`target/entity-matrix-isolated-AC1032/` holds the corresponding AC1032 probes. AutoCAD results:

| Version | Native, Audit-Clean | Proxy | Fails Direct Open | Version-Excluded |
|---|---:|---:|---:|---:|
| AC1021 | 60 | 3 | 5 | 3 |
| AC1032 | 64 | 3 | 4 | 0 |

Both versions have CAMERA, RTEXT and ARCALIGNEDTEXT proxies. EXTRUDEDSURFACE, LOFTEDSURFACE, REVOLVEDSURFACE and SWEPTSURFACE fail both versions; NURBSURFACE fails AC1021 but passes AC1032. The full BricsCAD DWG matrix was refreshed: AC1012 opens with all 44 cases and zero audit errors, while the later rich DWGs still fail. All eight ASCII/binary DXF pairs open in both engines with zero audit errors; proxy cells remain distinct from native passes.

The full Rust test suite and `cargo check --all-targets` pass. AutoCAD's optional integration regression passed all six controls, including the newly added AC1027/AC1032 cases.

## Remaining Work

- Advanced surface construction records still fail DWG direct open or round-trip. The atlas currently uses a generic planar SAT sheet and default construction metadata for subtypes. Its DXF inventory retains native subtype names, but an EXTRUDEDSURFACE saved from that DXF became a generic surface. Native inventory does not prove construction-history preservation on save.
- An AutoCAD-authored EXTRUDEDSURFACE with an embedded profile also fails after a library DWG round-trip. A lossless embedded LINE fallback fixes one independently demonstrated corruption, but other fields still need comparison.
- The full DWG atlases still require entity-by-entity repair; clean LINE controls are not whole-format certification.
- Proxy fallback and external asset dependencies remain visible in the matrices.

## Commands

```powershell
cargo run --example entity_atlas -- target/entity-atlas
scripts/validate_entity_atlas.ps1 -Engine ACAD2027
scripts/validate_entity_atlas.ps1 -Engine BCAD -Filter '*dxf*' -TimeoutSeconds 22
scripts/validate_entities_individually.ps1 -Version AC1021 -Engine ACAD2027 -Format dwg
scripts/validate_entities_individually.ps1 -Root target/entity-matrix-isolated-AC1032 -Version AC1032 -Engine ACAD2027 -Format dwg
scripts/entity_status_matrix.ps1
cargo test --tests
```

Tests and automation remain console-only. IntelliCAD is excluded.
