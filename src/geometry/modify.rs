use crate::entities::{Arc, Line, LwPolyline, LwVertex};
use crate::geometry::intersections;
use crate::types::{Vector2, Vector3};

const EPS: f64 = 1e-9;

fn nearest_endpoint_is_start(line: &Line, point: Vector3) -> bool {
    line.start.distance(&point) <= line.end.distance(&point)
}

fn direction_from_intersection(line: &Line, intersection: Vector3) -> Option<(Vector3, bool, f64)> {
    let ds = line.start.distance(&intersection);
    let de = line.end.distance(&intersection);

    if ds <= de {
        let v = line.end - intersection;
        let len = v.length();
        if len <= EPS {
            None
        } else {
            Some((v / len, true, de))
        }
    } else {
        let v = line.start - intersection;
        let len = v.length();
        if len <= EPS {
            None
        } else {
            Some((v / len, false, ds))
        }
    }
}

pub fn trim_line_at_intersection(line: &Line, intersection: Vector3, pick_point: Vector3) -> Option<Line> {
    let mut out = line.clone();
    if nearest_endpoint_is_start(&out, pick_point) {
        out.start = intersection;
    } else {
        out.end = intersection;
    }
    if out.length() <= EPS {
        None
    } else {
        Some(out)
    }
}

pub fn extend_line_to_intersection(line: &Line, intersection: Vector3, pick_point: Vector3) -> Option<Line> {
    let mut out = line.clone();
    if nearest_endpoint_is_start(&out, pick_point) {
        out.start = intersection;
    } else {
        out.end = intersection;
    }
    if out.length() <= EPS {
        None
    } else {
        Some(out)
    }
}

fn projected_parameter(line: &Line, point: Vector3) -> Option<f64> {
    let d = line.end - line.start;
    let len2 = d.dot(&d);
    if len2 <= EPS {
        None
    } else {
        Some((point - line.start).dot(&d) / len2)
    }
}

