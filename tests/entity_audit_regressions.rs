use std::io::Cursor;

use acadrust::entities::acis::{primitives, SatCoedge, SatDocument, SatPlaneSurface};
use acadrust::entities::*;
use acadrust::tables::TextStyle;
use acadrust::{
    CadDocument, DwgReader, DwgWriter, DxfReader, DxfVersion, DxfWriter, Handle, Vector3,
};

fn pairs(document: &CadDocument) -> Vec<(i32, String)> {
    let data = String::from_utf8(DxfWriter::new(document).write_to_vec().unwrap()).unwrap();
    data.lines()
        .collect::<Vec<_>>()
        .chunks_exact(2)
        .map(|pair| (pair[0].trim().parse().unwrap(), pair[1].trim().to_string()))
        .collect()
}

#[test]
fn history_header_uses_versioned_byte_codes() {
    for version in [
        DxfVersion::AC1012,
        DxfVersion::AC1015,
        DxfVersion::AC1021,
        DxfVersion::AC1032,
    ] {
        let mut doc = CadDocument::with_version(version);
        doc.header.record_solid_history = true;
        doc.header.show_solid_history = 2;
        let pairs = pairs(&doc);
        for name in ["$SOLIDHIST", "$SHOWHIST"] {
            let index = pairs.iter().position(|pair| pair.1 == name);
            if version < DxfVersion::AC1021 {
                assert!(index.is_none());
            } else {
                assert_eq!(pairs[index.unwrap() + 1].0, 280);
            }
        }
        for binary in [false, true] {
            let mut writer = DxfWriter::new(&doc);
            writer.binary = binary;
            let decoded = DxfReader::from_reader(Cursor::new(writer.write_to_vec().unwrap()))
                .unwrap()
                .read()
                .unwrap();
            if version >= DxfVersion::AC1021 {
                assert!(decoded.header.record_solid_history);
                assert_eq!(decoded.header.show_solid_history, 2);
            }
        }
    }
}

#[test]
fn shape_file_style_has_no_dxf_name() {
    let mut doc = CadDocument::new();
    let mut style = TextStyle::new("SHAPES");
    style.handle = doc.allocate_handle();
    style.is_shape_file = true;
    style.font_file = "ltypeshp.shx".into();
    doc.text_styles.add(style).unwrap();
    let pairs = pairs(&doc);
    let start = pairs
        .iter()
        .position(|pair| pair.1 == "ltypeshp.shx")
        .unwrap();
    let name = pairs[..start]
        .iter()
        .rev()
        .find(|pair| pair.0 == 2)
        .unwrap();
    assert_eq!(name.1, "");
}

#[test]
fn underlay_definitions_keep_reactor_back_references() {
    use acadrust::objects::ObjectType;
    let mut doc = CadDocument::with_version(DxfVersion::AC1021);
    let mut definition = acadrust::objects::UnderlayDefinition::dwf("test.dwf", "Model");
    definition.handle = doc.allocate_handle();
    definition.owner_handle = doc.header.named_objects_dict_handle;
    let def_handle = definition.handle;
    doc.objects
        .insert(def_handle, ObjectType::UnderlayDefinition(definition));
    if let Some(ObjectType::Dictionary(root)) =
        doc.objects.get_mut(&doc.header.named_objects_dict_handle)
    {
        root.add_entry("UNDERLAY_TEST", def_handle);
    }
    let mut underlay = Underlay::new(acadrust::entities::underlay::UnderlayType::Dwf);
    underlay.definition_handle = def_handle;
    let handle = doc.add_entity(EntityType::Underlay(underlay)).unwrap();
    let loaded = DxfReader::from_reader(Cursor::new(DxfWriter::new(&doc).write_to_vec().unwrap()))
        .unwrap()
        .read()
        .unwrap();
    let ObjectType::UnderlayDefinition(definition) = loaded.objects.get(&def_handle).unwrap()
    else {
        panic!()
    };
    assert_eq!(definition.reactors, vec![handle]);
    assert_eq!(
        definition.owner_handle,
        doc.header.named_objects_dict_handle
    );
    let ObjectType::UnderlayDefinition(source) = doc.objects.get(&def_handle).unwrap() else {
        panic!()
    };
    assert!(source.reactors.is_empty());
}

