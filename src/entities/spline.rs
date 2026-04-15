//! Spline entity (NURBS curve)

use super::{Entity, EntityCommon};
use crate::types::{BoundingBox3D, Color, Handle, LineWeight, Transparency, Vector3};

/// Spline flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplineFlags {
    /// Is the spline closed?
    pub closed: bool,
    /// Is the spline periodic?
    pub periodic: bool,
    /// Is the spline rational?
    pub rational: bool,
    /// Is the spline planar?
    pub planar: bool,
    /// Is the spline linear?
    pub linear: bool,
}

impl SplineFlags {
    /// Create default spline flags
    pub fn new() -> Self {
        SplineFlags {
            closed: false,
            periodic: false,
            rational: false,
            planar: false,
            linear: false,
        }
    }
}

impl Default for SplineFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// A spline entity (NURBS curve)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Spline {
    /// Common entity data
    pub common: EntityCommon,
    /// Degree of the spline (typically 3 for cubic)
    pub degree: i32,
    /// Spline flags
    pub flags: SplineFlags,
    /// Knot values
    pub knots: Vec<f64>,
    /// Control points
    pub control_points: Vec<Vector3>,
    /// Weights (for rational splines)
    pub weights: Vec<f64>,
    /// Fit points (if available)
    pub fit_points: Vec<Vector3>,
    /// Normal vector
    pub normal: Vector3,
}

impl Spline {
    /// Create a new spline
    pub fn new() -> Self {
        Spline {
            common: EntityCommon::new(),
            degree: 3,
            flags: SplineFlags::new(),
            knots: Vec::new(),
            control_points: Vec::new(),
            weights: Vec::new(),
            fit_points: Vec::new(),
            normal: Vector3::UNIT_Z,
        }
    }

    /// Create a spline from control points
    pub fn from_control_points(degree: i32, control_points: Vec<Vector3>) -> Self {
        let knots = Self::generate_clamped_knots(degree as usize, control_points.len());
        Spline {
            degree,
            control_points,
            knots,
            ..Self::new()
        }
    }

    /// Create a spline from fit points
    pub fn from_fit_points(fit_points: Vec<Vector3>) -> Self {
        Spline {
            fit_points,
            ..Self::new()
        }
    }

    /// Generate a clamped uniform knot vector for the given degree and
    /// number of control points.
    ///
    /// The result has `n + p + 1` elements: `p+1` zeros, evenly-spaced
    /// internal knots, and `p+1` ones.
    pub fn generate_clamped_knots(degree: usize, num_control_points: usize) -> Vec<f64> {
        if num_control_points == 0 {
            return Vec::new();
        }
        let n = num_control_points;
        let p = degree;
        let m = n + p + 1;
        let mut kv = Vec::with_capacity(m);
        for _ in 0..=p {
            kv.push(0.0);
        }
        let internal = m - 2 * (p + 1);
        for i in 1..=internal {
            kv.push(i as f64 / (internal + 1) as f64);
        }
        for _ in 0..=p {
            kv.push(1.0);
        }
        kv
    }

    /// Get the number of control points
    pub fn control_point_count(&self) -> usize {
        self.control_points.len()
    }

    /// Get the number of knots
    pub fn knot_count(&self) -> usize {
        self.knots.len()
    }

    /// Add a control point
    pub fn add_control_point(&mut self, point: Vector3) {
        self.control_points.push(point);
    }

    /// Add a knot value
    pub fn add_knot(&mut self, knot: f64) {
        self.knots.push(knot);
    }

    /// Evaluate a single B-spline basis function N_{i,p}(t) using the
    /// Cox-de Boor recursion.
    fn basis(i: usize, p: usize, t: f64, knots: &[f64]) -> f64 {
        if p == 0 {
            return if knots[i] <= t && t < knots[i + 1] { 1.0 } else { 0.0 };
        }
        let mut val = 0.0;
        let denom1 = knots[i + p] - knots[i];
        if denom1.abs() > 1e-14 {
            val += (t - knots[i]) / denom1 * Self::basis(i, p - 1, t, knots);
        }
        let denom2 = knots[i + p + 1] - knots[i + 1];
        if denom2.abs() > 1e-14 {
            val += (knots[i + p + 1] - t) / denom2 * Self::basis(i + 1, p - 1, t, knots);
        }
        val
    }

    /// Evaluate the spline at parameter `t` (in the knot domain).
    ///
    /// For a clamped knot vector this is typically `[0, 1]`.
    /// Uses the Cox-de Boor algorithm.
    ///
    /// Returns `None` if the spline has no control points or knots.
    pub fn evaluate(&self, t: f64) -> Option<Vector3> {
        let n = self.control_points.len();
        let p = self.degree as usize;
        if n == 0 || self.knots.len() < n + p + 1 {
            return None;
        }
        // Clamp t to the valid range to handle endpoint evaluation
        let t_min = self.knots[p];
        let t_max = self.knots[n];
        let t_clamped = if (t - t_max).abs() < 1e-14 {
            t_max - 1e-14
        } else {
            t.clamp(t_min, t_max)
        };
        let mut point = Vector3::ZERO;
        let rational = !self.weights.is_empty() && self.weights.len() == n;
        let mut w_sum = 0.0;
        for i in 0..n {
            let b = Self::basis(i, p, t_clamped, &self.knots);
            let w = if rational { self.weights[i] } else { 1.0 };
            let bw = b * w;
            point = Vector3::new(
                point.x + self.control_points[i].x * bw,
                point.y + self.control_points[i].y * bw,
                point.z + self.control_points[i].z * bw,
            );
            w_sum += bw;
        }
        if w_sum.abs() < 1e-20 {
            return None;
        }
        Some(Vector3::new(point.x / w_sum, point.y / w_sum, point.z / w_sum))
    }

