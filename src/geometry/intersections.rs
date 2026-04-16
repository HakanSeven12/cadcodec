use crate::entities::{Arc, Line};
use crate::types::{Vector2, Vector3};

const EPS: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineLineIntersection {
    pub point: Vector3,
    pub t1: f64,
    pub t2: f64,
}

fn xy(v: Vector3) -> Vector2 {
    Vector2::new(v.x, v.y)
}

fn cross2(a: Vector2, b: Vector2) -> f64 {
    a.x * b.y - a.y * b.x
}

pub fn line_line_2d(a: &Line, b: &Line, segment_only: bool) -> Option<LineLineIntersection> {
    let p = xy(a.start);
    let r = xy(a.end) - p;
    let q = xy(b.start);
    let s = xy(b.end) - q;

    let rxs = cross2(r, s);
    if rxs.abs() <= EPS {
        return None;
    }

    let q_minus_p = q - p;
    let t = cross2(q_minus_p, s) / rxs;
    let u = cross2(q_minus_p, r) / rxs;

    if segment_only {
        if !(-EPS..=1.0 + EPS).contains(&t) || !(-EPS..=1.0 + EPS).contains(&u) {
            return None;
        }
    }

    let point = a.start + (a.end - a.start) * t;
    Some(LineLineIntersection { point, t1: t, t2: u })
}

pub fn line_circle_2d(
    line: &Line,
    center: Vector3,
    radius: f64,
    segment_only: bool,
) -> Vec<(Vector3, f64)> {
    if radius <= EPS {
        return Vec::new();
    }

    let p0 = line.start;
    let p1 = line.end;
    let d = p1 - p0;
    let f = p0 - center;

    let a = d.dot(&d);
    if a <= EPS {
        return Vec::new();
    }

    let b = 2.0 * f.dot(&d);
    let c = f.dot(&f) - radius * radius;
    let disc = b * b - 4.0 * a * c;

    if disc < -EPS {
        return Vec::new();
    }

    let mut ts = Vec::new();
    if disc.abs() <= EPS {
        ts.push(-b / (2.0 * a));
    } else {
        let s = disc.sqrt();
        ts.push((-b - s) / (2.0 * a));
        ts.push((-b + s) / (2.0 * a));
    }

    ts.sort_by(|l, r| l.partial_cmp(r).unwrap_or(std::cmp::Ordering::Equal));

    let mut out: Vec<(Vector3, f64)> = Vec::new();
    for t in ts {
        if segment_only && !(-EPS..=1.0 + EPS).contains(&t) {
            continue;
        }
        let pt = p0 + d * t;
        if out.iter().all(|candidate| candidate.0.distance(&pt) > 1e-7) {
            out.push((pt, t));
        }
    }

    out
}

pub fn line_arc_2d(line: &Line, arc: &Arc, segment_only: bool) -> Vec<Vector3> {
    line_circle_2d(line, arc.center, arc.radius, segment_only)
        .into_iter()
        .map(|(p, _)| p)
        .filter(|p| arc.contains_point(*p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_line_intersection_works() {
        let a = Line::from_points(Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0));
        let b = Line::from_points(Vector3::new(5.0, -2.0, 0.0), Vector3::new(5.0, 2.0, 0.0));
        let i = line_line_2d(&a, &b, true).expect("intersection");
        assert!((i.point.x - 5.0).abs() < 1e-10);
        assert!(i.point.y.abs() < 1e-10);
    }

    #[test]
    fn line_arc_intersection_works() {
        let line = Line::from_points(Vector3::new(-2.0, 0.0, 0.0), Vector3::new(2.0, 0.0, 0.0));
        let arc = Arc::from_center_radius_angles(
            Vector3::ZERO,
            1.0,
            0.0,
            std::f64::consts::PI,
        );
        let pts = line_arc_2d(&line, &arc, true);
        assert_eq!(pts.len(), 2);
    }
}
