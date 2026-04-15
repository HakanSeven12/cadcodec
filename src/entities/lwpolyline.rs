//! Lightweight polyline entity (2D polyline with bulges)

use super::{Entity, EntityCommon};
use crate::types::{BoundingBox3D, Color, Handle, LineWeight, Transparency, Vector2, Vector3};

/// A vertex in a lightweight polyline
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LwVertex {
    /// Location of the vertex (2D)
    pub location: Vector2,
    /// Bulge value (for arc segments)
    /// 0 = straight line, positive = counterclockwise arc, negative = clockwise arc
    /// bulge = tan(angle/4) where angle is the included angle
    pub bulge: f64,
    /// Starting width at this vertex
    pub start_width: f64,
    /// Ending width at this vertex
    pub end_width: f64,
}

impl LwVertex {
    /// Create a new vertex
    pub fn new(location: Vector2) -> Self {
        LwVertex {
            location,
            bulge: 0.0,
            start_width: 0.0,
            end_width: 0.0,
        }
    }

    /// Create a vertex from coordinates
    pub fn from_coords(x: f64, y: f64) -> Self {
        LwVertex::new(Vector2::new(x, y))
    }

    /// Create a vertex with a bulge
    pub fn with_bulge(location: Vector2, bulge: f64) -> Self {
        LwVertex {
            location,
            bulge,
            start_width: 0.0,
            end_width: 0.0,
        }
    }
}

/// A lightweight (2D) polyline entity
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LwPolyline {
    /// Common entity data
    pub common: EntityCommon,
    /// Vertices of the polyline
    pub vertices: Vec<LwVertex>,
    /// Is the polyline closed?
    pub is_closed: bool,
    /// Use PLINEGEN linetype generation pattern across vertices
    pub plinegen: bool,
    /// Constant width (if all segments have same width)
    pub constant_width: f64,
    /// Elevation (Z coordinate)
    pub elevation: f64,
    /// Thickness (extrusion in Z direction)
    pub thickness: f64,
    /// Normal vector
    pub normal: Vector3,
}

impl LwPolyline {
    /// Create a new empty lightweight polyline
    pub fn new() -> Self {
        LwPolyline {
            common: EntityCommon::new(),
            vertices: Vec::new(),
            is_closed: false,
            plinegen: false,
            constant_width: 0.0,
            elevation: 0.0,
            thickness: 0.0,
            normal: Vector3::UNIT_Z,
        }
    }

    /// Create a polyline from a list of 2D points
    pub fn from_points(points: Vec<Vector2>) -> Self {
        LwPolyline {
            vertices: points.into_iter().map(LwVertex::new).collect(),
            ..Self::new()
        }
    }

    /// Create a closed rectangular polyline from two opposite corner points.
    pub fn from_rectangle(p1: Vector2, p2: Vector2) -> Self {
        let mut pl = LwPolyline::from_points(vec![
            p1,
            Vector2::new(p2.x, p1.y),
            p2,
            Vector2::new(p1.x, p2.y),
        ]);
        pl.is_closed = true;
        pl
    }

    /// Create a closed regular polygon polyline.
    ///
    /// `center` is the center of the polygon, `radius` is the circumscribed
    /// radius, and `sides` is the number of sides (must be ≥ 3).
    pub fn from_polygon(center: Vector2, radius: f64, sides: usize) -> Self {
        let sides = sides.max(3);
        let step = 2.0 * std::f64::consts::PI / sides as f64;
        let pts: Vec<Vector2> = (0..sides)
            .map(|i| {
                let a = step * i as f64;
                Vector2::new(center.x + radius * a.cos(), center.y + radius * a.sin())
            })
            .collect();
        let mut pl = LwPolyline::from_points(pts);
        pl.is_closed = true;
        pl
    }

    /// Add a vertex to the polyline
    pub fn add_vertex(&mut self, vertex: LwVertex) {
        self.vertices.push(vertex);
    }

    /// Add a point to the polyline
    pub fn add_point(&mut self, point: Vector2) {
        self.vertices.push(LwVertex::new(point));
    }

    /// Add a point with bulge
    pub fn add_point_with_bulge(&mut self, point: Vector2, bulge: f64) {
        self.vertices.push(LwVertex::with_bulge(point, bulge));
    }

    /// Get the number of vertices
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Close the polyline
    pub fn close(&mut self) {
        self.is_closed = true;
    }

    /// Compute the total length of the polyline.
    ///
    /// Straight segments use Euclidean distance; arc segments (non-zero bulge)
    /// compute the arc length based on the bulge value.
    pub fn length(&self) -> f64 {
        if self.vertices.len() < 2 {
            return 0.0;
        }
        let n = self.vertices.len();
        let seg_count = if self.is_closed { n } else { n - 1 };
        let mut total = 0.0;
        for i in 0..seg_count {
            let v0 = &self.vertices[i];
            let v1 = &self.vertices[(i + 1) % n];
            total += segment_length(v0, v1);
        }
        total
    }

    /// Compute the signed area of the polyline using the shoelace formula.
    ///
    /// Arc segments contribute their circular-segment area.
    /// The result is positive for counter-clockwise winding.
    /// For open polylines the implicit closing edge from last to first vertex
    /// is **not** included — use this primarily on closed polylines.
    pub fn area(&self) -> f64 {
        if self.vertices.len() < 3 {
            return 0.0;
        }
        let n = self.vertices.len();
        let seg_count = if self.is_closed { n } else { n - 1 };
        let mut area = 0.0;
        for i in 0..seg_count {
            let v0 = &self.vertices[i];
            let v1 = &self.vertices[(i + 1) % n];
            // Shoelace term for the chord
            area += v0.location.x * v1.location.y - v1.location.x * v0.location.y;
            // Arc segment correction
            if v0.bulge.abs() > 1e-10 {
                area += arc_segment_area(v0, v1);
            }
        }
        area / 2.0
    }

