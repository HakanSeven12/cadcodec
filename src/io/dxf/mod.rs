//! DXF (Drawing Exchange Format) reading and writing.
//!
//! Supports both **ASCII** and **Binary** DXF for versions R12 (AC1009)
//! through R2018+ (AC1032).
//!
//! # Reading
//!
//! ```rust,ignore
//! use acadrust::DxfReader;
//!
//! let doc = DxfReader::from_file("drawing.dxf")?.read()?;
//! ```
//!
//! # Writing
//!
//! ```rust,ignore
//! use acadrust::DxfWriter;
//!
//! DxfWriter::new(&doc).write_to_file("output.dxf")?;
//! ```

mod dxf_code;
mod group_code_value;
mod reader;
mod writer;
pub mod code_page;

pub use dxf_code::DxfCode;
pub use group_code_value::GroupCodeValueType;
pub use reader::{DxfReader, DxfReaderConfiguration};
pub use writer::{DxfWriter, DxfStreamWriter, DxfStreamWriterExt, DxfTextWriter, DxfBinaryWriter, SectionWriter};
pub use writer::{write_dxf, write_binary_dxf, value_type_for_code};

pub(crate) fn split_color_book_name(value: &str) -> (Option<String>, Option<String>) {
    let optional = |value: &str| (!value.is_empty()).then(|| value.to_string());
    match value.split_once('$') {
        Some((book_name, color_name)) => (optional(book_name), optional(color_name)),
        None => (None, optional(value)),
    }
}

pub(crate) fn join_color_book_name(
    book_name: Option<&str>,
    color_name: Option<&str>,
) -> Option<String> {
    let book_name = book_name.filter(|value| !value.is_empty());
    let color_name = color_name.filter(|value| !value.is_empty());
    match (book_name, color_name) {
        (Some(book_name), Some(color_name)) => Some(format!("{book_name}${color_name}")),
        (Some(book_name), None) => Some(format!("{book_name}$")),
        (None, Some(color_name)) => Some(color_name.to_string()),
        (None, None) => None,
    }
}

