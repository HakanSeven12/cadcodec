//! Polyline entities (2D and 3D polylines)

use super::{Entity, EntityCommon};
use crate::types::{BoundingBox3D, Color, Handle, LineWeight, Transparency, Vector2, Vector3};

/// Polyline flags (matches DXF group code 70)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PolylineFlags {
    bits: u16,
}

impl PolylineFlags {
    pub const CLOSED: Self = Self { bits: 1 };
    pub const CURVE_FIT: Self = Self { bits: 2 };
    pub const SPLINE_FIT: Self = Self { bits: 4 };
    pub const POLYLINE_3D: Self = Self { bits: 8 };
    pub const POLYGON_MESH: Self = Self { bits: 16 };
    pub const CLOSED_N: Self = Self { bits: 32 };
    pub const POLYFACE_MESH: Self = Self { bits: 64 };
    pub const LINETYPE_CONTINUOUS: Self = Self { bits: 128 };

    pub fn new() -> Self {
        Self { bits: 0 }
    }
    
    pub fn from_bits(bits: u16) -> Self {
        Self { bits }
    }
    
    pub fn bits(&self) -> u16 {
        self.bits
    }

    pub fn is_closed(&self) -> bool {
        self.bits & 1 != 0
    }

    pub fn is_3d(&self) -> bool {
        self.bits & 8 != 0
    }
    
    pub fn is_spline_fit(&self) -> bool {
        self.bits & 4 != 0
    }
    
    pub fn set_closed(&mut self, value: bool) {
        if value {
            self.bits |= 1;
        } else {
            self.bits &= !1;
        }
    }
    
    pub fn set_3d(&mut self, value: bool) {
        if value {
            self.bits |= 8;
        } else {
            self.bits &= !8;
        }
    }
}

impl std::ops::BitOr for PolylineFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self { bits: self.bits | rhs.bits }
    }
}

impl std::ops::BitOrAssign for PolylineFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.bits |= rhs.bits;
    }
}

/// Vertex flags (matches DXF group code 70)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VertexFlags {
    bits: u8,
}

impl VertexFlags {
    pub const EXTRA_VERTEX: Self = Self { bits: 1 };
    pub const CURVE_FIT_TANGENT: Self = Self { bits: 2 };
    pub const SPLINE_VERTEX: Self = Self { bits: 8 };
    pub const SPLINE_CONTROL: Self = Self { bits: 16 };
    pub const POLYLINE_3D: Self = Self { bits: 32 };
    pub const POLYGON_MESH: Self = Self { bits: 64 };
    pub const POLYFACE_FACE: Self = Self { bits: 128 };

    pub fn new() -> Self {
        Self { bits: 0 }
    }
    
    pub fn from_bits(bits: u8) -> Self {
        Self { bits }
    }
    
    pub fn bits(&self) -> u8 {
        self.bits
    }
}

/// Smooth surface type (matches DXF group code 75)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SmoothSurfaceType {
    #[default]
    None = 0,
    QuadraticBSpline = 5,
    CubicBSpline = 6,
    Bezier = 8,
}

impl From<i16> for SmoothSurfaceType {
    fn from(value: i16) -> Self {
        match value {
            5 => SmoothSurfaceType::QuadraticBSpline,
            6 => SmoothSurfaceType::CubicBSpline,
            8 => SmoothSurfaceType::Bezier,
            _ => SmoothSurfaceType::None,
        }
    }
}

/// A vertex in a 2D polyline
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex2D {
    /// Location of the vertex (X, Y in OCS, Z is elevation)
    pub location: Vector3,
    /// Vertex flags
    pub flags: VertexFlags,
    /// Start width (0 = use default)
    pub start_width: f64,
    /// End width (0 = use default)
    pub end_width: f64,
    /// Bulge (0 = straight segment, <0 = clockwise arc, >0 = counter-clockwise arc)
    pub bulge: f64,
    /// Curve fit tangent direction
    pub curve_tangent: f64,
    /// Vertex ID (R2010+)
    pub id: i32,
}

impl Vertex2D {
    pub fn new(location: Vector3) -> Self {
        Self {
            location,
            flags: VertexFlags::new(),
            start_width: 0.0,
            end_width: 0.0,
            bulge: 0.0,
            curve_tangent: 0.0,
            id: 0,
        }
    }
    
    pub fn from_point(point: Vector2) -> Self {
        Self::new(Vector3::new(point.x, point.y, 0.0))
    }
    
    pub fn with_bulge(mut self, bulge: f64) -> Self {
        self.bulge = bulge;
        self
    }
    
