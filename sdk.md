SDK Gap Analysis — Post-P6 Remaining Items
1. Arc Geometry Queries
File: src/entities/arc.rs

EXISTS:

new(), from_center_radius_angles(), from_coords() — constructors (L28-L65)
sweep_angle() (L68), arc_length() (L76), start_point() (L80), end_point() (L88), midpoint() (L96)
bounding_box(), translate(), apply_transform(), apply_mirror() — Entity trait
MISSING (vs Line/Circle):

closest_point(point) -> Vector3 — Line has it (L68), Circle has it (L73). Arc does NOT.
distance_to_point(point) -> f64 — Line has it (L79), Circle has it (L90). Arc does NOT.
contains_point(point) -> bool — Circle has it (L64). Arc does NOT (needs angle-range check).
point_at(t) -> Vector3 — Line has it (L83). Arc does NOT (parameterized point along arc).
point_at_angle(angle) -> Vector3 — Circle has it (L101). Arc does NOT.
2. Spline Methods
File: src/entities/spline.rs

EXISTS:

Struct: degree, flags (SplineFlags), knots, control_points, weights, fit_points, normal
new() (L75), from_control_points(degree, points) (L87), from_fit_points(points) (L95)
generate_clamped_knots(degree, n) (L101) — static helper
control_point_count() (L121), knot_count() (L126), add_control_point() (L131), add_knot() (L136)
Entity trait: bounding_box(), translate(), apply_transform()
MISSING:

No geometry queries at all — no length(), point_at(t), closest_point(), distance_to_point(), evaluate(t) (de Boor / basis function evaluation), tangent_at(t), curvature_at(t)
No apply_mirror() — not implemented (unlike Arc, Circle, Line)
No derivative/tessellation — to_polyline(tolerance) or tessellate(segments) for approximation
3. Text/MText Search
MISSING entirely. No text search capability exists anywhere.

Text struct has value: String field (src/entities/text.rs)
MText struct has value: String field (src/entities/mtext.rs)
No doc.entities_containing_text("X"), no doc.find_text("X"), no doc.text_entities() convenience
Users must use entities_where() + pattern match + manual string check. A dedicated entities_with_text(pattern) or find_text(query) would be a high-value addition.
4. Entity Statistics/Summary
EXISTS (partial):

doc.entity_count() -> usize (src/document.rs) — total count only
MISSING:

entity_type_counts() -> HashMap<&str, usize> — no per-type summary/histogram
Users must manually iterate entities() and match to count by type. A one-liner summary method is missing.
5. Polyline (Heavy) Geometry
File: src/entities/polyline.rs

EXISTS for Polyline (3D):

new(), from_points(points), add_vertex(), add_point(), vertex_count(), is_closed(), close()
Entity trait: bounding_box(), translate(), apply_transform()
EXISTS for Polyline2D:

new(), add_vertex(), is_closed(), close()
MISSING (vs LwPolyline which has all three):

length() -> f64 — LwPolyline has it (L153). Neither Polyline nor Polyline2D has it.
area() -> f64 — LwPolyline has it (L174). Neither Polyline nor Polyline2D has it.
centroid() -> Vector3 — LwPolyline has it (L195). Neither Polyline nor Polyline2D has it.
No closest_point(), distance_to_point(), contains_point()
6. 3D Geometry Gaps
Solid3D (src/entities/solid3d.rs):

Has from_sat(), from_sab(), parse_sat(), wire/silhouette management — all ACIS data access
No computed geometry: no volume(), surface_area(), centroid(), contains_point(). Geometry is opaque ACIS data.
Mesh (src/entities/mesh.rs):

