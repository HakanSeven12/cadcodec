//! DXF write -> read round-trip regressions: each test builds an entity with
//! non-default values, round-trips it through the DXF writer and reader, and
//! checks the fields the writer emits come back.

use std::io::Cursor;

use acadrust::entities::attribute_definition::{
    AttributeDefinition, AttributeFlags, HorizontalAlignment, VerticalAlignment,
};
use acadrust::entities::EntityType;
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DxfReader, DxfWriter};

fn dxf_roundtrip(doc: &CadDocument) -> CadDocument {
    let bytes = DxfWriter::new(doc).write_to_vec().expect("DXF write failed");
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("DXF reader init failed")
        .read()
        .expect("DXF read failed")
}

#[test]
fn attdef_text_and_attribute_fields_survive_dxf_roundtrip() {
    let mut a = AttributeDefinition::new("TAG".into(), "Prompt".into(), "Default".into());
    a.insertion_point = Vector3::new(1.0, 2.0, 0.0);
    a.alignment_point = Vector3::new(3.0, 4.0, 0.0);
    a.height = 2.0;
    a.width_factor = 0.8;
    a.oblique_angle = 15f64.to_radians();
    a.text_style = "Standard".into();
    a.text_generation_flags = 2;
    a.horizontal_alignment = HorizontalAlignment::Center;
    a.vertical_alignment = VerticalAlignment::Middle;
    a.flags = AttributeFlags {
        invisible: true,
        constant: true,
        ..Default::default()
    };
    a.field_length = 12;
    a.normal = Vector3::new(0.0, 0.0, -1.0);

    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    doc.add_entity(EntityType::AttributeDefinition(a)).unwrap();
    let rt = dxf_roundtrip(&doc);
    let b = rt
        .entities()
        .find_map(|e| match e {
            EntityType::AttributeDefinition(a) => Some(a.clone()),
            _ => None,
        })
        .expect("ATTDEF missing");

    assert_eq!(b.alignment_point, Vector3::new(3.0, 4.0, 0.0));
    assert_eq!(b.width_factor, 0.8);
    assert!((b.oblique_angle - 15f64.to_radians()).abs() < 1e-9);
    assert_eq!(b.text_style, "Standard");
    assert_eq!(b.text_generation_flags, 2);
    assert_eq!(b.horizontal_alignment, HorizontalAlignment::Center);
    assert_eq!(b.vertical_alignment, VerticalAlignment::Middle);
    assert!(b.flags.invisible && b.flags.constant && !b.flags.verify);
    assert_eq!(b.field_length, 12);
    assert_eq!(b.normal, Vector3::new(0.0, 0.0, -1.0));
}
