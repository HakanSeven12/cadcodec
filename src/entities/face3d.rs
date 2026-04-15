//! Face3D entity (3D face)

use crate::entities::{Entity, EntityCommon};
use crate::types::{BoundingBox3D, Color, Handle, LineWeight, Transparency, Vector3};

/// Invisible edge flags for Face3D
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InvisibleEdgeFlags {
    bits: u8,
}

impl InvisibleEdgeFlags {
    pub const NONE: Self = Self { bits: 0 };
    pub const FIRST: Self = Self { bits: 1 };
    pub const SECOND: Self = Self { bits: 2 };
    pub const THIRD: Self = Self { bits: 4 };
    pub const FOURTH: Self = Self { bits: 8 };

    pub fn new() -> Self {
        Self::NONE
    }

    /// Create from raw bits value
    pub fn from_bits(bits: u8) -> Self {
        Self { bits }
    }
    
    /// Get the raw bits value
    pub fn bits(&self) -> u8 {
        self.bits
    }

    pub fn is_first_invisible(&self) -> bool {
        self.bits & 1 != 0
    }

    pub fn is_second_invisible(&self) -> bool {
        self.bits & 2 != 0
    }

    pub fn is_third_invisible(&self) -> bool {
        self.bits & 4 != 0
    }

    pub fn is_fourth_invisible(&self) -> bool {
        self.bits & 8 != 0
    }

    pub fn set_first_invisible(&mut self, value: bool) {
        if value {
            self.bits |= 1;
        } else {
            self.bits &= !1;
        }
    }

    pub fn set_second_invisible(&mut self, value: bool) {
        if value {
            self.bits |= 2;
        } else {
            self.bits &= !2;
        }
    }

    pub fn set_third_invisible(&mut self, value: bool) {
        if value {
            self.bits |= 4;
        } else {
            self.bits &= !4;
        }
    }

    pub fn set_fourth_invisible(&mut self, value: bool) {
        if value {
            self.bits |= 8;
        } else {
            self.bits &= !8;
        }
    }
}

impl Default for InvisibleEdgeFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Face3D entity - a 3D face with 3 or 4 vertices
///
/// A Face3D entity is a 3D surface defined by 3 or 4 corner points.
/// Individual edges can be marked as invisible.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Face3D {
    pub common: EntityCommon,
    /// First corner point (in WCS)
    pub first_corner: Vector3,
    /// Second corner point (in WCS)
    pub second_corner: Vector3,
    /// Third corner point (in WCS)
    pub third_corner: Vector3,
    /// Fourth corner point (in WCS) - same as third if only 3 corners
    pub fourth_corner: Vector3,
    /// Invisible edge flags
    pub invisible_edges: InvisibleEdgeFlags,
}

impl Face3D {
    /// Create a new 3D face with four corners
    pub fn new(
        first: Vector3,
        second: Vector3,
        third: Vector3,
        fourth: Vector3,
    ) -> Self {
        Self {
            common: EntityCommon::default(),
            first_corner: first,
            second_corner: second,
            third_corner: third,
            fourth_corner: fourth,
            invisible_edges: InvisibleEdgeFlags::new(),
        }
    }

    /// Create a triangular 3D face (3 corners)
    pub fn triangle(first: Vector3, second: Vector3, third: Vector3) -> Self {
        Self::new(first, second, third, third)
    }

    /// Builder: Set invisible edge flags
    pub fn with_invisible_edges(mut self, flags: InvisibleEdgeFlags) -> Self {
        self.invisible_edges = flags;
        self
    }

    /// Check if this is a triangle (3 vertices)
    pub fn is_triangle(&self) -> bool {
        (self.third_corner - self.fourth_corner).length() < 1e-10
    }

    /// Get all corner points
    pub fn corners(&self) -> Vec<Vector3> {
        if self.is_triangle() {
            vec![self.first_corner, self.second_corner, self.third_corner]
        } else {
            vec![
                self.first_corner,
                self.second_corner,
                self.third_corner,
                self.fourth_corner,
            ]
        }
    }

    /// Calculate the area of the face
    pub fn area(&self) -> f64 {
        if self.is_triangle() {
            // Triangle area using cross product
            let v1 = self.second_corner - self.first_corner;
            let v2 = self.third_corner - self.first_corner;
            v1.cross(&v2).length() * 0.5
        } else {
            // Quadrilateral area (sum of two triangles)
            let v1 = self.second_corner - self.first_corner;
            let v2 = self.third_corner - self.first_corner;
            let area1 = v1.cross(&v2).length() * 0.5;

            let v3 = self.third_corner - self.first_corner;
            let v4 = self.fourth_corner - self.first_corner;
            let area2 = v3.cross(&v4).length() * 0.5;

            area1 + area2
        }
    }

