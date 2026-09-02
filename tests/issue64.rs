//! Repro for issue #64: a DXF round-trip writes a ByBlock linetype handle
//! as the DIMSTYLE text style (group 340).

use acadrust::entities::EntityType;
use acadrust::tables::{};
use acadrust::types::{DxfVersion, Handle};
use acadrust::{CadDocument, DxfReader, DxfWriter};

#[test]
fn repro_issue64() {
    // Input mirroring the reporter's file: a Standard text style at #11,
    // a ByBlock linetype at #14 (which collides with the handle that
    // acadrust's DEFAULT Standard dimstyle uses for its text style), and
    // no Standard dimstyle of its own (the default one survives).
    let mut doc = CadDocument::with_version(DxfVersion::AC1024);

    // The file's own records replace the defaults at these handles.
    doc.text_styles.get_mut("Standard").unwrap().handle = Handle::new(0x11);
    doc.line_types.get_mut("ByBlock").unwrap().handle = Handle::new(0x14);
    // No Standard DIMSTYLE in the input: the default one survives, still
    // pointing its dimtxsty at the DEFAULT text style handle (0x14) - which
    // the input has now given to the ByBlock linetype.
    doc.add_entity(EntityType::Line(acadrust::entities::Line::from_coords(
        0.0, 0.0, 0.0, 1.0, 1.0, 0.0,
    )))
    .unwrap();

    let input = std::env::temp_dir().join("issue64_input.dxf");
    DxfWriter::new(&doc).write_to_file(&input).unwrap();

    // The round-trip from the issue
    let loaded = DxfReader::from_file(&input).unwrap().read().unwrap();
    let output = std::env::temp_dir().join("issue64_output.dxf");
    DxfWriter::new(&loaded).write_to_file(&output).unwrap();

    let ts_handle = loaded.text_styles.get("Standard").unwrap().handle;
    let ltype_handle = loaded.line_types.get("ByBlock").unwrap().handle;
    let dimtxsty = loaded
        .dim_styles
        .get("Standard")
        .unwrap()
        .dimtxsty_handle;
    println!("text style Standard = {ts_handle:?}");
    println!("ByBlock LTYPE       = {ltype_handle:?}");
    println!("DIMSTYLE dimtxsty   = {dimtxsty:?}");
    assert_ne!(
        dimtxsty, ltype_handle,
        "DIMSTYLE text style points at the ByBlock linetype"
    );

    // The output record must reference the text style, not the linetype.
    let text = std::fs::read_to_string(&output).unwrap();
    let lines: Vec<&str> = text.split("\r\n").collect();
    let mut dimstyle_340 = String::new();
    for i in 0..lines.len() - 3 {
        if lines[i] == "DIMSTYLE" && lines[i - 1] == "  0" {
            let mut j = i;
            while j < lines.len() - 1 {
                if lines[j].trim() == "340" {
                    dimstyle_340 = lines[j + 1].trim().to_string();
                    break;
                }
                if lines[j] == "  0" && j > i {
                    break;
                }
                j += 1;
            }
            break;
        }
    }
    println!("output DIMSTYLE 340 -> {dimstyle_340}");
    assert_ne!(
        dimstyle_340,
        format!("{:X}", ltype_handle.value()),
        "DIMSTYLE 340 points at the ByBlock linetype"
    );
}