#[test]
fn insert_children_have_nonzero_unique_handles() {
    let mut doc = CadDocument::new();
    let mut insert = Insert::new("*Model_Space", Vector3::ZERO);
    insert
        .attributes
        .push(AttributeEntity::simple("TAG", "value"));
    let handle = doc.add_entity(EntityType::Insert(insert)).unwrap();
    let pairs = pairs(&doc);
    let index = pairs
        .iter()
        .position(|pair| pair == &(0, "ATTRIB".into()))
        .unwrap();
    let child = Handle::new(u64::from_str_radix(&pairs[index + 1].1, 16).unwrap());
    assert!(!child.is_null());
    assert_ne!(child, handle);
    assert_eq!(pairs[index + 2], (330, format!("{:X}", handle.value())));
    let seed = pairs.iter().position(|pair| pair.1 == "$HANDSEED").unwrap();
    assert!(u64::from_str_radix(&pairs[seed + 1].1, 16).unwrap() > child.value());
    let EntityType::Insert(source) = doc.get_entity(handle).unwrap() else {
        panic!()
    };
    assert!(source.attributes[0].common.handle.is_null());
}

#[test]
fn missing_table_block_is_serialized_without_mutating_source() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1021);
    let handle = doc
        .add_entity(EntityType::Table(Table::new(Vector3::ZERO, 2, 2)))
        .unwrap();
    let bytes = DwgWriter::write_to_vec(&doc).unwrap();
    let loaded = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let EntityType::Table(table) = loaded.get_entity(handle).unwrap() else {
        panic!()
    };
    let block = loaded
        .block_records
        .iter()
        .find(|block| Some(block.handle) == table.block_record_handle)
        .unwrap();
    assert!(block.flags.anonymous);
    assert!(loaded
        .objects
        .contains_key(&table.table_style_handle.unwrap()));
    assert!(!block.block_entity_handle.is_null());
    assert!(!block.block_end_handle.is_null());
    let EntityType::Table(original) = doc.get_entity(handle).unwrap() else {
        panic!()
    };
    assert!(original.block_record_handle.is_none());
}

#[test]
fn existing_table_block_gets_missing_record_and_marker_handles() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1021);
    doc.block_records
        .add(acadrust::BlockRecord::new("*T1"))
        .unwrap();
    let mut table = Table::new(Vector3::ZERO, 2, 2);
    table.block_name = "*T1".into();
    table.block_record_handle = Some(Handle::NULL);
    let handle = doc.add_entity(EntityType::Table(table)).unwrap();
    let loaded = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
        .read()
        .unwrap();
    let EntityType::Table(table) = loaded.get_entity(handle).unwrap() else {
        panic!()
    };
    let block = loaded
        .block_records
        .iter()
        .find(|block| Some(block.handle) == table.block_record_handle)
        .unwrap();
    assert!(!block.handle.is_null());
    assert!(block.flags.anonymous);
    assert!(!block.block_entity_handle.is_null());
    assert!(!block.block_end_handle.is_null());
    assert_ne!(block.block_entity_handle, block.block_end_handle);
    assert!(doc.block_records.get("*T1").unwrap().handle.is_null());
}

#[test]
fn r2007_mleader_and_table_styles_remain_reachable() {
    let doc = CadDocument::with_version(DxfVersion::AC1021);
    let decoded = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
        .read()
        .unwrap();
    for name in ["ACAD_MLEADERSTYLE", "ACAD_TABLESTYLE"] {
        let acadrust::objects::ObjectType::Dictionary(root) = decoded
            .objects
            .get(&decoded.header.named_objects_dict_handle)
            .unwrap()
        else {
            panic!()
        };
        let dictionary = root.get(name).unwrap();
        let acadrust::objects::ObjectType::Dictionary(styles) =
            decoded.objects.get(&dictionary).unwrap()
        else {
            panic!()
        };
        assert!(decoded
            .objects
            .contains_key(&styles.get("Standard").unwrap()));
    }
}

#[test]
fn mline_and_wipeout_use_valid_dxf_references_and_subclasses() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1021);
    doc.add_entity(EntityType::MLine(MLine::from_points(&[
        Vector3::ZERO,
        Vector3::UNIT_X,
    ])))
    .unwrap();
    doc.add_entity(EntityType::Wipeout(Wipeout::new())).unwrap();
    let data = pairs(&doc);
    let mline = data
        .iter()
        .position(|pair| pair == &(0, "MLINE".into()))
        .unwrap();
    let style = data[mline..].iter().find(|pair| pair.0 == 340).unwrap();
    assert_ne!(style.1, "0");
    let wipeout = data
        .iter()
        .position(|pair| pair == &(0, "WIPEOUT".into()))
        .unwrap();
    assert!(data[wipeout..]
        .iter()
        .take_while(|pair| pair.1 != "ENDSEC")
        .any(|pair| pair == &(100, "AcDbWipeout".into())));
}

