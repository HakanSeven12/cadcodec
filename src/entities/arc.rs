//! Arc entity

use super::{Entity, EntityCommon};
use crate::types::{BoundingBox3D, Color, Handle, LineWeight, Transparency, Vector3};

/// An arc entity (portion of a circle)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Arc {
    /// Common entity data
    pub common: EntityCommon,
    /// Center point of the arc
    pub center: Vector3,
    /// Radius of the arc
    pub radius: f64,
    /// Start angle in radians
    pub start_angle: f64,
    /// End angle in radians
    pub end_angle: f64,
    /// Thickness (extrusion in Z direction)
    pub thickness: f64,
    /// Normal vector
    pub normal: Vector3,
}

impl Arc {
    /// Create a new arc at the origin
    pub fn new() -> Self {
        Arc {
            common: EntityCommon::new(),
            center: Vector3::ZERO,
            radius: 1.0,
            start_angle: 0.0,
            end_angle: std::f64::consts::PI / 2.0, // 90 degrees
            thickness: 0.0,
            normal: Vector3::UNIT_Z,
        }
    }

    /// Create a new arc with center, radius, and angles
    pub fn from_center_radius_angles(
        center: Vector3,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    ) -> Self {
        Arc {
            center,
            radius,
            start_angle,
            end_angle,
            ..Self::new()
        }
    }

    /// Create a new arc from coordinates, radius, and angles
    pub fn from_coords(
        x: f64,
        y: f64,
        z: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
    ) -> Self {
        Arc::from_center_radius_angles(Vector3::new(x, y, z), radius, start_angle, end_angle)
    }

    /// Get the sweep angle (angular extent) in radians
    pub fn sweep_angle(&self) -> f64 {
        let mut sweep = self.end_angle - self.start_angle;
        if sweep < 0.0 {
            sweep += 2.0 * std::f64::consts::PI;
        }
        sweep
    }

    /// Get the arc length
    pub fn arc_length(&self) -> f64 {
        self.radius * self.sweep_angle()
    }

    /// Get the start point of the arc
    pub fn start_point(&self) -> Vector3 {
        Vector3::new(
            self.center.x + self.radius * self.start_angle.cos(),
            self.center.y + self.radius * self.start_angle.sin(),
            self.center.z,
        )
    }

    /// Get the end point of the arc
    pub fn end_point(&self) -> Vector3 {
        Vector3::new(
            self.center.x + self.radius * self.end_angle.cos(),
            self.center.y + self.radius * self.end_angle.sin(),
            self.center.z,
        )
    }

    /// Get the midpoint of the arc
    pub fn midpoint(&self) -> Vector3 {
        let mid_angle = self.start_angle + self.sweep_angle() / 2.0;
        Vector3::new(
            self.center.x + self.radius * mid_angle.cos(),
            self.center.y + self.radius * mid_angle.sin(),
            self.center.z,
        )
    }

    /// Check whether an angle (in radians) lies within the arc's angular range.
    fn angle_in_range(&self, angle: f64) -> bool {
        let two_pi = 2.0 * std::f64::consts::PI;
        let mut a = (angle - self.start_angle) % two_pi;
        if a < 0.0 {
            a += two_pi;
        }
        a <= self.sweep_angle() + 1e-10
    }

    /// Check whether a point lies on the arc (within tolerance).
    ///
    /// A point is on the arc when it is at the correct radius from the center
    /// and its angle falls within the arc's angular range.
    pub fn contains_point(&self, point: Vector3) -> bool {
        let dx = point.x - self.center.x;
        let dy = point.y - self.center.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if (dist - self.radius).abs() > 1e-6 {
            return false;
        }
        let angle = dy.atan2(dx);
        self.angle_in_range(angle)
    }

    /// Return the closest point on the arc to the given point (2D, ignoring Z).
    ///
    /// Projects the point onto the full circle, then clamps to the arc's
    /// angular range if necessary.
    pub fn closest_point(&self, point: Vector3) -> Vector3 {
        let dx = point.x - self.center.x;
        let dy = point.y - self.center.y;
        let angle = dy.atan2(dx);
        if self.angle_in_range(angle) {
            // Project onto the arc at this angle
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < 1e-20 {
                return self.start_point();
            }
            let scale = self.radius / dist;
            Vector3::new(
                self.center.x + dx * scale,
                self.center.y + dy * scale,
                self.center.z,
            )
        } else {
            // Clamp to whichever endpoint is closer
            let sp = self.start_point();
            let ep = self.end_point();
            let d_start = (point.x - sp.x).powi(2) + (point.y - sp.y).powi(2);
            let d_end = (point.x - ep.x).powi(2) + (point.y - ep.y).powi(2);
            if d_start <= d_end {
                sp
            } else {
                ep
            }
        }
    }

