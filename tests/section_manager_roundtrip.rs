use acadrust::objects::{ClassObject, ClassObjectData, ObjectType, SectionManager};
use acadrust::{CadDocument, DxfReader, DxfWriter, Handle};
use std::io::Cursor;

#[test]
fn dxf_preserves_section_manager_handles() {
    let mut document = CadDocument::new();
    let handle = document.allocate_handle();
    let section = Handle::new(0x1234);
    let mut object = ClassObject::new(ClassObjectData::SectionManager(SectionManager {
        is_live: true,
        sections: vec![section],
    }));
    object.handle = handle;
    object.owner = document.header.named_objects_dict_handle;
    document
        .objects
        .insert(handle, ObjectType::ClassObject(object));

    let bytes = DxfWriter::new(&document).write_to_vec().expect("write DXF");
    let roundtripped = DxfReader::from_reader(Cursor::new(bytes))
        .expect("create DXF reader")
        .read()
        .expect("read DXF");
    let manager = roundtripped
        .objects
        .values()
        .find_map(|object| match object {
            ObjectType::ClassObject(object) => match &object.data {
                ClassObjectData::SectionManager(manager) => Some(manager),
                _ => None,
            },
            _ => None,
        })
        .expect("section manager should round-trip");

    assert!(manager.is_live);
    assert_eq!(manager.sections, vec![section]);
}