    /// Compute the face normal (unit vector perpendicular to the face).
    ///
    /// Uses the cross product of two edges. Returns `None` if the face
    /// is degenerate (zero-area).
    pub fn normal(&self) -> Option<Vector3> {
        let v1 = self.second_corner - self.first_corner;
        let v2 = self.third_corner - self.first_corner;
        let n = v1.cross(&v2);
        let len = n.length();
        if len < 1e-14 {
            return None;
        }
        Some(Vector3::new(n.x / len, n.y / len, n.z / len))
    }

    /// Compute the centroid (geometric center) of the face.
    pub fn centroid(&self) -> Vector3 {
        if self.is_triangle() {
            Vector3::new(
                (self.first_corner.x + self.second_corner.x + self.third_corner.x) / 3.0,
                (self.first_corner.y + self.second_corner.y + self.third_corner.y) / 3.0,
                (self.first_corner.z + self.second_corner.z + self.third_corner.z) / 3.0,
            )
        } else {
            Vector3::new(
                (self.first_corner.x + self.second_corner.x + self.third_corner.x + self.fourth_corner.x) / 4.0,
                (self.first_corner.y + self.second_corner.y + self.third_corner.y + self.fourth_corner.y) / 4.0,
                (self.first_corner.z + self.second_corner.z + self.third_corner.z + self.fourth_corner.z) / 4.0,
            )
        }
    }

    /// Check whether a point lies on the plane of this face.
    ///
    /// Returns `true` if the point is within `tolerance` of the face plane.
    pub fn is_point_on_plane(&self, point: Vector3, tolerance: f64) -> bool {
        if let Some(n) = self.normal() {
            let d = point - self.first_corner;
            (d.x * n.x + d.y * n.y + d.z * n.z).abs() < tolerance
        } else {
            false
        }
    }
}

impl Entity for Face3D {
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
        BoundingBox3D::from_points(&self.corners()).unwrap_or_else(|| BoundingBox3D::new(Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 0.0)))
    }

    fn translate(&mut self, offset: Vector3) {
        super::translate::translate_face3d(self, offset);
    }

    fn entity_type(&self) -> &'static str {
        "3DFACE"
    }
    
    fn apply_transform(&mut self, transform: &crate::types::Transform) {
        super::transform::transform_face3d(self, transform);
    }
    
    fn apply_mirror(&mut self, transform: &crate::types::Transform) {
        super::mirror::mirror_face3d(self, transform);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> Face3D {
        Face3D::triangle(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(4.0, 0.0, 0.0),
            Vector3::new(0.0, 3.0, 0.0),
        )
    }

    fn quad() -> Face3D {
        Face3D::new(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(2.0, 0.0, 0.0),
            Vector3::new(2.0, 3.0, 0.0),
            Vector3::new(0.0, 3.0, 0.0),
        )
    }

    #[test]
    fn test_face3d_triangle_area() {
        let f = triangle();
        assert!((f.area() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_face3d_quad_area() {
        let f = quad();
        assert!((f.area() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_face3d_normal_triangle() {
        let f = triangle();
        let n = f.normal().unwrap();
        assert!(n.x.abs() < 1e-10);
        assert!(n.y.abs() < 1e-10);
        assert!((n.z - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_face3d_normal_quad() {
        let f = quad();
        let n = f.normal().unwrap();
        assert!((n.z - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_face3d_centroid_triangle() {
        let f = triangle();
        let c = f.centroid();
        assert!((c.x - 4.0 / 3.0).abs() < 1e-10);
        assert!((c.y - 1.0).abs() < 1e-10);
        assert!(c.z.abs() < 1e-10);
    }

    #[test]
    fn test_face3d_centroid_quad() {
        let f = quad();
        let c = f.centroid();
        assert!((c.x - 1.0).abs() < 1e-10);
        assert!((c.y - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_face3d_is_point_on_plane() {
        let f = triangle();
        assert!(f.is_point_on_plane(Vector3::new(1.0, 1.0, 0.0), 1e-6));
        assert!(!f.is_point_on_plane(Vector3::new(1.0, 1.0, 1.0), 1e-6));
    }

    #[test]
    fn test_face3d_is_triangle() {
        assert!(triangle().is_triangle());
        assert!(!quad().is_triangle());
    }
}
