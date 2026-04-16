use acadrust::entities::{EntityType, Line};
use acadrust::{CadDocument, Vector3};

#[test]
fn offset_entity_line_creates_new_entity() {
    let mut doc = CadDocument::new();
    let base = Line::from_points(Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 0.0, 0.0));
    let h = doc.add_entity(EntityType::Line(base)).unwrap();

    let off = doc.offset_entity(h, 2.0).unwrap();
    assert_ne!(h, off);

    let line = doc.get_entity(off).unwrap().as_line().unwrap();
    assert!((line.start.y - 2.0).abs() < 1e-8);
    assert!((line.end.y - 2.0).abs() < 1e-8);
}

#[test]
fn fillet_entities_line_line_inserts_arc() {
    let mut doc = CadDocument::new();

    let h1 = doc
        .add_entity(EntityType::Line(Line::from_points(
            Vector3::new(-10.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 0.0),
        )))
        .unwrap();

    let h2 = doc
        .add_entity(EntityType::Line(Line::from_points(
            Vector3::new(0.0, -10.0, 0.0),
            Vector3::new(0.0, 0.0, 0.0),
        )))
        .unwrap();

    let arc_handle = doc.fillet_entities(h1, h2, 1.0).unwrap();
    assert!(doc.get_entity(arc_handle).unwrap().is_arc());
}

#[test]
fn break_entity_line_creates_two_segments() {
    let mut doc = CadDocument::new();
    let h = doc
        .add_entity(EntityType::Line(Line::from_points(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 0.0),
        )))
        .unwrap();

    let out = doc
        .break_entity(h, Vector3::new(4.0, 0.0, 0.0), None)
        .unwrap();

    assert_eq!(out.len(), 2);
    for nh in out {
        assert!(doc.get_entity(nh).is_some());
    }
}

#[test]
fn trim_and_extend_entities_update_lines() {
    let mut doc = CadDocument::new();

    let boundary = doc
        .add_entity(EntityType::Line(Line::from_points(
            Vector3::new(5.0, -10.0, 0.0),
            Vector3::new(5.0, 10.0, 0.0),
        )))
        .unwrap();

    let trim_target = doc
        .add_entity(EntityType::Line(Line::from_points(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 0.0),
        )))
        .unwrap();

    let trimmed = doc
        .trim_entities(boundary, &[trim_target], Vector3::new(1.0, 0.0, 0.0))
        .unwrap();
    assert_eq!(trimmed, 1);

    let l = doc.get_entity(trim_target).unwrap().as_line().unwrap();
    assert!((l.start.x - 5.0).abs() < 1e-8 || (l.end.x - 5.0).abs() < 1e-8);

    let extend_target = doc
        .add_entity(EntityType::Line(Line::from_points(
            Vector3::new(0.0, 2.0, 0.0),
            Vector3::new(2.0, 2.0, 0.0),
        )))
        .unwrap();

    let extended = doc
        .extend_entities(boundary, &[extend_target], Vector3::new(2.0, 2.0, 0.0))
        .unwrap();
    assert_eq!(extended, 1);

    let e = doc.get_entity(extend_target).unwrap().as_line().unwrap();
    assert!((e.start.x - 5.0).abs() < 1e-8 || (e.end.x - 5.0).abs() < 1e-8);
}

#[test]
fn join_entities_lines_to_lwpolyline() {
    let mut doc = CadDocument::new();

    let h1 = doc
        .add_entity(EntityType::Line(Line::from_points(
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(5.0, 0.0, 0.0),
        )))
        .unwrap();
    let h2 = doc
        .add_entity(EntityType::Line(Line::from_points(
            Vector3::new(5.0, 0.0, 0.0),
            Vector3::new(10.0, 0.0, 0.0),
        )))
        .unwrap();

    let joined = doc.join_entities(&[h1, h2], 1e-6).unwrap();
    assert!(doc.get_entity(joined).unwrap().is_lwpolyline());
    assert!(doc.get_entity(h1).is_none());
    assert!(doc.get_entity(h2).is_none());
}