    /// Compute the centroid of the polyline vertices (simple average).
    pub fn centroid(&self) -> Vector2 {
        if self.vertices.is_empty() {
            return Vector2::new(0.0, 0.0);
        }
        let n = self.vertices.len() as f64;
        let (sx, sy) = self.vertices.iter().fold((0.0, 0.0), |(sx, sy), v| {
            (sx + v.location.x, sy + v.location.y)
        });
        Vector2::new(sx / n, sy / n)
    }
}

/// Compute the length of a polyline segment between two vertices.
fn segment_length(v0: &LwVertex, v1: &LwVertex) -> f64 {
    let dx = v1.location.x - v0.location.x;
    let dy = v1.location.y - v0.location.y;
    let chord = (dx * dx + dy * dy).sqrt();
    if v0.bulge.abs() < 1e-10 {
        return chord;
    }
    // Arc: bulge = tan(included_angle / 4)
    let angle = 4.0 * v0.bulge.abs().atan();
    let radius = chord / (2.0 * (angle / 2.0).sin());
    radius * angle.abs()
}

/// Signed area correction for a circular-segment arc between two vertices.
fn arc_segment_area(v0: &LwVertex, v1: &LwVertex) -> f64 {
    let dx = v1.location.x - v0.location.x;
    let dy = v1.location.y - v0.location.y;
    let chord = (dx * dx + dy * dy).sqrt();
    if chord < 1e-10 {
        return 0.0;
    }
    let angle = 4.0 * v0.bulge.abs().atan();
    let radius = chord / (2.0 * (angle / 2.0).sin());
    // Circular segment area = r^2 * (θ - sin θ) / 2
    let seg_area = radius * radius * (angle - angle.sin()) / 2.0;
    // Sign: positive bulge → CCW → adds area; negative → subtracts
    if v0.bulge > 0.0 { seg_area } else { -seg_area }
}

impl Default for LwPolyline {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for LwPolyline {
    fn handle(&self) -> Handle {
        self.common.handle
    }

    fn set_handle(&mut self, handle: Handle) {
        self.common.handle = handle;
    }

    fn layer(&self) -> &str {
        &self.common.layer
    }

    fn set_layer(&mut self, layer: String) {
        self.common.layer = layer;
    }

    fn color(&self) -> Color {
        self.common.color
    }

    fn set_color(&mut self, color: Color) {
        self.common.color = color;
    }

    fn line_weight(&self) -> LineWeight {
        self.common.line_weight
    }

    fn set_line_weight(&mut self, weight: LineWeight) {
        self.common.line_weight = weight;
    }

    fn transparency(&self) -> Transparency {
        self.common.transparency
    }

    fn set_transparency(&mut self, transparency: Transparency) {
        self.common.transparency = transparency;
    }

    fn is_invisible(&self) -> bool {
        self.common.invisible
    }

    fn set_invisible(&mut self, invisible: bool) {
        self.common.invisible = invisible;
    }

    fn bounding_box(&self) -> BoundingBox3D {
        if self.vertices.is_empty() {
            return BoundingBox3D::from_point(Vector3::ZERO);
        }

        let points: Vec<Vector3> = self
            .vertices
            .iter()
            .map(|v| Vector3::new(v.location.x, v.location.y, self.elevation))
            .collect();
        BoundingBox3D::from_points(&points).unwrap()
    }

    fn translate(&mut self, offset: Vector3) {
        super::translate::translate_lwpolyline(self, offset);
    }

    fn entity_type(&self) -> &'static str {
        "LWPOLYLINE"
    }
    
    fn apply_transform(&mut self, transform: &crate::types::Transform) {
        super::transform::transform_lwpolyline(self, transform);
    }
    
    fn apply_mirror(&mut self, transform: &crate::types::Transform) {
        super::mirror::mirror_lwpolyline(self, transform);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_length_and_area() {
        let r = LwPolyline::from_rectangle(
            Vector2::new(0.0, 0.0),
            Vector2::new(10.0, 5.0),
        );
        assert!(r.is_closed);
        assert_eq!(r.vertices.len(), 4);
        assert!((r.length() - 30.0).abs() < 1e-10); // perimeter 2*(10+5)
        assert!((r.area() - 50.0).abs() < 1e-10); // 10 * 5
    }

    #[test]
    fn polygon_hexagon() {
        let hex = LwPolyline::from_polygon(Vector2::new(0.0, 0.0), 10.0, 6);
        assert!(hex.is_closed);
        assert_eq!(hex.vertices.len(), 6);
        // Regular hexagon side = radius, perimeter = 6 * 10 = 60
        assert!((hex.length() - 60.0).abs() < 1e-8);
    }

    #[test]
    fn open_polyline_length() {
        let pl = LwPolyline::from_points(vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(3.0, 0.0),
            Vector2::new(3.0, 4.0),
        ]);
        assert!((pl.length() - 7.0).abs() < 1e-10); // 3 + 4 (not closed)
    }

    #[test]
    fn centroid_of_square() {
        let sq = LwPolyline::from_rectangle(
            Vector2::new(0.0, 0.0),
            Vector2::new(4.0, 4.0),
        );
        let c = sq.centroid();
        assert!((c.x - 2.0).abs() < 1e-10);
        assert!((c.y - 2.0).abs() < 1e-10);
    }
}