    pub fn with_width(mut self, start_width: f64, end_width: f64) -> Self {
        self.start_width = start_width;
        self.end_width = end_width;
        self
    }
}

/// A vertex in a 3D polyline
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex3D {
    /// Location of the vertex
    pub location: Vector3,
    /// Vertex flags
    pub flags: VertexFlags,
}

impl Vertex3D {
    /// Create a new vertex
    pub fn new(location: Vector3) -> Self {
        Self {
            location,
            flags: VertexFlags::new(),
        }
    }

    /// Create a vertex from coordinates
    pub fn from_coords(x: f64, y: f64, z: f64) -> Self {
        Vertex3D::new(Vector3::new(x, y, z))
    }
}

/// A 2D polyline entity (heavy polyline with vertices)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Polyline2D {
    /// Common entity data
    pub common: EntityCommon,
    /// Polyline flags
    pub flags: PolylineFlags,
    /// Smooth surface type
    pub smooth_surface: SmoothSurfaceType,
    /// Default start width
    pub start_width: f64,
    /// Default end width
    pub end_width: f64,
    /// Thickness (extrusion height)
    pub thickness: f64,
    /// Elevation (Z coordinate in OCS)
    pub elevation: f64,
    /// Normal vector (extrusion direction)
    pub normal: Vector3,
    /// Vertices
    pub vertices: Vec<Vertex2D>,
}

impl Polyline2D {
    pub fn new() -> Self {
        Self {
            common: EntityCommon::new(),
            flags: PolylineFlags::new(),
            smooth_surface: SmoothSurfaceType::None,
            start_width: 0.0,
            end_width: 0.0,
            thickness: 0.0,
            elevation: 0.0,
            normal: Vector3::new(0.0, 0.0, 1.0),
            vertices: Vec::new(),
        }
    }
    
    pub fn add_vertex(&mut self, vertex: Vertex2D) {
        self.vertices.push(vertex);
    }
    
    pub fn is_closed(&self) -> bool {
        self.flags.is_closed()
    }
    
    pub fn close(&mut self) {
        self.flags.set_closed(true);
    }

    /// Compute the total length of the 2D polyline.
    ///
    /// Handles bulge (arc segments) between vertices. If the polyline is
    /// closed, includes the closing segment.
    pub fn length(&self) -> f64 {
        if self.vertices.len() < 2 {
            return 0.0;
        }
        let n = self.vertices.len();
        let count = if self.is_closed() { n } else { n - 1 };
        let mut len = 0.0;
        for i in 0..count {
            let j = (i + 1) % n;
            let p0 = &self.vertices[i].location;
            let p1 = &self.vertices[j].location;
            let bulge = self.vertices[i].bulge;
            let chord = ((p1.x - p0.x).powi(2) + (p1.y - p0.y).powi(2)).sqrt();
            if bulge.abs() < 1e-10 {
                len += chord;
            } else {
                // Arc segment: arc_length = radius * |sweep|
                let s = bulge.abs();
                let radius = chord * (s * s + 1.0) / (4.0 * s);
                let sweep = 4.0 * s.atan();
                len += radius * sweep;
            }
        }
        len
    }

    /// Compute the area of the 2D polyline (closed only).
    ///
    /// Accounts for bulge (arc) segments using the circular-segment
    /// correction.  Returns `0.0` for open polylines or fewer than 3
    /// vertices.
    pub fn area(&self) -> f64 {
        if self.vertices.len() < 3 || !self.is_closed() {
            return 0.0;
        }
        let n = self.vertices.len();
        let mut sum = 0.0;
        for i in 0..n {
            let j = (i + 1) % n;
            let pi = &self.vertices[i].location;
            let pj = &self.vertices[j].location;
            // Shoelace contribution
            sum += pi.x * pj.y - pj.x * pi.y;
            // Bulge correction: add signed circular segment area
            let bulge = self.vertices[i].bulge;
            if bulge.abs() > 1e-10 {
                let chord = ((pj.x - pi.x).powi(2) + (pj.y - pi.y).powi(2)).sqrt();
                let s = bulge.abs();
                let radius = chord * (s * s + 1.0) / (4.0 * s);
                let sweep = 4.0 * s.atan();
                let segment_area = radius * radius * (sweep - sweep.sin()) / 2.0;
                if bulge > 0.0 {
                    sum += 2.0 * segment_area;
                } else {
                    sum -= 2.0 * segment_area;
                }
            }
        }
        sum.abs() / 2.0
    }

