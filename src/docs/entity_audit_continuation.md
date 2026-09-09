# Audit Continuation

## Current Tracking

[Entity status matrix](entity_status_matrix.md) catalogs 71 fixture cases, every public `EntityType` variant, indirect structural records, and excluded extended/dynamic families across AC1012 through AC1032. Separate matrices cover AutoCAD 2027 and BricsCAD, each for DWG, ASCII DXF, and binary DXF. The JSON sibling contains evidence paths and hashes.

Statuses distinguish native presence, proxy fallback, changed type, missing records, audit errors, blocked drawing opens, stale evidence, unavailable fixtures, and untested cases. A successful audit alone is not marked as native compatibility. Surface subtypes are checked explicitly. BricsCAD's PDFREFERENCE/DWFREFERENCE/DGNREFERENCE are accepted native underlay aliases.

## Confirmed Fixes

- Removed two abstract base classes from fresh default registration: `AcDbAssocActionParam` and `AcDbAssocPointRefActionParam`. Each declaration independently makes AutoCAD 2027 reject an otherwise valid AC1032 LINE drawing. Other action-parameter subclasses remain registered. Imported declarations are retained.
- AutoCAD now directly opens LINE controls for all eight DWG versions with zero audit errors. AC1027 and AC1032 previously failed. BricsCAD's AC1027 control also passes.
- Embedded LINE profiles now use the same lossless verification policy as embedded REGION: if decoding and re-encoding do not reproduce the original meaningful bits, preserve the opaque profile. This avoids corrupting unrecognized native construction payloads; it is not a claim that the complete EXTRUDEDSURFACE round-trip is fixed.
- R13/R14 MTEXT no longer writes the R2000-only line-spacing fields. The reader uses the legacy defaults for these versions.
- R13/R14 VIEWPORT uses its short entity body, ACAD/MVIEW EED, and VX headers. Slot zero is reserved so the paper-space overview and floating viewport retain valid IDs. MVIEW updates preserve unrelated structured ACAD values and reject malformed view parameters.
- Legacy EED string code pages use the native big-endian byte order. Reading also accepts the byte-swapped form emitted by older acadrust versions.
- Legacy modeler SAT blocks use the native flag, printable-byte substitution excluding spaces, CRLF separators, and no standalone SAT end marker. Classic SAT 700 is normalized to SAT 400 for R13/R14/R2000, including the older edge layout. R2004 SAT-origin geometry uses version-2 SAB. Verified geometry is the seven analytic solid fixtures, REGION and BODY, not arbitrary newer ASM schemas.
- ARCALIGNEDTEXT's six D2T numeric fields are written/read as text, not binary doubles. This fixes BricsCAD's `Object improperly read: <AcDbArcAlignedText>` failure. DXF arc-text angles now convert between API radians and DXF degrees.
- The DWG reader now dispatches all mapped entity sentinels, including arc/jogged dimensions, CAMERA, SECTIONOBJECT, ARCALIGNEDTEXT, RTEXT, position markers, point clouds, MPOLYGON and proxies. The previous entity predicate stopped at LIGHT and silently omitted the later types.

## Evidence

`target/class-single/validation/ACAD2027/` contains the isolated declaration probes. Classes 56 (`ACDBASSOCACTIONPARAM`) and 59 (`ACDBASSOCPOINTREFACTIONPARAM`) failed, while the other declarations in that probe range passed. `target/class-modes/` confirms that omitting both permits the full remaining class table to open; merely changing DWG metadata or DXF names does not fix it. This replaces the earlier unconfirmed oversized-class-table hypothesis.

`target/entity-version-controls/` contains the regenerated eight-version LINE controls. The optional AutoCAD integration test also covers AC1027/AC1032 alongside the existing empty, mixed, preview and multipage AC1021 controls.

`target/entity-matrix-isolated/` holds the fresh per-entity AC1021 DWG probes. Do not infer that their results extend to other untested versions.

