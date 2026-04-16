//! Non-graphical object readers for DWG object section.
//!
//! Each reader is the exact inverse of the corresponding writer in
//! `dwg_stream_writers/object_writer/objects.rs`. They read object-specific
//! fields after common non-entity data has already been parsed.

use crate::io::dwg::dwg_stream_readers::merged_reader::DwgMergedReader;
use crate::io::dwg::dwg_version::DwgVersion;
use crate::types::{Color, DxfVersion, Vector2, Vector3};
use super::safe_count;

// ════════════════════════════════════════════════════════════════════════
//  Result structs
// ════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DictionaryEntry {
    pub name: String,
    pub handle: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DictionaryData {
    pub duplicate_cloning: i16,
    pub hard_owner: bool,
    pub entries: Vec<DictionaryEntry>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DictionaryWithDefaultData {
    pub duplicate_cloning: i16,
    pub hard_owner: bool,
    pub entries: Vec<DictionaryEntry>,
    pub default_handle: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DictionaryVariableData {
    pub schema_number: u8,
    pub value: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlotSettingsData {
    pub page_name: String,
    pub printer_name: String,
    pub plot_flags: i16,
    pub left_margin: f64,
    pub bottom_margin: f64,
    pub right_margin: f64,
    pub top_margin: f64,
    pub paper_width: f64,
    pub paper_height: f64,
    pub paper_size: String,
    pub origin_x: f64,
    pub origin_y: f64,
    pub paper_units: i16,
    pub rotation: i16,
    pub plot_type: i16,
    pub window_min_x: f64,
    pub window_min_y: f64,
    pub window_max_x: f64,
    pub window_max_y: f64,
    pub scale_numerator: f64,
    pub scale_denominator: f64,
    pub current_style_sheet: String,
    pub scale_type: i16,
    pub scale_factor: f64,
    pub paper_image_x: f64,
    pub paper_image_y: f64,
    pub plot_view_name: String,
    pub shade_plot_mode: i16,
    pub shade_plot_resolution: i16,
    pub shade_plot_dpi: i16,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LayoutData {
    pub plot_settings: PlotSettingsData,
    pub name: String,
    pub tab_order: i32,
    pub flags: i16,
    pub ucs_origin: Vector3,
    pub min_limits: (f64, f64),
    pub max_limits: (f64, f64),
    pub insertion_base: Vector3,
    pub x_axis: Vector3,
    pub y_axis: Vector3,
    pub elevation: f64,
    pub ucs_ortho_type: i16,
    pub min_extents: Vector3,
    pub max_extents: Vector3,
    pub viewport_count: i32,
    pub block_record_handle: u64,
    pub viewport_handle: u64,
    pub base_ucs_handle: u64,
    pub named_ucs_handle: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GroupData {
    pub description: String,
    pub unnamed: i16,
    pub selectable: bool,
    pub entity_handles: Vec<u64>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MLineStyleElementData {
    pub offset: f64,
    pub color: Color,
    pub linetype_index_or_handle: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MLineStyleData {
    pub name: String,
    pub description: String,
    pub flags: i16,
    pub fill_color: Color,
    pub start_angle: f64,
    pub end_angle: f64,
    pub elements: Vec<MLineStyleElementData>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MultiLeaderStyleData {
    pub content_type: i16,
    pub multileader_draw_order: i16,
    pub leader_draw_order: i16,
    pub max_leader_points: i32,
    pub first_segment_angle: f64,
    pub second_segment_angle: f64,
    pub path_type: i16,
    pub line_color: Color,
    pub line_type_handle: u64,
    pub line_weight: i32,
    pub enable_landing: bool,
    pub landing_gap: f64,
    pub enable_dogleg: bool,
    pub landing_distance: f64,
    pub description: String,
    pub arrowhead_handle: u64,
    pub arrowhead_size: f64,
    pub default_text: String,
    pub text_style_handle: u64,
    pub text_left_attachment: i16,
    pub text_right_attachment: i16,
    pub text_angle_type: i16,
    pub text_alignment: i16,
    pub text_color: Color,
    pub text_height: f64,
    pub text_frame: bool,
    pub text_always_left: bool,
    pub align_space: f64,
    pub block_content_handle: u64,
    pub block_content_color: Color,
    pub block_content_scale_x: f64,
    pub block_content_scale_y: f64,
    pub block_content_scale_z: f64,
    pub enable_block_scale: bool,
    pub block_content_rotation: f64,
    pub enable_block_rotation: bool,
    pub block_content_connection: i16,
    pub scale_factor: f64,
    pub property_changed: bool,
    pub is_annotative: bool,
    pub break_gap_size: f64,
    pub text_attachment_direction: i16,
    pub text_top_attachment: i16,
    pub text_bottom_attachment: i16,
    pub unknown_flag_298: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImageDefinitionData {
    pub class_version: i32,
    pub size_in_pixels: Vector2,
    pub file_name: String,
    pub is_loaded: bool,
    pub resolution_unit: u8,
    pub pixel_size: Vector2,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImageDefinitionReactorData {
    pub class_version: i32,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScaleData {
    pub unknown_bs: i16,
    pub name: String,
    pub paper_units: f64,
    pub drawing_units: f64,
    pub is_unit_scale: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SortEntitiesEntry {
    pub sort_handle: u64,
    pub entity_handle: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SortEntitiesTableData {
    pub entries: Vec<SortEntitiesEntry>,
    pub block_owner_handle: u64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct XRecordData {
    pub cloning_flags: i16,
    pub data_size: i32,
    pub raw_data: Vec<u8>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RasterVariablesData {
    pub class_version: i32,
    pub display_image_frame: i16,
    pub image_quality: i16,
    pub units: i16,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BookColorData {
    pub color_name: String,
    pub book_name: String,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WipeoutVariablesData {
    pub display_frame: i16,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableCellBorderData {
    pub property_flags: i32,
    pub border_type: i16,
    pub line_weight: i32,
    pub color: Color,
    pub is_invisible: bool,
    pub double_line_spacing: f64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RowCellStyleData {
    pub text_style_name: String,
    pub text_style_handle: u64,
    pub text_height: f64,
    pub alignment: i16,
    pub text_color: Color,
    pub fill_enabled: bool,
    pub fill_color: Color,
    pub data_type: i32,
    pub unit_type: i32,
    pub format_string: String,
    pub left_border: TableCellBorderData,
    pub right_border: TableCellBorderData,
    pub top_border: TableCellBorderData,
    pub bottom_border: TableCellBorderData,
    pub horizontal_inside_border: TableCellBorderData,
    pub vertical_inside_border: TableCellBorderData,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TableStyleData {
    pub version: i16,
    pub name: String,
    pub description: String,
    pub flow_direction: i16,
    pub flags: i16,
    pub horizontal_margin: f64,
    pub vertical_margin: f64,
    pub title_suppressed: bool,
    pub header_suppressed: bool,
    pub data_row_style: RowCellStyleData,
    pub header_row_style: RowCellStyleData,
    pub title_row_style: RowCellStyleData,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VisualStyleData {
    pub description: String,
    pub style_type: i16,
    pub face_lighting_model: i16,
    pub face_lighting_quality: i16,
    pub face_color_mode: i16,
    pub face_modifier: i32,
    pub edge_model: i32,
    pub edge_style: i32,
    pub edge_color_mode: i16,
    pub face_opacity: i16,
    pub edge_opacity: i16,
    pub face_tint_color: i32,
    pub internal_use_only: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MaterialData {
    pub name: String,
    pub description: String,
    pub ambient_color: Color,
    pub diffuse_color: Color,
    pub roughness: f64,
    pub opacity: f64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GeoDataData {
    pub version: i32,
    pub coordinate_type: i16,
    pub design_point: Vector3,
    pub reference_point: Vector3,
    pub north_direction: Vector2,
    pub horizontal_unit_scale: f64,
    pub vertical_unit_scale: f64,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpatialFilterData {
    pub is_enabled: bool,
    pub is_front_clipping_on: bool,
    pub front_clipping_distance: f64,
    pub is_back_clipping_on: bool,
    pub back_clipping_distance: f64,
    pub clip_boundary_points: Vec<Vector2>,
}

// ════════════════════════════════════════════════════════════════════════
//  Reader functions
// ════════════════════════════════════════════════════════════════════════

pub fn read_dictionary(reader: &mut DwgMergedReader, version: DwgVersion) -> DictionaryData {
    let num_entries = safe_count(reader.read_bit_long());

    let mut duplicate_cloning = 0i16;
    let mut hard_owner = false;
    if version.r2000_plus() {
        duplicate_cloning = reader.read_bit_short();
        hard_owner = reader.read_byte() != 0;
    }

    let mut entries = Vec::with_capacity(num_entries as usize);
    for _ in 0..num_entries {
        let name = reader.read_variable_text();
        let handle = reader.read_handle();
        entries.push(DictionaryEntry { name, handle });
    }

    DictionaryData { duplicate_cloning, hard_owner, entries }
}

pub fn read_dictionary_with_default(reader: &mut DwgMergedReader, version: DwgVersion) -> DictionaryWithDefaultData {
    let num_entries = safe_count(reader.read_bit_long());

    let mut duplicate_cloning = 0i16;
    let mut hard_owner = false;
    if version.r2000_plus() {
        duplicate_cloning = reader.read_bit_short();
        hard_owner = reader.read_byte() != 0;
    }

    let mut entries = Vec::with_capacity(num_entries as usize);
    for _ in 0..num_entries {
        let name = reader.read_variable_text();
        let handle = reader.read_handle();
        entries.push(DictionaryEntry { name, handle });
    }

    let default_handle = reader.read_handle();

    DictionaryWithDefaultData { duplicate_cloning, hard_owner, entries, default_handle }
}

pub fn read_dictionary_variable(reader: &mut DwgMergedReader) -> DictionaryVariableData {
    let schema_number = reader.read_byte();
    let value = reader.read_variable_text();
    DictionaryVariableData { schema_number, value }
}

/// Read the PlotSettings data portion (shared by Layout and standalone PlotSettings).
pub fn read_plot_settings_data(reader: &mut DwgMergedReader, version: DwgVersion) -> PlotSettingsData {
    let page_name = reader.read_variable_text();
    let printer_name = reader.read_variable_text();
    let plot_flags = reader.read_bit_short();

    let left_margin = reader.read_bit_double();
    let bottom_margin = reader.read_bit_double();
    let right_margin = reader.read_bit_double();
    let top_margin = reader.read_bit_double();

    let paper_width = reader.read_bit_double();
    let paper_height = reader.read_bit_double();

    let paper_size = reader.read_variable_text();

    let origin_x = reader.read_bit_double();
    let origin_y = reader.read_bit_double();

    let paper_units = reader.read_bit_short();
    let rotation = reader.read_bit_short();
    let plot_type = reader.read_bit_short();

    let window_min_x = reader.read_bit_double();
    let window_min_y = reader.read_bit_double();
    let window_max_x = reader.read_bit_double();
    let window_max_y = reader.read_bit_double();

    let plot_view_name = if version.r13_15_only() {
        reader.read_variable_text()
    } else {
        String::new()
    };

    let scale_numerator = reader.read_bit_double();
    let scale_denominator = reader.read_bit_double();
    let current_style_sheet = reader.read_variable_text();
    let scale_type = reader.read_bit_short();
    let scale_factor = reader.read_bit_double();
    let paper_image_x = reader.read_bit_double();
    let paper_image_y = reader.read_bit_double();

    let mut shade_plot_mode = 0i16;
    let mut shade_plot_resolution = 0i16;
    let mut shade_plot_dpi = 0i16;
    if version.r2004_plus() {
        shade_plot_mode = reader.read_bit_short();
        shade_plot_resolution = reader.read_bit_short();
        shade_plot_dpi = reader.read_bit_short();
        let _plot_view_handle = reader.read_handle();
    }
    if version.r2007_plus() {
        let _visual_style_handle = reader.read_handle();
    }

    PlotSettingsData {
        page_name, printer_name, plot_flags,
        left_margin, bottom_margin, right_margin, top_margin,
        paper_width, paper_height, paper_size,
        origin_x, origin_y,
        paper_units, rotation, plot_type,
        window_min_x, window_min_y, window_max_x, window_max_y,
        scale_numerator, scale_denominator, current_style_sheet,
        scale_type, scale_factor, paper_image_x, paper_image_y,
        plot_view_name, shade_plot_mode, shade_plot_resolution, shade_plot_dpi,
    }
}

pub fn read_layout(reader: &mut DwgMergedReader, version: DwgVersion) -> LayoutData {
    let plot_settings = read_plot_settings_data(reader, version);

    let name = reader.read_variable_text();
    let tab_order = reader.read_bit_long();
    let flags = reader.read_bit_short();
    let ucs_origin = reader.read_3bit_double();

    let min_lim_x = reader.read_raw_double();
    let min_lim_y = reader.read_raw_double();
    let max_lim_x = reader.read_raw_double();
    let max_lim_y = reader.read_raw_double();

    let insertion_base = reader.read_3bit_double();
    let x_axis = reader.read_3bit_double();
    let y_axis = reader.read_3bit_double();
    let elevation = reader.read_bit_double();
    let ucs_ortho_type = reader.read_bit_short();
    let min_extents = reader.read_3bit_double();
    let max_extents = reader.read_3bit_double();

    let viewport_count = if version.r2004_plus() { safe_count(reader.read_bit_long()) } else { 0 };

    let block_record_handle = reader.read_handle();
    let viewport_handle = reader.read_handle();
    let base_ucs_handle = reader.read_handle();
    let named_ucs_handle = reader.read_handle();

    // R2004+: viewport handles
    if version.r2004_plus() {
        for _ in 0..viewport_count {
            let _vp_handle = reader.read_handle();
        }
    }

    LayoutData {
        plot_settings, name, tab_order, flags, ucs_origin,
        min_limits: (min_lim_x, min_lim_y),
        max_limits: (max_lim_x, max_lim_y),
        insertion_base, x_axis, y_axis, elevation, ucs_ortho_type,
        min_extents, max_extents, viewport_count,
        block_record_handle, viewport_handle, base_ucs_handle, named_ucs_handle,
    }
}

pub fn read_plot_settings_obj(reader: &mut DwgMergedReader, version: DwgVersion) -> PlotSettingsData {
    read_plot_settings_data(reader, version)
}

pub fn read_group(reader: &mut DwgMergedReader) -> GroupData {
    let description = reader.read_variable_text();
    let unnamed = reader.read_bit_short();
    let selectable = reader.read_bit_short() != 0;

    let num_entities = safe_count(reader.read_bit_long());
    let mut entity_handles = Vec::with_capacity(num_entities as usize);
    for _ in 0..num_entities {
        entity_handles.push(reader.read_handle());
    }

    GroupData { description, unnamed, selectable, entity_handles }
}

pub fn read_mlinestyle(reader: &mut DwgMergedReader, version: DwgVersion, dxf_version: DxfVersion) -> MLineStyleData {
    let name = reader.read_variable_text();
    let description = reader.read_variable_text();
    let flags = reader.read_bit_short();
    let fill_color = reader.read_cm_color();
    let start_angle = reader.read_bit_double();
    let end_angle = reader.read_bit_double();

    let num_elements = reader.read_byte();
    let mut elements = Vec::with_capacity(num_elements as usize);
    for _ in 0..num_elements {
        let offset = reader.read_bit_double();
        let color = reader.read_cm_color();
        let linetype_index_or_handle = if version.r2018_plus(dxf_version) {
            reader.read_handle()
        } else {
            reader.read_bit_short() as u64
        };
        elements.push(MLineStyleElementData { offset, color, linetype_index_or_handle });
    }

    MLineStyleData { name, description, flags, fill_color, start_angle, end_angle, elements }
}

pub fn read_multileader_style(reader: &mut DwgMergedReader, version: DwgVersion, dxf_version: crate::types::DxfVersion) -> MultiLeaderStyleData {
    // R2010+: Version (BS, expected 2)
    if version.r2010_plus() {
        let _style_version = reader.read_bit_short();
    }

    let content_type = reader.read_bit_short();
    let multileader_draw_order = reader.read_bit_short();
    let leader_draw_order = reader.read_bit_short();
    let max_leader_points = reader.read_bit_long();
    let first_segment_angle = reader.read_bit_double();
    let second_segment_angle = reader.read_bit_double();
    let path_type = reader.read_bit_short();
    let line_color = reader.read_cm_color();
    let line_type_handle = reader.read_handle();
    let line_weight = reader.read_bit_long();
    let enable_landing = reader.read_bit();
    let landing_gap = reader.read_bit_double();
    let enable_dogleg = reader.read_bit();
    let landing_distance = reader.read_bit_double();
    let description = reader.read_variable_text();
    let arrowhead_handle = reader.read_handle();
    let arrowhead_size = reader.read_bit_double();
    let default_text = reader.read_variable_text();
    let text_style_handle = reader.read_handle();
    let text_left_attachment = reader.read_bit_short();
    let text_right_attachment = reader.read_bit_short();
    let text_angle_type = reader.read_bit_short();
    let text_alignment = reader.read_bit_short();
    let text_color = reader.read_cm_color();
    let text_height = reader.read_bit_double();
    let text_frame = reader.read_bit();
    let text_always_left = reader.read_bit();
    let align_space = reader.read_bit_double();
    let block_content_handle = reader.read_handle();
    let block_content_color = reader.read_cm_color();
    let block_content_scale_x = reader.read_bit_double();
    let block_content_scale_y = reader.read_bit_double();
    let block_content_scale_z = reader.read_bit_double();
    let enable_block_scale = reader.read_bit();
    let block_content_rotation = reader.read_bit_double();
    let enable_block_rotation = reader.read_bit();
    let block_content_connection = reader.read_bit_short();
    let scale_factor = reader.read_bit_double();
    let property_changed = reader.read_bit();
    let is_annotative = reader.read_bit();
    let break_gap_size = reader.read_bit_double();

    let mut text_attachment_direction = 0i16;
    let mut text_top_attachment = 0i16;
    let mut text_bottom_attachment = 0i16;
    if version.r2010_plus() {
        text_attachment_direction = reader.read_bit_short();
        text_top_attachment = reader.read_bit_short();
        text_bottom_attachment = reader.read_bit_short();
    }

    let mut unknown_flag_298 = false;
    if dxf_version >= crate::types::DxfVersion::AC1027 {
        unknown_flag_298 = reader.read_bit();
    }

    MultiLeaderStyleData {
        content_type, multileader_draw_order, leader_draw_order,
        max_leader_points, first_segment_angle, second_segment_angle,
        path_type, line_color, line_type_handle, line_weight,
        enable_landing, landing_gap, enable_dogleg, landing_distance,
        description, arrowhead_handle, arrowhead_size,
        default_text, text_style_handle,
        text_left_attachment, text_right_attachment, text_angle_type, text_alignment,
        text_color, text_height, text_frame, text_always_left, align_space,
        block_content_handle, block_content_color,
        block_content_scale_x, block_content_scale_y, block_content_scale_z,
        enable_block_scale, block_content_rotation, enable_block_rotation,
        block_content_connection, scale_factor, property_changed, is_annotative,
        break_gap_size, text_attachment_direction, text_top_attachment, text_bottom_attachment,
        unknown_flag_298,
    }
}

pub fn read_image_definition(reader: &mut DwgMergedReader) -> ImageDefinitionData {
    let class_version = reader.read_bit_long();
    let size_in_pixels = reader.read_2raw_double();
    let file_name = reader.read_variable_text();
    let is_loaded = reader.read_bit();
    let resolution_unit = reader.read_byte();
    let pixel_size = reader.read_2raw_double();

    ImageDefinitionData { class_version, size_in_pixels, file_name, is_loaded, resolution_unit, pixel_size }
}

pub fn read_image_definition_reactor(reader: &mut DwgMergedReader) -> ImageDefinitionReactorData {
    let class_version = reader.read_bit_long();
    ImageDefinitionReactorData { class_version }
}

pub fn read_scale(reader: &mut DwgMergedReader) -> ScaleData {
    let unknown_bs = reader.read_bit_short();
    let name = reader.read_variable_text();
    let paper_units = reader.read_bit_double();
    let drawing_units = reader.read_bit_double();
    let is_unit_scale = reader.read_bit();
    ScaleData { unknown_bs, name, paper_units, drawing_units, is_unit_scale }
}

pub fn read_sort_entities_table(reader: &mut DwgMergedReader) -> SortEntitiesTableData {
    let num_entries = safe_count(reader.read_bit_long());
    let mut entries = Vec::with_capacity(num_entries as usize);
    for _ in 0..num_entries {
        let sort_handle = reader.read_handle();
        let entity_handle = reader.read_handle();
        entries.push(SortEntitiesEntry { sort_handle, entity_handle });
    }
    let block_owner_handle = reader.read_handle();
    SortEntitiesTableData { entries, block_owner_handle }
}

pub fn read_xrecord(reader: &mut DwgMergedReader) -> XRecordData {
    let data_size = safe_count(reader.read_bit_long());
    let mut raw_data = Vec::with_capacity(data_size as usize);
    for _ in 0..data_size {
        raw_data.push(reader.read_byte());
    }
    let cloning_flags = reader.read_bit_short();
    XRecordData { cloning_flags, data_size, raw_data }
}

pub fn read_raster_variables(reader: &mut DwgMergedReader) -> RasterVariablesData {
    let class_version = reader.read_bit_long();
    let display_image_frame = reader.read_bit_short();
    let image_quality = reader.read_bit_short();
    let units = reader.read_bit_short();
    RasterVariablesData { class_version, display_image_frame, image_quality, units }
}

pub fn read_placeholder(_reader: &mut DwgMergedReader) {
    // PlaceHolder has no object-specific data
}

pub fn read_book_color(reader: &mut DwgMergedReader) -> BookColorData {
    let color_name = reader.read_variable_text();
    let book_name = reader.read_variable_text();
    BookColorData { color_name, book_name }
}

pub fn read_wipeout_variables(reader: &mut DwgMergedReader) -> WipeoutVariablesData {
    let display_frame = reader.read_bit_short();
    WipeoutVariablesData { display_frame }
}

fn read_table_cell_border(reader: &mut DwgMergedReader) -> TableCellBorderData {
    TableCellBorderData {
        property_flags: reader.read_bit_long(),
        border_type: reader.read_bit_short(),
        line_weight: reader.read_bit_long(),
        color: reader.read_cm_color(),
        is_invisible: reader.read_bit(),
        double_line_spacing: reader.read_bit_double(),
    }
}

fn read_row_cell_style(reader: &mut DwgMergedReader) -> RowCellStyleData {
    let text_style_name = reader.read_variable_text();
    let text_style_handle = reader.read_handle();
    let text_height = reader.read_bit_double();
    let alignment = reader.read_bit_short();
    let text_color = reader.read_cm_color();
    let fill_enabled = reader.read_bit();
    let fill_color = reader.read_cm_color();
    let data_type = reader.read_bit_long();
    let unit_type = reader.read_bit_long();
    let format_string = reader.read_variable_text();

    let left_border = read_table_cell_border(reader);
    let right_border = read_table_cell_border(reader);
    let top_border = read_table_cell_border(reader);
    let bottom_border = read_table_cell_border(reader);
    let horizontal_inside_border = read_table_cell_border(reader);
    let vertical_inside_border = read_table_cell_border(reader);

    RowCellStyleData {
        text_style_name,
        text_style_handle,
        text_height,
        alignment,
        text_color,
        fill_enabled,
        fill_color,
        data_type,
        unit_type,
        format_string,
        left_border,
        right_border,
        top_border,
        bottom_border,
        horizontal_inside_border,
        vertical_inside_border,
    }
}

pub fn read_table_style(reader: &mut DwgMergedReader) -> TableStyleData {
    let version = reader.read_byte() as i16;
    let name = reader.read_variable_text();
    let description = reader.read_variable_text();
    let flow_direction = reader.read_bit_short();
    let flags = reader.read_bit_short();
    let horizontal_margin = reader.read_bit_double();
    let vertical_margin = reader.read_bit_double();
    let title_suppressed = reader.read_bit();
    let header_suppressed = reader.read_bit();

    let data_row_style = read_row_cell_style(reader);
    let header_row_style = read_row_cell_style(reader);
    let title_row_style = read_row_cell_style(reader);

    TableStyleData {
        version,
        name,
        description,
        flow_direction,
        flags,
        horizontal_margin,
        vertical_margin,
        title_suppressed,
        header_suppressed,
        data_row_style,
        header_row_style,
        title_row_style,
    }
}

pub fn read_visual_style(reader: &mut DwgMergedReader) -> VisualStyleData {
    VisualStyleData {
        description: reader.read_variable_text(),
        style_type: reader.read_bit_short(),
        face_lighting_model: reader.read_bit_short(),
        face_lighting_quality: reader.read_bit_short(),
        face_color_mode: reader.read_bit_short(),
        face_modifier: reader.read_bit_long(),
        edge_model: reader.read_bit_long(),
        edge_style: reader.read_bit_long(),
        edge_color_mode: reader.read_bit_short(),
        face_opacity: reader.read_bit_short(),
        edge_opacity: reader.read_bit_short(),
        face_tint_color: reader.read_bit_long(),
        internal_use_only: reader.read_bit(),
    }
}

pub fn read_material(reader: &mut DwgMergedReader) -> MaterialData {
    MaterialData {
        name: reader.read_variable_text(),
        description: reader.read_variable_text(),
        ambient_color: reader.read_cm_color(),
        diffuse_color: reader.read_cm_color(),
        roughness: reader.read_bit_double(),
        opacity: reader.read_bit_double(),
    }
}

pub fn read_geodata(reader: &mut DwgMergedReader) -> GeoDataData {
    GeoDataData {
        version: reader.read_bit_long(),
        coordinate_type: reader.read_bit_short(),
        design_point: reader.read_3bit_double(),
        reference_point: reader.read_3bit_double(),
        north_direction: reader.read_2bit_double(),
        horizontal_unit_scale: reader.read_bit_double(),
        vertical_unit_scale: reader.read_bit_double(),
    }
}

pub fn read_spatial_filter(reader: &mut DwgMergedReader) -> SpatialFilterData {
    let is_enabled = reader.read_bit();
    let is_front_clipping_on = reader.read_bit();
    let front_clipping_distance = reader.read_bit_double();
    let is_back_clipping_on = reader.read_bit();
    let back_clipping_distance = reader.read_bit_double();
    let count = safe_count(reader.read_bit_long());
    let mut clip_boundary_points = Vec::with_capacity(count as usize);
    for _ in 0..count {
        clip_boundary_points.push(reader.read_2bit_double());
    }

    SpatialFilterData {
        is_enabled,
        is_front_clipping_on,
        front_clipping_distance,
        is_back_clipping_on,
        back_clipping_distance,
        clip_boundary_points,
    }
}

// ════════════════════════════════════════════════════════════════════════
//  Tests
// ════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::dwg::dwg_stream_writers::merged_writer::DwgMergedWriter;
    use crate::io::dwg::dwg_version::DwgVersion;
    use crate::io::dwg::dwg_reference_type::DwgReferenceType;
    use crate::types::DxfVersion;

    fn make_reader(dwg: DwgVersion, dxf: DxfVersion, f: impl FnOnce(&mut DwgMergedWriter)) -> DwgMergedReader {
        let mut writer = DwgMergedWriter::new(dwg, dxf);
        f(&mut writer);
        let data = writer.merge();
        let hsb = writer.handle_start_bits();
        DwgMergedReader::new(data, dxf, hsb)
    }

    #[test]
    fn test_dictionary_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;
        let mut r = make_reader(v, d, |w| {
            w.write_bit_long(2); // 2 entries
            w.write_bit_short(1); // cloning
            w.write_byte(0); // not hard owner
            w.write_variable_text("ACAD_GROUP");
            w.write_handle(DwgReferenceType::SoftOwnership, 0x10);
            w.write_variable_text("ACAD_MLINESTYLE");
            w.write_handle(DwgReferenceType::SoftOwnership, 0x20);
        });
        let dict = read_dictionary(&mut r, v);
        assert_eq!(dict.entries.len(), 2);
        assert_eq!(dict.entries[0].name, "ACAD_GROUP");
        assert_eq!(dict.entries[1].name, "ACAD_MLINESTYLE");
        assert_eq!(dict.duplicate_cloning, 1);
        assert!(!dict.hard_owner);
    }

    #[test]
    fn test_dictionary_variable_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;
        let mut r = make_reader(v, d, |w| {
            w.write_byte(0); // schema number
            w.write_variable_text("test_value");
        });
        let dv = read_dictionary_variable(&mut r);
        assert_eq!(dv.schema_number, 0);
        assert_eq!(dv.value, "test_value");
    }

    #[test]
    fn test_group_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;
        let mut r = make_reader(v, d, |w| {
            w.write_variable_text("My Group");
            w.write_bit_short(1); // unnamed
            w.write_bit_short(1); // selectable
            w.write_bit_long(2); // 2 entities
            w.write_handle(DwgReferenceType::HardPointer, 0xA0);
            w.write_handle(DwgReferenceType::HardPointer, 0xB0);
        });
        let g = read_group(&mut r);
        assert_eq!(g.description, "My Group");
        assert!(g.selectable);
        assert_eq!(g.entity_handles.len(), 2);
    }

    #[test]
    fn test_scale_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;
        let mut r = make_reader(v, d, |w| {
            w.write_bit_short(0); // unknown
            w.write_variable_text("1:1");
            w.write_bit_double(1.0);
            w.write_bit_double(1.0);
            w.write_bit(true);
        });
        let s = read_scale(&mut r);
        assert_eq!(s.name, "1:1");
        assert_eq!(s.paper_units, 1.0);
        assert_eq!(s.drawing_units, 1.0);
        assert!(s.is_unit_scale);
    }

    #[test]
    fn test_xrecord_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;
        let mut r = make_reader(v, d, |w| {
            w.write_bit_long(0); // data size (comes first per spec)
            w.write_bit_short(0); // cloning flags (comes after data)
        });
        let xr = read_xrecord(&mut r);
        assert_eq!(xr.cloning_flags, 0);
        assert_eq!(xr.data_size, 0);
    }

    #[test]
    fn test_raster_variables_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;
        let mut r = make_reader(v, d, |w| {
            w.write_bit_long(0); // class version
            w.write_bit_short(1); // display frame
            w.write_bit_short(0); // quality
            w.write_bit_short(3); // units
        });
        let rv = read_raster_variables(&mut r);
        assert_eq!(rv.class_version, 0);
        assert_eq!(rv.display_image_frame, 1);
        assert_eq!(rv.units, 3);
    }

    #[test]
    fn test_book_color_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;
        let mut r = make_reader(v, d, |w| {
            w.write_variable_text("Red");
            w.write_variable_text("Main Colors");
        });
        let bc = read_book_color(&mut r);
        assert_eq!(bc.color_name, "Red");
        assert_eq!(bc.book_name, "Main Colors");
    }

    #[test]
    fn test_wipeout_variables_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;
        let mut r = make_reader(v, d, |w| {
            w.write_bit_short(1);
        });
        let wv = read_wipeout_variables(&mut r);
        assert_eq!(wv.display_frame, 1);
    }

    #[test]
    fn test_image_definition_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;
        let mut r = make_reader(v, d, |w| {
            w.write_bit_long(0); // class version
            w.write_2raw_double(Vector2::new(1024.0, 768.0)); // size
            w.write_variable_text("test.png"); // filename
            w.write_bit(true); // is_loaded
            w.write_byte(3); // resolution_unit
            w.write_2raw_double(Vector2::new(1.0, 1.0)); // pixel_size
        });
        let def = read_image_definition(&mut r);
        assert_eq!(def.file_name, "test.png");
        assert!(def.is_loaded);
        assert_eq!(def.size_in_pixels.x, 1024.0);
    }

    #[test]
    fn test_sort_entities_table_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;
        let mut r = make_reader(v, d, |w| {
            w.write_bit_long(1); // 1 entry
            w.write_handle(DwgReferenceType::SoftPointer, 0x10);
            w.write_handle(DwgReferenceType::HardPointer, 0x20);
            w.write_handle(DwgReferenceType::HardPointer, 0x30); // block owner
        });
        let st = read_sort_entities_table(&mut r);
        assert_eq!(st.entries.len(), 1);
        assert_eq!(st.block_owner_handle, 0x30);
    }

    #[test]
    fn test_table_style_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;
        let mut r = make_reader(v, d, |w| {
            w.write_byte(1);
            w.write_variable_text("Standard");
            w.write_variable_text("Table style description");
            w.write_bit_short(0);
            w.write_bit_short(3);
            w.write_bit_double(0.06);
            w.write_bit_double(0.07);
            w.write_bit(false);
            w.write_bit(true);

            for i in 0..3 {
                w.write_variable_text("Standard");
                w.write_handle(DwgReferenceType::HardPointer, 0x100 + i);
                w.write_bit_double(0.18 + (i as f64 * 0.01));
                w.write_bit_short(5);
                w.write_cm_color(&Color::ByBlock);
                w.write_bit(i == 0);
                w.write_cm_color(&Color::Index(7));
                w.write_bit_long(512);
                w.write_bit_long(0);
                w.write_variable_text("%lu2");

                for _ in 0..6 {
                    w.write_bit_long(0);
                    w.write_bit_short(1);
                    w.write_bit_long(-2);
                    w.write_cm_color(&Color::ByBlock);
                    w.write_bit(false);
                    w.write_bit_double(0.0);
                }
            }
        });

        let ts = read_table_style(&mut r);
        assert_eq!(ts.version, 1);
        assert_eq!(ts.name, "Standard");
        assert_eq!(ts.description, "Table style description");
        assert_eq!(ts.flags, 3);
        assert!(ts.header_suppressed);
        assert!((ts.data_row_style.text_height - 0.18).abs() < 1e-12);
        assert_eq!(ts.header_row_style.text_style_handle, 0x101);
    }

    #[test]
    fn test_visual_style_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;
        let mut r = make_reader(v, d, |w| {
            w.write_variable_text("Conceptual");
            w.write_bit_short(2);
            w.write_bit_short(1);
            w.write_bit_short(2);
            w.write_bit_short(3);
            w.write_bit_long(10);
            w.write_bit_long(11);
            w.write_bit_long(12);
            w.write_bit_short(4);
            w.write_bit_short(85);
            w.write_bit_short(65);
            w.write_bit_long(42);
            w.write_bit(true);
        });

        let vs = read_visual_style(&mut r);
        assert_eq!(vs.description, "Conceptual");
        assert_eq!(vs.style_type, 2);
        assert_eq!(vs.edge_style, 12);
        assert_eq!(vs.edge_color_mode, 4);
        assert_eq!(vs.face_opacity, 85);
        assert_eq!(vs.edge_opacity, 65);
        assert_eq!(vs.face_tint_color, 42);
        assert!(vs.internal_use_only);
    }

    #[test]
    fn test_material_geodata_spatial_filter_roundtrip() {
        let v = DwgVersion::AC15;
        let d = DxfVersion::AC1015;

        let mut r_mat = make_reader(v, d, |w| {
            w.write_variable_text("Concrete");
            w.write_variable_text("Gray matte");
            w.write_cm_color(&Color::Index(8));
            w.write_cm_color(&Color::Index(253));
            w.write_bit_double(0.35);
            w.write_bit_double(0.8);
        });
        let mat = read_material(&mut r_mat);
        assert_eq!(mat.name, "Concrete");
        assert_eq!(mat.description, "Gray matte");
        assert_eq!(mat.ambient_color, Color::Index(8));
        assert_eq!(mat.diffuse_color, Color::Index(253));
        assert!((mat.roughness - 0.35).abs() < 1e-12);
        assert!((mat.opacity - 0.8).abs() < 1e-12);

        let mut r_geo = make_reader(v, d, |w| {
            w.write_bit_long(2);
            w.write_bit_short(3);
            w.write_3bit_double(Vector3::new(100.0, 200.0, 0.0));
            w.write_3bit_double(Vector3::new(37.5, -122.4, 0.0));
            w.write_2bit_double(Vector2::new(0.0, 1.0));
            w.write_bit_double(1.0);
            w.write_bit_double(0.9996);
        });
        let geo = read_geodata(&mut r_geo);
        assert_eq!(geo.version, 2);
        assert_eq!(geo.coordinate_type, 3);
        assert!((geo.design_point.x - 100.0).abs() < 1e-12);
        assert!((geo.reference_point.y - (-122.4)).abs() < 1e-12);
        assert!((geo.north_direction.y - 1.0).abs() < 1e-12);
        assert!((geo.vertical_unit_scale - 0.9996).abs() < 1e-12);

        let mut r_sp = make_reader(v, d, |w| {
            w.write_bit(true);
            w.write_bit(true);
            w.write_bit_double(12.0);
            w.write_bit(false);
            w.write_bit_double(0.0);
            w.write_bit_long(3);
            w.write_2bit_double(Vector2::new(0.0, 0.0));
            w.write_2bit_double(Vector2::new(10.0, 0.0));
            w.write_2bit_double(Vector2::new(10.0, 5.0));
        });
        let sp = read_spatial_filter(&mut r_sp);
        assert!(sp.is_enabled);
        assert!(sp.is_front_clipping_on);
        assert!((sp.front_clipping_distance - 12.0).abs() < 1e-12);
        assert!(!sp.is_back_clipping_on);
        assert_eq!(sp.clip_boundary_points.len(), 3);
        assert!((sp.clip_boundary_points[2].y - 5.0).abs() < 1e-12);
    }
}
