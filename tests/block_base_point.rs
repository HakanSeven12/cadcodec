use acadrust::tables::TableEntry;
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter};
use std::io::Cursor;

#[test]
fn writes_block_record_base_point_without_a_block_marker() {
    for version in [
        DxfVersion::AC1015,
        DxfVersion::AC1018,
        DxfVersion::AC1021,
        DxfVersion::AC1024,
        DxfVersion::AC1027,
        DxfVersion::AC1032,
    ] {
        let mut document = CadDocument::with_version(version);
        let mut record = acadrust::tables::BlockRecord::new("Desk");
        record.set_handle(document.allocate_handle());
        record.block_end_handle = document.allocate_handle();
        record.base_point = Vector3::new(50.0, 25.0, 0.0);
        document.block_records.add(record).unwrap();

        let bytes = DwgWriter::write_to_vec(&document).unwrap();
        let mut reader = DwgReader::from_stream(Cursor::new(bytes));
        let roundtripped = reader.read().unwrap();
        assert_eq!(
            roundtripped.block_records.get("Desk").unwrap().base_point,
            Vector3::new(50.0, 25.0, 0.0),
            "failed for {version:?}"
        );
    }
}