    /// Compute the centroid of the polyline vertices.
    pub fn centroid(&self) -> Vector3 {
        if self.vertices.is_empty() {
            return Vector3::ZERO;
        }
        let n = self.vertices.len() as f64;
        let sum = self
            .vertices
            .iter()
            .fold(Vector3::ZERO, |acc, v| acc + v.location);
        Vector3::new(sum.x / n, sum.y / n, sum.z / n)
    }
}

impl Default for Polyline2D {
    fn default() -> Self {
        Self::new()
    }
}

/// A 3D polyline entity
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Polyline {
    /// Common entity data
    pub common: EntityCommon,
    /// Polyline flags
    pub flags: PolylineFlags,
    /// Vertices of the polyline
    pub vertices: Vec<Vertex3D>,
}

impl Polyline {
    /// Create a new empty polyline
    pub fn new() -> Self {
        let mut flags = PolylineFlags::new();
        flags.set_3d(true);
        Polyline {
            common: EntityCommon::new(),
            flags,
            vertices: Vec::new(),
        }
    }

    /// Create a polyline from a list of points
    pub fn from_points(points: Vec<Vector3>) -> Self {
        Polyline {
            vertices: points.into_iter().map(Vertex3D::new).collect(),
            ..Self::new()
        }
    }

    /// Add a vertex to the polyline
    pub fn add_vertex(&mut self, vertex: Vertex3D) {
        self.vertices.push(vertex);
    }

    /// Add a point to the polyline
    pub fn add_point(&mut self, point: Vector3) {
        self.vertices.push(Vertex3D::new(point));
    }

    /// Get the number of vertices
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Check if closed
    pub fn is_closed(&self) -> bool {
        self.flags.is_closed()
    }

    /// Close the polyline
    pub fn close(&mut self) {
        self.flags.set_closed(true);
    }

    /// Compute the total length of the polyline.
    ///
    /// Sums the Euclidean distances between consecutive vertices.
    /// If the polyline is closed, includes the closing segment.
    pub fn length(&self) -> f64 {
        if self.vertices.len() < 2 {
            return 0.0;
        }
        let mut len = 0.0;
        for i in 0..self.vertices.len() - 1 {
            len += (self.vertices[i + 1].location - self.vertices[i].location).length();
        }
        if self.is_closed() && self.vertices.len() > 2 {
            len += (self.vertices[0].location - self.vertices.last().unwrap().location).length();
        }
        len
    }

    /// Compute the area of the polyline (using the shoelace formula).
    ///
    /// Only meaningful for closed, planar polylines. Returns the absolute
    /// area (always non-negative). For open polylines or fewer than 3
    /// vertices, returns `0.0`.
    pub fn area(&self) -> f64 {
        if self.vertices.len() < 3 || !self.is_closed() {
            return 0.0;
        }
        let n = self.vertices.len();
        let mut sum = 0.0;
        for i in 0..n {
            let j = (i + 1) % n;
            let pi = &self.vertices[i].location;
            let pj = &self.vertices[j].location;
            sum += pi.x * pj.y - pj.x * pi.y;
        }
        sum.abs() / 2.0
    }

    /// Compute the centroid of the polyline vertices.
    ///
    /// Returns the arithmetic mean of all vertex positions. Returns
    /// [`Vector3::ZERO`] if the polyline has no vertices.
    pub fn centroid(&self) -> Vector3 {
        if self.vertices.is_empty() {
            return Vector3::ZERO;
        }
        let n = self.vertices.len() as f64;
        let sum = self
            .vertices
            .iter()
            .fold(Vector3::ZERO, |acc, v| acc + v.location);
        Vector3::new(sum.x / n, sum.y / n, sum.z / n)
    }
}

impl Default for Polyline {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for Polyline2D {
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

        let points: Vec<Vector3> = self.vertices.iter().map(|v| v.location).collect();
        BoundingBox3D::from_points(&points).unwrap()
    }

    fn translate(&mut self, offset: Vector3) {
        super::translate::translate_polyline2d(self, offset);
    }

    fn entity_type(&self) -> &'static str {
        "POLYLINE"
    }
    
    fn apply_transform(&mut self, transform: &crate::types::Transform) {
        super::transform::transform_polyline2d(self, transform);
    }
    
    fn apply_mirror(&mut self, transform: &crate::types::Transform) {
        super::mirror::mirror_polyline2d(self, transform);
    }
}

impl Entity for Polyline {
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

