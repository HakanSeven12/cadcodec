use crate::entities::{Arc, Line, LwPolyline, Polyline2D};
use crate::types::Vector2;

const EPS: f64 = 1e-9;

fn segment_left_normal(a: Vector2, b: Vector2) -> Option<Vector2> {
    let d = b - a;
    let len = d.length();
    if len <= EPS {
        None
    } else {
        Some(Vector2::new(-d.y / len, d.x / len))
    }
}

fn vertex_offset_normals(points: &[Vector2], closed: bool) -> Vec<Vector2> {
    let n = points.len();
    let mut normals = Vec::with_capacity(n);

    for i in 0..n {
        let prev_idx = if i == 0 { n.saturating_sub(1) } else { i - 1 };
        let next_idx = if i + 1 >= n { 0 } else { i + 1 };

        let prev_normal = if closed || i > 0 {
            segment_left_normal(points[prev_idx], points[i])
        } else {
            None
        };

        let next_normal = if closed || i + 1 < n {
            segment_left_normal(points[i], points[next_idx])
        } else {
            None
        };

        let nrm = match (prev_normal, next_normal) {
            (Some(a), Some(b)) => {
                let sum = a + b;
                if sum.length() > EPS {
                    sum.normalize()
                } else {
                    b
                }
            }
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => Vector2::ZERO,
        };

        normals.push(nrm);
    }

    normals
}

pub fn offset_line_2d(line: &Line, distance: f64) -> Option<Line> {
    let dir = line.end - line.start;
    let len = (dir.x * dir.x + dir.y * dir.y).sqrt();
    if len <= EPS {
        return None;
    }

    let nx = -dir.y / len;
    let ny = dir.x / len;

    let mut out = line.clone();
    out.start.x += nx * distance;
    out.start.y += ny * distance;
    out.end.x += nx * distance;
    out.end.y += ny * distance;
    Some(out)
}

pub fn offset_arc_2d(arc: &Arc, distance: f64) -> Option<Arc> {
    let radius = arc.radius + distance;
    if radius <= EPS {
        return None;
    }
    let mut out = arc.clone();
    out.radius = radius;
    Some(out)
}

pub fn offset_lwpolyline_2d(poly: &LwPolyline, distance: f64) -> Option<LwPolyline> {
    if poly.vertices.len() < 2 {
        return None;
    }

    let points: Vec<Vector2> = poly.vertices.iter().map(|v| v.location).collect();
    let normals = vertex_offset_normals(&points, poly.is_closed);

    let mut out = poly.clone();
    for (i, vertex) in out.vertices.iter_mut().enumerate() {
        vertex.location = vertex.location + normals[i] * distance;
    }
    Some(out)
}

pub fn offset_polyline2d_2d(poly: &Polyline2D, distance: f64) -> Option<Polyline2D> {
    if poly.vertices.len() < 2 {
        return None;
    }

    let points: Vec<Vector2> = poly
        .vertices
        .iter()
        .map(|v| Vector2::new(v.location.x, v.location.y))
        .collect();

    let normals = vertex_offset_normals(&points, poly.is_closed());

    let mut out = poly.clone();
    for (i, vertex) in out.vertices.iter_mut().enumerate() {
        vertex.location.x += normals[i].x * distance;
        vertex.location.y += normals[i].y * distance;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::LwVertex;
    use crate::types::{Vector2, Vector3};

    #[test]
    fn offset_line_parallel() {
        let line = Line::from_points(Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0));
        let off = offset_line_2d(&line, 2.0).expect("offset line");
        assert!((off.start.y - 2.0).abs() < 1e-10);
        assert!((off.end.y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn offset_lwpolyline_basic() {
        let mut pl = LwPolyline::new();
        pl.vertices = vec![
            LwVertex::from_coords(0.0, 0.0),
            LwVertex::from_coords(10.0, 0.0),
            LwVertex::from_coords(10.0, 10.0),
        ];
        let off = offset_lwpolyline_2d(&pl, 1.0).expect("offset pline");
        assert_eq!(off.vertices.len(), 3);
        assert_ne!(off.vertices[0].location, Vector2::new(0.0, 0.0));
    }
}