    /// Return the minimum distance from a point to the arc.
    pub fn distance_to_point(&self, point: Vector3) -> f64 {
        let cp = self.closest_point(point);
        let dx = point.x - cp.x;
        let dy = point.y - cp.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Return the point on the arc at parameter `t` in `[0, 1]`.
    ///
    /// `t = 0` gives the start point; `t = 1` gives the end point.
    pub fn point_at(&self, t: f64) -> Vector3 {
        let angle = self.start_angle + t * self.sweep_angle();
        Vector3::new(
            self.center.x + self.radius * angle.cos(),
            self.center.y + self.radius * angle.sin(),
            self.center.z,
        )
    }

    /// Return the point on the arc at a given angle (in radians).
    ///
    /// The angle is absolute (not relative to the start angle).
    /// No range check is performed.
    pub fn point_at_angle(&self, angle: f64) -> Vector3 {
        Vector3::new(
            self.center.x + self.radius * angle.cos(),
            self.center.y + self.radius * angle.sin(),
            self.center.z,
        )
    }
}

impl Default for Arc {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for Arc {
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
        // Simplified bounding box - full circle bounds
        // A proper implementation would calculate exact arc bounds
        BoundingBox3D::new(
            Vector3::new(
                self.center.x - self.radius,
                self.center.y - self.radius,
                self.center.z,
            ),
            Vector3::new(
                self.center.x + self.radius,
                self.center.y + self.radius,
                self.center.z,
            ),
        )
    }

    fn translate(&mut self, offset: Vector3) {
        super::translate::translate_arc(self, offset);
    }

    fn entity_type(&self) -> &'static str {
        "ARC"
    }
    
    fn apply_transform(&mut self, transform: &crate::types::Transform) {
        super::transform::transform_arc(self, transform);
    }
    
