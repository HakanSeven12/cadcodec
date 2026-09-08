use acadrust::entities::{solid3d::Solid3D, EntityType};
use acadrust::objects::{
    SolidHistoryBox, SolidHistoryFillet, SolidHistoryNodeBase, SolidHistoryOperation,
};
use acadrust::CadDocument;

fn box_step(step_id: i32) -> SolidHistoryOperation {
    SolidHistoryOperation::Box(SolidHistoryBox {
        base: SolidHistoryNodeBase::new(step_id),
        length: 2.0,
        width: 3.0,
        height: 4.0,
        ..SolidHistoryBox::default()
    })
}

fn fillet_step() -> SolidHistoryOperation {
    SolidHistoryOperation::Fillet(SolidHistoryFillet {
        base: SolidHistoryNodeBase::new(0),
        radii: vec![0.25],
        ..SolidHistoryFillet::default()
    })
}

#[test]
fn appended_history_is_returned_root_to_active() {
    let mut document = CadDocument::new();
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document.create_solid_history(entity, box_step(1)).unwrap();
    document.append_solid_history(entity, fillet_step()).unwrap();

    let operations = document.solid_history_operations(entity).unwrap();
    assert_eq!(operations.len(), 2);
    assert!(matches!(operations[0], SolidHistoryOperation::Box(_)));
    assert!(matches!(operations[1], SolidHistoryOperation::Fillet(_)));
    assert_eq!(operations[0].base().unwrap().eval.parent_id, 0);
    assert_eq!(operations[1].base().unwrap().eval.parent_id, 1);
}

#[test]
fn updating_a_step_preserves_its_graph_identity() {
    let mut document = CadDocument::new();
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document.create_solid_history(entity, box_step(1)).unwrap();
    document.append_solid_history(entity, fillet_step()).unwrap();

    let mut replacement = document.solid_history_operations(entity).unwrap()[0].clone();
    let base = replacement.base_mut().unwrap();
    base.eval.parent_id = 99;
    if let SolidHistoryOperation::Box(value) = &mut replacement {
        value.length = 8.0;
    }
    document
        .update_solid_history_step(entity, replacement)
        .unwrap();

    let operations = document.solid_history_operations(entity).unwrap();
    assert_eq!(operations[0].base().unwrap().eval.parent_id, 0);
    assert_eq!(operations[1].base().unwrap().eval.parent_id, 1);
    assert!(matches!(
        &operations[0],
        SolidHistoryOperation::Box(value) if value.length == 8.0
    ));
}
