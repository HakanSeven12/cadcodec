use crate::entities::Hatch;
use crate::types::{Vector2, Vector3};
use crate::xdata::{ExtendedDataRecord, XDataValue};

fn origin_index(record: &ExtendedDataRecord) -> Option<usize> {
    let mut depth = 0usize;
    for (index, value) in record.values.iter().enumerate() {
        match value {
            XDataValue::ControlString(text) if text == "{" => depth += 1,
            XDataValue::ControlString(text) if text == "}" => depth = depth.saturating_sub(1),
            XDataValue::Point3D(_) if depth == 0 => return Some(index),
            _ => {},
        }
    }
    None
}

impl Hatch {
    /// Pattern origin recorded as the top-level ACAD 1010 point, in hatch coordinates.
    pub fn stored_pattern_origin(&self) -> Option<Vector2> {
        let record = self.common.extended_data.get_record("ACAD")?;
        let XDataValue::Point3D(point) = &record.values[origin_index(record)?] else { return None; };
        (point.x.is_finite() && point.y.is_finite()).then_some(Vector2::new(point.x, point.y))
    }

    /// Absent origin metadata denotes the coordinate origin, not a pattern line base.
    pub fn pattern_origin(&self) -> Vector2 {
        self.stored_pattern_origin().unwrap_or(Vector2::new(0.0, 0.0))
    }

    /// Record an already-applied origin without changing pattern geometry.
    /// Unrelated application records and nested ACAD payloads remain intact.
    pub fn record_pattern_origin(&mut self, origin: Vector2) -> bool {
        if !origin.x.is_finite() || !origin.y.is_finite() { return false; }
        let mut record = self.common.extended_data.get_record("ACAD").cloned()
            .unwrap_or_else(|| ExtendedDataRecord::new("ACAD"));
        let value = XDataValue::Point3D(Vector3::new(origin.x, origin.y, 0.0));
        if let Some(index) = origin_index(&record) { record.values[index] = value; }
        else { record.values.push(value); }
        self.common.extended_data.upsert_record(record);
        true
    }

    /// Move pattern lines by the change in origin, preserving their intrinsic offsets.
    pub fn set_pattern_origin(&mut self, origin: Vector2) -> bool {
        if !origin.x.is_finite() || !origin.y.is_finite() { return false; }
        let previous = self.pattern_origin();
        let dx = origin.x - previous.x;
        let dy = origin.y - previous.y;
        if !dx.is_finite() || !dy.is_finite() || self.pattern.lines.iter().any(|line|
            !(line.base_point.x + dx).is_finite() || !(line.base_point.y + dy).is_finite()) { return false; }
        for line in &mut self.pattern.lines { line.base_point.x += dx; line.base_point.y += dy; }
        self.record_pattern_origin(origin)
    }

    /// Scale the stored pattern geometry about its recorded origin.
    pub fn scale_pattern_about_origin(&mut self, factor: f64) {
        if !factor.is_finite() || factor <= 0.0 { return; }
        let origin = self.pattern_origin();
        for line in &mut self.pattern.lines {
            line.base_point.x = origin.x + (line.base_point.x - origin.x) * factor;
            line.base_point.y = origin.y + (line.base_point.y - origin.y) * factor;
            line.offset.x *= factor; line.offset.y *= factor;
            for dash in &mut line.dash_lengths { *dash *= factor; }
        }
    }

    /// Rotate the stored pattern geometry about its recorded origin.
    pub fn rotate_pattern_about_origin(&mut self, angle: f64) {
        if !angle.is_finite() { return; }
        let origin = self.pattern_origin();
        let (sin, cos) = angle.sin_cos();
        for line in &mut self.pattern.lines {
            let x = line.base_point.x - origin.x; let y = line.base_point.y - origin.y;
            line.base_point.x = origin.x + x*cos - y*sin;
            line.base_point.y = origin.y + x*sin + y*cos;
            let x = line.offset.x; let y = line.offset.y;
            line.offset.x = x*cos - y*sin; line.offset.y = x*sin + y*cos;
            line.angle += angle;
        }
    }
}