pub fn break_line(line: &Line, first_point: Vector3, second_point: Option<Vector3>) -> Vec<Line> {
    let Some(mut t1) = projected_parameter(line, first_point) else {
        return Vec::new();
    };

    let mut out = Vec::new();

    if let Some(second) = second_point {
        let Some(mut t2) = projected_parameter(line, second) else {
            return Vec::new();
        };

        t1 = t1.clamp(0.0, 1.0);
        t2 = t2.clamp(0.0, 1.0);

        if (t1 - t2).abs() <= EPS {
            // Single-point break behaves like split.
            let t = t1.clamp(EPS, 1.0 - EPS);
            let split = line.point_at(t);
            let mut a = line.clone();
            a.end = split;
            let mut b = line.clone();
            b.start = split;
            if a.length() > EPS {
                out.push(a);
            }
            if b.length() > EPS {
                out.push(b);
            }
            return out;
        }

        let (ta, tb) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
        let pa = line.point_at(ta);
        let pb = line.point_at(tb);

        let mut left = line.clone();
        left.end = pa;
        if left.length() > EPS {
            out.push(left);
        }

        let mut right = line.clone();
        right.start = pb;
        if right.length() > EPS {
            out.push(right);
        }

        out
    } else {
        t1 = t1.clamp(EPS, 1.0 - EPS);
        let split = line.point_at(t1);

        let mut a = line.clone();
        a.end = split;
        let mut b = line.clone();
        b.start = split;

        if a.length() > EPS {
            out.push(a);
        }
        if b.length() > EPS {
            out.push(b);
        }

        out
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilletResult {
    pub first: Line,
    pub second: Line,
    pub arc: Arc,
}

pub fn fillet_lines(first: &Line, second: &Line, radius: f64) -> Option<FilletResult> {
    if radius <= EPS {
        return None;
    }

    let i = intersections::line_line_2d(first, second, false)?;
    let p = i.point;

    let (v1, first_start_near, first_available) = direction_from_intersection(first, p)?;
    let (v2, second_start_near, second_available) = direction_from_intersection(second, p)?;

    let dot = v1.dot(&v2).clamp(-1.0, 1.0);
    let theta = dot.acos();
    if theta <= 1e-6 || (std::f64::consts::PI - theta).abs() <= 1e-5 {
        return None;
    }

    let tangent_dist = radius / (theta / 2.0).tan();
    if tangent_dist <= EPS || tangent_dist > first_available || tangent_dist > second_available {
        return None;
    }

    let t1 = p + v1 * tangent_dist;
    let t2 = p + v2 * tangent_dist;

    let bisector = (v1 + v2).normalize();
    if bisector.length() <= EPS {
        return None;
    }

    let center_dist = radius / (theta / 2.0).sin();
    let center = p + bisector * center_dist;

    let mut first_out = first.clone();
    if first_start_near {
        first_out.start = t1;
    } else {
        first_out.end = t1;
    }

    let mut second_out = second.clone();
    if second_start_near {
        second_out.start = t2;
    } else {
        second_out.end = t2;
    }

    if first_out.length() <= EPS || second_out.length() <= EPS {
        return None;
    }

    let a1 = (t1.y - center.y).atan2(t1.x - center.x);
    let a2 = (t2.y - center.y).atan2(t2.x - center.x);

    let cross = v1.x * v2.y - v1.y * v2.x;
    let (start_angle, end_angle) = if cross >= 0.0 { (a1, a2) } else { (a2, a1) };

    let mut arc = Arc::new();
    arc.center = center;
    arc.radius = radius;
    arc.start_angle = start_angle;
    arc.end_angle = end_angle;

    Some(FilletResult {
        first: first_out,
        second: second_out,
        arc,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChamferResult {
    pub first: Line,
    pub second: Line,
    pub chamfer: Line,
}

pub fn chamfer_lines(first: &Line, second: &Line, first_distance: f64, second_distance: f64) -> Option<ChamferResult> {
    if first_distance <= EPS || second_distance <= EPS {
        return None;
    }

    let i = intersections::line_line_2d(first, second, false)?;
    let p = i.point;

    let (v1, first_start_near, first_available) = direction_from_intersection(first, p)?;
    let (v2, second_start_near, second_available) = direction_from_intersection(second, p)?;

    if first_distance > first_available || second_distance > second_available {
        return None;
    }

    let c1 = p + v1 * first_distance;
    let c2 = p + v2 * second_distance;

    let mut first_out = first.clone();
    if first_start_near {
        first_out.start = c1;
    } else {
        first_out.end = c1;
    }

    let mut second_out = second.clone();
    if second_start_near {
        second_out.start = c2;
    } else {
        second_out.end = c2;
    }

    let mut chamfer = Line::from_points(c1, c2);
    chamfer.common = first.common.clone();

    if first_out.length() <= EPS || second_out.length() <= EPS || chamfer.length() <= EPS {
        return None;
    }

    Some(ChamferResult {
        first: first_out,
        second: second_out,
        chamfer,
    })
}

pub fn join_lines_to_lwpolyline(lines: &[Line], tolerance: f64) -> Option<LwPolyline> {
    if lines.is_empty() {
        return None;
    }
    if lines.len() == 1 {
        let mut poly = LwPolyline::new();
        poly.elevation = lines[0].start.z;
        poly.vertices = vec![
            LwVertex::new(Vector2::new(lines[0].start.x, lines[0].start.y)),
            LwVertex::new(Vector2::new(lines[0].end.x, lines[0].end.y)),
        ];
        return Some(poly);
    }

    let mut used = vec![false; lines.len()];
    used[0] = true;

    let mut chain: Vec<Vector3> = vec![lines[0].start, lines[0].end];

    while used.iter().any(|u| !*u) {
        let mut found = false;
        let first = chain[0];
        let last = *chain.last()?;

        for (i, line) in lines.iter().enumerate() {
            if used[i] {
                continue;
            }

            if line.start.distance(&last) <= tolerance {
                chain.push(line.end);
                used[i] = true;
                found = true;
                break;
            }
            if line.end.distance(&last) <= tolerance {
                chain.push(line.start);
                used[i] = true;
                found = true;
                break;
            }
            if line.start.distance(&first) <= tolerance {
                chain.insert(0, line.end);
                used[i] = true;
                found = true;
                break;
            }
            if line.end.distance(&first) <= tolerance {
                chain.insert(0, line.start);
                used[i] = true;
                found = true;
                break;
            }
        }

        if !found {
            return None;
        }
    }

    let mut poly = LwPolyline::new();
    poly.elevation = chain[0].z;

    if chain[0].distance(chain.last()?) <= tolerance {
        poly.is_closed = true;
        chain.pop();
    }

    poly.vertices = chain
        .into_iter()
        .map(|p| LwVertex::new(Vector2::new(p.x, p.y)))
        .collect();

    Some(poly)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn break_line_splits() {
        let line = Line::from_points(Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0));
        let out = break_line(&line, Vector3::new(4.0, 0.0, 0.0), None);
        assert_eq!(out.len(), 2);
        assert!((out[0].length() - 4.0).abs() < 1e-10);
        assert!((out[1].length() - 6.0).abs() < 1e-10);
    }

    #[test]
    fn fillet_lines_creates_arc() {
        let a = Line::from_points(Vector3::new(-10.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 0.0));
        let b = Line::from_points(Vector3::new(0.0, -10.0, 0.0), Vector3::new(0.0, 0.0, 0.0));
        let fillet = fillet_lines(&a, &b, 1.0).expect("fillet");
        assert!(fillet.arc.radius > 0.0);
    }

    #[test]
    fn chamfer_lines_creates_segment() {
        let a = Line::from_points(Vector3::new(-10.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 0.0));
        let b = Line::from_points(Vector3::new(0.0, -10.0, 0.0), Vector3::new(0.0, 0.0, 0.0));
        let chamfer = chamfer_lines(&a, &b, 2.0, 2.0).expect("chamfer");
        assert!(chamfer.chamfer.length() > 0.0);
    }
}