    /// Evaluate the spline at a normalized parameter `t` in `[0, 1]`.
    ///
    /// `t = 0` → start of the spline, `t = 1` → end.
    pub fn point_at(&self, t: f64) -> Option<Vector3> {
        let n = self.control_points.len();
        let p = self.degree as usize;
        if n == 0 || self.knots.len() < n + p + 1 {
            return None;
        }
        let t_min = self.knots[p];
        let t_max = self.knots[n];
        let knot_t = t_min + t * (t_max - t_min);
        self.evaluate(knot_t)
    }

    /// Approximate the arc-length of the spline by evaluating many points.
    ///
    /// `segments` controls the accuracy — more segments yield a more
    /// accurate result.
    pub fn approx_length(&self, segments: usize) -> f64 {
        let segments = segments.max(1);
        let mut length = 0.0;
        let mut prev = match self.point_at(0.0) {
            Some(p) => p,
            None => return 0.0,
        };
        for i in 1..=segments {
            let t = i as f64 / segments as f64;
            if let Some(p) = self.point_at(t) {
                length += (p - prev).length();
                prev = p;
            }
        }
        length
    }

    /// Convert the spline to a sequence of line segments (tessellation).
    ///
    /// Returns `segments + 1` points that approximate the curve.
    pub fn tessellate(&self, segments: usize) -> Vec<Vector3> {
        let segments = segments.max(1);
        let mut points = Vec::with_capacity(segments + 1);
        for i in 0..=segments {
            let t = i as f64 / segments as f64;
            if let Some(p) = self.point_at(t) {
                points.push(p);
            }
        }
        points
    }
}

impl Default for Spline {
    fn default() -> Self {
        Self::new()
    }
}

impl Entity for Spline {
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
        if self.control_points.is_empty() {
            if self.fit_points.is_empty() {
                return BoundingBox3D::from_point(Vector3::ZERO);
            }
            return BoundingBox3D::from_points(&self.fit_points).unwrap();
        }
        BoundingBox3D::from_points(&self.control_points).unwrap()
    }

    fn translate(&mut self, offset: Vector3) {
        super::translate::translate_spline(self, offset);
    }

    fn entity_type(&self) -> &'static str {
        "SPLINE"
    }
    
    fn apply_transform(&mut self, transform: &crate::types::Transform) {
        super::transform::transform_spline(self, transform);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linear_spline() -> Spline {
        // Degree-1 linear spline from (0,0,0) to (10,0,0)
        Spline::from_control_points(1, vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 0.0),
        ])
    }

    fn cubic_spline() -> Spline {
        Spline::from_control_points(3, vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 2.0, 0.0),
            Vector3::new(3.0, 2.0, 0.0),
            Vector3::new(4.0, 0.0, 0.0),
        ])
    }

    #[test]
    fn test_spline_evaluate_linear_start() {
        let s = linear_spline();
        let p = s.evaluate(0.0).unwrap();
        assert!((p.x).abs() < 1e-8);
    }

    #[test]
    fn test_spline_evaluate_linear_end() {
        let s = linear_spline();
        let p = s.point_at(1.0).unwrap();
        assert!((p.x - 10.0).abs() < 1e-8);
    }

    #[test]
    fn test_spline_evaluate_linear_mid() {
        let s = linear_spline();
        let p = s.point_at(0.5).unwrap();
        assert!((p.x - 5.0).abs() < 1e-8);
    }

    #[test]
    fn test_spline_point_at_cubic_endpoints() {
        let s = cubic_spline();
        let start = s.point_at(0.0).unwrap();
        let end = s.point_at(1.0).unwrap();
        assert!((start.x).abs() < 1e-6);
        assert!((start.y).abs() < 1e-6);
        assert!((end.x - 4.0).abs() < 1e-6);
        assert!((end.y).abs() < 1e-6);
    }

    #[test]
    fn test_spline_point_at_cubic_midpoint() {
        let s = cubic_spline();
        let mid = s.point_at(0.5).unwrap();
        // midpoint should be roughly between control points
        assert!(mid.x > 0.5 && mid.x < 3.5);
        assert!(mid.y > 0.0);
    }

    #[test]
    fn test_spline_approx_length_linear() {
        let s = linear_spline();
        let len = s.approx_length(100);
        assert!((len - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_spline_approx_length_cubic() {
        let s = cubic_spline();
        let len = s.approx_length(200);
        // The arc length of this cubic should be > chord length of 4.0
        assert!(len > 4.0);
    }

    #[test]
    fn test_spline_tessellate() {
        let s = cubic_spline();
        let pts = s.tessellate(10);
        assert_eq!(pts.len(), 11);
        assert!((pts[0].x).abs() < 1e-6);
        assert!((pts[10].x - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_spline_tessellate_linear() {
        let s = linear_spline();
        let pts = s.tessellate(4);
        assert_eq!(pts.len(), 5);
        for (i, p) in pts.iter().enumerate() {
            let expected_x = 10.0 * i as f64 / 4.0;
            assert!((p.x - expected_x).abs() < 0.1);
        }
    }

    #[test]
    fn test_spline_evaluate_empty() {
        let s = Spline::new();
        assert!(s.evaluate(0.5).is_none());
        assert!(s.point_at(0.5).is_none());
    }

    #[test]
    fn test_spline_approx_length_empty() {
        let s = Spline::new();
        assert_eq!(s.approx_length(100), 0.0);
    }
}
