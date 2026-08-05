//! DWG Document Builder — maps raw DWG parsed data into CadDocument.
//!
//! This module bridges the gap between the low-level object readers
//! (which produce `*Data` structs) and the high-level domain model
//! (entities, objects, tables in `CadDocument`).
//!
//! ## Two-Pass Architecture
//!
//! **Pass 1 (Tables):** Read all table entries (layers, block headers,
//! text styles, linetypes) and build handle→name lookup maps.
//!
//! **Pass 2 (Entities & Objects):** Read entities and objects, resolving
//! handle references (e.g., layer_handle → layer name, block_handle →
//! block name) using the maps built in Pass 1.

use crate::document::CadDocument;
use crate::entities::EntityCommon;
use crate::entities::*;
use crate::io::dwg::dwg_stream_readers::object_reader::common::*;
use crate::io::dwg::dwg_stream_readers::object_reader::entities;
use crate::io::dwg::dwg_stream_readers::object_reader::objects;
use crate::io::dwg::dwg_stream_readers::object_reader::tables;
use crate::io::dwg::dwg_stream_readers::object_reader::{DwgObjectReader, EntityCommonData};
use crate::notification::{NotificationCollection, NotificationType};
use crate::types::Handle;
use crate::types::LineWeight;
use std::collections::HashMap;

/// Pending vertex data collected during Pass 2, keyed by owner (parent polyline) handle.
enum PendingVertex {
    V2D(entities::Vertex2DData),
    V3D(entities::Vertex3DData, EntityCommon),
    PfaceFace(entities::PfaceFaceData, EntityCommon),
}

/// Pending polyline entities awaiting vertex assembly.
struct PendingPolylines {
    /// Vertex data keyed by owner (parent polyline) handle.
    vertices: HashMap<u64, Vec<PendingVertex>>,
    /// SEQEND handle keyed by owner (parent polyline) handle.
    seqends: HashMap<u64, crate::types::Handle>,
    /// Polyline entities awaiting vertex assembly, keyed by their handle.
    polylines: Vec<(u64, EntityType)>,
}

/// Handle-to-name resolution maps built from table entries.
struct HandleMaps {
    /// handle → layer name
    layers: HashMap<u64, String>,
    /// handle → block name
    blocks: HashMap<u64, String>,
    /// handle → text style name
    text_styles: HashMap<u64, String>,
    /// handle → linetype name
    linetypes: HashMap<u64, String>,
    /// handle → dimension style name
    dim_styles: HashMap<u64, String>,
}

impl HandleMaps {
    fn new() -> Self {
        Self {
            layers: HashMap::new(),
            blocks: HashMap::new(),
            text_styles: HashMap::new(),
            linetypes: HashMap::new(),
            dim_styles: HashMap::new(),
        }
    }

    fn layer_name(&self, handle: u64) -> String {
        self.layers
            .get(&handle)
            .cloned()
            .unwrap_or_else(|| "0".to_string())
    }

    fn block_name(&self, handle: u64) -> String {
        self.blocks
            .get(&handle)
            .cloned()
            .unwrap_or_else(|| format!("*U{}", handle))
    }

    fn style_name(&self, handle: u64) -> String {
        self.text_styles
            .get(&handle)
            .cloned()
            .unwrap_or_else(|| "STANDARD".to_string())
    }

    #[allow(dead_code)]
    fn dimstyle_name(&self, handle: u64) -> String {
        self.dim_styles
            .get(&handle)
            .cloned()
            .unwrap_or_else(|| "Standard".to_string())
    }
}

/// Builds a `CadDocument` from parsed DWG object data.
pub struct DwgDocumentBuilder {
    obj_reader: DwgObjectReader,
    /// Whether to use failsafe mode (report skipped records via notifications).
    failsafe: bool,
    /// Notifications collected during building.
    notifications: NotificationCollection,
}

impl DwgDocumentBuilder {
    /// Create a new builder wrapping the object reader.
    pub fn new(obj_reader: DwgObjectReader) -> Self {
        Self {
            obj_reader,
            failsafe: false,
            notifications: NotificationCollection::new(),
        }
    }

    /// Enable or disable failsafe mode.
    ///
    /// When enabled, skipped records are reported as notifications
    /// instead of being silently lost.
    pub fn set_failsafe(&mut self, failsafe: bool) {
        self.failsafe = failsafe;
    }

    /// Build the document by iterating all handles and dispatching objects.
    ///
    /// Uses a two-pass approach:
    /// 1. Read table entries → build handle→name maps
    /// 2. Read entities and objects → resolve handle references
    ///
    /// Returns collected notifications (skipped records, warnings).
    pub fn build(mut self, document: &mut CadDocument) -> NotificationCollection {
        let mut handles = self.obj_reader.handles();
        // Sort handles numerically so that entity records are processed in
        // allocation order.  This ensures polyline vertex records are
        // encountered in the correct sequence (the writer allocates
        // sequential handles for child entities).
        handles.sort_unstable();
        let mut skipped_pass1 = 0u32;
        let mut skipped_pass2 = 0u32;
        let total_handles = handles.len();

        // Build class_number → internal type code mapping for non-fixed types.
        // The DWG binary uses class numbers (500+) for object types defined in
        // the CLASSES section.  We translate these to our internal OBJ_*
        // constants so the match statements work correctly.
        let class_map = Self::build_class_type_map(document);

        // Build a set of class numbers that represent graphical entities
        // (as opposed to non-entity objects).  Used in Pass 2 to correctly
        // classify unresolved class-based types (≥500) that aren't in
        // dxf_name_to_type_code.
        let entity_class_numbers: std::collections::HashSet<i16> = document
            .classes
            .iter()
            .filter(|c| c.is_an_entity && c.class_number >= 500)
            .map(|c| c.class_number)
            .collect();

        // ── Pass 1: Build handle→name maps from table entries ──────────
        //
        // In addition to building handle→name lookup maps (for Pass 2
        // entity resolution), we now also create full domain objects
        // (Layer, BlockRecord, TextStyle, LineType, DimStyle) and
        // populate the document tables.  This mirrors what the DXF
        // reader does in its TABLES section reader.
        let mut maps = HandleMaps::new();

        // Parsed table entries collected for post-loop domain-object creation.
        // We collect first and create domain objects after the loop so that
        // cross-references (e.g. layer → linetype name) can be resolved
        // using the fully-populated handle→name maps.
        enum ParsedEntry {
            Layer(u64, tables::LayerData),
            Block(u64, tables::BlockHeaderData),
            Style(u64, tables::TextStyleData),
            Ltype(u64, tables::LinetypeData),
            DimStyle(u64, tables::DimStyleData),
            View(u64, tables::ViewData),
            Ucs(u64, tables::UcsData),
            VPort(u64, tables::VPortData),
            AppId(u64, tables::AppIdData),
            /// BLOCK_CONTROL hard-owner refs: (model_space_handle, paper_space_handle).
            /// These are the authoritative active model/paper space designation —
            /// the file header's block handles are unreliable on some versions.
            BlockControl(u64, u64),
        }
        let mut parsed_entries: Vec<ParsedEntry> = Vec::new();

        for &handle in &handles {
            let offset = match self.obj_reader.offset_for(handle) {
                Some(o) if o >= 0 => o,
                _ => continue,
            };
            let (raw_type_code, mut reader) = match self.obj_reader.read_record_at(offset as usize)
            {
                Ok(r) => r,
                Err(_) => continue,
            };
            let type_code = Self::resolve_type_code(raw_type_code, &class_map);

            if is_table_type(type_code) {
                // Wrap in catch_unwind to survive corrupt/misaligned records
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let non_entity = self
                        .obj_reader
                        .read_common_non_entity_data(&mut reader, type_code);
                    let obj_handle = non_entity.common.handle;
                    let eed_raw = non_entity.common.eed_raw;
                    let xdic = non_entity.xdictionary_handle;
                    let reactors = non_entity.reactors.clone();
                    (obj_handle, type_code, eed_raw, xdic, reactors)
                }));
                let (obj_handle, type_code, eed_raw_pass1, xdic_pass1, reactors_pass1) =
                    match result {
                        Ok(v) => v,
                        Err(_) => {
                            skipped_pass1 += 1;
                            self.notifications.notify(
                            NotificationType::Error,
                            format!(
                                "Skipped corrupt table record at handle {:#X} (panic in common data)",
                                handle
                            ),
                        );
                            continue;
                        }
                    };
                // Save EED for DWG round-trip write-back
                if !eed_raw_pass1.is_empty() {
                    document
                        .eed_by_handle
                        .insert(Handle::from(obj_handle), eed_raw_pass1);
                }
                // Save xdictionary handle for DWG round-trip write-back
                if let Some(xdic) = xdic_pass1 {
                    document
                        .xdic_by_handle
                        .insert(Handle::from(obj_handle), Handle::from(xdic));
                }
                // Save reactors for DWG round-trip write-back
                if !reactors_pass1.is_empty() {
                    document.reactors_by_handle.insert(
                        Handle::from(obj_handle),
                        reactors_pass1.iter().map(|&h| Handle::from(h)).collect(),
                    );
                }