Rich mesh manipulation: create_box(), from_triangles(), from_quads(), compute_edges(), scale(), translate(), flip_normals(), bounding_box(), center()
No area(), volume(), centroid() (has center() but that's bbox center, not mesh centroid)
Face3D (src/entities/face3d.rs):

Has area() (L161) — only 3D entity with a geometry query
No normal(), centroid(), contains_point(), is_planar()
7. Distance Between Entities
MISSING entirely. No entity-to-entity distance function exists anywhere. No distance_between(entity_a, entity_b) or min_distance() utility. Individual entities have distance_to_point() (Line, Circle, Ray) but there's no composition.

8. Transform Convenience on CadDocument
MISSING. No doc-level transform helpers exist.

No doc.rotate_entity(handle, center, angle)
No doc.scale_entity(handle, base, factor)
No doc.move_entity(handle, offset)
Only the Entity trait's apply_transform(&mut self, transform: &Transform) and translate(&mut self, offset) exist. Users must get a mutable entity reference, build a Transform, and call apply_transform() manually.
9. Style Configuration Builders
MISSING entirely. No builder pattern for any table style.

BlockBuilder exists (src/api/block.rs) — the only builder in the project
No DimStyleBuilder — DimStyle (src/tables/dimstyle.rs) has 60+ fields, all set directly. A builder would be very high-value.
No TextStyleBuilder — TextStyle (src/tables/textstyle.rs) has new(), standard(), with_truetype() but no builder pattern.
No LayerBuilder, LinetypeBuilder, MLineStyleBuilder
10. Import/Export Convenience
EXISTS (reader/writer pattern):

DxfReader::from_file("input.dxf")?.read()? -> CadDocument
DxfWriter::new(&doc).write_to_file("output.dxf")?
DwgReader::from_file("input.dwg")?.read()? -> CadDocument
DwgWriter::write_to_file("output.dwg", &doc)?
MISSING:

No CadDocument::from_file(path) — auto-detecting format from extension (.dwg/.dxf)
No doc.save(path) — auto-selecting writer from extension
No CadDocument::from_bytes(bytes) / doc.to_bytes() for in-memory round-trips
Users must choose the specific reader/writer and chain calls manually.
11. Iterator/Conversion Utilities on EntityType
EXISTS:

EntityType::as_entity() -> &dyn Entity (src/entities/mod.rs)
EntityType::as_entity_mut() -> &mut dyn Entity (L484)
EntityType::common() -> &EntityCommon (L531)
EntityType::common_mut() -> &mut EntityCommon (L579)
EntityType::bounding_box() (L627)
EntityVariant trait with from_entity_type() / from_entity_type_mut() (L645) — generic downcasting
MISSING:

No is_line(), is_circle(), is_arc() etc. — convenience boolean type-check methods
No as_line() -> Option<&Line>, as_circle() -> Option<&Circle> etc. — users must either match or use <Line as EntityVariant>::from_entity_type(&et). A direct et.as_line() would be more ergonomic.
The EntityVariant trait works but the call syntax is inverted (Circle::from_entity_type(&et) rather than et.as_circle()).
12. Notification/Event System
File: src/notification.rs

EXISTS:

NotificationType enum: NotImplemented, NotSupported, Warning, Error (L14)
Notification struct with notification_type + message (L43)
NotificationCollection: notify(), is_empty(), len(), iter(), of_type(), has_type(), extend(), into_vec() (L67-L108)
This is a post-hoc diagnostic system — collect warnings during read/write, inspect after.
MISSING:

No document change notification / event system — no on_entity_added, on_entity_removed, on_entity_modified callbacks
No observer/listener pattern — can't subscribe to document mutations
No hook for undo/redo events — the command stack (P3) has no event dispatch
The existing system is purely for IO diagnostics, not a live event bus.
Summary Priority Matrix
Gap	Complexity	User Value
Arc geometry (closest_point, distance_to, contains)	Low	High
Text search (entities_with_text)	Low	High
Entity type counts	Low	Medium
CadDocument::from_file / save convenience	Low	High
EntityType::is_*() / as_*() helpers	Low	Medium
Polyline length/area/centroid	Medium	Medium
Doc-level transform helpers	Medium	Medium
Spline evaluate/point_at	High	High
DimStyleBuilder	Medium	Medium
Mesh area/volume	Medium	Low
Entity-to-entity distance	High	Medium
Document change events	High	Low
Read content.txt