    fn apply_mirror(&mut self, transform: &crate::types::Transform) {
        super::mirror::mirror_arc(self, transform);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_creation() {
        let arc = Arc::new();
        assert_eq!(arc.center, Vector3::ZERO);
        assert_eq!(arc.radius, 1.0);
        assert_eq!(arc.entity_type(), "ARC");
    }

    #[test]
    fn test_arc_sweep_angle() {
        let arc = Arc::from_coords(0.0, 0.0, 0.0, 5.0, 0.0, std::f64::consts::PI);
        assert!((arc.sweep_angle() - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn test_arc_length() {
        let arc = Arc::from_coords(0.0, 0.0, 0.0, 5.0, 0.0, std::f64::consts::PI);
        let expected = 5.0 * std::f64::consts::PI;
        assert!((arc.arc_length() - expected).abs() < 1e-10);
    }

    #[test]
    fn test_arc_endpoints() {
        let arc = Arc::from_coords(0.0, 0.0, 0.0, 5.0, 0.0, std::f64::consts::PI / 2.0);
        let start = arc.start_point();
        let end = arc.end_point();
        assert!((start.x - 5.0).abs() < 1e-10);
        assert!((start.y - 0.0).abs() < 1e-10);
        assert!((end.x - 0.0).abs() < 1e-10);
        assert!((end.y - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_arc_translate() {
        let mut arc = Arc::from_coords(0.0, 0.0, 0.0, 5.0, 0.0, std::f64::consts::PI);
        arc.translate(Vector3::new(10.0, 20.0, 30.0));
        assert_eq!(arc.center, Vector3::new(10.0, 20.0, 30.0));
        assert_eq!(arc.radius, 5.0);
    }
    
    #[test]
    fn test_arc_mirror_x() {
        use std::f64::consts::PI;
        // Arc from 0° to 90° at origin, radius 5
        let mut arc = Arc::from_coords(0.0, 0.0, 0.0, 5.0, 0.0, PI / 2.0);
        
        // Save original endpoints
        let orig_start = arc.start_point();
        let orig_end = arc.end_point();
        
        arc.mirror_x();
        
        // Center should be mirrored (x negated)
        assert!((arc.center.x - 0.0).abs() < 1e-10);
        
        // New endpoints should match mirrored original endpoints (swapped)
        let new_start = arc.start_point();
        let new_end = arc.end_point();
        // Mirrored original end → new start
        assert!((new_start.x - (-orig_end.x)).abs() < 1e-8);
        assert!((new_start.y - orig_end.y).abs() < 1e-8);
        // Mirrored original start → new end
        assert!((new_end.x - (-orig_start.x)).abs() < 1e-8);
        assert!((new_end.y - orig_start.y).abs() < 1e-8);
    }
    
    #[test]
    fn test_arc_mirror_y() {
        use std::f64::consts::PI;
        let mut arc = Arc::from_coords(0.0, 0.0, 0.0, 5.0, 0.0, PI / 2.0);
        let orig_start = arc.start_point();
        let orig_end = arc.end_point();
        
        arc.mirror_y();
        
        let new_start = arc.start_point();
        let new_end = arc.end_point();
        // Mirrored original end → new start
        assert!((new_start.x - orig_end.x).abs() < 1e-8);
        assert!((new_start.y - (-orig_end.y)).abs() < 1e-8);
        // Mirrored original start → new end
        assert!((new_end.x - orig_start.x).abs() < 1e-8);
        assert!((new_end.y - (-orig_start.y)).abs() < 1e-8);
    }

    #[test]
    fn test_arc_contains_point_on_arc() {
        use std::f64::consts::PI;
        let arc = Arc::from_center_radius_angles(Vector3::ZERO, 5.0, 0.0, PI / 2.0);
        // Start point should be on arc
        assert!(arc.contains_point(arc.start_point()));
        // End point should be on arc
        assert!(arc.contains_point(arc.end_point()));
        // Midpoint should be on arc
        assert!(arc.contains_point(arc.midpoint()));
    }

    #[test]
    fn test_arc_contains_point_off_arc() {
        use std::f64::consts::PI;
        let arc = Arc::from_center_radius_angles(Vector3::ZERO, 5.0, 0.0, PI / 2.0);
        // Point at 180° is on the circle but NOT on the arc
        assert!(!arc.contains_point(Vector3::new(-5.0, 0.0, 0.0)));
        // Point inside the circle
        assert!(!arc.contains_point(Vector3::new(1.0, 1.0, 0.0)));
        // Point far away
        assert!(!arc.contains_point(Vector3::new(100.0, 100.0, 0.0)));
    }

    #[test]
    fn test_arc_closest_point_projection() {
        use std::f64::consts::PI;
        let arc = Arc::from_center_radius_angles(Vector3::ZERO, 5.0, 0.0, PI / 2.0);
        // A point at (10, 0) should project to (5, 0) — start of arc
        let cp = arc.closest_point(Vector3::new(10.0, 0.0, 0.0));
        assert!((cp.x - 5.0).abs() < 1e-10);
        assert!(cp.y.abs() < 1e-10);
    }

    #[test]
    fn test_arc_closest_point_clamps_to_endpoint() {
        use std::f64::consts::PI;
        let arc = Arc::from_center_radius_angles(Vector3::ZERO, 5.0, 0.0, PI / 2.0);
        // Point at angle 180° is outside arc range — closest should be an endpoint
        let cp = arc.closest_point(Vector3::new(-10.0, 0.0, 0.0));
        let sp = arc.start_point();
        let ep = arc.end_point();
        let is_start = (cp.x - sp.x).abs() < 1e-10 && (cp.y - sp.y).abs() < 1e-10;
        let is_end = (cp.x - ep.x).abs() < 1e-10 && (cp.y - ep.y).abs() < 1e-10;
        assert!(is_start || is_end);
    }

    #[test]
    fn test_arc_distance_to_point_on_arc() {
        use std::f64::consts::PI;
        let arc = Arc::from_center_radius_angles(Vector3::ZERO, 5.0, 0.0, PI / 2.0);
        assert!(arc.distance_to_point(arc.start_point()) < 1e-10);
        assert!(arc.distance_to_point(arc.midpoint()) < 1e-10);
    }

    #[test]
    fn test_arc_distance_to_point_outside() {
        use std::f64::consts::PI;
        let arc = Arc::from_center_radius_angles(Vector3::ZERO, 5.0, 0.0, PI / 2.0);
        // Distance from (10, 0) to arc — closest point is (5,0), distance = 5
        let d = arc.distance_to_point(Vector3::new(10.0, 0.0, 0.0));
        assert!((d - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_arc_point_at_parametric() {
        use std::f64::consts::PI;
        let arc = Arc::from_center_radius_angles(Vector3::ZERO, 5.0, 0.0, PI / 2.0);
        let p0 = arc.point_at(0.0);
        let p1 = arc.point_at(1.0);
        let pm = arc.point_at(0.5);
        // t=0 → start
        assert!((p0.x - 5.0).abs() < 1e-10);
        assert!(p0.y.abs() < 1e-10);
        // t=1 → end
        assert!(p1.x.abs() < 1e-10);
        assert!((p1.y - 5.0).abs() < 1e-10);
        // t=0.5 → midpoint at 45°
        let expected = 5.0 * (PI / 4.0).cos();
        assert!((pm.x - expected).abs() < 1e-10);
        assert!((pm.y - expected).abs() < 1e-10);
    }

    #[test]
    fn test_arc_point_at_angle() {
        use std::f64::consts::PI;
        let arc = Arc::from_center_radius_angles(Vector3::ZERO, 5.0, 0.0, PI);
        let p = arc.point_at_angle(PI / 2.0);
        assert!(p.x.abs() < 1e-10);
        assert!((p.y - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_arc_wrapping_angle_range() {
        // Arc from 350° to 10° (wraps around 0°)
        let arc = Arc::from_center_radius_angles(
            Vector3::ZERO,
            5.0,
            350.0_f64.to_radians(),
            10.0_f64.to_radians(),
        );
        // Point at 0° should be on this arc
        assert!(arc.contains_point(Vector3::new(5.0, 0.0, 0.0)));
        // Point at 180° should NOT be on this arc
        assert!(!arc.contains_point(Vector3::new(-5.0, 0.0, 0.0)));
    }
}