                let table_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    match type_code {
                        OBJ_LAYER => {
                            let data = tables::read_layer(
                                &mut reader,
                                self.obj_reader.version(),
                                self.obj_reader.dxf_version(),
                            );
                            Some(ParsedEntry::Layer(obj_handle, data))
                        }
                        OBJ_BLOCK_HEADER => {
                            let data =
                                tables::read_block_header(&mut reader, self.obj_reader.version());
                            Some(ParsedEntry::Block(obj_handle, data))
                        }
                        OBJ_BLOCK_CONTROL => {
                            // Capture the authoritative *Model_Space / *Paper_Space
                            // designation (hard-owner refs) so block-name dedup can
                            // keep the canonical names on the correct records.
                            let data = tables::read_block_control(&mut reader);
                            Some(ParsedEntry::BlockControl(
                                data.model_space_handle,
                                data.paper_space_handle,
                            ))
                        }
                        OBJ_STYLE => {
                            let data =
                                tables::read_text_style(&mut reader, self.obj_reader.version());
                            Some(ParsedEntry::Style(obj_handle, data))
                        }
                        OBJ_LTYPE => {
                            let data =
                                tables::read_linetype(&mut reader, self.obj_reader.version());
                            Some(ParsedEntry::Ltype(obj_handle, data))
                        }
                        OBJ_DIMSTYLE => {
                            let data = tables::read_dimstyle(
                                &mut reader,
                                self.obj_reader.version(),
                                self.obj_reader.dxf_version(),
                            );
                            Some(ParsedEntry::DimStyle(obj_handle, data))
                        }
                        OBJ_VIEW => {
                            let data = tables::read_view(&mut reader, self.obj_reader.version());
                            Some(ParsedEntry::View(obj_handle, data))
                        }
                        OBJ_UCS => {
                            let data = tables::read_ucs(&mut reader, self.obj_reader.version());
                            Some(ParsedEntry::Ucs(obj_handle, data))
                        }
                        OBJ_VPORT => {
                            let data = tables::read_vport(&mut reader, self.obj_reader.version());
                            Some(ParsedEntry::VPort(obj_handle, data))
                        }
                        OBJ_APPID => {
                            let data = tables::read_appid(&mut reader, self.obj_reader.version());
                            Some(ParsedEntry::AppId(obj_handle, data))
                        }
                        _ => None,
                    }
                }));
                match table_result {
                    Ok(Some(entry)) => {
                        // Populate handle→name maps (needed by Pass 2)
                        match &entry {
                            ParsedEntry::Layer(h, data) => {
                                maps.layers.insert(*h, data.name.clone());
                            }
                            ParsedEntry::Block(h, data) => {
                                maps.blocks.insert(*h, data.name.clone());
                            }
                            ParsedEntry::Style(h, data) => {
                                maps.text_styles.insert(*h, data.name.clone());
                            }
                            ParsedEntry::Ltype(h, data) => {
                                maps.linetypes.insert(*h, data.name.clone());
                            }
                            ParsedEntry::DimStyle(h, data) => {
                                maps.dim_styles.insert(*h, data.name.clone());
                            }
                            ParsedEntry::View(_, _) => {}
                            ParsedEntry::Ucs(_, _) => {}
                            ParsedEntry::VPort(_, _) => {}
                            ParsedEntry::AppId(_, _) => {}
                            ParsedEntry::BlockControl(m, p) => {
                                // Seed the authoritative active model/paper space
                                // handles (used by the block-name dedup below).
                                if *m != 0 {
                                    document.header.model_space_block_handle = Handle::from(*m);
                                }
                                if *p != 0 {
                                    document.header.paper_space_block_handle = Handle::from(*p);
                                }
                            }
                        }
                        // The block control is not a table record — don't store it.
                        if !matches!(entry, ParsedEntry::BlockControl(..)) {
                            parsed_entries.push(entry);
                        }
                    }
                    Ok(None) => {}
                    Err(_) => {
                        skipped_pass1 += 1;
                        self.notifications.notify(
                            NotificationType::Error,
                            format!(
                                "Skipped corrupt table record at handle {:#X}, type_code={}",
                                handle, type_code
                            ),
                        );
                    }
                }
            }
        }

        // ── Deduplicate block names ────────────────────────────────────
        //
        // DWG binary format stores ALL paper-space blocks as "*Paper_Space"
        // and anonymous blocks share names ("*U", "*D", etc.).  Our
        // Table<BlockRecord> is keyed by name, so duplicates would
        // overwrite each other.  Rename duplicates using the DXF
        // convention: *Paper_Space, *Paper_Space0, *Paper_Space1, …
        //
        // The header's model_space_block_handle / paper_space_block_handle
        // (read from the DWG file header before this function) identify
        // the "active" model/paper space blocks, which keep their
        // canonical names.
        {
            let active_model = document.header.model_space_block_handle;
            let active_paper = document.header.paper_space_block_handle;

            // Collect (index, handle, base_name) for all Block entries
            let block_info: Vec<(usize, u64, String)> = parsed_entries
                .iter()
                .enumerate()
                .filter_map(|(idx, e)| {
                    if let ParsedEntry::Block(h, data) = e {
                        Some((idx, *h, data.name.clone()))
                    } else {
                        None
                    }
                })
                .collect();

            // Group by name
            let mut name_groups: std::collections::HashMap<String, Vec<(usize, u64)>> =
                std::collections::HashMap::new();
            for (idx, h, name) in &block_info {
                name_groups
                    .entry(name.clone())
                    .or_default()
                    .push((*idx, *h));
            }

            // Rename duplicates
            for (base_name, entries) in &name_groups {
                if entries.len() <= 1 {
                    continue;
                }
                // Determine which entry keeps the canonical (un-suffixed)
                // name.  Prefer the one matching the header's active
                // model/paper space handle; fall back to the first entry.
                let active_h = if base_name == "*Model_Space" {
                    active_model
                } else if base_name == "*Paper_Space" {
                    active_paper
                } else {
                    Handle::NULL
                };

                let canonical_idx = entries
                    .iter()
                    .find(|(_, h)| !active_h.is_null() && Handle::from(*h) == active_h)
                    .or_else(|| entries.first())
                    .map(|&(idx, _)| idx);

                let mut suffix = 0usize;
                for &(idx, h) in entries {
                    if Some(idx) == canonical_idx {
                        continue; // keep canonical name
                    }
                    let new_name = format!("{}{}", base_name, suffix);
                    if let ParsedEntry::Block(_, ref mut data) = parsed_entries[idx] {
                        data.name = new_name.clone();
                    }
                    maps.blocks.insert(h, new_name);
                    suffix += 1;
                }
            }
        }

        // ── Post-Pass 1: Populate document tables from parsed data ─────
        //
        // Now that all handle→name maps are complete, create domain objects
        // with resolved cross-references and add them to the document.
        //
        // Clear initialisation-defaults for block records first: the
        // defaults (created by CadDocument::new()) use handles 0x15 / 0x18
        // which may collide with objects from the DWG file.
        let _ = document.block_records.remove("*Model_Space");
        let _ = document.block_records.remove("*Paper_Space");

        let mut cleared_default_vports = false;
        for entry in &parsed_entries {
            match entry {
                ParsedEntry::Layer(h, data) => {
                    let mut layer = crate::tables::Layer::new(&data.name);
                    layer.handle = Handle::from(*h);
                    layer.flags.frozen = data.frozen;
                    layer.flags.off = data.off;
                    layer.flags.locked = data.locked;
                    layer.flags.xref_dependent = data.xref_dependent;
                    layer.is_plottable = data.plottable;
                    layer.line_weight = LineWeight::from_value(data.line_weight);
                    layer.color = data.color;
                    // Resolve linetype handle → name
                    layer.line_type = maps
                        .linetypes
                        .get(&data.linetype_handle)
                        .cloned()
                        .unwrap_or_else(|| "Continuous".to_string());
                    // Material handle
                    if let Some(mh) = data.material_handle {
                        layer.material = Handle::from(mh);
                    }
                    // Plotstyle handle (R2000+)
                    if let Some(ph) = data.plotstyle_handle {
                        layer.plotstyle_handle = Handle::from(ph);
                    }
                    // External reference block record handle
                    if data.xref_handle != 0 {
                        layer.xref_block_record_handle = Handle::from(data.xref_handle);
                    }
                    // Remove default entry if it exists, then add
                    let _ = document.layers.remove(&data.name);
                    let _ = document.layers.add(layer);
                }
                ParsedEntry::Block(h, data) => {
                    let mut br = crate::tables::BlockRecord::new(&data.name);
                    br.handle = Handle::from(*h);
                    br.flags.anonymous = data.anonymous;
                    br.flags.has_attributes = data.has_attributes;
                    br.flags.is_xref = data.is_xref;
                    br.flags.is_xref_overlay = data.is_xref_overlay;
                    br.block_entity_handle = Handle::from(data.block_entity_handle);
                    br.block_end_handle = Handle::from(data.endblk_handle);
                    br.units = data.units.unwrap_or(0);
                    br.explodable = data.explodable.unwrap_or(true);
                    br.scale_uniformly = data.scale_uniformly.map(|v| v != 0).unwrap_or(false);
                    br.xref_path = data.xref_path.clone();
                    br.description = data.description.clone().unwrap_or_default();
                    br.insert_count_bytes = data.insert_count_bytes.clone();
                    br.preview_data = data.preview_data.clone();
                    br.insert_handles = data
                        .insert_handles
                        .iter()
                        .map(|&h| Handle::from(h))
                        .collect();
                    br.base_point = data.base_point;
                    if let Some(layout_h) = data.layout_handle {
                        br.layout = Handle::from(layout_h);
                    }
                    // Update header handles for model/paper space
                    // (uses the deduplicated name, so only the active block
                    // with the canonical name "*Model_Space" / "*Paper_Space"
                    // sets the header handle)
                    if data.name.eq_ignore_ascii_case("*Model_Space") {
                        document.header.model_space_block_handle = br.handle;
                    } else if data.name == "*Paper_Space" {
                        document.header.paper_space_block_handle = br.handle;
                    }
                    // Remove default entry if it exists, then add
                    let _ = document.block_records.remove(&data.name);
                    let _ = document.block_records.add(br);
                }
                ParsedEntry::Style(h, data) => {
                    let mut style = crate::tables::TextStyle::new(&data.name);
                    style.handle = Handle::from(*h);
                    style.height = data.height;
                    style.width_factor = data.width_factor;
                    style.oblique_angle = data.oblique_angle;
                    style.last_height = data.last_height;
                    style.font_file = data.font_file.clone();
                    style.big_font_file = data.big_font_file.clone();
                    style.flags.backward = (data.generation & 2) != 0;
                    style.flags.upside_down = (data.generation & 4) != 0;
                    // Only mark xref-dependent if the xref block record handle is valid
                    style.xref_dependent = data.xref_dependent && data.xref_handle != 0;
                    // Use add_allow_duplicate for shape-file-only styles (empty name)
                    // so multiple empty-named styles are preserved. Named styles use
                    // add_or_replace to avoid duplicates (e.g. "Standard").
                    if data.name.is_empty() {
                        document.text_styles.add_allow_duplicate(style);
                    } else {
                        document.text_styles.add_or_replace(style);
                    }
                }
                ParsedEntry::Ltype(h, data) => {
                    let mut lt = crate::tables::LineType::new(&data.name);
                    lt.handle = Handle::from(*h);
                    lt.description = data.description.clone();
                    lt.pattern_length = data.pattern_length;
                    lt.xref_dependent = data.xref_dependent;
                    lt.elements = data
                        .segments
                        .iter()
                        .zip(data.shape_handles.iter().chain(std::iter::repeat(&0u64)))
                        .map(|(s, &sh)| {
                            use crate::tables::linetype::{
                                LineTypeComplexContent, LineTypeComplexData, LineTypeElement,
                            };
                            let is_complex = s.dwg_flags != 0
                                || s.offset_x.abs() > 1e-12
                                || s.offset_y.abs() > 1e-12
                                || (s.scale - 1.0).abs() > 1e-12
                                || s.rotation.abs() > 1e-12
                                || s.shape_number != 0
                                || sh != 0;
                            let complex = if is_complex {
                                let content = if s.dwg_flags & 0x02 != 0 {
                                    LineTypeComplexContent::Text {
                                        text: s.text.clone(),
                                    }
                                } else {
                                    LineTypeComplexContent::Shape {
                                        shape_number: s.shape_number,
                                    }
                                };
                                Some(LineTypeComplexData {
                                    content,
                                    style_handle: Handle::from(sh),
                                    scale: s.scale,
                                    rotation: s.rotation,
                                    absolute_rotation: s.dwg_flags & 0x01 != 0,
                                    offset: [s.offset_x, s.offset_y],
                                })
                            } else {
                                None
                            };
                            LineTypeElement {
                                length: s.length,
                                complex,
                            }
                        })
                        .collect();
                    let _ = document.line_types.remove(&data.name);
                    let _ = document.line_types.add(lt);
                }
                ParsedEntry::DimStyle(h, data) => {
                    let mut ds = crate::tables::DimStyle::new(&data.name);
                    ds.handle = Handle::from(*h);
                    ds.dimscale = data.dimscale;
                    ds.dimasz = data.dimasz;
                    ds.dimexo = data.dimexo;
                    ds.dimdli = data.dimdli;
                    ds.dimexe = data.dimexe;
                    ds.dimrnd = data.dimrnd;
                    ds.dimdle = data.dimdle;
                    ds.dimtp = data.dimtp;
                    ds.dimtm = data.dimtm;
                    ds.dimtol = data.dimtol;
                    ds.dimlim = data.dimlim;
                    ds.dimtih = data.dimtih;
                    ds.dimtoh = data.dimtoh;
                    ds.dimse1 = data.dimse1;
                    ds.dimse2 = data.dimse2;
                    ds.dimtad = data.dimtad;
                    ds.dimzin = data.dimzin;
                    ds.dimazin = data.dimazin;
                    ds.dimtxt = data.dimtxt;
                    ds.dimcen = data.dimcen;
                    ds.dimtsz = data.dimtsz;
                    ds.dimaltf = data.dimaltf;
                    ds.dimlfac = data.dimlfac;
                    ds.dimtvp = data.dimtvp;
                    ds.dimtfac = data.dimtfac;
                    ds.dimgap = data.dimgap;
                    ds.dimalt = data.dimalt;
                    ds.dimaltd = data.dimaltd;
                    ds.dimtofl = data.dimtofl;
                    ds.dimsah = data.dimsah;
                    ds.dimtix = data.dimtix;
                    ds.dimsoxd = data.dimsoxd;
                    ds.dimclrd = data.dimclrd.index().unwrap_or(0) as i16;
                    ds.dimclre = data.dimclre.index().unwrap_or(0) as i16;
                    ds.dimclrt = data.dimclrt.index().unwrap_or(0) as i16;
                    ds.dimsd1 = data.dimsd1;
                    ds.dimsd2 = data.dimsd2;
                    ds.dimtolj = data.dimtolj;
                    ds.dimtzin = data.dimtzin;
                    ds.dimupt = data.dimupt;
                    ds.dimfit = data.dimfit;
                    ds.dimlwd = data.dimlwd;
                    ds.dimlwe = data.dimlwe;
                    ds.dimpost = data.dimpost.clone();
                    ds.dimapost = data.dimapost.clone();
                    ds.dimaltrnd = data.dimaltrnd;
                    ds.dimadec = data.dimadec;
                    ds.dimdec = data.dimdec;
                    ds.dimtdec = data.dimtdec;
                    ds.dimaltu = data.dimaltu;
                    ds.dimalttd = data.dimalttd;
                    ds.dimaunit = data.dimaunit;
                    ds.dimfrac = data.dimfrac;
                    ds.dimlunit = data.dimlunit;
                    ds.dimdsep = data.dimdsep;
                    ds.dimtmove = data.dimtmove;
                    ds.dimjust = data.dimjust;
                    ds.dimaltz = data.dimaltz;
                    ds.dimalttz = data.dimalttz;
                    // R2007+ fields
                    ds.dimfxl = data.dimfxl;
                    ds.dimjogang = data.dimjogang;
                    ds.dimtfill = data.dimtfill;
                    ds.dimtfillclr = data.dimtfillclr.index().unwrap_or(0) as i16;
                    ds.dimarcsym = data.dimarcsym;
                    ds.dimfxlon = data.dimfxlon;
                    ds.dimtxtdirection = data.dimtxtdirection;
                    // Resolve text style handle
                    if data.dimtxsty_handle != 0 {
                        ds.dimtxsty_handle = Handle::from(data.dimtxsty_handle);
                        ds.dimtxsty = maps
                            .text_styles
                            .get(&data.dimtxsty_handle)
                            .cloned()
                            .unwrap_or_else(|| "Standard".to_string());
                    }
                    // R2000+ block handles
                    if let Some(h) = data.dimldrblk_handle {
                        ds.dimldrblk = Handle::from(h);
                    }
                    if let Some(h) = data.dimblk_handle {
                        ds.dimblk = Handle::from(h);
                    }
                    if let Some(h) = data.dimblk1_handle {
                        ds.dimblk1 = Handle::from(h);
                    }
                    if let Some(h) = data.dimblk2_handle {
                        ds.dimblk2 = Handle::from(h);
                    }
                    // R2007+ linetype handles
                    if data.dimltype_handle != 0 {
                        ds.dimltex_handle = Handle::from(data.dimltype_handle);
                    }
                    if data.dimltex1_handle != 0 {
                        ds.dimltex1_handle = Handle::from(data.dimltex1_handle);
                    }
                    if data.dimltex2_handle != 0 {
                        ds.dimltex2_handle = Handle::from(data.dimltex2_handle);
                    }
                    let _ = document.dim_styles.remove(&data.name);
                    let _ = document.dim_styles.add(ds);
                }
                ParsedEntry::View(h, data) => {
                    let mut view = crate::tables::View::new(&data.name);
                    view.handle = Handle::from(*h);
                    view.height = data.height;
                    view.width = data.width;
                    view.center = crate::types::Vector3::new(data.center.x, data.center.y, 0.0);
                    view.target = data.target;
                    view.direction = data.direction;
                    view.twist_angle = data.twist_angle;
                    view.lens_length = data.lens_length;
                    view.front_clip = data.front_clip;
                    view.back_clip = data.back_clip;
                    view.perspective = data.perspective;
                    let _ = document.views.remove(&data.name);
                    let _ = document.views.add(view);
                }
                ParsedEntry::Ucs(h, data) => {
                    let mut ucs = crate::tables::Ucs::new(&data.name);
                    ucs.handle = Handle::from(*h);
                    ucs.origin = data.origin;
                    ucs.x_axis = data.x_axis;
                    ucs.y_axis = data.y_axis;
                    let _ = document.ucss.remove(&data.name);
                    let _ = document.ucss.add(ucs);
                }
                ParsedEntry::VPort(h, data) => {
                    if !cleared_default_vports {
                        document.vports.clear();
                        cleared_default_vports = true;
                    }
                    let mut vp = crate::tables::VPort::new(&data.name);
                    vp.handle = Handle::from(*h);
                    vp.lower_left = data.lower_left;
                    vp.upper_right = data.upper_right;
                    vp.view_center = data.view_center;
                    vp.snap_base = data.snap_base;
                    vp.snap_spacing = data.snap_spacing;
                    vp.grid_spacing = data.grid_spacing;
                    vp.view_direction = data.view_direction;
                    vp.view_target = data.view_target;
                    vp.view_height = data.view_height;
                    vp.aspect_ratio = if data.view_height.abs() > 1e-10 {
                        data.aspect_ratio_times_height / data.view_height
                    } else {
                        1.0
                    };
                    vp.lens_length = data.lens_length;
                    vp.view_twist = data.view_twist;
                    vp.front_clip = data.front_clip;
                    vp.back_clip = data.back_clip;
                    vp.ucsfollow = data.ucsfollow;
                    vp.circle_zoom = data.circle_zoom;
                    vp.fast_zoom = data.fast_zoom;
                    vp.grid_on = data.grid_on;
                    vp.snap_on = data.snap_on;
                    vp.snap_style = data.snap_style;
                    vp.snap_isopair = data.snap_isopair;
                    vp.snap_rotation = data.snap_rotation;
                    vp.render_mode =
                        ViewportRenderMode::from_value(data.render_mode.unwrap_or(0) as i16);
                    document.vports.add_allow_duplicate(vp);
                }
                ParsedEntry::AppId(h, data) => {
                    let mut app = crate::tables::AppId::new(&data.name);
                    app.handle = Handle::from(*h);
                    let _ = document.app_ids.remove(&data.name);
                    let _ = document.app_ids.add(app);
                }
                // Block control is consumed during Pass 1 (header seeding); it is
                // never stored as a parsed table entry.
                ParsedEntry::BlockControl(..) => {}
            }
        }

        // Build a reverse map: entity_handle → block_record_handle
        // from the canonical entity_handles read from the DWG binary
        // (R2004+).  This is needed because entity_mode=1 only says
        // "paper space" without specifying WHICH paper space.
        let mut binary_entity_owner: HashMap<Handle, Handle> = HashMap::new();
        for entry in &parsed_entries {
            if let ParsedEntry::Block(h, data) = entry {
                let br_handle = Handle::from(*h);
                // Save original entity_handles from the DWG binary for the writer
                let orig_handles: Vec<Handle> = data
                    .entity_handles
                    .iter()
                    .map(|&eh| Handle::from(eh))
                    .collect();
                document
                    .block_entity_handles
                    .insert(br_handle, orig_handles);
                for &eh in &data.entity_handles {
                    binary_entity_owner.insert(Handle::from(eh), br_handle);
                }
            }
        }

        // ── Clear default objects before reading file objects ─────────
        //
        // initialize_defaults() created placeholder dictionaries, layouts,
        // and other objects.  The DWG file supplies its own complete set of
        // objects, so the defaults must be removed to avoid phantom layouts
        // (with stale block_record handles) and orphaned dictionary entries
        // that corrupt the file when written back as DXF.
        document.objects.clear();

        // ── Pass 2: Read entities and non-table objects ────────────────
        let mut pending = PendingPolylines {
            vertices: HashMap::new(),
            seqends: HashMap::new(),
            polylines: Vec::new(),
        };
        // Pending attribute entities keyed by owner (INSERT) handle.
        let mut pending_attributes: HashMap<u64, Vec<AttributeEntity>> = HashMap::new();
        for &handle in &handles {
            let offset = match self.obj_reader.offset_for(handle) {
                Some(o) if o >= 0 => o,
                _ => {
                    continue;
                }
            };
            let (raw_type_code, reader) = match self.obj_reader.read_record_at(offset as usize) {
                Ok(r) => r,
                Err(_e) => {
                    continue;
                }
            };
            let type_code = Self::resolve_type_code(raw_type_code, &class_map);

            // Wrap per-object processing in catch_unwind to survive
            // corrupt or misaligned records without crashing the entire read.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.process_pass2_record(
                    handle,
                    raw_type_code,
                    type_code,
                    reader,
                    document,
                    &maps,
                    &mut pending,
                    &mut pending_attributes,
                    &entity_class_numbers,
                );
            }));
            if let Err(ref _e) = result {
                skipped_pass2 += 1;
                self.notifications.notify(
                    NotificationType::Error,
                    format!(
                        "Skipped corrupt record at handle {:#X}, type_code={} (panic recovered)",
                        handle, type_code
                    ),
                );
                continue;
            }
        }

        // ── Post-pass: Assemble polyline vertices and add to document ──
        for (poly_handle, mut entity) in pending.polylines {
            if let Some(verts) = pending.vertices.remove(&poly_handle) {
                match &mut entity {
                    EntityType::Polyline2D(ref mut e) => {
                        e.vertices = verts
                            .into_iter()
                            .filter_map(|v| {
                                if let PendingVertex::V2D(d) = v {
                                    Some(crate::entities::polyline::Vertex2D {
                                        location: crate::types::Vector3::new(d.x, d.y, d.z),
                                        flags: crate::entities::polyline::VertexFlags::from_bits(
                                            d.flags,
                                        ),
                                        start_width: d.start_width,
                                        end_width: d.end_width,
                                        bulge: d.bulge,
                                        curve_tangent: d.tangent_dir,
                                        id: d.vertex_id,
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();
                    }
                    EntityType::Polyline3D(ref mut e) => {
                        e.vertices = verts
                            .into_iter()
                            .filter_map(|v| {
                                if let PendingVertex::V3D(d, _ec) = v {
                                    Some(crate::entities::polyline3d::Vertex3DPolyline {
                                        handle: d.handle,
                                        layer: String::new(),
                                        position: d.position,
                                        flags: d.flags as i32,
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();
                    }
                    EntityType::PolyfaceMesh(ref mut e) => {
                        for v in verts {
                            match v {
                                PendingVertex::V3D(d, ec) => {
                                    e.vertices.push(crate::entities::polyface_mesh::PolyfaceVertex {
                                        common: ec,
                                        location: d.position,
                                        flags: crate::entities::polyface_mesh::PolyfaceVertexFlags::from_bits_truncate(d.flags as i16),
                                        bulge: 0.0,
                                        start_width: 0.0,
                                        end_width: 0.0,
                                        curve_tangent: 0.0,
                                        id: 0,
                                    });
                                }
                                PendingVertex::PfaceFace(f, ec) => {
                                    e.faces.push(crate::entities::polyface_mesh::PolyfaceFace {
                                        common: ec,
                                        flags: crate::entities::polyface_mesh::PolyfaceVertexFlags::NONE,
                                        index1: f.index1,
                                        index2: f.index2,
                                        index3: f.index3,
                                        index4: f.index4,
                                        color: None,
                                    });
                                }
                                _ => {}
                            }
                        }
                        // Restore the seqend handle for this polyface mesh
                        if let Some(sh) = pending.seqends.get(&poly_handle).copied() {
                            e.seqend_handle = Some(sh);
                        }
                    }
                    EntityType::PolygonMesh(ref mut e) => {
                        e.vertices = verts
                            .into_iter()
                            .filter_map(|v| {
                                if let PendingVertex::V3D(d, _ec) = v {
                                    let mut c = crate::entities::EntityCommon::new();
                                    c.handle = d.handle;
                                    Some(crate::entities::polygon_mesh::PolygonMeshVertex {
                                        common: c,
                                        location: d.position,
                                        flags: 0,
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();
                    }
                    _ => {}
                }
            }
            let _ = document.add_entity(entity);
        }

        // ── Post-pass: Attach pending attribute entities to parent INSERTs ──
        if !pending_attributes.is_empty() {
            for entity in &mut document.entities {
                let entity = std::sync::Arc::make_mut(entity);
                if let EntityType::Insert(ref mut ins) = entity {
                    let insert_handle = ins.common.handle.value();
                    if let Some(attribs) = pending_attributes.remove(&insert_handle) {
                        ins.attributes = attribs;
                    }
                }
            }
        }

        // ── Post-pass: cache each RasterImage's path from its IMAGEDEF ──
        //
        // An IMAGE entity carries no path of its own — the referenced
        // ImageDefinition object holds it (the entity's `file_path` is only a
        // convenience cache). Copy it across so rendering and loading can see
        // the path directly: a resolvable local image loads its pixels, and an
        // unresolved reference (a URL, a missing file) can show its path as
        // text instead of a blank frame.
        {
            let def_paths: HashMap<Handle, String> = document
                .objects
                .iter()
                .filter_map(|(h, o)| match o {
                    crate::objects::ObjectType::ImageDefinition(d) if !d.file_name.is_empty() => {
                        Some((*h, d.file_name.clone()))
                    }
                    _ => None,
                })
                .collect();
            if !def_paths.is_empty() {
                for entity in &mut document.entities {
                    let needs = matches!(&**entity, EntityType::RasterImage(im)
                        if im.file_path.is_empty()
                            && im.definition_handle.is_some_and(|h| def_paths.contains_key(&h)));
                    if !needs {
                        continue;
                    }
                    if let EntityType::RasterImage(im) = std::sync::Arc::make_mut(entity) {
                        if let Some(p) = im.definition_handle.and_then(|h| def_paths.get(&h)) {
                            im.file_path = p.clone();
                        }
                    }
                }
            }
        }

        // ── Post-pass: Correct entity ownership from binary data ───────
        //
        // The DWG entity_mode=1 flag means "paper space entity" but does
        // NOT specify WHICH paper space.  During Pass 2, all entity_mode=1
        // entities were routed to the single *Paper_Space block record.
        // Use the canonical entity_handle lists from the binary block
        // records (R2004+) to correct ownership for entities that belong
        // to non-active paper spaces (*Paper_Space0, *Paper_Space1, etc.).
        if !binary_entity_owner.is_empty() {
            // 1. Fix entity owner handles from the binary source of truth
            for entity in &mut document.entities {
                let eh = entity.common().handle;
                if let Some(&correct_owner) = binary_entity_owner.get(&eh) {
                    if entity.common().owner_handle != correct_owner {
                        std::sync::Arc::make_mut(entity).common_mut().owner_handle = correct_owner;
                    }
                }
            }
            // 2. Rebuild block_record.entity_handles from corrected owners,
            //    excluding AttributeEntity (sub-entities of INSERT, not
            //    direct block record children).
            for br in document.block_records.iter_mut() {
                br.entity_handles.clear();
            }
            let ms_handle = document.header.model_space_block_handle;
            let entity_owners: Vec<(Handle, Handle, bool)> = document
                .entities
                .iter()
                .map(|e| {
                    (
                        e.common().handle,
                        e.common().owner_handle,
                        matches!(
                            e.as_ref(),
                            EntityType::AttributeEntity(_)
                                | EntityType::Block(_)
                                | EntityType::BlockEnd(_)
                        ),
                    )
                })
                .collect();
            for (eh, owner, is_excluded) in entity_owners {
                // AttributeEntity is a sub-entity of INSERT.
                // Block/BlockEnd are structural markers with separate handle fields.
                // None of these should appear in block_record.entity_handles.
                if is_excluded {
                    continue;
                }
                let mut added = false;
                if !owner.is_null() {
                    for br in document.block_records.iter_mut() {
                        if br.handle == owner {
                            br.entity_handles.push(eh);
                            added = true;
                            break;
                        }
                    }
                }
                // Fallback: route to *Model_Space if owner match not found
                if !added && !ms_handle.is_null() {
                    for br in document.block_records.iter_mut() {
                        if br.handle == ms_handle {
                            br.entity_handles.push(eh);
                            break;
                        }
                    }
                }
            }
        }

        // ── Post-pass: Resolve root dictionary handle ──────────────────
        //
        // The DWG header often stores dictionary handles as relative
        // references that resolve to 0 during header reading.  Now that
        // all objects have been read, scan for the actual root dictionary
        // (owner == NULL) and update the header.
        if document.header.named_objects_dict_handle.is_null()
            || !document
                .objects
                .contains_key(&document.header.named_objects_dict_handle)
        {
            let mut best = Handle::NULL;
            let mut best_count = 0usize;
            for (h, obj) in &document.objects {
                if let crate::objects::ObjectType::Dictionary(dict) = obj {
                    if dict.owner.is_null() {
                        if dict.entries.len() > best_count
                            || (dict.entries.len() == best_count && h.value() > best.value())
                        {
                            best = *h;
                            best_count = dict.entries.len();
                        }
                    }
                }
            }
            if !best.is_null() {
                document.header.named_objects_dict_handle = best;
            }
        }

        // Summary notification
        let total_skipped = skipped_pass1 + skipped_pass2;
        if total_skipped > 0 {
            self.notifications.notify(
                NotificationType::Warning,
                format!(
                    "DWG build summary: {} of {} handles processed, {} records skipped ({} table, {} entity/object)",
                    total_handles as u32 - total_skipped,
                    total_handles,
                    total_skipped,
                    skipped_pass1,
                    skipped_pass2,
                ),
            );
        }

        // Ensure handle_seed reflects the true maximum handle present in the
        // source file's Handles section.
        let max_from_reader = handles.iter().max().copied().unwrap_or(0);
        if max_from_reader + 1 > document.header.handle_seed {
            document.header.handle_seed = max_from_reader + 1;
        }

        // ── Annotative flag from `AcadAnnotative` EED (STYLE / DIMSTYLE) ──
        // These records have no native annotative field; the flag is stored as
        // extended data under the `AcadAnnotative` application.
        if let Some(anno_h) = document
            .app_ids
            .get("AcadAnnotative")
            .map(|a| a.handle.value())
        {
            let wide = self.obj_reader.version().r2007_plus();
            let flags: std::collections::HashMap<Handle, bool> = document
                .eed_by_handle
                .iter()
                .filter_map(|(h, blocks)| {
                    blocks
                        .iter()
                        .find(|(a, _)| *a == anno_h)
                        .and_then(|(_, bytes)| {
                            crate::io::dwg::annotative_eed::decode_flag(bytes, wide)
                        })
                        .map(|f| (*h, f))
                })
                .collect();
            for ts in document.text_styles.iter_mut() {
                if let Some(&f) = flags.get(&ts.handle) {
                    ts.annotative = f;
                }
            }
            for ds in document.dim_styles.iter_mut() {
                if let Some(&f) = flags.get(&ds.handle) {
                    ds.annotative = f;
                }
            }
        }

        // ── Decode entity EED blobs into structured records ──────────────────
        // The object reader keeps every EED block as verbatim `raw_dwg_eed`
        // bytes (preserved for a byte-exact re-save). Additionally decode each
        // block whose application is known into `records`, so callers — plugins
        // reading XDATA via `read_record`, the DXF writer — see the same values
        // a DXF read would surface. The raw blob is kept, so a plain round-trip
        // still emits it verbatim; the writer prefers raw over records per app.
        {
            let wide = self.obj_reader.version().r2007_plus();
            let app_name_by_handle: std::collections::HashMap<u64, String> = document
                .app_ids
                .iter()
                .map(|a| (a.handle.value(), a.name.clone()))
                .collect();
            let layer_name_by_handle: std::collections::HashMap<u64, String> = document
                .layers
                .iter()
                .map(|l| (l.handle.value(), l.name.clone()))
                .collect();
            if !app_name_by_handle.is_empty() {
                for entity in document.entities.iter_mut() {
                    if entity.common().extended_data.raw_dwg_eed.is_empty() {
                        continue;
                    }
                    let xd = &mut std::sync::Arc::make_mut(entity).common_mut().extended_data;
                    let blocks = xd.raw_dwg_eed.clone();
                    for (app_handle, bytes) in &blocks {
                        let Some(name) = app_name_by_handle.get(app_handle) else {
                            continue;
                        };
                        if xd.get_record(name).is_some() {
                            continue;
                        }
                        if let Some(values) =
                            crate::io::dwg::eed_codec::decode_values(bytes, wide, |h| {
                                layer_name_by_handle.get(&h).cloned()
                            })
                        {
                            let mut rec = crate::xdata::ExtendedDataRecord::new(name.clone());
                            rec.values = values;
                            xd.add_record(rec);
                        }
                    }
                }
            }
        }

        // ── AcDs SAB ordering ──────────────────────────────────────────────
        // R2013+ modeler geometry (3DSOLID/REGION/BODY/SURFACE) is stored as
        // SAB blobs in the AcDs section, one per entity whose `has_ds_data` bit
        // is set. The AcDs data-store indexes those blobs through a search
        // segment sorted ascending by owning-entity handle, and the blobs are
        // laid out in that same record order — so the i-th blob (in file order)
        // belongs to the i-th flagged modeler entity taken in ascending handle
        // order. `attach_acds_sab_blobs` pairs blob[i] with this list's i-th
        // handle. (Ordering by object-stream file offset instead mispaired
        // blobs whenever the object-stream order diverged from handle order.)
        {
            let mut ordered: Vec<Handle> = document
                .entities()
                .filter(|e| {
                    matches!(
                        e,
                        EntityType::Solid3D(_)
                            | EntityType::Region(_)
                            | EntityType::Body(_)
                            | EntityType::Surface(_)
                    ) && e.common().has_ds_data
                })
                .map(|e| e.common().handle)
                .collect();
            ordered.sort_by_key(|h| h.value());
            document.acis_sab_handles = ordered;
        }

        // ── Handle-collision repair ────────────────────────────────────────
        // The document is seeded with standard table entries (Standard dim
        // style, default block records, …) at low handles before the file's
        // objects are read, so a synthesized entry can end up sharing a handle
        // with a file object that legitimately owns it — e.g. the Standard dim
        // style vs a paper-space block record. A duplicate handle makes that
        // reference ambiguous and a strict reader rejects the owning object
        // ("improperly read"). Re-home any dim-style entry whose handle also
        // belongs to a block record, following the header references so the
        // Standard style stays reachable.
        {
            use std::collections::HashSet;
            let block_handles: HashSet<u64> = document
                .block_records
                .iter()
                .map(|b| b.handle.value())
                .collect();
            let colliding: Vec<u64> = document
                .dim_styles
                .iter()
                .map(|d| d.handle.value())
                .filter(|h| block_handles.contains(h))
                .collect();
            for old in colliding {
                let new_h = document.allocate_handle();
                for d in document.dim_styles.iter_mut() {
                    if d.handle.value() == old {
                        d.handle = new_h;
                    }
                }
                if document.header.current_dimstyle_handle.value() == old {
                    document.header.current_dimstyle_handle = new_h;
                }
                if document.header.dim_text_style_handle.value() == old {
                    document.header.dim_text_style_handle = new_h;
                }
            }
        }

        // ── Post-pass: guarantee the mandatory *Model_Space / *Paper_Space ──
        // block records exist and enumerate their geometry.
        //
        // The block-control table names the model/paper-space handles, but a
        // file can reach here without their BLOCK_HEADER ever materialising as a
        // record (absent from the object stream). The DWG writer emits a block's
        // contents by walking `BlockRecord::entity_handles`, so a missing record
        // — or one whose owned list stayed empty while entities point at it via
        // `owner_handle` — serialises to nothing, silently dropping that space's
        // geometry on the next save. Synthesize the missing records (the writer
        // fabricates their BLOCK/ENDBLK markers from the allocated handles) and
        // rebuild any empty owned-list from ownership so the round-trip is
        // lossless.
        {
            use std::collections::HashMap;
            for (h, is_model) in [
                (document.header.model_space_block_handle, true),
                (document.header.paper_space_block_handle, false),
            ] {
                if h.is_null() || document.block_records.iter().any(|br| br.handle == h) {
                    continue;
                }
                let mut br = if is_model {
                    crate::tables::BlockRecord::model_space()
                } else {
                    crate::tables::BlockRecord::paper_space()
                };
                // The captured handle may be POISON: a damaged file can point
                // its Layout at an object that is not a block record at all
                // (seen in the wild: BLOCK_CONTROL.model_space NULL and the
                // "Model" Layout pointing at the LAYER_CONTROL handle).
                // Synthesizing the record under that handle duplicates it in
                // the object stream on the next save — AutoCAD/ODA then follow
                // the handle, find the layer table, and abort the whole file.
                // Allocate a fresh handle instead and re-point the header and
                // the owning Layout at it.
                let collides = document.objects.contains_key(&h)
                    || document.layers.handle() == h
                    || document.line_types.handle() == h
                    || document.text_styles.handle() == h
                    || document.dim_styles.handle() == h
                    || document.layers.iter().any(|l| l.handle == h)
                    || document.get_entity(h).is_some();
                let h = if collides {
                    let fresh = document.allocate_handle();
                    if is_model {
                        document.header.model_space_block_handle = fresh;
                    } else {
                        document.header.paper_space_block_handle = fresh;
                    }
                    for obj in document.objects.values_mut() {
                        if let crate::objects::ObjectType::Layout(l) = obj {
                            if l.block_record == h {
                                l.block_record = fresh;
                            }
                        }
                    }
                    fresh
                } else {
                    h
                };
                br.handle = h;
                br.block_entity_handle = document.allocate_handle();
                br.block_end_handle = document.allocate_handle();
                // Cross-link the owning Layout object, if present, so the record
                // and its Layout reference each other like a normally-read pair.
                for (oh, obj) in document.objects.iter() {
                    if let crate::objects::ObjectType::Layout(l) = obj {
                        if l.block_record == h {
                            br.layout = *oh;
                            break;
                        }
                    }
                }
                let _ = document.block_records.add(br);
            }
            // Fill any empty owned-entity list from `owner_handle`, in document
            // (draw) order, excluding structural markers and INSERT sub-entities
            // — the same set the writer excludes.
            let mut by_owner: HashMap<Handle, Vec<Handle>> = HashMap::new();
            for e in &document.entities {
                if matches!(
                    e.as_ref(),
                    EntityType::Block(_) | EntityType::BlockEnd(_) | EntityType::AttributeEntity(_)
                ) {
                    continue;
                }
                let owner = e.common().owner_handle;
                if owner.is_null() {
                    continue;
                }
                by_owner.entry(owner).or_default().push(e.common().handle);
            }
            for br in document.block_records.iter_mut() {
                if br.entity_handles.is_empty() {
                    if let Some(list) = by_owner.get(&br.handle) {
                        br.entity_handles = list.clone();
                    }
                }
            }
        }

        // The current model-space annotation scale (CANNOSCALE) is not carried
        // in the DWG header stream, only in the AcDbVariableDictionary. Reflect
        // it into the header so consumers (and DXF export) see the real scale
        // rather than the "1:1" default.
        Self::reflect_annotation_scale(document);

        self.notifications
    }

    /// Populate the header's current annotation scale (CANNOSCALE) from the
    /// AcDbVariableDictionary — the DWG header stream omits it. Sets the scale
    /// name and, from the referenced AcDbScale, the numeric value
    /// (paper units / drawing units, e.g. "1:70" → 1/70).
    fn reflect_annotation_scale(document: &mut CadDocument) {
        let var_handle = document.objects.values().find_map(|o| match o {
            crate::objects::ObjectType::Dictionary(d) => d
                .entries
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("CANNOSCALE"))
                .map(|(_, vh)| *vh),
            _ => None,
        });
        let Some(vh) = var_handle else {
            return;
        };
        let name = match document.objects.get(&vh) {
            Some(crate::objects::ObjectType::DictionaryVariable(dv)) => dv.value.clone(),
            _ => return,
        };
        if name.trim().is_empty() {
            return;
        }
        let value = document.objects.values().find_map(|o| match o {
            crate::objects::ObjectType::Scale(s)
                if s.name.eq_ignore_ascii_case(&name) && s.drawing_units != 0.0 =>
            {
                Some(s.paper_units / s.drawing_units)
            }
            _ => None,
        });
        document.header.current_annotation_scale = name;
        if let Some(v) = value {
            document.header.annotation_scale_value = v;
        }
    }

    /// Process a single object record in Pass 2.
    fn process_pass2_record(
        &self,
        handle: u64,
        raw_type_code: i16,
        type_code: i16,
        mut reader: crate::io::dwg::dwg_stream_readers::merged_reader::DwgMergedReader,
        document: &mut CadDocument,
        maps: &HandleMaps,
        pending: &mut PendingPolylines,
        pending_attributes: &mut HashMap<u64, Vec<AttributeEntity>>,
        entity_class_numbers: &std::collections::HashSet<i16>,
    ) {
        // For class-based types (≥500) that weren't resolved via the class
        // map, check the class's is_an_entity flag.  This prevents misreading
        // object data as entity data (different binary layout).
        let is_entity = if type_code >= 500 {
            entity_class_numbers.contains(&type_code)
        } else {
            is_entity_type(type_code)
        };
        if is_entity {
            let entity_data = self
                .obj_reader
                .read_common_entity_data(&mut reader, type_code);
            let entity_common = map_entity_common(
                &entity_data,
                maps,
                document.header.model_space_block_handle,
                document.header.paper_space_block_handle,
            );

            match type_code {
                // ── Simple entities ────────────────────────────────
                OBJ_LINE => {
                    let data = entities::read_line(&mut reader, self.obj_reader.version());
                    let mut e = Line::new();
                    e.common = entity_common;
                    e.start = data.start;
                    e.end = data.end;
                    e.thickness = data.thickness;
                    e.normal = data.normal;
                    let _ = document.add_entity(EntityType::Line(e));
                }
                OBJ_POINT => {
                    let data = entities::read_point(&mut reader);
                    let mut e = Point::new();
                    e.common = entity_common;
                    e.location = data.location;
                    e.thickness = data.thickness;
                    e.normal = data.normal;
                    e.x_axis_angle = data.x_axis_angle;
                    let _ = document.add_entity(EntityType::Point(e));
                }
                OBJ_CIRCLE => {
                    let data = entities::read_circle(&mut reader);
                    let mut e = Circle::new();
                    e.common = entity_common;
                    e.center = data.center;
                    e.radius = data.radius;
                    e.thickness = data.thickness;
                    e.normal = data.normal;
                    let _ = document.add_entity(EntityType::Circle(e));
                }
                OBJ_LIGHT => {
                    let data = entities::read_light(&mut reader);
                    let mut e = Light::new();
                    e.common = entity_common;
                    e.name = data.name;
                    e.light_type = data.light_type;
                    e.position = data.position;
                    e.target = data.target;
                    // Preserve the raw record verbatim so write-back keeps the
                    // full photometric body (no native light encoder yet), just
                    // like the Surface / Unknown arms below.
                    e.dwg_type_code = type_code;
                    e.dwg_handle_bits = reader.get_handle_bits();
                    e.raw_dwg_data = Some(reader.raw_merged_data());
                    e.dwg_source_version = Some(document.version);
                    let _ = document.add_entity(EntityType::Light(e));
                }
                OBJ_ARC => {
                    let data = entities::read_arc(&mut reader);
                    let mut e = Arc::new();
                    e.common = entity_common;
                    e.center = data.center;
                    e.radius = data.radius;
                    e.start_angle = data.start_angle;
                    e.end_angle = data.end_angle;
                    e.thickness = data.thickness;
                    e.normal = data.normal;
                    let _ = document.add_entity(EntityType::Arc(e));
                }
                OBJ_ELLIPSE => {
                    let data = entities::read_ellipse(&mut reader);
                    let mut e = Ellipse::new();
                    e.common = entity_common;
                    e.center = data.center;
                    e.major_axis = data.major_axis;
                    e.minor_axis_ratio = data.minor_axis_ratio;
                    e.start_parameter = data.start_parameter;
                    e.end_parameter = data.end_parameter;
                    e.normal = data.normal;
                    let _ = document.add_entity(EntityType::Ellipse(e));
                }
                OBJ_RAY => {
                    let data = entities::read_ray(&mut reader);
                    let mut e = Ray::new(data.base_point, data.direction);
                    e.common = entity_common;
                    let _ = document.add_entity(EntityType::Ray(e));
                }
                OBJ_XLINE => {
                    let data = entities::read_xline(&mut reader);
                    let mut e = XLine::new(data.base_point, data.direction);
                    e.common = entity_common;
                    let _ = document.add_entity(EntityType::XLine(e));
                }
                OBJ_SOLID | OBJ_TRACE => {
                    let data = entities::read_solid(&mut reader);
                    let z = data.elevation;
                    let mut e = Solid::new(
                        crate::types::Vector3::new(data.first_corner.x, data.first_corner.y, z),
                        crate::types::Vector3::new(data.second_corner.x, data.second_corner.y, z),
                        crate::types::Vector3::new(data.third_corner.x, data.third_corner.y, z),
                        crate::types::Vector3::new(data.fourth_corner.x, data.fourth_corner.y, z),
                    );
                    e.common = entity_common;
                    e.thickness = data.thickness;
                    e.normal = data.normal;
                    let _ = document.add_entity(EntityType::Solid(e));
                }
                OBJ_3DFACE => {
                    let data = entities::read_face3d(&mut reader, self.obj_reader.version());
                    let mut e = Face3D::new(
                        data.first_corner,
                        data.second_corner,
                        data.third_corner,
                        data.fourth_corner,
                    );
                    // The reader already decoded the invisible-edge flags; the
                    // DXF path applies them but the DWG builder used to drop
                    // them, so file-hidden 3DFACE edges rendered visible.
                    e.invisible_edges = crate::entities::face3d::InvisibleEdgeFlags::from_bits(
                        data.invisible_edges as u8,
                    );
                    e.common = entity_common;
                    let _ = document.add_entity(EntityType::Face3D(e));
                }
                OBJ_SHAPE => {
                    let data = entities::read_shape(&mut reader);
                    let mut e = Shape::new();
                    e.common = entity_common;
                    e.insertion_point = data.insertion_point;
                    e.size = data.size;
                    e.rotation = data.rotation;
                    e.relative_x_scale = data.relative_x_scale;
                    e.oblique_angle = data.oblique_angle;
                    e.thickness = data.thickness;
                    e.shape_number = data.shape_number as i32;
                    e.normal = data.normal;
                    e.style_handle = Some(Handle::from(data.style_handle));
                    let _ = document.add_entity(EntityType::Shape(e));
                }

                // ── Moderate entities ──────────────────────────────
                OBJ_INSERT => {
                    let data = entities::read_insert(&mut reader, self.obj_reader.version());
                    let block_name = maps.block_name(data.block_handle);
                    let mut e = Insert::new(block_name, data.insert_point);
                    e.common = entity_common;
                    e.set_x_scale(data.x_scale);
                    e.set_y_scale(data.y_scale);
                    e.set_z_scale(data.z_scale);
                    e.rotation = data.rotation;
                    e.normal = data.normal;
                    let _ = document.add_entity(EntityType::Insert(e));
                }
                OBJ_MINSERT => {
                    let data = entities::read_minsert(&mut reader, self.obj_reader.version());
                    let block_name = maps.block_name(data.insert.block_handle);
                    let mut e = Insert::new(block_name, data.insert.insert_point);
                    e.common = entity_common;
                    e.set_x_scale(data.insert.x_scale);
                    e.set_y_scale(data.insert.y_scale);
                    e.set_z_scale(data.insert.z_scale);
                    e.rotation = data.insert.rotation;
                    e.normal = data.insert.normal;
                    e.column_count = data.column_count as u16;
                    e.row_count = data.row_count as u16;
                    e.column_spacing = data.column_spacing;
                    e.row_spacing = data.row_spacing;
                    let _ = document.add_entity(EntityType::Insert(e));
                }
                OBJ_TABLE => {
                    // ACAD_TABLE is INSERT-derived: the insert base positions the
                    // table and links it to the block that renders its cells; on
                    // R2010+ the inline table content (columns/rows/cells) follows.
                    let data = entities::read_table(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut e = crate::entities::Table::default();
                    e.common = entity_common;
                    e.insertion_point = data.insert.insert_point;
                    e.normal = data.insert.normal;
                    e.horizontal_direction = data.horizontal_direction;
                    if data.insert.block_handle != 0 {
                        e.block_record_handle = Some(Handle::from(data.insert.block_handle));
                    }
                    if data.style_handle != 0 {
                        e.table_style_handle = Some(Handle::from(data.style_handle));
                    }
                    e.columns = data.columns;
                    e.rows = data.rows;
                    let _ = document.add_entity(EntityType::Table(e));
                }
                OBJ_LWPOLYLINE => {
                    let data = entities::read_lwpolyline(&mut reader, self.obj_reader.version());
                    let mut e = LwPolyline::new();
                    e.common = entity_common;
                    e.vertices = data
                        .vertices
                        .into_iter()
                        .map(|v| crate::entities::lwpolyline::LwVertex {
                            location: crate::types::Vector2::new(v.x, v.y),
                            start_width: v.start_width,
                            end_width: v.end_width,
                            bulge: v.bulge,
                        })
                        .collect();
                    e.elevation = data.elevation;
                    e.thickness = data.thickness;
                    e.constant_width = data.constant_width;
                    e.normal = data.normal;
                    e.is_closed = (data.flag & 0x200) != 0;
                    e.plinegen = (data.flag & 0x100) != 0;
                    let _ = document.add_entity(EntityType::LwPolyline(e));
                }
                OBJ_SPLINE => {
                    let data = entities::read_spline(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut e = Spline::new();
                    e.common = entity_common;
                    e.degree = data.degree;
                    e.flags.rational = data.rational;
                    e.flags.closed = data.closed;
                    e.flags.periodic = data.periodic;
                    e.knots = data.knots;
                    e.control_points = data.control_points;
                    e.weights = data.weights;
                    e.fit_points = data.fit_points;
                    e.knot_tolerance = data.knot_tolerance;
                    e.control_tolerance = data.control_tolerance;
                    e.fit_tolerance = data.fit_tolerance;
                    e.begin_tangent = data.begin_tangent;
                    e.end_tangent = data.end_tangent;
                    e.knot_parameterization = data.knot_param;
                    let _ = document.add_entity(EntityType::Spline(e));
                }
                OBJ_HELIX => {
                    // HELIX = full spline record + helix parameters.
                    let data = entities::read_spline(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut e = crate::entities::Helix::new();
                    e.common = entity_common;
                    e.spline.degree = data.degree;
                    e.spline.flags.rational = data.rational;
                    e.spline.flags.closed = data.closed;
                    e.spline.flags.periodic = data.periodic;
                    e.spline.knots = data.knots;
                    e.spline.control_points = data.control_points;
                    e.spline.weights = data.weights;
                    e.spline.fit_points = data.fit_points;
                    e.spline.knot_tolerance = data.knot_tolerance;
                    e.spline.control_tolerance = data.control_tolerance;
                    e.spline.fit_tolerance = data.fit_tolerance;
                    e.spline.begin_tangent = data.begin_tangent;
                    e.spline.end_tangent = data.end_tangent;
                    e.spline.knot_parameterization = data.knot_param;
                    // AcDbHelix parameters follow the spline record.
                    e.major_version = reader.read_bit_long();
                    e.maintenance_version = reader.read_bit_long();
                    e.axis_base_point = reader.read_3bit_double();
                    e.start_point = reader.read_3bit_double();
                    e.axis_vector = reader.read_3bit_double();
                    e.radius = reader.read_bit_double();
                    e.turns = reader.read_bit_double();
                    e.turn_height = reader.read_bit_double();
                    e.handedness = reader.read_bit();
                    e.constraint = crate::entities::HelixConstraint::from_code(reader.read_byte());
                    let _ = document.add_entity(EntityType::Helix(e));
                }
                OBJ_TEXT => {
                    let data = entities::read_text(&mut reader, self.obj_reader.version());
                    let mut e = Text::new();
                    e.common = entity_common;
                    e.value = data.value;
                    e.insertion_point = data.insertion_point;
                    e.height = data.height;
                    e.horizontal_alignment = match data.horizontal_alignment {
                        1 => TextHorizontalAlignment::Center,
                        2 => TextHorizontalAlignment::Right,
                        3 => TextHorizontalAlignment::Aligned,
                        4 => TextHorizontalAlignment::Middle,
                        5 => TextHorizontalAlignment::Fit,
                        _ => TextHorizontalAlignment::Left,
                    };
                    e.vertical_alignment = match data.vertical_alignment {
                        1 => TextVerticalAlignment::Bottom,
                        2 => TextVerticalAlignment::Middle,
                        3 => TextVerticalAlignment::Top,
                        _ => TextVerticalAlignment::Baseline,
                    };
                    // Only set alignment_point when alignment mode actually uses it
                    e.alignment_point =
                        if data.horizontal_alignment != 0 || data.vertical_alignment != 0 {
                            Some(data.alignment_point)
                        } else {
                            None
                        };
                    e.rotation = data.rotation;
                    e.oblique_angle = data.oblique_angle;
                    e.width_factor = data.width_factor;
                    e.normal = data.normal;
                    e.style = maps.style_name(data.style_handle);
                    e.thickness = data.thickness;
                    e.generation_flags = data.generation;
                    let _ = document.add_entity(EntityType::Text(e));
                }
                OBJ_MTEXT => {
                    let data = entities::read_mtext(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut e = MText::new();
                    e.common = entity_common;
                    e.value = data.value;
                    e.insertion_point = data.insertion_point;
                    e.height = data.height;
                    e.rectangle_width = data.rectangle_width;
                    if data.rectangle_height != 0.0 {
                        e.rectangle_height = Some(data.rectangle_height);
                    }
                    e.normal = data.normal;
                    e.attachment_point = match data.attachment_point {
                        2 => AttachmentPoint::TopCenter,
                        3 => AttachmentPoint::TopRight,
                        4 => AttachmentPoint::MiddleLeft,
                        5 => AttachmentPoint::MiddleCenter,
                        6 => AttachmentPoint::MiddleRight,
                        7 => AttachmentPoint::BottomLeft,
                        8 => AttachmentPoint::BottomCenter,
                        9 => AttachmentPoint::BottomRight,
                        _ => AttachmentPoint::TopLeft,
                    };
                    e.drawing_direction = match data.drawing_direction {
                        2 => DrawingDirection::TopToBottom,
                        3 => DrawingDirection::ByStyle,
                        _ => DrawingDirection::LeftToRight,
                    };
                    // Compute rotation from x_direction vector
                    e.rotation = data.x_direction.y.atan2(data.x_direction.x);
                    e.line_spacing_factor = data.linespacing_factor;
                    e.line_spacing_style =
                        crate::entities::LineSpacingStyle::from(data.linespacing_style);
                    e.background_fill_flags = data.background_flags;
                    e.background_scale = data.background_scale;
                    e.background_color = data.background_color;
                    e.background_transparency = data.background_transparency;
                    e.is_annotative = data.is_annotative;
                    e.column_data = MTextColumnData {
                        column_type: data.column_type,
                        column_count: data.column_count,
                        flow_reversed: data.column_flow_reversed,
                        auto_height: data.column_auto_height,
                        width: data.column_width,
                        gutter: data.column_gutter,
                        heights: data.column_heights,
                    };
                    e.style = maps.style_name(data.style_handle);
                    let _ = document.add_entity(EntityType::MText(e));
                }
                OBJ_LEADER => {
                    let data = entities::read_leader(&mut reader, self.obj_reader.version());
                    let mut e = Leader::new();
                    e.common = entity_common;
                    e.vertices = data.vertices;
                    e.normal = data.normal;
                    e.horizontal_direction = data.horizontal_direction;
                    e.annotation_handle = Handle::from(data.annotation_handle);
                    e.dimension_style = maps.dimstyle_name(data.dimstyle_handle);
                    e.arrow_enabled = data.arrowhead_on;
                    e.path_type = LeaderPathType::from_value(data.path_type);
                    e.creation_type = LeaderCreationType::from_value(data.annotation_type);
                    e.hookline_direction =
                        HooklineDirection::from_value(data.hookline_on_x_dir as i16);
                    // text_height/text_width only present in DWG for versions < R2010
                    if !self.obj_reader.version().r2010_plus() {
                        e.text_height = data.text_height;
                        e.text_width = data.text_width;
                    }
                    e.block_offset = data.block_offset;
                    e.annotation_offset = data.annotation_offset;
                    let _ = document.add_entity(EntityType::Leader(e));
                }
                OBJ_TOLERANCE => {
                    let data = entities::read_tolerance(&mut reader, self.obj_reader.version());
                    let mut e = Tolerance::new();
                    e.common = entity_common;
                    e.insertion_point = data.insertion_point;
                    e.text = data.text;
                    e.direction = data.direction;
                    e.dimension_style_handle = Some(Handle::from(data.dimstyle_handle));
                    let _ = document.add_entity(EntityType::Tolerance(e));
                }

                // ── Complex entities ───────────────────────────────
                OBJ_HATCH => {
                    let data = entities::read_hatch(&mut reader, self.obj_reader.version());
                    let mut e = Hatch::new();
                    e.common = entity_common;
                    e.elevation = data.elevation;
                    e.normal = data.normal;
                    let mut pat = HatchPattern::new(&data.pattern_name);
                    pat.lines = data
                        .pattern_lines
                        .into_iter()
                        .map(|pl| crate::entities::hatch::HatchPatternLine {
                            angle: pl.angle,
                            base_point: pl.base_point,
                            offset: pl.offset,
                            dash_lengths: pl.dashes,
                        })
                        .collect();
                    e.pattern = pat;
                    e.is_solid = data.is_solid;
                    e.is_associative = data.is_associative;
                    e.is_double = data.is_double;
                    e.pattern_angle = data.pattern_angle;
                    e.pattern_scale = data.pattern_scale;
                    e.pattern_type = match data.pattern_type {
                        0 => crate::entities::hatch::HatchPatternType::UserDefined,
                        2 => crate::entities::hatch::HatchPatternType::Custom,
                        _ => crate::entities::hatch::HatchPatternType::Predefined,
                    };
                    e.style = match data.style {
                        1 => crate::entities::hatch::HatchStyleType::Outer,
                        2 => crate::entities::hatch::HatchStyleType::Ignore,
                        _ => crate::entities::hatch::HatchStyleType::Normal,
                    };
                    e.pixel_size = data.pixel_size;
                    // Collect boundary handle counts before consuming paths
                    let boundary_handle_counts: Vec<i32> =
                        data.paths.iter().map(|p| p.boundary_handle_count).collect();
                    // Convert DWG boundary paths to entity BoundaryPath
                    e.paths = data.paths.into_iter().map(|hp| {
                        use crate::entities::hatch::*;
                        let mut bp = BoundaryPath::with_flags(
                            BoundaryPathFlags::from_bits(hp.flags as u32),
                        );
                        // Polyline boundary path
                        if !hp.polyline_vertices.is_empty() {
                            let pe = PolylineEdge {
                                vertices: hp.polyline_vertices.iter()
                                    .map(|(pt, bulge)| crate::types::Vector3::new(pt.x, pt.y, *bulge))
                                    .collect(),
                                is_closed: hp.polyline_closed,
                            };
                            bp.add_edge(BoundaryEdge::Polyline(pe));
                        }
                        // Edge-type boundary path
                        for edge in hp.edges {
                            match edge {
                                crate::io::dwg::dwg_stream_readers::object_reader::entities::HatchEdge::Line(l) => {
                                    bp.add_edge(BoundaryEdge::Line(LineEdge {
                                        start: l.start,
                                        end: l.end,
                                    }));
                                }
                                crate::io::dwg::dwg_stream_readers::object_reader::entities::HatchEdge::Arc(a) => {
                                    bp.add_edge(BoundaryEdge::CircularArc(CircularArcEdge {
                                        center: a.center,
                                        radius: a.radius,
                                        start_angle: a.start_angle,
                                        end_angle: a.end_angle,
                                        counter_clockwise: a.ccw,
                                    }));
                                }
                                crate::io::dwg::dwg_stream_readers::object_reader::entities::HatchEdge::Ellipse(el) => {
                                    bp.add_edge(BoundaryEdge::EllipticArc(EllipticArcEdge {
                                        center: el.center,
                                        major_axis_endpoint: el.major_endpoint,
                                        minor_axis_ratio: el.minor_ratio,
                                        start_angle: el.start_angle,
                                        end_angle: el.end_angle,
                                        counter_clockwise: el.ccw,
                                    }));
                                }
                                crate::io::dwg::dwg_stream_readers::object_reader::entities::HatchEdge::Spline(s) => {
                                    bp.add_edge(BoundaryEdge::Spline(SplineEdge {
                                        degree: s.degree,
                                        rational: s.rational,
                                        periodic: s.periodic,
                                        knots: s.knots,
                                        control_points: s.control_points,
                                        fit_points: s.fit_points,
                                        start_tangent: s.start_tangent,
                                        end_tangent: s.end_tangent,
                                    }));
                                }
                            }
                        }
                        bp
                    }).collect();
                    e.seed_points = data.seed_points;
                    // Map gradient data
                    e.gradient_color.enabled = data.gradient_enabled;
                    e.gradient_color.reserved = data.gradient_reserved;
                    e.gradient_color.angle = data.gradient_angle;
                    e.gradient_color.shift = data.gradient_shift;
                    e.gradient_color.is_single_color = data.gradient_single_color;
                    e.gradient_color.color_tint = data.gradient_tint;
                    e.gradient_color.colors = data
                        .gradient_colors
                        .into_iter()
                        .map(
                            |(value, color)| crate::entities::hatch::GradientColorEntry {
                                value,
                                color,
                            },
                        )
                        .collect();
                    e.gradient_color.name = data.gradient_name;
                    // Read boundary object handles from handle stream
                    for (path, &count) in e.paths.iter_mut().zip(boundary_handle_counts.iter()) {
                        for _ in 0..count {
                            let h = reader.read_handle();
                            if h != 0 {
                                path.add_boundary_handle(Handle::new(h));
                            }
                        }
                    }
                    let _ = document.add_entity(EntityType::Hatch(e));
                }
                OBJ_VIEWPORT => {
                    let data = entities::read_viewport(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut e = Viewport::new();
                    e.common = entity_common;
                    e.center = data.center;
                    e.width = data.width;
                    e.height = data.height;
                    e.view_center =
                        crate::types::Vector3::new(data.view_center.x, data.view_center.y, 0.0);
                    e.view_direction = data.view_direction;
                    e.view_target = data.view_target;
                    e.view_height = data.view_height;
                    e.lens_length = data.lens_length;
                    e.front_clip_z = data.front_clip_z;
                    e.back_clip_z = data.back_clip_z;
                    e.twist_angle = data.twist_angle;
                    e.snap_angle = data.snap_angle;
                    e.snap_base =
                        crate::types::Vector3::new(data.snap_base.x, data.snap_base.y, 0.0);
                    e.snap_spacing =
                        crate::types::Vector3::new(data.snap_spacing.x, data.snap_spacing.y, 0.0);
                    e.grid_spacing =
                        crate::types::Vector3::new(data.grid_spacing.x, data.grid_spacing.y, 0.0);
                    e.circle_sides = data.circle_sides;
                    if self.obj_reader.version().r2007_plus() {
                        e.grid_major = data.grid_major;
                    }
                    e.status = ViewportStatusFlags::from_bits(data.status_flags);
                    e.render_mode = ViewportRenderMode::from_value(data.render_mode as i16);
                    e.ucs_per_viewport = data.ucs_per_viewport;
                    e.ucs_origin = data.ucs_origin;
                    e.ucs_x_axis = data.ucs_x_axis;
                    e.ucs_y_axis = data.ucs_y_axis;
                    e.elevation = data.ucs_elevation;
                    e.ucs_ortho_type = data.ucs_ortho_type;
                    if self.obj_reader.version().r2004_plus() {
                        e.shade_plot_mode = data.shade_plot_mode;
                    }
                    if self.obj_reader.version().r2007_plus() {
                        e.default_lighting = data.default_lighting;
                        e.default_lighting_type = data.default_lighting_type as i16;
                        e.brightness = data.brightness;
                        e.contrast = data.contrast;
                    }
                    // Read frozen layer handles
                    for _ in 0..data.frozen_layer_count {
                        let h = reader.read_handle();
                        if h != 0 {
                            e.frozen_layers.push(Handle::new(h));
                        }
                    }
                    // Clip-boundary handle (H 340): first entity-specific handle
                    // after the frozen layers. Non-NULL => the viewport is
                    // clipped by a boundary entity.
                    let clip = reader.read_handle();
                    if clip != 0 {
                        e.clip_boundary_handle = Handle::new(clip);
                    }
                    let _ = document.add_entity(EntityType::Viewport(e));
                }
                OBJ_POLYLINE_2D => {
                    let data = entities::read_polyline2d(&mut reader, self.obj_reader.version());
                    let mut e = Polyline2D::new();
                    e.common = entity_common;
                    e.flags = PolylineFlags::from_bits(data.flags as u16);
                    e.smooth_surface = SmoothSurfaceType::from(data.smooth_surface);
                    e.elevation = data.elevation;
                    e.thickness = data.thickness;
                    e.normal = data.normal;
                    e.start_width = data.start_width;
                    e.end_width = data.end_width;
                    let h = e.common.handle.value();
                    pending.polylines.push((h, EntityType::Polyline2D(e)));
                }
                OBJ_POLYLINE_3D => {
                    let data = entities::read_polyline3d(&mut reader, self.obj_reader.version());
                    let mut e = Polyline3D::new();
                    e.common = entity_common;
                    e.flags.closed = (data.closed_flag & 1) != 0;
                    // smooth_type was decoded by the reader but the builder used
                    // to drop it (spline/curve-fit 3D polylines lost their fit).
                    e.smooth_type = crate::entities::polyline3d::SmoothSurfaceType::from_value(
                        data.smooth_type as i16,
                    );
                    e.flags.spline_fit = data.smooth_type != 0;
                    let h = e.common.handle.value();
                    pending.polylines.push((h, EntityType::Polyline3D(e)));
                }

                // ── Dimension types ────────────────────────────────
                OBJ_DIMENSION_LINEAR => {
                    let data = entities::read_dimension_linear(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut dim = DimensionLinear::new(data.first_point, data.second_point);
                    dim.base.common = entity_common;
                    map_dimension_common(&mut dim.base, &data.common, &maps);
                    dim.definition_point = data.definition_point;
                    dim.base.definition_point = data.definition_point;
                    dim.rotation = data.rotation;
                    dim.ext_line_rotation = data.ext_line_rotation;
                    let _ = document.add_entity(EntityType::Dimension(Dimension::Linear(dim)));
                }
                OBJ_DIMENSION_ALIGNED => {
                    let data = entities::read_dimension_aligned(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut dim = DimensionAligned::new(data.first_point, data.second_point);
                    dim.base.common = entity_common;
                    map_dimension_common(&mut dim.base, &data.common, &maps);
                    dim.definition_point = data.definition_point;
                    dim.base.definition_point = data.definition_point;
                    dim.ext_line_rotation = data.ext_line_rotation;
                    let _ = document.add_entity(EntityType::Dimension(Dimension::Aligned(dim)));
                }
                OBJ_DIMENSION_RADIUS => {
                    let data = entities::read_dimension_radius(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut dim = DimensionRadius::new(data.angle_vertex, data.definition_point);
                    dim.base.common = entity_common;
                    map_dimension_common(&mut dim.base, &data.common, &maps);
                    dim.base.definition_point = data.definition_point;
                    dim.leader_length = data.leader_length;
                    let _ = document.add_entity(EntityType::Dimension(Dimension::Radius(dim)));
                }
                OBJ_DIMENSION_DIAMETER => {
                    let data = entities::read_dimension_diameter(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut dim = DimensionDiameter::new(data.angle_vertex, data.definition_point);
                    dim.base.common = entity_common;
                    map_dimension_common(&mut dim.base, &data.common, &maps);
                    dim.base.definition_point = data.definition_point;
                    dim.leader_length = data.leader_length;
                    let _ = document.add_entity(EntityType::Dimension(Dimension::Diameter(dim)));
                }
                OBJ_DIMENSION_ANG_2LN => {
                    let data = entities::read_dimension_angular_2ln(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut dim = DimensionAngular2Ln::default();
                    dim.base.common = entity_common;
                    map_dimension_common(&mut dim.base, &data.common, &maps);
                    dim.dimension_arc =
                        crate::types::Vector3::new(data.dimension_arc.x, data.dimension_arc.y, 0.0);
                    dim.first_point = data.first_point;
                    dim.second_point = data.second_point;
                    dim.angle_vertex = data.angle_vertex;
                    dim.definition_point = data.definition_point;
                    dim.base.definition_point = data.definition_point;
                    let _ = document.add_entity(EntityType::Dimension(Dimension::Angular2Ln(dim)));
                }
                OBJ_DIMENSION_ANG_3PT => {
                    let data = entities::read_dimension_angular_3pt(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut dim = DimensionAngular3Pt::default();
                    dim.base.common = entity_common;
                    map_dimension_common(&mut dim.base, &data.common, &maps);
                    dim.first_point = data.first_point;
                    dim.second_point = data.second_point;
                    dim.angle_vertex = data.angle_vertex;
                    dim.definition_point = data.definition_point;
                    dim.base.definition_point = data.definition_point;
                    let _ = document.add_entity(EntityType::Dimension(Dimension::Angular3Pt(dim)));
                }
                OBJ_DIMENSION_ORDINATE => {
                    let data = entities::read_dimension_ordinate(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut dim = DimensionOrdinate::new(
                        data.feature_location,
                        data.leader_endpoint,
                        data.is_ordinate_type_x,
                    );
                    dim.base.common = entity_common;
                    map_dimension_common(&mut dim.base, &data.common, &maps);
                    dim.definition_point = data.definition_point;
                    dim.base.definition_point = data.definition_point;
                    let _ = document.add_entity(EntityType::Dimension(Dimension::Ordinate(dim)));
                }

                OBJ_MLINE => {
                    let data = entities::read_mline(&mut reader);
                    let mut e = MLine::new();
                    e.common = entity_common;
                    e.scale_factor = data.scale_factor;
                    e.justification = MLineJustification::from(data.justification as i16);
                    e.start_point = data.start_point;
                    e.normal = data.normal;
                    e.style_element_count = data.lines_in_style as usize;
                    // Link the entity to its MLINESTYLE via the hard-pointer handle
                    // read from the handle stream. Without this the entity keeps the
                    // `MLine::new()` default ("Standard" / no handle), so a drawing's
                    // custom multiline style (element offsets, per-line colours and
                    // linetypes) is lost and the multiline is drawn with Standard's
                    // ±0.5 offsets in the entity colour.
                    if data.style_handle != 0 {
                        let sh = Handle::new(data.style_handle);
                        e.style_handle = Some(sh);
                        if let Some(crate::objects::ObjectType::MLineStyle(s)) =
                            document.objects.get(&sh)
                        {
                            e.style_name = s.name.clone();
                        }
                    }
                    // Populate vertices from parsed data
                    e.vertices = data
                        .vertices
                        .into_iter()
                        .map(|vd| {
                            use crate::entities::mline::{MLineSegment, MLineVertex};
                            let mut mv = MLineVertex::new(vd.position);
                            mv.direction = vd.direction;
                            mv.miter = vd.miter;
                            mv.segments = vd
                                .segments
                                .into_iter()
                                .map(|sd| MLineSegment {
                                    parameters: sd.parameters,
                                    area_fill_parameters: sd.area_fill_parameters,
                                })
                                .collect();
                            mv
                        })
                        .collect();
                    let _ = document.add_entity(EntityType::MLine(e));
                }

                OBJ_POLYLINE_PFACE => {
                    let (_num_verts, _num_faces, _owned_count) =
                        entities::read_polyface_mesh(&mut reader, self.obj_reader.version());
                    let mut e = PolyfaceMesh::new();
                    e.common = entity_common;
                    let h = e.common.handle.value();
                    pending.polylines.push((h, EntityType::PolyfaceMesh(e)));
                }

                OBJ_MESH => {
                    let data = entities::read_mesh(&mut reader);
                    let mut e = Mesh::new();
                    e.common = entity_common;
                    e.version = data.version;
                    e.blend_crease = data.blend_crease;
                    e.subdivision_level = data.subdivision_level;
                    e.vertices = data.vertices;
                    e.faces = data
                        .faces
                        .into_iter()
                        .map(|f| MeshFace {
                            vertices: f.into_iter().map(|v| v as usize).collect(),
                        })
                        .collect();
                    e.edges = data
                        .edges
                        .into_iter()
                        .map(|(a, b)| MeshEdge {
                            start: a as usize,
                            end: b as usize,
                            crease: None,
                        })
                        .collect();
                    let _ = document.add_entity(EntityType::Mesh(e));
                }

                OBJ_MULTILEADER => {
                    let data = entities::read_multileader(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut e = MultiLeader::new();
                    e.common = entity_common;
                    e.context = data.context;
                    e.style_handle = if data.style_handle != 0 {
                        Some(Handle::from(data.style_handle))
                    } else {
                        None
                    };
                    // Retain (not truncate) so flag bits the enum doesn't name
                    // are preserved for a lossless re-write.
                    e.property_override_flags = MultiLeaderPropertyOverrideFlags::from_bits_retain(
                        data.property_override_flags,
                    );
                    e.path_type = MultiLeaderPathType::from(data.path_type);
                    e.line_color = data.line_color;
                    e.line_type_handle = if data.line_type_handle != 0 {
                        Some(Handle::from(data.line_type_handle))
                    } else {
                        None
                    };
                    e.line_weight = LineWeight::from_value(data.line_weight as i16);
                    e.enable_landing = data.enable_landing;
                    e.enable_dogleg = data.enable_dogleg;
                    e.dogleg_length = data.dogleg_length;
                    e.arrowhead_handle = if data.arrowhead_handle != 0 {
                        Some(Handle::from(data.arrowhead_handle))
                    } else {
                        None
                    };
                    e.arrowhead_size = data.arrowhead_size;
                    e.content_type = LeaderContentType::from(data.content_type);
                    e.text_style_handle = if data.text_style_handle != 0 {
                        Some(Handle::from(data.text_style_handle))
                    } else {
                        None
                    };
                    e.text_left_attachment = TextAttachmentType::from(data.text_left_attachment);
                    e.text_right_attachment = TextAttachmentType::from(data.text_right_attachment);
                    e.text_angle_type = TextAngleType::from(data.text_angle_type);
                    e.text_alignment = TextAlignmentType::from(data.text_alignment);
                    e.text_color = data.text_color;
                    e.text_frame = data.text_frame;
                    e.block_content_handle = if data.block_content_handle != 0 {
                        Some(Handle::from(data.block_content_handle))
                    } else {
                        None
                    };
                    e.block_content_color = data.block_content_color;
                    e.block_scale = data.block_scale;
                    e.block_rotation = data.block_rotation;
                    e.block_connection_type =
                        BlockContentConnectionType::from(data.block_connection_type);
                    e.enable_annotation_scale = data.enable_annotation_scale;
                    e.block_attributes = data.block_attributes;
                    e.text_direction_negative = data.text_direction_negative;
                    e.text_align_in_ipe = data.text_align_in_ipe;
                    e.text_attachment_point =
                        TextAttachmentPointType::from(data.text_attachment_point);
                    e.scale_factor = data.scale_factor;
                    e.text_attachment_direction =
                        TextAttachmentDirectionType::from(data.text_attachment_direction);
                    e.text_bottom_attachment =
                        TextAttachmentType::from(data.text_bottom_attachment);
                    e.text_top_attachment = TextAttachmentType::from(data.text_top_attachment);
                    e.extend_leader_to_text = data.extend_leader_to_text;
                    // Preserve the raw record for verbatim write-back (native
                    // MLEADER encoder is not yet byte-exact).
                    e.dwg_handle_bits = reader.get_handle_bits();
                    e.raw_dwg_data = Some(reader.raw_merged_data());
                    e.dwg_source_version = Some(document.version);
                    let _ = document.add_entity(EntityType::MultiLeader(e));
                }

                // ── Attribute entities ─────────────────────────────
                OBJ_ATTDEF => {
                    let data = entities::read_attribute_definition(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut e = AttributeDefinition::new(
                        data.tag.clone(),
                        data.prompt.clone(),
                        data.text_data.value.clone(),
                    );
                    e.common = entity_common;
                    e.insertion_point = data.text_data.insertion_point;
                    e.height = data.text_data.height;
                    e.rotation = data.text_data.rotation;
                    // Carry the full text geometry the reader parsed — same as
                    // ATTRIB. Without these the attribute reverts to
                    // left/baseline default width/oblique/style and, crucially,
                    // loses its flags, so a CONSTANT attribute (whose value is
                    // drawn straight from the block, with no ATTRIB) is treated
                    // as a plain template and never rendered.
                    e.horizontal_alignment = match data.text_data.horizontal_alignment {
                        1 => HorizontalAlignment::Center,
                        2 => HorizontalAlignment::Right,
                        3 => HorizontalAlignment::Aligned,
                        4 => HorizontalAlignment::Middle,
                        5 => HorizontalAlignment::Fit,
                        _ => HorizontalAlignment::Left,
                    };
                    e.vertical_alignment = match data.text_data.vertical_alignment {
                        1 => VerticalAlignment::Bottom,
                        2 => VerticalAlignment::Middle,
                        3 => VerticalAlignment::Top,
                        _ => VerticalAlignment::Baseline,
                    };
                    e.alignment_point = if data.text_data.horizontal_alignment != 0
                        || data.text_data.vertical_alignment != 0
                    {
                        data.text_data.alignment_point
                    } else {
                        crate::types::Vector3::ZERO
                    };
                    e.width_factor = data.text_data.width_factor;
                    e.oblique_angle = data.text_data.oblique_angle;
                    e.normal = data.text_data.normal;
                    e.text_style = maps.style_name(data.text_data.style_handle);
                    e.flags = AttributeFlags::from_bits(data.flags as i32);
                    e.text_generation_flags = data.text_data.generation;
                    e.field_length = data.field_length;
                    e.lock_position = data.lock_position;
                    let _ = document.add_entity(EntityType::AttributeDefinition(e));
                }
                OBJ_ATTRIB => {
                    let data = entities::read_attribute_entity(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut e =
                        AttributeEntity::new(data.tag.clone(), data.text_data.value.clone());
                    e.common = entity_common;
                    e.insertion_point = data.text_data.insertion_point;
                    e.height = data.text_data.height;
                    e.rotation = data.text_data.rotation;
                    // Carry the full text geometry the reader parsed. Without
                    // these the attribute reverts to left/baseline with no
                    // alignment point (DataFlags 0x02|0x40), discarding the
                    // real placement — AutoCAD's R2018 reader rejects it.
                    e.horizontal_alignment = match data.text_data.horizontal_alignment {
                        1 => HorizontalAlignment::Center,
                        2 => HorizontalAlignment::Right,
                        3 => HorizontalAlignment::Aligned,
                        4 => HorizontalAlignment::Middle,
                        5 => HorizontalAlignment::Fit,
                        _ => HorizontalAlignment::Left,
                    };
                    e.vertical_alignment = match data.text_data.vertical_alignment {
                        1 => VerticalAlignment::Bottom,
                        2 => VerticalAlignment::Middle,
                        3 => VerticalAlignment::Top,
                        _ => VerticalAlignment::Baseline,
                    };
                    // Match the writer/reader convention: an alignment point is
                    // only meaningful when the text is not left/baseline.
                    e.alignment_point = if data.text_data.horizontal_alignment != 0
                        || data.text_data.vertical_alignment != 0
                    {
                        data.text_data.alignment_point
                    } else {
                        crate::types::Vector3::ZERO
                    };
                    e.width_factor = data.text_data.width_factor;
                    e.oblique_angle = data.text_data.oblique_angle;
                    e.normal = data.text_data.normal;
                    e.text_style = maps.style_name(data.text_data.style_handle);
                    // Carry the flag byte the reader parsed. Dropping it left
                    // `flags.invisible` false, so an attribute tagged invisible
                    // (ATTMODE 1 should hide it) was still drawn. Also carry the
                    // text-generation (backward / upside-down), field length and
                    // lock-position, which were likewise being discarded.
                    e.flags = AttributeFlags::from_bits(data.flags as i32);
                    e.text_generation_flags = data.text_data.generation;
                    e.field_length = data.field_length;
                    e.lock_position = data.lock_position;
                    // Collect pending — will be attached to parent INSERT
                    // after Pass 2 (owner_handle = INSERT handle).
                    pending_attributes
                        .entry(entity_data.owner_handle)
                        .or_default()
                        .push(e);
                }

                // ── Structural markers (BLOCK / ENDBLK / SEQEND) ──
                // These are DWG-internal structural entities. They mark
                // block boundaries and sequence terminators. They are
                // silently consumed — their information is already
                // represented by BlockRecord table entries.
                OBJ_BLOCK => {
                    // BLOCK entity: read block name after common entity data
                    let name = reader.read_variable_text();
                    let mut b = crate::entities::Block::new(name, crate::types::Vector3::ZERO);
                    b.common = entity_common;
                    let _ = document.add_entity(EntityType::Block(b));
                }
                OBJ_ENDBLK => {
                    // ENDBLK marks the end of a block definition.
                    let mut be = crate::entities::BlockEnd::new();
                    be.common = entity_common;
                    let _ = document.add_entity(EntityType::BlockEnd(be));
                }
                OBJ_SEQEND => {
                    // SEQEND terminates a polyline vertex or INSERT
                    // attribute sequence. Store the seqend handle so
                    // it can be preserved on the parent polyline.
                    entities::read_seqend(&mut reader);
                    pending
                        .seqends
                        .insert(entity_data.owner_handle, entity_common.handle);
                }

                // ── Vertex child entities ──────────────────────────
                // Vertex records are children of POLYLINE_2D,
                // POLYLINE_3D, POLYLINE_PFACE, or POLYLINE_MESH.
                // Collect vertex data and attach to parent polylines
                // in the post-processing step after Pass 2.
                OBJ_VERTEX_2D => {
                    let mut data = entities::read_vertex2d(&mut reader, self.obj_reader.version());
                    data.handle = entity_common.handle;
                    pending
                        .vertices
                        .entry(entity_data.owner_handle)
                        .or_default()
                        .push(PendingVertex::V2D(data));
                }
                OBJ_VERTEX_3D | OBJ_VERTEX_MESH => {
                    let mut data = entities::read_vertex3d(&mut reader);
                    data.handle = entity_common.handle;
                    pending
                        .vertices
                        .entry(entity_data.owner_handle)
                        .or_default()
                        .push(PendingVertex::V3D(data, entity_common));
                }
                OBJ_VERTEX_PFACE => {
                    let mut data = entities::read_vertex3d(&mut reader);
                    data.handle = entity_common.handle;
                    pending
                        .vertices
                        .entry(entity_data.owner_handle)
                        .or_default()
                        .push(PendingVertex::V3D(data, entity_common));
                }
                OBJ_VERTEX_PFACE_FACE => {
                    let mut data = entities::read_pface_face(&mut reader);
                    data.handle = entity_common.handle;
                    pending
                        .vertices
                        .entry(entity_data.owner_handle)
                        .or_default()
                        .push(PendingVertex::PfaceFace(data, entity_common));
                }

                // ── Underlay reference (PDF / DWF / DGN) ───────────
                code @ (OBJ_PDFUNDERLAY | OBJ_DWFUNDERLAY | OBJ_DGNUNDERLAY) => {
                    use crate::entities::underlay::{Underlay, UnderlayDisplayFlags, UnderlayType};
                    let utype = if code == OBJ_DWFUNDERLAY {
                        UnderlayType::Dwf
                    } else if code == OBJ_DGNUNDERLAY {
                        UnderlayType::Dgn
                    } else {
                        UnderlayType::Pdf
                    };
                    let data = entities::read_underlay(&mut reader);
                    let mut e = Underlay::new(utype);
                    e.common = entity_common;
                    e.normal = data.normal;
                    e.insertion_point = data.insertion_point;
                    e.rotation = data.rotation;
                    e.x_scale = data.x_scale;
                    e.y_scale = data.y_scale;
                    e.z_scale = data.z_scale;
                    let eflags = UnderlayDisplayFlags::from_bits_truncate(data.flags);
                    e.flags = eflags;
                    // The "clip inside" bit doubles as the clip-inversion flag.
                    e.clip_inverted = eflags.contains(UnderlayDisplayFlags::CLIP_INSIDE);
                    e.contrast = data.contrast;
                    e.fade = data.fade;
                    if data.definition_handle != 0 {
                        e.definition_handle = Handle::from(data.definition_handle);
                    }
                    e.clip_boundary_vertices = data.clip_boundary_vertices;
                    let _ = document.add_entity(EntityType::Underlay(e));
                }

                // ── Raster image / Wipeout ─────────────────────────
                OBJ_IMAGE => {
                    let data = entities::read_raster_image(&mut reader, self.obj_reader.version());
                    let mut e =
                        RasterImage::new("", data.insertion_point, data.size.x, data.size.y);
                    e.common = entity_common;
                    e.class_version = data.class_version;
                    e.u_vector = data.u_vector;
                    e.v_vector = data.v_vector;
                    e.flags = ImageDisplayFlags::from_bits_truncate(data.flags);
                    e.clipping_enabled = data.clipping_enabled;
                    e.brightness = data.brightness;
                    e.contrast = data.contrast;
                    e.fade = data.fade;
                    // Propagate clip boundary the same way Wipeout does — the
                    // parser used to discard the vertices, leaving the default
                    // boundary on the entity. Without this, clip regions
                    // shrink/expand by orders of magnitude on render.
                    e.clip_boundary = crate::entities::raster_image::ClipBoundary {
                        clip_type: if data.clip_type == 1 {
                            crate::entities::raster_image::ClipType::Rectangular
                        } else {
                            crate::entities::raster_image::ClipType::Polygonal
                        },
                        clip_mode: if data.clip_inverted {
                            crate::entities::raster_image::ClipMode::Inside
                        } else {
                            crate::entities::raster_image::ClipMode::Outside
                        },
                        vertices: data.clip_boundary_vertices,
                    };
                    if data.definition_handle != 0 {
                        e.definition_handle = Some(Handle::from(data.definition_handle));
                    }
                    if data.reactor_handle != 0 {
                        e.definition_reactor_handle = Some(Handle::from(data.reactor_handle));
                    }
                    let _ = document.add_entity(EntityType::RasterImage(e));
                }
                OBJ_WIPEOUT => {
                    let data = entities::read_wipeout(&mut reader, self.obj_reader.version());
                    let mut e = Wipeout::new();
                    e.common = entity_common;
                    e.class_version = data.class_version;
                    e.insertion_point = data.insertion_point;
                    e.u_vector = data.u_vector;
                    e.v_vector = data.v_vector;
                    e.size = data.size;
                    e.flags = WipeoutDisplayFlags::from_bits_truncate(data.flags);
                    e.clipping_enabled = data.clipping_enabled;
                    e.brightness = data.brightness;
                    e.contrast = data.contrast;
                    e.fade = data.fade;
                    e.clip_type = if data.clip_type == 1 {
                        crate::entities::WipeoutClipType::Rectangular
                    } else {
                        crate::entities::WipeoutClipType::Polygonal
                    };
                    e.clip_boundary_vertices = data.clip_boundary_vertices;
                    if data.definition_handle != 0 {
                        e.definition_handle = Some(Handle::from(data.definition_handle));
                    }
                    if data.reactor_handle != 0 {
                        e.definition_reactor_handle = Some(Handle::from(data.reactor_handle));
                    }
                    let _ = document.add_entity(EntityType::Wipeout(e));
                }

                // ── OLE2 Frame ──────────────────────────────────────
                OBJ_OLE2FRAME => {
                    let data = entities::read_ole2frame(&mut reader, self.obj_reader.version());
                    let mut e = Ole2Frame::new();
                    e.common = entity_common;
                    e.version = data.version;
                    e.upper_left_corner = data.upper_left;
                    e.lower_right_corner = data.lower_right;
                    e.binary_data = data.data;
                    e.dwg_mode = data.mode;
                    e.dwg_trailing_byte = data.trailing_byte;
                    let _ = document.add_entity(EntityType::Ole2Frame(e));
                }

                // ── Polygon mesh (POLYLINE with mesh flag) ──────────
                OBJ_POLYLINE_MESH => {
                    let (flags, smooth_type, m_count, n_count, m_smooth, n_smooth, _owned_count) =
                        entities::read_polygon_mesh(&mut reader, self.obj_reader.version());
                    let mut e = PolygonMeshEntity::new();
                    e.common = entity_common;
                    e.flags = PolygonMeshFlags::from_bits_truncate(flags);
                    e.m_vertex_count = m_count;
                    e.n_vertex_count = n_count;
                    e.m_smooth_density = m_smooth;
                    e.n_smooth_density = n_smooth;
                    e.smooth_type = SurfaceSmoothType::from_i16(smooth_type);
                    // Vertices will be assembled from VERTEX_MESH records
                    let poly_handle = entity_data.common.handle;
                    pending
                        .polylines
                        .push((poly_handle, EntityType::PolygonMesh(e)));
                }

                // ── ACIS entities (3DSOLID, REGION, BODY) ───────────
                OBJ_3DSOLID => {
                    let data = entities::read_acis_entity(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut e = Solid3D::new();
                    e.common = entity_common;
                    e.acis_data.version = if data.is_binary {
                        crate::entities::solid3d::AcisVersion::Version2
                    } else {
                        crate::entities::solid3d::AcisVersion::Version1
                    };
                    e.acis_data.sat_data = data.sat_data;
                    e.acis_data.sab_data = data.sab_data;
                    e.acis_data.is_binary = data.is_binary;
                    e.acis_data.revision = data.revision;
                    // A 3D solid has no insertion point of its own; the file's
                    // point field is usually zero. Prefer the ACIS placement
                    // origin so the reference reflects where the body sits.
                    e.point_of_reference = e.acis_data.placement_origin().unwrap_or(data.point);
                    e.wires = data.wires;
                    e.silhouettes = data.silhouettes;

                    // 3DSOLID R2007+: history_id handle
                    // (always present since R2007, regardless of ACIS version)
                    if self.obj_reader.version().r2007_plus() {
                        let h = reader.read_handle();
                        if h != 0 {
                            e.history_handle = Some(Handle::new(h));
                        }
                    }

                    let _ = document.add_entity(EntityType::Solid3D(e));
                }
                OBJ_REGION => {
                    let data = entities::read_acis_entity(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut e = Region::new();
                    e.common = entity_common;
                    e.acis_data.version = if data.is_binary {
                        crate::entities::solid3d::AcisVersion::Version2
                    } else {
                        crate::entities::solid3d::AcisVersion::Version1
                    };
                    e.acis_data.sat_data = data.sat_data;
                    e.acis_data.sab_data = data.sab_data;
                    e.acis_data.is_binary = data.is_binary;
                    e.acis_data.revision = data.revision;
                    e.point_of_reference = e.acis_data.placement_origin().unwrap_or(data.point);
                    e.wires = data.wires;
                    e.silhouettes = data.silhouettes;
                    // REGION R2007+: history_id handle (same slot as 3DSOLID).
                    if self.obj_reader.version().r2007_plus() {
                        let h = reader.read_handle();
                        if h != 0 {
                            e.history_handle = Some(Handle::new(h));
                        }
                    }
                    let _ = document.add_entity(EntityType::Region(e));
                }
                OBJ_BODY => {
                    let data = entities::read_acis_entity(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut e = Body::new();
                    e.common = entity_common;
                    e.acis_data.version = if data.is_binary {
                        crate::entities::solid3d::AcisVersion::Version2
                    } else {
                        crate::entities::solid3d::AcisVersion::Version1
                    };
                    e.acis_data.sat_data = data.sat_data;
                    e.acis_data.sab_data = data.sab_data;
                    e.acis_data.is_binary = data.is_binary;
                    e.acis_data.revision = data.revision;
                    e.point_of_reference = e.acis_data.placement_origin().unwrap_or(data.point);
                    e.wires = data.wires;
                    e.silhouettes = data.silhouettes;
                    // BODY R2007+: history_id handle (same slot as 3DSOLID).
                    if self.obj_reader.version().r2007_plus() {
                        let h = reader.read_handle();
                        if h != 0 {
                            e.history_handle = Some(Handle::new(h));
                        }
                    }
                    let _ = document.add_entity(EntityType::Body(e));
                }

                // ── ACAD_SURFACE family (ACIS-backed) ───────────────
                OBJ_SURFACE | OBJ_PLANESURFACE | OBJ_EXTRUDEDSURFACE | OBJ_LOFTEDSURFACE
                | OBJ_REVOLVEDSURFACE | OBJ_SWEPTSURFACE | OBJ_NURBSURFACE => {
                    let kind = match type_code {
                        OBJ_PLANESURFACE => crate::entities::SurfaceKind::Plane,
                        OBJ_EXTRUDEDSURFACE => crate::entities::SurfaceKind::Extruded,
                        OBJ_LOFTEDSURFACE => crate::entities::SurfaceKind::Lofted,
                        OBJ_REVOLVEDSURFACE => crate::entities::SurfaceKind::Revolved,
                        OBJ_SWEPTSURFACE => crate::entities::SurfaceKind::Swept,
                        OBJ_NURBSURFACE => crate::entities::SurfaceKind::Nurb,
                        _ => crate::entities::SurfaceKind::Generic,
                    };
                    let data = entities::read_acis_entity(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut e = Surface::new(kind);
                    e.common = entity_common;
                    e.acis_data.version = if data.is_binary {
                        crate::entities::solid3d::AcisVersion::Version2
                    } else {
                        crate::entities::solid3d::AcisVersion::Version1
                    };
                    e.acis_data.sat_data = data.sat_data;
                    e.acis_data.sab_data = data.sab_data;
                    e.acis_data.is_binary = data.is_binary;
                    e.acis_data.revision = data.revision;
                    e.wires = data.wires;
                    e.silhouettes = data.silhouettes;
                    // Preserve the raw object verbatim so DWG write-back keeps
                    // the original surface type (no native surface encoder yet).
                    e.dwg_handle_bits = reader.get_handle_bits();
                    e.raw_dwg_data = Some(reader.raw_merged_data());
                    e.dwg_source_version = Some(document.version);
                    let _ = document.add_entity(EntityType::Surface(e));
                }

                // ── Catch-all ──────────────────────────────────────
                _ => {
                    // Class numbers ≥500 are per-file; resolve the class name so
                    // the model-documentation decodes below are portable.
                    let cpp_class = document
                        .classes
                        .iter()
                        .find(|c| c.class_number == type_code)
                        .map(|c| c.cpp_class_name.as_str())
                        .unwrap_or("");
                    match cpp_class {
                        // AcDbSectionSymbol ("SECTIONLINE"): decode the section
                        // "A-A" mark for display. The reader is positioned at
                        // the class-specific data (common entity data already
                        // consumed), so the geometry reads cleanly from here.
                        // The raw bytes still drive verbatim write-back.
                        "AcDbSectionSymbol" => {
                            let mut e = decode_section_symbol(&mut reader)
                                .unwrap_or_else(SectionSymbol::new);
                            e.common = entity_common;
                            e.dwg_type_code = type_code;
                            e.dwg_handle_bits = reader.get_handle_bits();
                            e.raw_dwg_data = Some(reader.raw_merged_data());
                            e.dwg_source_version = Some(document.version);
                            let _ = document.add_entity(EntityType::SectionSymbol(e));
                        }
                        // AcDbViewBorder ("DRAWINGVIEW"): the view's paper
                        // rectangle / scale, and — as the first object-specific
                        // handle — the view's *active* viewport (the one
                        // carrying the real camera), the last hop of the
                        // section-mark viewing-direction chain.
                        "AcDbViewBorder" => {
                            let mut e = ViewBorder::new();
                            e.common = entity_common;
                            e.active_viewport = Handle::from(reader.read_handle());
                            // Paper placement: a version BL then a raw-double
                            // run — rectangle min/max corners, the view scale
                            // denominator, a reserved zero, and the view centre
                            // (redundantly the rectangle midpoint, which
                            // cross-validates the corner reads).
                            let _ver = reader.read_bit_long();
                            let min_x = reader.read_raw_double();
                            let min_y = reader.read_raw_double();
                            let max_x = reader.read_raw_double();
                            let max_y = reader.read_raw_double();
                            let scale = reader.read_raw_double();
                            let _reserved = reader.read_raw_double();
                            let cx = reader.read_raw_double();
                            let cy = reader.read_raw_double();
                            let all = [min_x, min_y, max_x, max_y, scale, cx, cy];
                            if all.iter().all(|v| v.is_finite() && v.abs() < 1.0e9)
                                && min_x < max_x
                                && min_y < max_y
                            {
                                e.min = [min_x, min_y];
                                e.max = [max_x, max_y];
                                e.center = [cx, cy];
                                e.scale = scale;
                            }
                            e.dwg_type_code = type_code;
                            e.dwg_handle_bits = reader.get_handle_bits();
                            e.raw_dwg_data = Some(reader.raw_merged_data());
                            e.dwg_source_version = Some(document.version);
                            let _ = document.add_entity(EntityType::ViewBorder(e));
                        }
                        _ => {
                            let mut e =
                                UnknownEntity::new(format!("DWG_TYPE_{}", type_code));
                            e.common = entity_common;
                            e.dwg_type_code = type_code;
                            e.dwg_handle_bits = reader.get_handle_bits();
                            e.raw_dwg_data = Some(reader.raw_merged_data());
                            e.dwg_source_version = Some(document.version);
                            let _ = document.add_entity(EntityType::Unknown(e));
                        }
                    }
                }
            }
        } else if !is_table_type(type_code) {
            // ── Non-graphical objects ──────────────────────────────
            let non_entity_data = self
                .obj_reader
                .read_common_non_entity_data(&mut reader, type_code);
            let owner_handle = Handle::from(non_entity_data.owner_handle);
            // Save raw EED blobs for DWG round-trip write-back
            if !non_entity_data.common.eed_raw.is_empty() {
                document.eed_by_handle.insert(
                    Handle::from(non_entity_data.common.handle),
                    non_entity_data.common.eed_raw.clone(),
                );
            }
            // Save xdictionary handle for DWG round-trip write-back
            if let Some(xdic) = non_entity_data.xdictionary_handle {
                document.xdic_by_handle.insert(
                    Handle::from(non_entity_data.common.handle),
                    Handle::from(xdic),
                );
            }
            // Save reactors for DWG round-trip write-back
            if !non_entity_data.reactors.is_empty() {
                document.reactors_by_handle.insert(
                    Handle::from(non_entity_data.common.handle),
                    non_entity_data
                        .reactors
                        .iter()
                        .map(|&h| Handle::from(h))
                        .collect(),
                );
            }

            match type_code {
                OBJ_DICTIONARY => {
                    let data = objects::read_dictionary(&mut reader, self.obj_reader.version());
                    let mut obj = crate::objects::Dictionary::new();
                    obj.handle = Handle::from(handle);
                    obj.owner = owner_handle;
                    obj.hard_owner = data.hard_owner;
                    obj.duplicate_cloning = data.duplicate_cloning;
                    for entry in data.entries {
                        obj.add_entry(entry.name, Handle::from(entry.handle));
                    }
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::Dictionary(obj),
                    );
                }
                OBJ_DICTIONARYWDFLT => {
                    let data = objects::read_dictionary_with_default(
                        &mut reader,
                        self.obj_reader.version(),
                    );
                    let mut obj = crate::objects::DictionaryWithDefault::new();
                    obj.handle = Handle::from(handle);
                    obj.owner = owner_handle;
                    obj.hard_owner = data.hard_owner;
                    obj.duplicate_cloning = data.duplicate_cloning;
                    obj.default_handle = Handle::from(data.default_handle);
                    for entry in data.entries {
                        obj.entries.push((entry.name, Handle::from(entry.handle)));
                    }
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::DictionaryWithDefault(obj),
                    );
                }
                OBJ_DICTIONARYVAR => {
                    let data = objects::read_dictionary_variable(&mut reader);
                    let mut obj = crate::objects::DictionaryVariable::new("", &data.value);
                    obj.handle = Handle::from(handle);
                    obj.owner_handle = owner_handle;
                    obj.schema_number = data.schema_number as i16;
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::DictionaryVariable(obj),
                    );
                }
                OBJ_LAYOUT => {
                    let data = objects::read_layout(&mut reader, self.obj_reader.version());
                    let mut obj = crate::objects::Layout::new(&data.name);
                    obj.handle = Handle::from(handle);
                    obj.owner = owner_handle;
                    obj.flags = data.flags;
                    obj.tab_order = data.tab_order as i16;
                    obj.min_limits = data.min_limits;
                    obj.max_limits = data.max_limits;
                    obj.insertion_base = (
                        data.insertion_base.x,
                        data.insertion_base.y,
                        data.insertion_base.z,
                    );
                    obj.min_extents = (data.min_extents.x, data.min_extents.y, data.min_extents.z);
                    obj.max_extents = (data.max_extents.x, data.max_extents.y, data.max_extents.z);
                    obj.elevation = data.elevation;
                    obj.ucs_origin = (data.ucs_origin.x, data.ucs_origin.y, data.ucs_origin.z);
                    obj.ucs_x_axis = (data.x_axis.x, data.x_axis.y, data.x_axis.z);
                    obj.ucs_y_axis = (data.y_axis.x, data.y_axis.y, data.y_axis.z);
                    obj.ucs_ortho_type = data.ucs_ortho_type;
                    obj.block_record = Handle::from(data.block_record_handle);
                    obj.viewport = Handle::from(data.viewport_handle);
                    obj.paper_width = data.plot_settings.paper_width;
                    obj.paper_height = data.plot_settings.paper_height;
                    obj.plot_rotation = data.plot_settings.rotation;
                    let ps = &data.plot_settings;
                    obj.plot_page_name = ps.page_name.clone();
                    obj.plot_printer_name = ps.printer_name.clone();
                    obj.paper_size = ps.paper_size.clone();
                    obj.plot_view_name = ps.plot_view_name.clone();
                    obj.plot_style_sheet = ps.current_style_sheet.clone();
                    obj.plot_margin_left = ps.left_margin;
                    obj.plot_margin_bottom = ps.bottom_margin;
                    obj.plot_margin_right = ps.right_margin;
                    obj.plot_margin_top = ps.top_margin;
                    obj.plot_origin_x = ps.origin_x;
                    obj.plot_origin_y = ps.origin_y;
                    obj.plot_window_min_x = ps.window_min_x;
                    obj.plot_window_min_y = ps.window_min_y;
                    obj.plot_window_max_x = ps.window_max_x;
                    obj.plot_window_max_y = ps.window_max_y;
                    obj.plot_paper_units = ps.paper_units;
                    obj.plot_type = ps.plot_type;
                    obj.plot_scale_numerator = ps.scale_numerator;
                    obj.plot_scale_denominator = ps.scale_denominator;
                    obj.plot_scale_type = ps.scale_type;
                    obj.plot_scale_factor = ps.scale_factor;
                    obj.paper_image_origin_x = ps.paper_image_x;
                    obj.paper_image_origin_y = ps.paper_image_y;
                    obj.shade_plot_mode = ps.shade_plot_mode;
                    obj.shade_plot_resolution = ps.shade_plot_resolution;
                    obj.shade_plot_dpi = ps.shade_plot_dpi;
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::Layout(obj),
                    );
                }
                OBJ_GROUP => {
                    let data = objects::read_group(&mut reader);
                    let mut obj = crate::objects::Group::new("");
                    obj.handle = Handle::from(handle);
                    obj.owner = owner_handle;
                    obj.description = data.description;
                    obj.selectable = data.selectable;
                    for eh in data.entity_handles {
                        obj.entities.push(Handle::from(eh));
                    }
                    document
                        .objects
                        .insert(Handle::from(handle), crate::objects::ObjectType::Group(obj));
                }
                OBJ_MLINESTYLE => {
                    let data = objects::read_mlinestyle(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut obj = crate::objects::MLineStyle::new(&data.name);
                    obj.handle = Handle::from(handle);
                    obj.owner = owner_handle;
                    obj.description = data.description;
                    obj.fill_color = data.fill_color;
                    obj.start_angle = data.start_angle;
                    obj.end_angle = data.end_angle;
                    // DWG binary swaps some flag pairs vs DXF:
                    //   DWG bit 1=DisplayJoints, 2=FillOn (DXF: 1=FillOn, 2=DisplayJoints)
                    //   DWG bit 0x20=StartRound, 0x40=StartInner (DXF: 0x20=StartInner, 0x40=StartRound)
                    //   DWG bit 0x200=EndRound, 0x400=EndInner (DXF: 0x200=EndInner, 0x400=EndRound)
                    let f = data.flags as i32;
                    obj.flags = crate::objects::MLineStyleFlags {
                        display_joints: (f & 1) != 0,
                        fill_on: (f & 2) != 0,
                        start_square_cap: (f & 16) != 0,
                        start_round_cap: (f & 0x20) != 0,
                        start_inner_arcs_cap: (f & 0x40) != 0,
                        end_square_cap: (f & 0x100) != 0,
                        end_round_cap: (f & 0x200) != 0,
                        end_inner_arcs_cap: (f & 0x400) != 0,
                    };
                    // Transfer elements
                    obj.elements = data
                        .elements
                        .iter()
                        .map(|e| {
                            let linetype = if self
                                .obj_reader
                                .version()
                                .r2018_plus(self.obj_reader.dxf_version())
                            {
                                maps.linetypes
                                    .get(&e.linetype_index_or_handle)
                                    .cloned()
                                    .unwrap_or_else(|| "BYLAYER".to_string())
                            } else {
                                // Pre-R2018: linetype index (0 = BYLAYER)
                                "BYLAYER".to_string()
                            };
                            crate::objects::MLineStyleElement::full(e.offset, e.color, linetype)
                        })
                        .collect();
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::MLineStyle(obj),
                    );
                }
                OBJ_XRECORD => {
                    let data = objects::read_xrecord(&mut reader);
                    let mut obj = crate::objects::XRecord::new();
                    obj.handle = Handle::from(handle);
                    obj.owner = owner_handle;
                    obj.cloning_flags =
                        crate::objects::DictionaryCloningFlags::from_value(data.cloning_flags);
                    obj.raw_data = data.raw_data;
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::XRecord(obj),
                    );
                }
                OBJ_PLOTSETTINGS => {
                    let data =
                        objects::read_plot_settings_obj(&mut reader, self.obj_reader.version());
                    let mut obj = crate::objects::PlotSettings::new(&data.page_name);
                    obj.handle = Handle::from(handle);
                    obj.owner = owner_handle;
                    obj.printer_name = data.printer_name;
                    obj.paper_size = data.paper_size;
                    obj.plot_view_name = data.plot_view_name;
                    obj.current_style_sheet = data.current_style_sheet;
                    obj.paper_width = data.paper_width;
                    obj.paper_height = data.paper_height;
                    obj.margins = crate::objects::PaperMargin::new(
                        data.left_margin,
                        data.bottom_margin,
                        data.right_margin,
                        data.top_margin,
                    );
                    obj.origin_x = data.origin_x;
                    obj.origin_y = data.origin_y;
                    obj.plot_window = crate::objects::PlotWindow::new(
                        data.window_min_x,
                        data.window_min_y,
                        data.window_max_x,
                        data.window_max_y,
                    );
                    obj.scale_numerator = data.scale_numerator;
                    obj.scale_denominator = data.scale_denominator;
                    obj.paper_units = crate::objects::PlotPaperUnits::from_code(data.paper_units);
                    obj.rotation = crate::objects::PlotRotation::from_code(data.rotation);
                    obj.plot_type = crate::objects::PlotType::from_code(data.plot_type);
                    obj.scale_type = crate::objects::ScaledType::from_code(data.scale_type);
                    obj.shade_plot_mode =
                        crate::objects::ShadePlotMode::from_code(data.shade_plot_mode);
                    obj.shade_plot_resolution = crate::objects::ShadePlotResolutionLevel::from_code(
                        data.shade_plot_resolution,
                    );
                    obj.shade_plot_dpi = data.shade_plot_dpi;
                    obj.flags = crate::objects::PlotFlags::from_bits(data.plot_flags as i32);
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::PlotSettings(obj),
                    );
                }
                OBJ_MLEADERSTYLE => {
                    let data = objects::read_multileader_style(
                        &mut reader,
                        self.obj_reader.version(),
                        self.obj_reader.dxf_version(),
                    );
                    let mut obj = crate::objects::MultiLeaderStyle::new("");
                    obj.handle = Handle::from(handle);
                    obj.owner_handle = owner_handle;
                    obj.description = data.description;
                    obj.content_type = crate::objects::LeaderContentType::from(data.content_type);
                    obj.multileader_draw_order =
                        crate::objects::MultiLeaderDrawOrderType::from(data.multileader_draw_order);
                    obj.leader_draw_order =
                        crate::objects::LeaderDrawOrderType::from(data.leader_draw_order);
                    obj.max_leader_points = data.max_leader_points;
                    obj.first_segment_angle = data.first_segment_angle;
                    obj.second_segment_angle = data.second_segment_angle;
                    obj.path_type = crate::objects::MultiLeaderPathType::from(data.path_type);
                    obj.line_color = data.line_color;
                    obj.line_type_handle = if data.line_type_handle != 0 {
                        Some(Handle::from(data.line_type_handle))
                    } else {
                        None
                    };
                    obj.line_weight = LineWeight::from_value(data.line_weight as i16);
                    obj.enable_landing = data.enable_landing;
                    obj.landing_gap = data.landing_gap;
                    obj.enable_dogleg = data.enable_dogleg;
                    obj.landing_distance = data.landing_distance;
                    obj.arrowhead_handle = if data.arrowhead_handle != 0 {
                        Some(Handle::from(data.arrowhead_handle))
                    } else {
                        None
                    };
                    obj.arrowhead_size = data.arrowhead_size;
                    obj.default_text = data.default_text;
                    obj.text_style_handle = if data.text_style_handle != 0 {
                        Some(Handle::from(data.text_style_handle))
                    } else {
                        None
                    };
                    obj.text_left_attachment =
                        crate::objects::TextAttachmentType::from(data.text_left_attachment);
                    obj.text_right_attachment =
                        crate::objects::TextAttachmentType::from(data.text_right_attachment);
                    obj.text_angle_type = crate::objects::TextAngleType::from(data.text_angle_type);
                    obj.text_alignment =
                        crate::objects::TextAlignmentType::from(data.text_alignment);
                    obj.text_color = data.text_color;
                    obj.text_height = data.text_height;
                    obj.text_frame = data.text_frame;
                    obj.text_always_left = data.text_always_left;
                    obj.align_space = data.align_space;
                    obj.block_content_handle = if data.block_content_handle != 0 {
                        Some(Handle::from(data.block_content_handle))
                    } else {
                        None
                    };
                    obj.block_content_color = data.block_content_color;
                    obj.block_content_scale_x = data.block_content_scale_x;
                    obj.block_content_scale_y = data.block_content_scale_y;
                    obj.block_content_scale_z = data.block_content_scale_z;
                    obj.enable_block_scale = data.enable_block_scale;
                    obj.block_content_rotation = data.block_content_rotation;
                    obj.enable_block_rotation = data.enable_block_rotation;
                    obj.block_content_connection = crate::objects::BlockContentConnectionType::from(
                        data.block_content_connection,
                    );
                    obj.scale_factor = data.scale_factor;
                    obj.property_changed = data.property_changed;
                    obj.is_annotative = data.is_annotative;
                    obj.break_gap_size = data.break_gap_size;
                    obj.text_attachment_direction =
                        crate::objects::TextAttachmentDirectionType::from(
                            data.text_attachment_direction,
                        );
                    obj.text_top_attachment =
                        crate::objects::TextAttachmentType::from(data.text_top_attachment);
                    obj.text_bottom_attachment =
                        crate::objects::TextAttachmentType::from(data.text_bottom_attachment);
                    obj.unknown_flag_298 = data.unknown_flag_298;
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::MultiLeaderStyle(obj),
                    );
                }
                OBJ_IMAGEDEF => {
                    let data = objects::read_image_definition(&mut reader);
                    let mut obj = crate::objects::ImageDefinition::new(&data.file_name);
                    obj.handle = Handle::from(handle);
                    obj.owner = owner_handle;
                    obj.class_version = data.class_version;
                    obj.is_loaded = data.is_loaded;
                    obj.size_in_pixels =
                        (data.size_in_pixels.x as u32, data.size_in_pixels.y as u32);
                    obj.pixel_size = (data.pixel_size.x, data.pixel_size.y);
                    obj.resolution_unit =
                        crate::objects::ResolutionUnit::from_code(data.resolution_unit as i32);
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::ImageDefinition(obj),
                    );
                }
                code @ (OBJ_PDFDEFINITION | OBJ_DWFDEFINITION | OBJ_DGNDEFINITION) => {
                    use crate::entities::underlay::UnderlayType;
                    let utype = if code == OBJ_DWFDEFINITION {
                        UnderlayType::Dwf
                    } else if code == OBJ_DGNDEFINITION {
                        UnderlayType::Dgn
                    } else {
                        UnderlayType::Pdf
                    };
                    let data = objects::read_underlay_definition(&mut reader);
                    let mut obj = crate::objects::UnderlayDefinition::new(utype);
                    obj.handle = Handle::from(handle);
                    obj.owner_handle = owner_handle;
                    obj.file_path = data.file_path;
                    obj.page_name = data.page_name;
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::UnderlayDefinition(obj),
                    );
                }
                OBJ_IMAGEDEFREACTOR => {
                    let _data = objects::read_image_definition_reactor(&mut reader);
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::ImageDefinitionReactor(
                            crate::objects::ImageDefinitionReactor {
                                handle: Handle::from(handle),
                                owner: owner_handle,
                                image_handle: Handle::NULL,
                            },
                        ),
                    );
                }
                OBJ_SCALE => {
                    let data = objects::read_scale(&mut reader);
                    let mut obj = crate::objects::Scale::new(
                        &data.name,
                        data.paper_units,
                        data.drawing_units,
                    );
                    obj.handle = Handle::from(handle);
                    obj.owner_handle = owner_handle;
                    obj.is_unit_scale = data.is_unit_scale;
                    document
                        .objects
                        .insert(Handle::from(handle), crate::objects::ObjectType::Scale(obj));
                }
                OBJ_SORTENTSTABLE => {
                    let data = objects::read_sort_entities_table(&mut reader);
                    let mut obj = crate::objects::SortEntitiesTable::new();
                    obj.handle = Handle::from(handle);
                    obj.owner_handle = owner_handle;
                    obj.block_owner_handle = Handle::from(data.block_owner_handle);
                    for entry in data.entries {
                        obj.add_entry(
                            Handle::from(entry.entity_handle),
                            Handle::from(entry.sort_handle),
                        );
                    }
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::SortEntitiesTable(obj),
                    );
                }
                OBJ_RASTERVARIABLES => {
                    let data = objects::read_raster_variables(&mut reader);
                    let obj = crate::objects::RasterVariables {
                        handle: Handle::from(handle),
                        owner: owner_handle,
                        class_version: data.class_version,
                        display_image_frame: data.display_image_frame,
                        image_quality: data.image_quality,
                        units: data.units,
                    };
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::RasterVariables(obj),
                    );
                }
                OBJ_DBCOLOR => {
                    let data = objects::read_book_color(&mut reader);
                    let obj = crate::objects::BookColor {
                        handle: Handle::from(handle),
                        owner: owner_handle,
                        color_name: data.color_name,
                        book_name: data.book_name,
                    };
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::BookColor(obj),
                    );
                }
                OBJ_PLACEHOLDER => {
                    objects::read_placeholder(&mut reader);
                    let obj = crate::objects::PlaceHolder {
                        handle: Handle::from(handle),
                        owner: owner_handle,
                    };
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::PlaceHolder(obj),
                    );
                }
                OBJ_WIPEOUTVARIABLES => {
                    let data = objects::read_wipeout_variables(&mut reader);
                    let obj = crate::objects::WipeoutVariables {
                        handle: Handle::from(handle),
                        owner: owner_handle,
                        display_frame: data.display_frame,
                    };
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::WipeoutVariables(obj),
                    );
                }
                OBJ_GEODATA => {
                    let data = objects::read_geodata(&mut reader);
                    let mut obj = crate::objects::GeoData::new();
                    obj.handle = Handle::from(handle);
                    obj.owner = owner_handle;
                    obj.version = data.version;
                    obj.host_block = Handle::from(data.host_block);
                    obj.coordinate_type = data.coordinate_type;
                    obj.design_point = data.design_point;
                    obj.reference_point = data.reference_point;
                    obj.north_direction = data.north_direction;
                    obj.up_direction = data.up_direction;
                    obj.horizontal_unit_scale = data.horizontal_unit_scale;
                    obj.vertical_unit_scale = data.vertical_unit_scale;
                    obj.horizontal_units = data.horizontal_units;
                    obj.vertical_units = data.vertical_units;
                    obj.scale_estimation_method = data.scale_estimation_method;
                    obj.user_scale_factor = data.user_scale_factor;
                    obj.sea_level_correction = data.sea_level_correction;
                    obj.sea_level_elevation = data.sea_level_elevation;
                    obj.coordinate_projection_radius = data.coordinate_projection_radius;
                    obj.coordinate_system_definition = data.coordinate_system_definition;
                    obj.geo_rss_tag = data.geo_rss_tag;
                    obj.observation_from_tag = data.observation_from_tag;
                    obj.observation_to_tag = data.observation_to_tag;
                    obj.observation_coverage_tag = data.observation_coverage_tag;
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::GeoData(obj),
                    );
                }
                OBJ_SPATIALFILTER => {
                    let data = objects::read_spatial_filter(&mut reader);
                    let mut obj = crate::objects::SpatialFilter::new();
                    obj.handle = Handle::from(handle);
                    obj.owner = owner_handle;
                    obj.boundary_points = data.points;
                    obj.normal = data.extrusion;
                    obj.origin = data.clip_bound_origin;
                    obj.display_enabled = data.display_enabled;
                    obj.front_clip = data.front_clip;
                    obj.back_clip = data.back_clip;
                    obj.inverse_block_transform =
                        matrix_from_row_major(&data.inverse_block_transform);
                    obj.clip_bound_transform = matrix_from_row_major(&data.clip_bound_transform);
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::SpatialFilter(obj),
                    );
                }
                OBJ_BLOCKVISIBILITYPARAMETER => {
                    // Parse the visibility states into the side map, then still
                    // store the object verbatim as Unknown so DWG round-trip is
                    // byte-exact (no typed writer needed).
                    let mut param = objects::read_block_visibility_parameter(&mut reader);
                    param.handle = Handle::from(handle);
                    param.owner = owner_handle;
                    document
                        .block_visibility_params
                        .insert(Handle::from(handle), param);

                    let type_name = Self::unknown_object_type_name(document, raw_type_code);
                    let raw_handle_bits = reader.get_handle_bits();
                    let raw_data = reader.raw_merged_data();
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::Unknown {
                            type_name,
                            handle: Handle::from(handle),
                            owner: owner_handle,
                            raw_dxf_codes: None,
                            raw_dwg_data: Some(raw_data),
                            raw_dwg_handle_bits: raw_handle_bits,
                            raw_dwg_version: Some(document.version),
                        },
                    );
                }
                OBJ_BLOCKREPRESENTATIONDATA => {
                    let block = objects::read_block_representation_data(&mut reader);
                    document
                        .block_representations
                        .insert(Handle::from(handle), block);

                    let type_name = Self::unknown_object_type_name(document, raw_type_code);
                    let raw_handle_bits = reader.get_handle_bits();
                    let raw_data = reader.raw_merged_data();
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::Unknown {
                            type_name,
                            handle: Handle::from(handle),
                            owner: owner_handle,
                            raw_dxf_codes: None,
                            raw_dwg_data: Some(raw_data),
                            raw_dwg_handle_bits: raw_handle_bits,
                            raw_dwg_version: Some(document.version),
                        },
                    );
                }
                OBJ_FIELD => {
                    // Decode the evaluator id + field-code + referenced-object
                    // handles into the side map; keep the object verbatim as
                    // Unknown for round-trip.
                    let data = objects::read_field(&mut reader);
                    document.fields.insert(
                        Handle::from(handle),
                        crate::document::FieldDef {
                            handle: Handle::from(handle),
                            owner: owner_handle,
                            evaluator: data.id,
                            code: data.code,
                            objects: data.objects.into_iter().map(Handle::from).collect(),
                        },
                    );
                    let type_name = Self::unknown_object_type_name(document, raw_type_code);
                    let raw_handle_bits = reader.get_handle_bits();
                    let raw_data = reader.raw_merged_data();
                    document.objects.insert(
                        Handle::from(handle),
                        crate::objects::ObjectType::Unknown {
                            type_name,
                            handle: Handle::from(handle),
                            owner: owner_handle,
                            raw_dxf_codes: None,
                            raw_dwg_data: Some(raw_data),
                            raw_dwg_handle_bits: raw_handle_bits,
                            raw_dwg_version: Some(document.version),
                        },
                    );
                }
                _ => {
                    // Annotative object-context leaves (*OBJECTCONTEXTDATA) carry
                    // their annotation scale as the FIRST object-specific handle
                    // (right after the common owner/reactors/xdict handles that
                    // read_common_non_entity_data already consumed). The
                    // data-stream and handle-stream read cursors are independent,
                    // and raw_merged_data()/get_handle_bits() snapshot the whole
                    // object independent of either cursor — so we can decode the
                    // fields we understand AND still capture the verbatim record.
                    let class_name: Option<String> = document
                        .classes
                        .iter()
                        .find(|c| c.class_number == type_code)
                        .map(|c| c.dxf_name.to_uppercase());
                    let is_context_data = class_name
                        .as_deref()
                        .map(|n| n.contains("OBJECTCONTEXTDATA"))
                        .unwrap_or(false);

                    // For the context leaves whose placement payload we model,
                    // decode into a typed ObjectContextData (lets OCS create /
                    // edit them). The verbatim `source_raw` is still kept so
                    // reading an existing file re-emits it byte-for-byte.
                    let modeled = if is_context_data {
                        match class_name.as_deref().unwrap_or("") {
                            "ACDB_BLKREFOBJECTCONTEXTDATA_CLASS" => {
                                let class_version = reader.read_bit_short();
                                let is_default = reader.read_bit();
                                let rotation = reader.read_bit_double();
                                let insertion = crate::types::Vector3::new(
                                    reader.read_bit_double(),
                                    reader.read_bit_double(),
                                    reader.read_bit_double(),
                                );
                                let scale_factor = crate::types::Vector3::new(
                                    reader.read_bit_double(),
                                    reader.read_bit_double(),
                                    reader.read_bit_double(),
                                );
                                let scale = reader.read_handle();
                                Some((
                                    crate::objects::ObjectContextKind::BlkRef {
                                        rotation,
                                        insertion,
                                        scale_factor,
                                    },
                                    class_version,
                                    is_default,
                                    scale,
                                ))
                            }
                            "ACDB_TEXTOBJECTCONTEXTDATA_CLASS" => {
                                let class_version = reader.read_bit_short();
                                let is_default = reader.read_bit();
                                let horizontal_mode = reader.read_bit_short();
                                let rotation = reader.read_bit_double();
                                let insertion = crate::types::Vector2::new(
                                    reader.read_raw_double(),
                                    reader.read_raw_double(),
                                );
                                let alignment = crate::types::Vector2::new(
                                    reader.read_raw_double(),
                                    reader.read_raw_double(),
                                );
                                let scale = reader.read_handle();
                                Some((
                                    crate::objects::ObjectContextKind::Text {
                                        horizontal_mode,
                                        rotation,
                                        insertion,
                                        alignment,
                                    },
                                    class_version,
                                    is_default,
                                    scale,
                                ))
                            }
                            "ACDB_MTEXTOBJECTCONTEXTDATA_CLASS" => {
                                let class_version = reader.read_bit_short();
                                let is_default = reader.read_bit();
                                let attachment = reader.read_bit_long();
                                // Binary stores x_axis_dir BEFORE ins_pt.
                                let x_axis_dir = crate::types::Vector3::new(
                                    reader.read_bit_double(),
                                    reader.read_bit_double(),
                                    reader.read_bit_double(),
                                );
                                let insertion = crate::types::Vector3::new(
                                    reader.read_bit_double(),
                                    reader.read_bit_double(),
                                    reader.read_bit_double(),
                                );
                                let rect_width = reader.read_bit_double();
                                let rect_height = reader.read_bit_double();
                                let extents_width = reader.read_bit_double();
                                let extents_height = reader.read_bit_double();
                                let column_type = reader.read_bit_long();
                                let columns = if column_type != 0 {
                                    let num_heights = reader.read_bit_long();
                                    let width = reader.read_bit_double();
                                    let gutter = reader.read_bit_double();
                                    let auto_height = reader.read_bit();
                                    let flow_reversed = reader.read_bit();
                                    let heights = if !auto_height && column_type == 2 {
                                        (0..num_heights.max(0))
                                            .map(|_| reader.read_bit_double())
                                            .collect()
                                    } else {
                                        Vec::new()
                                    };
                                    Some(crate::objects::MTextColumns {
                                        num_heights,
                                        width,
                                        gutter,
                                        auto_height,
                                        flow_reversed,
                                        heights,
                                    })
                                } else {
                                    None
                                };
                                let scale = reader.read_handle();
                                Some((
                                    crate::objects::ObjectContextKind::MText(
                                        crate::objects::MTextContext {
                                            attachment,
                                            x_axis_dir,
                                            insertion,
                                            rect_width,
                                            rect_height,
                                            extents_width,
                                            extents_height,
                                            column_type,
                                            columns,
                                        },
                                    ),
                                    class_version,
                                    is_default,
                                    scale,
                                ))
                            }
                            "ACDB_ALDIMOBJECTCONTEXTDATA_CLASS"
                            | "ACDB_ANGDIMOBJECTCONTEXTDATA_CLASS"
                            | "ACDB_DMDIMOBJECTCONTEXTDATA_CLASS"
                            | "ACDB_RADIMOBJECTCONTEXTDATA_CLASS"
                            | "ACDB_RADIMLGOBJECTCONTEXTDATA_CLASS"
                            | "ACDB_ORDDIMOBJECTCONTEXTDATA_CLASS" => {
                                let class_version = reader.read_bit_short();
                                let is_default = reader.read_bit();
                                // AcDbDimensionObjectContextData base (data stream).
                                let def_pt = crate::types::Vector2::new(
                                    reader.read_raw_double(),
                                    reader.read_raw_double(),
                                );
                                let is_def_textloc = reader.read_bit();
                                let text_rotation = reader.read_bit_double();
                                let b293 = reader.read_bit();
                                let dimtofl = reader.read_bit();
                                let dimosxd = reader.read_bit();
                                let dimatfit = reader.read_bit();
                                let dimtix = reader.read_bit();
                                let dimtmove = reader.read_bit();
                                let override_code = reader.read_byte();
                                let has_arrow2 = reader.read_bit();
                                let flip_arrow2 = reader.read_bit();
                                let flip_arrow1 = reader.read_bit();
                                let mut p3 = || {
                                    crate::types::Vector3::new(
                                        reader.read_bit_double(),
                                        reader.read_bit_double(),
                                        reader.read_bit_double(),
                                    )
                                };
                                let subtype = match class_name.as_deref().unwrap_or("") {
                                    "ACDB_ALDIMOBJECTCONTEXTDATA_CLASS" => {
                                        crate::objects::DimSubtype::Aligned { dimline_pt: p3() }
                                    }
                                    "ACDB_ANGDIMOBJECTCONTEXTDATA_CLASS" => {
                                        crate::objects::DimSubtype::Angular { arc_pt: p3() }
                                    }
                                    "ACDB_DMDIMOBJECTCONTEXTDATA_CLASS" => {
                                        crate::objects::DimSubtype::Diametric {
                                            first_arc_pt: p3(),
                                            def_pt: p3(),
                                        }
                                    }
                                    "ACDB_RADIMOBJECTCONTEXTDATA_CLASS" => {
                                        crate::objects::DimSubtype::Radial { first_arc_pt: p3() }
                                    }
                                    "ACDB_RADIMLGOBJECTCONTEXTDATA_CLASS" => {
                                        crate::objects::DimSubtype::RadialLarge {
                                            ovr_center: p3(),
                                            jog_point: p3(),
                                        }
                                    }
                                    _ => crate::objects::DimSubtype::Ordinate {
                                        feature_location_pt: p3(),
                                        leader_endpt: p3(),
                                    },
                                };
                                drop(p3);
                                // Handle stream: scale (soft owner) then block (hard ptr).
                                let scale = reader.read_handle();
                                let block = reader.read_handle();
                                Some((
                                    crate::objects::ObjectContextKind::Dim(
                                        crate::objects::DimContext {
                                            def_pt,
                                            is_def_textloc,
                                            text_rotation,
                                            block: Handle::from(block),
                                            b293,
                                            dimtofl,
                                            dimosxd,
                                            dimatfit,
                                            dimtix,
                                            dimtmove,
                                            override_code,
                                            has_arrow2,
                                            flip_arrow2,
                                            flip_arrow1,
                                            subtype,
                                        },
                                    ),
                                    class_version,
                                    is_default,
                                    scale,
                                ))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };

                    let raw_handle_bits = reader.get_handle_bits();
                    let raw_data = reader.raw_merged_data();

                    // Model-documentation view graph, recorded so section marks
                    // can derive their viewing direction from real file data:
                    // - AcDbViewRep: keep its object-specific handle references
                    //   (they include the view's border entity).
                    // - AcDbViewRepSectionDefinition: its owner is the section
                    //   (result) view's AcDbViewRep.
                    match class_name.as_deref() {
                        Some("ACDBVIEWREP") => {
                            let mut hs: Vec<Handle> = Vec::new();
                            for _ in 0..20 {
                                let h = reader.read_handle();
                                hs.push(Handle::from(h));
                            }
                            while hs.last().map(|h| h.value()) == Some(0) {
                                hs.pop();
                            }
                            document.view_rep_refs.insert(Handle::from(handle), hs);
                        }
                        Some("ACDBVIEWREPSECTIONDEFINITION") => {
                            let owner = Handle::from(non_entity_data.owner_handle);
                            if owner.value() != 0
                                && !document.section_view_reps.contains(&owner)
                            {
                                document.section_view_reps.push(owner);
                            }
                        }
                        _ => {}
                    }

                    // AcDbSectionViewStyle: decode the display fields (arrow
                    // size, label height, line/arrow visibility) that drive the
                    // section-mark renderer. The reader sits at the class-specific
                    // data (common non-entity data already consumed); the raw bytes
                    // above still drive verbatim write-back. Keep the first found —
                    // a drawing normally has a single active section-view style.
                    if class_name.as_deref() == Some("ACDBSECTIONVIEWSTYLE")
                        && document.section_view_style.is_none()
                    {
                        let r2018 = self
                            .obj_reader
                            .version()
                            .r2018_plus(self.obj_reader.dxf_version());
                        if let Some(svs) = decode_section_view_style(&mut reader, r2018) {
                            document.section_view_style = Some(svs);
                        }
                    }

                    // DGN line-style objects (AcDbLS*): decode the header + the
                    // component tree (handle references) into typed side tables
                    // for rendering. Identified by class name so it is not tied to
                    // this file's class numbering. The object still falls through
                    // to the verbatim `Unknown` storage below, so the DWG
                    // round-trips byte-for-byte; only the leaf stroke/placement
                    // data-stream fields remain undecoded. The data and handle
                    // read cursors are independent and the raw snapshot was
                    // already taken, so these reads are side-effect free.
                    if let Some(cn) = class_name.as_deref() {
                        let is_ls_def = cn == "LSDEFINITION";
                        let is_ls_comp = matches!(
                            cn,
                            "LSSYMBOLCOMPONENT"
                                | "LSCOMPOUNDCOMPONENT"
                                | "LSSTROKEPATTERNCOMPONENT"
                                | "LSPOINTCOMPONENT"
                        );
                        if is_ls_def || is_ls_comp {
                            use crate::objects::{
                                DgnLsComponent, DgnLsComponentType, DgnLsDefinition,
                            };
                            let description = reader.read_variable_text();
                            let _version = reader.read_bit_long();
                            let type_field = reader.read_bit_long();
                            let h = Handle::from(handle);
                            if is_ls_def {
                                let root = reader.read_handle();
                                document.dgn_ls_definitions.insert(
                                    h,
                                    DgnLsDefinition {
                                        handle: h,
                                        name: description,
                                        root_component: Handle::from(root),
                                    },
                                );
                            } else if let Some(ct) = DgnLsComponentType::from_code(type_field) {
                                // Component tree references from the handle stream
                                // (after the common owner/reactor/xdict handles).
                                let mut refs = Vec::new();
                                for _ in 0..16 {
                                    let r = reader.read_handle();
                                    if r == 0 {
                                        break;
                                    }
                                    refs.push(Handle::from(r));
                                }
                                // Symbol scale: a byte-aligned big-endian f64 in
                                // the leaf (the DGN line-style leaf stores raw
                                // big-endian floats). Empirically at byte 35.
                                let scale = if ct == DgnLsComponentType::Symbol
                                    && raw_data.len() >= 43
                                {
                                    let v =
                                        f64::from_be_bytes(raw_data[35..43].try_into().unwrap());
                                    if v.is_finite() && v.abs() > 1e-9 && v.abs() < 1e6 {
                                        v
                                    } else {
                                        1.0
                                    }
                                } else {
                                    1.0
                                };
                                document.dgn_ls_components.insert(
                                    h,
                                    DgnLsComponent {
                                        handle: h,
                                        component_type: ct,
                                        description,
                                        refs,
                                        scale,
                                    },
                                );
                            }
                        }
                    }

                    if let Some((kind, class_version, is_default, scale)) = modeled {
                        if scale != 0 {
                            document
                                .context_scales
                                .insert(Handle::from(handle), Handle::from(scale));
                        }
                        let reactors = non_entity_data
                            .reactors
                            .iter()
                            .map(|&h| Handle::from(h))
                            .collect();
                        let xdictionary_handle =
                            non_entity_data.xdictionary_handle.map(Handle::from);
                        document.objects.insert(
                            Handle::from(handle),
                            crate::objects::ObjectType::ObjectContextData(
                                crate::objects::ObjectContextData {
                                    handle: Handle::from(handle),
                                    owner_handle,
                                    reactors,
                                    xdictionary_handle,
                                    class_version,
                                    is_default,
                                    scale: Handle::from(scale),
                                    kind,
                                    source_raw: Some(raw_data),
                                    source_handle_bits: raw_handle_bits,
                                    source_version: Some(document.version),
                                },
                            ),
                        );
                    } else {
                        // Non-modeled context leaf: still capture its annotation
                        // scale (first object handle) into the side map, then
                        // preserve the whole object verbatim as Unknown. Other
                        // unrecognised non-entity objects: verbatim only.
                        if is_context_data {
                            let scale = reader.read_handle();
                            if scale != 0 {
                                document
                                    .context_scales
                                    .insert(Handle::from(handle), Handle::from(scale));
                            }
                        }
                        let type_name = Self::unknown_object_type_name(document, raw_type_code);
                        document.objects.insert(
                            Handle::from(handle),
                            crate::objects::ObjectType::Unknown {
                                type_name,
                                handle: Handle::from(handle),
                                owner: owner_handle,
                                raw_dxf_codes: None,
                                raw_dwg_data: Some(raw_data),
                                raw_dwg_handle_bits: raw_handle_bits,
                                raw_dwg_version: Some(document.version),
                            },
                        );
                    }
                }
            }
        }
        // Table types already processed in Pass 1
    }

    /// Build a class_number → internal OBJ_* type code mapping.
    ///
    /// The DWG binary uses class numbers (≥500) for non-fixed object types.
    /// This builds a translation table so the builder can match them against
    /// the internal OBJ_* constants.
    fn build_class_type_map(document: &CadDocument) -> HashMap<i16, i16> {
        let mut map = HashMap::new();
        for class in document.classes.iter() {
            if let Some(internal_code) = dxf_name_to_type_code(&class.dxf_name) {
                if class.class_number >= 500 {
                    map.insert(class.class_number, internal_code);
                }
            }
        }
        map
    }

    /// Resolve a raw DWG type code to the internal OBJ_* constant.
    ///
    /// Fixed type codes (0–82) pass through unchanged.
    /// Class-based codes (≥500) are looked up in the class map.
    fn resolve_type_code(raw: i16, class_map: &HashMap<i16, i16>) -> i16 {
        if raw >= 500 {
            class_map.get(&raw).copied().unwrap_or(raw)
        } else {
            raw
        }
    }

    /// Name an unsupported object by its CLASSES-section dxfname when the
    /// drawing's class map knows it, falling back to the positional code.
    fn unknown_object_type_name(document: &CadDocument, raw_type_code: i16) -> String {
        document
            .classes
            .iter()
            .find(|class| class.class_number == raw_type_code)
            .map(|class| class.dxf_name.clone())
            .unwrap_or_else(|| format!("DWG_OBJ_{}", raw_type_code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classes::DxfClass;

    #[test]
    fn unknown_object_type_name_uses_class_dxf_name() {
        let mut document = CadDocument::new();
        document.classes.clear();
        let mut class = DxfClass::new("IRD_OBJ_RECORD", "AcDbIrdObjRecord");
        class.class_number = 524;
        document.classes.push_preserving(class);

        assert_eq!(
            DwgDocumentBuilder::unknown_object_type_name(&document, 524),
            "IRD_OBJ_RECORD"
        );
    }

    #[test]
    fn unknown_object_type_name_falls_back_to_raw_class_number() {
        let mut document = CadDocument::new();
        document.classes.clear();

        assert_eq!(
            DwgDocumentBuilder::unknown_object_type_name(&document, 524),
            "DWG_OBJ_524"
        );
    }

    #[test]
    fn unknown_object_type_name_uses_raw_number_after_type_resolution() {
        let mut document = CadDocument::new();
        document.classes.clear();
        let mut class = DxfClass::new("IRD_OBJ_RECORD", "AcDbIrdObjRecord");
        class.class_number = 524;
        document.classes.push_preserving(class);

        let mut class_map = HashMap::new();
        class_map.insert(524, 82);

        assert_eq!(DwgDocumentBuilder::resolve_type_code(524, &class_map), 82);
        assert_eq!(
            DwgDocumentBuilder::unknown_object_type_name(&document, 524),
            "IRD_OBJ_RECORD"
        );
    }
}

/// Build a [`Matrix4`](crate::types::Matrix4) from 12 doubles holding a 3×4
/// transform in row-major order (3 rows of 4: `[R | t]`); bottom row implied.
/// Decode an `AcDbSectionSymbol` (DWG class 825) into its display geometry.
///
/// `reader` must be positioned at the class-specific data (i.e. after
/// `read_common_entity_data`). The section-symbol serialization is undocumented,
/// so rather than parse every field we locate the two cut-line endpoints
/// structurally: each is a 2-D point stored as **two consecutive full IEEE
/// doubles** (BD prefix `00`) 66 bits apart. The variable-length header and the
/// lone signed "tick" doubles between/after the ends have no such paired
/// neighbour, so scanning for the first two full-BD *pairs* in a plausible
/// coordinate range reliably isolates the endpoints regardless of the header
/// field layout. The identifier string ("A") comes from the string stream.
///
/// Returns `None` if the record is too short or no endpoint pair is found; the
/// caller then keeps the entity as a plain unknown (raw bytes preserved).
fn decode_section_symbol(
    reader: &mut crate::io::dwg::dwg_stream_readers::merged_reader::DwgMergedReader,
) -> Option<SectionSymbol> {
    let start = reader.position_in_bits() as usize;
    let full = reader.raw_merged_data();
    let total = full.len() * 8;
    let bit = |off: usize| -> u8 { (full[off / 8] >> (7 - off % 8)) & 1 };
    // Read a full-precision BD (2-bit prefix `00` then 64 IEEE bits) at bit `b`.
    // Byte reconstruction matches the bit reader (MSB-first 8-bit groups).
    let read_bd_full = |b: usize| -> Option<f64> {
        if b + 66 > total || bit(b) != 0 || bit(b + 1) != 0 {
            return None;
        }
        let mut by = [0u8; 8];
        for k in 0..8 {
            let mut v = 0u8;
            for j in 0..8 {
                v = (v << 1) | bit(b + 2 + k * 8 + j);
            }
            by[k] = v;
        }
        Some(f64::from_le_bytes(by))
    };
    // A drawing coordinate: finite, non-trivial magnitude, not padding. Both
    // components of a real endpoint clear this (a mantissa window that lands in
    // range by chance is usually paired with a 0.0 padding double, which fails).
    let coord = |v: f64| v.is_finite() && v.abs() > 1e-6 && v.abs() < 5.0e6;
    // First bit position where two full-BDs 66 bits apart are both real
    // coordinates — i.e. a 2-D (X, Y) endpoint.
    let find_point = |from: usize| -> Option<(usize, f64, f64)> {
        let mut b = from;
        while b + 132 <= total {
            if let (Some(x), Some(y)) = (read_bd_full(b), read_bd_full(b + 66)) {
                if coord(x) && coord(y) {
                    return Some((b, x, y));
                }
            }
            b += 1;
        }
        None
    };
    // A lone in-range full-BD (a signed tick length) in [from, to); returns its
    // value and end bit so the caller can resume the scan past it.
    let find_tick = |from: usize, to: usize| -> (f64, usize) {
        let mut b = from;
        while b + 66 <= to.min(total) {
            if let Some(v) = read_bd_full(b) {
                if v.abs() > 1e-9 && v.abs() < 1.0e5 {
                    return (v, b + 66);
                }
            }
            b += 1;
        }
        (0.0, from)
    };

    let (a_bit, ax, ay) = find_point(start)?;
    let after_a = a_bit + 132;
    // The tick sits between the two ends; search for END-B only *past* it so a
    // window inside the tick double's mantissa can't masquerade as an endpoint.
    let (tick_a, past_tick_a) = find_tick(after_a, after_a + 200);
    let (b_bit, bx, by) = find_point(past_tick_a.max(after_a))?;
    let after_b = b_bit + 132;
    let (tick_b, _) = find_tick(after_b, total);
    // Identifier from the string stream (first variable text of the record).
    let label = reader.read_variable_text();
    // Object-specific handle references (the common entity handles were already
    // consumed): the section-view style, then the parent view's AcDbViewRep.
    // The handle cursor is independent of the data/string cursors, so these
    // reads don't disturb the geometry above.
    let style_handle = reader.read_handle();
    let view_rep_handle = reader.read_handle();

    Some(SectionSymbol {
        end_a: [ax, ay],
        end_b: [bx, by],
        tick_a,
        tick_b,
        label,
        style_handle,
        view_rep_handle,
        ..SectionSymbol::new()
    })
}

/// Decode the display fields of an `AcDbSectionViewStyle` (DWG class 795).
///
/// `reader` must be positioned at the class-specific data (after
/// `read_common_non_entity_data`). Fields are read in LibreDWG `dwg2.spec` order
/// (cross-validated against a real sample): the `AcDbModelDocViewStyle` base
/// (version, description, modified-flag), then the section-view fields through
/// `arrow_symbol_extension_length`. The DATA-stream reads and the interleaved
/// handle reads use independent cursors, so reading the two null arrow-symbol
/// handles in place keeps the DATA cursor aligned. R2013 files have no R2018+
/// base fields.
///
/// Returns the fields the renderer needs; the caller keeps the raw record for
/// verbatim write-back.
fn decode_section_view_style(
    reader: &mut crate::io::dwg::dwg_stream_readers::merged_reader::DwgMergedReader,
    r2018_plus: bool,
) -> Option<SectionViewStyle> {
    // AcDbModelDocViewStyle base.
    let _mdoc_class_version = reader.read_bit_short();
    let _desc = reader.read_variable_text();
    let _is_modified = reader.read_bit();
    // R2018+ added a display name and a style-flags word to the base class.
    if r2018_plus {
        let _display_name = reader.read_variable_text();
        let _viewstyle_flags = reader.read_bit_long();
    }
    // AcDbSectionViewStyle.
    let _class_version = reader.read_bit_short();
    let flags = reader.read_bit_long();
    let _identifier_color = reader.read_cm_color();
    let identifier_height = reader.read_bit_double();
    // Handle stream (independent cursor), in order: identifier_style, then the
    // two arrow-symbol handles. A null (0) arrow handle means the default arrow.
    let _identifier_style = reader.read_handle();
    let arrow_start = reader.read_handle();
    let arrow_end = reader.read_handle();
    let _arrow_color = reader.read_cm_color();
    let arrow_size = reader.read_bit_double();
    let _exclude = reader.read_variable_text();
    let arrow_extension = reader.read_bit_double();
    // Continue through the DATA stream (LibreDWG order) to the late-stored
    // placement fields: identifier_position/offset and arrow_position are
    // physically at the record's tail, after the plane/bend/end/view-label
    // and hatch groups.
    let _plane_linewt = reader.read_bit_long();
    let _plane_color = reader.read_cm_color();
    let _bend_linewt = reader.read_bit_long();
    let _bend_color = reader.read_cm_color();
    let _bend_line_length = reader.read_bit_double();
    let end_line_length = reader.read_bit_double();
    let _viewlabel_color = reader.read_cm_color();
    let _viewlabel_height = reader.read_bit_double();
    let _viewlabel_attachment = reader.read_bit_long();
    let _viewlabel_offset = reader.read_bit_double();
    let _viewlabel_alignment = reader.read_bit_long();
    let _hatch_color = reader.read_cm_color();
    let _hatch_bg_color = reader.read_cm_color();
    let _hatch_scale = reader.read_bit_double();
    let _hatch_transparency = reader.read_bit_long();
    let _unknown_b1 = reader.read_bit();
    let _unknown_b2 = reader.read_bit();
    let identifier_position = reader.read_bit_long();
    let identifier_offset = reader.read_bit_double();
    let arrow_position = reader.read_bit_long();
    let end_line_overshoot = reader.read_bit_double();

    // Sanity gate: a valid style has finite sizes.
    if !identifier_height.is_finite() || !arrow_size.is_finite() || !arrow_extension.is_finite() {
        return None;
    }
    Some(SectionViewStyle {
        show_arrows: flags & 0x02 != 0,
        show_plane_line: flags & 0x08 != 0,
        show_end_lines: flags & 0x20 != 0,
        arrow_size,
        arrow_extension,
        label_height: identifier_height,
        label_offset: if identifier_offset.is_finite() { identifier_offset } else { 0.0 },
        label_position: identifier_position,
        arrow_position,
        end_line_length: if end_line_length.is_finite() { end_line_length } else { 0.0 },
        end_line_overshoot: if end_line_overshoot.is_finite() { end_line_overshoot } else { 0.0 },
        arrow_start_handle: arrow_start,
        arrow_end_handle: arrow_end,
        arrow_is_default: arrow_start == 0 && arrow_end == 0,
    })
}

/// DWG stores the spatial-filter transforms row-major (unlike DXF code 40,
/// which is column-major).
fn matrix_from_row_major(v: &[f64; 12]) -> crate::types::Matrix4 {
    crate::types::Matrix4 {
        m: [
            [v[0], v[1], v[2], v[3]],
            [v[4], v[5], v[6], v[7]],
            [v[8], v[9], v[10], v[11]],
            [0.0, 0.0, 0.0, 1.0],
        ],
    }
}

fn map_dimension_common(
    base: &mut crate::entities::dimension::DimensionBase,
    common: &entities::DimensionCommonData,
    maps: &HandleMaps,
) {
    base.version = common.version_byte;
    base.normal = common.normal;
    base.text_middle_point = common.text_middle_point;
    // Flags byte bit 0: dimension text positioned at a user-defined location.
    base.text_user_positioned = (common.flags_byte & 0x01) != 0;
    base.text = common.text.clone();
    base.text_rotation = common.text_rotation;
    base.horizontal_direction = common.horizontal_direction;
    base.attachment_point = match common.attachment_point {
        1 => crate::entities::dimension::AttachmentPointType::TopLeft,
        2 => crate::entities::dimension::AttachmentPointType::TopCenter,
        3 => crate::entities::dimension::AttachmentPointType::TopRight,
        4 => crate::entities::dimension::AttachmentPointType::MiddleLeft,
        5 => crate::entities::dimension::AttachmentPointType::MiddleCenter,
        6 => crate::entities::dimension::AttachmentPointType::MiddleRight,
        7 => crate::entities::dimension::AttachmentPointType::BottomLeft,
        8 => crate::entities::dimension::AttachmentPointType::BottomCenter,
        9 => crate::entities::dimension::AttachmentPointType::BottomRight,
        _ => crate::entities::dimension::AttachmentPointType::MiddleCenter,
    };
    base.line_spacing_factor = common.linespacing_factor;
    base.actual_measurement = common.actual_measurement;
    base.insertion_point =
        crate::types::Vector3::new(common.insertion_point.x, common.insertion_point.y, 0.0);
    base.style_name = maps.dimstyle_name(common.dimstyle_handle);
    base.block_name = maps.block_name(common.block_handle);
}

fn map_entity_common(
    data: &EntityCommonData,
    maps: &HandleMaps,
    model_space_handle: Handle,
    paper_space_handle: Handle,
) -> EntityCommon {
    let mut common = EntityCommon::new();
    common.handle = Handle::from(data.common.handle);
    // Resolve owner from entity_mode:
    //   0 = explicit owner (handle read from stream)
    //   1 = paper space (implicit)
    //   2 = model space (implicit)
    common.owner_handle = match data.entity_mode {
        1 => paper_space_handle,
        2 => model_space_handle,
        _ => Handle::from(data.owner_handle),
    };
    common.color = data.color;
    common.transparency = data.transparency;
    common.invisible = data.invisible;
    common.linetype_scale = data.linetype_scale;
    common.layer = maps.layer_name(data.layer_handle);
    // Line weight (raw DWG index byte → LineWeight)
    common.line_weight = crate::types::LineWeight::from_dwg_index(data.line_weight);
    // Reactors
    common.reactors = data.reactors.iter().map(|&h| Handle::from(h)).collect();
    // XDictionary handle
    common.xdictionary_handle = data.xdictionary_handle.map(Handle::from);
    // Linetype (from flags + optional handle)
    // EntityCommon uses empty string for "ByLayer" convention
    common.linetype = match data.linetype_flags {
        0b00 => String::new(), // ByLayer → empty (EntityCommon convention)
        0b01 => "ByBlock".to_string(),
        0b10 => "Continuous".to_string(),
        0b11 => maps
            .linetypes
            .get(&data.linetype_handle)
            .cloned()
            .unwrap_or_default(),
        _ => String::new(),
    };
    // EED raw bytes for DWG round-trip
    common.extended_data.raw_dwg_eed = data.common.eed_raw.clone();
    // Graphic data for DWG round-trip
    common.graphic_data = data.graphic_data.clone();
    // DWG round-trip: preserve entity_mode, material/plotstyle/shadow flags
    common.entity_mode = Some(data.entity_mode);
    common.material_flags = data.material_flags;
    common.material_handle = data.material_handle.map(Handle::from);
    common.shadow_flags = data.shadow_flags;
    common.plotstyle_flags = data.plotstyle_flags;
    common.plotstyle_handle = data.plotstyle_handle.map(Handle::from);
    // R2013+: geometry-in-AcDs flag, needed to pair AcDs SAB blobs with the
    // right modeler entity in object-stream order.
    common.has_ds_data = data.has_ds_data;
    common
}