`target/entity-matrix-isolated-AC1012/` through `target/entity-matrix-isolated-AC1032/` contain the other version probes. The latest source regenerated every isolated corpus so obsolete hashes cannot be mistaken for fresh evidence. The live matrix is authoritative for current per-entity results.

Complete DWG atlas results after the legacy and arc-text repairs:

| DWG Version | Included Cases | AutoCAD 2027 | BricsCAD |
|---|---:|---|---|
| AC1012 | 44 | 44 present, audit 0 | 44 present, audit 0 |
| AC1014 | 49 | 49 present, audit 0; 2 proxies | 49 present, audit 0; 1 proxy |
| AC1015 | 49 | 49 present, audit 0; 2 proxies | 49 present, audit 0; 1 proxy |
| AC1018 | 52 | 52 present, audit 0; 2 proxies | 52 present, audit 0; 1 proxy |
| AC1021 | 68 | Direct open blocked | Direct open blocked |
| AC1024 | 70 | Direct open blocked | Direct open blocked |
| AC1027 | 71 | Direct open blocked | Direct open blocked |
| AC1032 | 71 | Direct open blocked | Direct open blocked |

ARCALIGNEDTEXT is now native and audit-clean in BricsCAD on every included version (AC1014 through AC1032); AutoCAD Core Console retains it as a proxy. Its repaired reader also decodes BricsCAD's native reference from `target/arc-native-reference.dwg`. The six D2T fields agree with [LibreDWG's record description](https://github.com/LibreDWG/libredwg/blob/master/src/dwg2.spec); the supplied ODA PDF covers the common DWG/EED layouts but does not list this Express Tools entity. AC1021 ELLIPSE and LWPOLYLINE pass on retry; their previous failures were engine startup failures.

All eight ASCII/binary DXF pairs have completed audit-clean runs in both engines. Proxy cells remain distinct from native passes. Native identity and audit do not prove geometry, and the arc-text angle correction demonstrates why field-level tests are also required.

The matrix has a per-version DWG summary and 3,408 detailed cells. Engine startup failures are separate from invalid drawing failures. Resume checks now compare source hashes and retry timeouts/startup failures. Isolated runners use a private generator executable so a concurrent rebuild cannot replace their running program.

Verification on 2026-09-10: the full Rust test suite passes (1,277 library tests plus integration suites). The audit regression suite has 19 tests. The optional AutoCAD integration test opens all six controls directly with zero audit errors. `cargo check --all-targets` passes.

## Remaining Work

- Advanced surface construction records still fail DWG direct open or round-trip. The atlas currently uses a generic planar SAT sheet and default construction metadata for subtypes. Its DXF inventory retains native subtype names, but an EXTRUDEDSURFACE saved from that DXF became a generic surface. Native inventory does not prove construction-history preservation on save.
- An AutoCAD-authored EXTRUDEDSURFACE with an embedded profile fails after a full-document DWG round-trip, but copying the decoded surface into a fresh document opens natively and audits clean. A REGION reference behaves similarly. Investigate imported document metadata separately from entity bodies; these observations do not prove the surface record layout is defective.
- Advanced surface fixtures currently use a generic planar SAT sheet and absent/default construction profiles. Build valid subtype fixtures without removing or downgrading the failed cases. NURBSURFACE passes AC1027/AC1032 in AutoCAD and AC1021 in BricsCAD, but fails older AutoCAD surface versions.
- AC1024 TABLE still times out in AutoCAD's isolated test with a 30-second limit. Extend isolated BricsCAD coverage beyond AC1021. Newer combined DWG failures must not mark unrelated entities as failing or passing without isolated evidence.
- SAT 700-to-400 conversion is demonstrated for analytic fixtures only. Arbitrary newer ASM schemas and mixed imported VX tables require additional coverage.
- Library DXF readback loses the surface fixture layers in the current atlas; external engine acceptance does not certify the library's DXF import fidelity. Track this separately from DWG open failures.
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
