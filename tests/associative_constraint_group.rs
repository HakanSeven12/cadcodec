use std::io::Cursor;

use acadrust::objects::{
    Assoc2dConstraintGroup, AssocAction, AssocConstraintNode, AssocConstraintNodeData,
    AssociativeData, AssociativeObject, ObjectType,
};
use acadrust::{CadDocument, DwgReader, DwgWriter};

#[test]
fn dwg_constraint_group_counts_only_registered_nodes() {
    let mut document = CadDocument::new();
    let handle = document.allocate_handle();
    let owner = document.header.named_objects_dict_handle;
    document.objects.insert(
        handle,
        ObjectType::Associative(AssociativeObject {
            handle,
            owner,
            dxf_name: "ASSOC2DCONSTRAINTGROUP".to_string(),
            cpp_class_name: "AcDbAssoc2dConstraintGroup".to_string(),
            data: AssociativeData::ConstraintGroup(Assoc2dConstraintGroup {
                action: AssocAction {
                    class_version: 2,
                    ..Default::default()
                },
                version: 2,
                nodes: vec![
                    AssocConstraintNode::default(),
                    AssocConstraintNode {
                        node_id: 1,
                        class_name: "AcHorizontalConstraint".to_string(),
                        data: AssocConstraintNodeData::Geometrical {
                            owner_id: 0,
                            is_implied: false,
                            is_active: true,
                        },
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }),
            ..Default::default()
        }),
    );

    let bytes = DwgWriter::write_to_vec(&document).expect("write DWG");
    let decoded = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read DWG");
    let group = decoded
        .objects
        .values()
        .find_map(|object| match object {
            ObjectType::Associative(AssociativeObject {
                data: AssociativeData::ConstraintGroup(group),
                ..
            }) => Some(group),
            _ => None,
        })
        .expect("constraint group should round-trip");

    assert_eq!(group.nodes.len(), 2);
    assert_eq!(group.nodes[0].node_id, 0);
    assert_eq!(group.nodes[1].node_id, 1);
    assert_eq!(group.nodes[1].class_name, "AcHorizontalConstraint");
}
