use std::io::Cursor;

use acadrust::tables::Layer;
use acadrust::types::{Color, DxfVersion};
use acadrust::{CadDocument, DwgReader, DwgWriter};

#[test]
fn layer_color_book_identity_survives_dwg_roundtrip() {
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let mut layer = Layer::with_color("Named color", Color::from_rgb(12, 34, 56));
    layer.color_name = Some("Accent".to_string());
    layer.book_name = Some("Brand colors".to_string());
    document.layers.add(layer).unwrap();

    let bytes = DwgWriter::write_to_vec(&document).expect("DWG write");
    let roundtripped = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("DWG read");
    let layer = roundtripped.layers.get("Named color").expect("layer");

    assert_eq!(layer.color, Color::from_rgb(12, 34, 56));
    assert_eq!(layer.color_name.as_deref(), Some("Accent"));
    assert_eq!(layer.book_name.as_deref(), Some("Brand colors"));
}
