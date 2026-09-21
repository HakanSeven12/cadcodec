//! DXF write -> read round-trip for text styles and dimension styles: each test
//! builds a style with non-default values, round-trips it through the DXF writer
//! and reader, and checks what the writer emits comes back.

use std::io::Cursor;

use acadrust::tables::TextStyle;
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DxfReader, DxfWriter};

fn dxf_roundtrip(doc: &CadDocument) -> CadDocument {
    let bytes = DxfWriter::new(doc).write_to_vec().expect("DXF write failed");
    DxfReader::from_reader(Cursor::new(bytes))
        .expect("DXF reader init failed")
        .read()
        .expect("DXF read failed")
}

#[test]
fn text_style_generation_flags_survive() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut style = TextStyle::new("Mirrored");
    style.flags.backward = true;
    style.flags.upside_down = true;
    doc.text_styles.add(style).unwrap();
    let rt = dxf_roundtrip(&doc);
    let style = rt.text_styles.get("Mirrored").expect("style missing");
    assert!(style.flags.backward, "backward");
    assert!(style.flags.upside_down, "upside down");

    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut style = TextStyle::new("OnlyBackward");
    style.flags.backward = true;
    doc.text_styles.add(style).unwrap();
    let style = dxf_roundtrip(&doc).text_styles.get("OnlyBackward").cloned().unwrap();
    assert!(style.flags.backward && !style.flags.upside_down);
}