#[test]
fn light_binary_field_widths_and_viewport_id_match_group_codes() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1021);
    let light = doc.add_entity(EntityType::Light(Light::new())).unwrap();
    let mut viewport = Viewport::new();
    viewport.id = 7;
    let viewport = doc.add_entity(EntityType::Viewport(viewport)).unwrap();
    let data = DxfWriter::new_binary(&doc).write_to_vec().unwrap();
    let loaded = DxfReader::from_reader(Cursor::new(data))
        .unwrap()
        .read()
        .unwrap();
    assert!(matches!(
        loaded.get_entity(light),
        Some(EntityType::Light(_))
    ));
    let Some(EntityType::Viewport(viewport)) = loaded.get_entity(viewport) else {
        panic!()
    };
    assert_eq!(viewport.id, 7);
}

#[test]
fn sat_origin_bodies_survive_dwg_modeler_encoding() {
    for version in [
        DxfVersion::AC1015,
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1027,
        DxfVersion::AC1032,
    ] {
        let mut doc = CadDocument::with_version(version);
        let mut region =
            Region::from_sat(include_str!("../examples/entity_atlas_assets/region.sat"));
        region.apply_transform(&acadrust::types::Transform::from_translation(Vector3::new(
            20., 30., 0.,
        )));
        let h = doc.add_entity(EntityType::Region(region)).unwrap();
        let loaded = DwgReader::from_stream(Cursor::new(DwgWriter::write_to_vec(&doc).unwrap()))
            .read()
            .unwrap();
        let EntityType::Region(region) = loaded.get_entity(h).unwrap() else {
            panic!()
        };
        let sat = region.acis_data.parse().unwrap();
        assert_eq!(sat.bodies().len(), 1);
        assert_eq!(sat.faces().len(), 1);
        assert_eq!(sat.placement().1, [20., 30., 0.]);
    }
}

#[test]
fn dxf_sat_cipher_matches_autocad_and_roundtrips_binary_spaces() {
    assert_eq!(AcisData::encode_sat("700 0 1 0"), "hoo o n o");
    assert_eq!(AcisData::encode_sat("ACIS A B"), "^ \\VL ^  ]");
    let sat =
        "700 0 1 0\n5 A B C 3 ASM 0 \n1 0.000001 0.0000000001\nbody $-1 -1 $-1 $-1 $-1 $-1 #\n";
    for binary in [false, true] {
        let mut doc = CadDocument::with_version(DxfVersion::AC1021);
        let h = doc
            .add_entity(EntityType::Region(Region::from_sat(sat)))
            .unwrap();
        let mut writer = DxfWriter::new(&doc);
        writer.binary = binary;
        let read = DxfReader::from_reader(Cursor::new(writer.write_to_vec().unwrap()))
            .unwrap()
            .read()
            .unwrap();
        let EntityType::Region(region) = read.get_entity(h).unwrap() else {
            panic!()
        };
        assert_eq!(region.acis_data.sat_data, sat);
    }
}

#[test]
fn sat_transform_and_booleans_use_modeler_tokens() {
    let mut doc =
        SatDocument::parse(include_str!("../examples/entity_atlas_assets/region.sat")).unwrap();
    let matrix = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    doc.set_placement(matrix, [20., 30., 0.], 1.);
    let sat = doc.to_sat_string();
    assert!(!sat.lines().nth(1).unwrap().starts_with('@'));
    assert!(!sat.contains("FALSE"));
    assert!(sat
        .contains("transform $-1 -1 1 0 0 0 1 0 0 0 1 20 30 0 1 no_rotate no_reflect no_shear #"));
    assert_eq!(
        SatDocument::parse(&sat).unwrap().placement(),
        (matrix, [20., 30., 0.], 1.)
    );
}

#[test]
fn pyramid_planes_contain_apex_and_coedges_have_reciprocal_partners() {
    let doc = primitives::build_pyramid([3., 4., 5.], 20., 30.);
    let apex = Vector3::new(3., 4., 35.);
    for plane in doc
        .records
        .iter()
        .filter_map(SatPlaneSurface::from_record)
        .skip(1)
    {
        let (x, y, z) = plane.root_point();
        let (nx, ny, nz) = plane.normal();
        assert!(
            (apex - Vector3::new(x, y, z))
                .dot(&Vector3::new(nx, ny, nz))
                .abs()
                < 1e-10
        );
    }
    for record in &doc.records {
        if let Some(edge) = SatCoedge::from_record(record) {
            let partner =
                SatCoedge::from_record(doc.record(edge.partner().0 as usize).unwrap()).unwrap();
            assert_eq!(partner.partner().0, record.index);
            assert_eq!(partner.edge(), edge.edge());
        }
    }
}