        let points: Vec<Vector3> = self.vertices.iter().map(|v| v.location).collect();
        BoundingBox3D::from_points(&points).unwrap()
    }

    fn translate(&mut self, offset: Vector3) {
        super::translate::translate_polyline(self, offset);
    }

    fn entity_type(&self) -> &'static str {
        "POLYLINE"
    }
    
    fn apply_transform(&mut self, transform: &crate::types::Transform) {
        super::transform::transform_polyline(self, transform);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Polyline (3D) tests ──────────────────────────────────────────

    #[test]
    fn test_polyline_length_open() {
        let pl = Polyline::from_points(vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(3.0, 0.0, 0.0),
            Vector3::new(3.0, 4.0, 0.0),
        ]);
        // 3 + 4 = 7
        assert!((pl.length() - 7.0).abs() < 1e-10);
    }

    #[test]
    fn test_polyline_length_closed() {
        let mut pl = Polyline::from_points(vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(3.0, 0.0, 0.0),
            Vector3::new(3.0, 4.0, 0.0),
        ]);
        pl.close();
        // 3 + 4 + 5 = 12
        assert!((pl.length() - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_polyline_length_single_vertex() {
        let pl = Polyline::from_points(vec![Vector3::ZERO]);
        assert_eq!(pl.length(), 0.0);
    }

    #[test]
    fn test_polyline_area_closed_rectangle() {
        let mut pl = Polyline::from_points(vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(4.0, 0.0, 0.0),
            Vector3::new(4.0, 3.0, 0.0),
            Vector3::new(0.0, 3.0, 0.0),
        ]);
        pl.close();
        assert!((pl.area() - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_polyline_area_open_returns_zero() {
        let pl = Polyline::from_points(vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(4.0, 0.0, 0.0),
            Vector3::new(4.0, 3.0, 0.0),
        ]);
        assert_eq!(pl.area(), 0.0);
    }

    #[test]
    fn test_polyline_centroid() {
        let pl = Polyline::from_points(vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(4.0, 0.0, 0.0),
            Vector3::new(4.0, 4.0, 0.0),
            Vector3::new(0.0, 4.0, 0.0),
        ]);
        let c = pl.centroid();
        assert!((c.x - 2.0).abs() < 1e-10);
        assert!((c.y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_polyline_centroid_empty() {
        let pl = Polyline::new();
        assert_eq!(pl.centroid(), Vector3::ZERO);
    }

    // ── Polyline2D tests ─────────────────────────────────────────────

    fn make_rect_2d() -> Polyline2D {
        let mut pl = Polyline2D::new();
        pl.add_vertex(Vertex2D::new(Vector3::new(0.0, 0.0, 0.0)));
        pl.add_vertex(Vertex2D::new(Vector3::new(4.0, 0.0, 0.0)));
        pl.add_vertex(Vertex2D::new(Vector3::new(4.0, 3.0, 0.0)));
        pl.add_vertex(Vertex2D::new(Vector3::new(0.0, 3.0, 0.0)));
        pl.close();
        pl
    }

    #[test]
    fn test_polyline2d_length_straight() {
        let pl = make_rect_2d();
        // 4 + 3 + 4 + 3 = 14
        assert!((pl.length() - 14.0).abs() < 1e-10);
    }

    #[test]
    fn test_polyline2d_area_rectangle() {
        let pl = make_rect_2d();
        assert!((pl.area() - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_polyline2d_length_with_bulge() {
        // Two-vertex closed polyline with bulge = 1 (semicircle)
        let mut pl = Polyline2D::new();
        let mut v0 = Vertex2D::new(Vector3::new(0.0, 0.0, 0.0));
        v0.bulge = 1.0; // semicircular arc
        pl.add_vertex(v0);
        pl.add_vertex(Vertex2D::new(Vector3::new(2.0, 0.0, 0.0)));
        // Open: one segment with bulge=1, chord=2, radius=1, sweep=π
        let len = pl.length();
        let expected = std::f64::consts::PI; // semicircle of radius 1
        assert!((len - expected).abs() < 0.01);
    }

    #[test]
    fn test_polyline2d_centroid() {
        let pl = make_rect_2d();
        let c = pl.centroid();
        assert!((c.x - 2.0).abs() < 1e-10);
        assert!((c.y - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_polyline2d_length_empty() {
        let pl = Polyline2D::new();
        assert_eq!(pl.length(), 0.0);
    }

    #[test]
    fn test_polyline2d_area_open() {
        let mut pl = Polyline2D::new();
        pl.add_vertex(Vertex2D::new(Vector3::new(0.0, 0.0, 0.0)));
        pl.add_vertex(Vertex2D::new(Vector3::new(1.0, 0.0, 0.0)));
        pl.add_vertex(Vertex2D::new(Vector3::new(1.0, 1.0, 0.0)));
        assert_eq!(pl.area(), 0.0); // not closed
    }
}

