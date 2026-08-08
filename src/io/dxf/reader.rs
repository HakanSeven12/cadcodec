//! DXF file reader

mod stream_reader;
mod text_reader;
mod binary_reader;
mod section_reader;

pub use stream_reader::DxfStreamReader;
pub use text_reader::DxfTextReader;
pub use binary_reader::DxfBinaryReader;

use section_reader::SectionReader;

use crate::document::CadDocument;
use crate::entities::solid3d::AcisVersion;
use crate::entities::EntityType;
use crate::error::{DxfError, Result};
use crate::io::read::{push_read_diagnostic, ReadDiagnostic, ReadStage, SourceFormat};
use crate::types::Handle;
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

/// Configuration for the DXF reader.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DxfReaderConfiguration {
    /// When `true`, parse errors within individual entities/objects/sections
    /// are caught and reported as notifications instead of aborting the read.
    ///
    /// Default: `false` (strict mode — errors propagate).
    pub failsafe: bool,

    /// Default encoding to use for non-UTF8 strings if the DXF file does not
    /// specify it via $DWGCODEPAGE.
    ///
    /// Only applies to DXF versions prior to AC1021 (AutoCAD 2007).
    pub default_encoding: Option<String>,
}

impl Default for DxfReaderConfiguration {
    fn default() -> Self {
        Self {
            failsafe: false,
            default_encoding: None,
        }
    }
}

/// DXF file reader
pub struct DxfReader {
    reader: Box<dyn DxfStreamReader>,
    config: DxfReaderConfiguration,
    /// Estimated entity count based on stream size (used for pre-allocation).
    estimated_entities: usize,
}

impl DxfReader {
    /// Create a new DXF reader from any reader
    pub fn from_reader<R: Read + Seek + 'static>(reader: R) -> Result<Self> {
        let mut buf_reader = BufReader::with_capacity(64 * 1024, reader);

        // Estimate entity count from stream size (~300 bytes per entity on average)
        let stream_size = buf_reader.seek(std::io::SeekFrom::End(0)).unwrap_or(0);
        buf_reader.seek(std::io::SeekFrom::Start(0))?;
        let estimated_entities = (stream_size as usize / 300).max(16);

        // Detect if binary
        let is_binary = Self::is_binary(&mut buf_reader)?;

        // Create appropriate reader
        let reader: Box<dyn DxfStreamReader> = if is_binary {
            Box::new(DxfBinaryReader::new(buf_reader)?)
        } else {
            // Seek back to start for text DXF files
            buf_reader.seek(std::io::SeekFrom::Start(0))?;
            Box::new(DxfTextReader::new(buf_reader)?)
        };

        Ok(Self {
            reader,
            config: DxfReaderConfiguration::default(),
            estimated_entities,
        })
    }

    /// Create a new DXF reader from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mut buf_reader = BufReader::with_capacity(64 * 1024, file);

        // Estimate entity count from stream size (~300 bytes per entity on average)
        let stream_size = buf_reader.seek(std::io::SeekFrom::End(0)).unwrap_or(0);
        buf_reader.seek(std::io::SeekFrom::Start(0))?;
        let estimated_entities = (stream_size as usize / 300).max(16);
        
        // Detect if binary
        let is_binary = Self::is_binary(&mut buf_reader)?;
        
        // Create appropriate reader
        let reader: Box<dyn DxfStreamReader> = if is_binary {
            Box::new(DxfBinaryReader::new(buf_reader)?)
        } else {
            // Seek back to start for text DXF files
            buf_reader.seek(std::io::SeekFrom::Start(0))?;
            Box::new(DxfTextReader::new(buf_reader)?)
        };
        
        Ok(Self {
            reader,
            config: DxfReaderConfiguration::default(),
            estimated_entities,
        })
    }
    
    /// Check if a stream contains binary DXF data
    fn is_binary<R: Read + Seek>(reader: &mut R) -> Result<bool> {
        const SENTINEL: &[u8] = b"AutoCAD Binary DXF";
        let mut buffer = vec![0u8; SENTINEL.len()];
        
        // Try to read the sentinel bytes
        let bytes_read = reader.read(&mut buffer)?;
        
        // Always seek back to start after checking
        reader.seek(std::io::SeekFrom::Start(0))?;
        
        // If file is too small or doesn't match, it's not binary
        if bytes_read < SENTINEL.len() {
            return Ok(false);
        }
        
        Ok(buffer == SENTINEL)
    }

    /// Set the reader configuration.
    pub fn with_configuration(mut self, config: DxfReaderConfiguration) -> Self {
        self.config = config;
        self
    }

    fn read_diagnostic(
        &self,
        code: impl Into<String>,
        stage: ReadStage,
        message: impl Into<String>,
    ) -> ReadDiagnostic {
        let context = self.reader.diagnostic_context();
        let mut diagnostic = ReadDiagnostic::new(code, stage, message);
        diagnostic.source_offset = context.source_offset;
        diagnostic.source_offset_basis = Some("file-byte".to_string());
        diagnostic.source_line = context.source_line;
        diagnostic.record_handle = context.record_handle;
        diagnostic.record_type = context.record_type;
        diagnostic
    }

    /// Read a DXF file and return a CadDocument
    pub fn read(self) -> Result<CadDocument> {
        self.read_with_stats().map(|outcome| outcome.document)
    }

    /// Read a DXF file and return the document with source/decode statistics.
    pub fn read_with_stats(mut self) -> Result<crate::io::read::ReadOutcome> {
        // Set default encoding if provided
        if let Some(ref encoding_name) = self.config.default_encoding {
            if let Some(enc) = crate::io::dxf::code_page::encoding_from_code_page(encoding_name) {
                self.reader.set_encoding(enc);
            }
        }

        // Create document with pre-allocated entity storage
        let mut document = CadDocument::new();
        document.entities.reserve(self.estimated_entities);
        document.entity_index.reserve(self.estimated_entities);
        
        // Read all sections
        let failsafe = self.config.failsafe;
        let mut source_sections = 0usize;
        let mut source_records = 0usize;
        let mut decoded_source_records = 0usize;
        let mut skipped_source_records = 0usize;
        let mut record_stream_read = false;
        let mut stream_completed = false;
        let mut diagnostics = Vec::new();

        loop {
            let pair = match self.reader.read_pair() {
                Ok(Some(pair)) => pair,
                Ok(None) if failsafe && decoded_source_records > 0 => {
                    let message = "Drawing stream ended before the EOF marker".to_string();
                    document.notifications.notify(
                        crate::notification::NotificationType::Error,
                        message.clone(),
                    );
                    push_read_diagnostic(&mut diagnostics, self.read_diagnostic(
                        "stream-ended-early",
                        ReadStage::RecordStream,
                        message,
                    ));
                    break;
                }
                Ok(None) => {
                    return Err(DxfError::Parse(
                        "drawing stream ended before the EOF marker".to_string(),
                    ));
                }
                Err(error) if failsafe && decoded_source_records > 0 => {
                    let message = format!("Error reading drawing stream: {}", error);
                    document.notifications.notify(
                        crate::notification::NotificationType::Error,
                        message.clone(),
                    );
                    push_read_diagnostic(&mut diagnostics, self.read_diagnostic(
                        "stream-read-failed",
                        ReadStage::RecordStream,
                        message,
                    ));
                    break;
                }
                Err(error) => return Err(error),
            };
            if pair.code == 0 && pair.value_string == "SECTION" {
                // Read section name
                let section_pair = match self.reader.read_pair() {
                    Ok(Some(section_pair)) => Some(section_pair),
                    Ok(None) if failsafe && decoded_source_records > 0 => {
                        let message = "Drawing stream ended before the section name".to_string();
                        document.notifications.notify(
                            crate::notification::NotificationType::Error,
                            message.clone(),
                        );
                        push_read_diagnostic(&mut diagnostics, self.read_diagnostic(
                            "section-name-missing",
                            ReadStage::Section,
                            message,
                        ));
                        break;
                    }
                    Ok(None) => {
                        return Err(DxfError::Parse(
                            "drawing stream ended before the section name".to_string(),
                        ));
                    }
                    Err(error) if failsafe && decoded_source_records > 0 => {
                        let message = format!("Error reading section name: {}", error);
                        document.notifications.notify(
                            crate::notification::NotificationType::Error,
                            message.clone(),
                        );
                        push_read_diagnostic(&mut diagnostics, self.read_diagnostic(
                            "section-name-read-failed",
                            ReadStage::Section,
                            message,
                        ));
                        break;
                    }
                    Err(error) => return Err(error),
                };
                if let Some(section_pair) = section_pair {
                    if section_pair.code == 2 {
                        let section_name = section_pair.value_string.clone();
                        let carries_records = matches!(
                            section_name.as_str(),
                            "TABLES" | "BLOCKS" | "ENTITIES" | "OBJECTS" | "ACDSDATA"
                        );
                        let decoded_before = decoded_source_records;
                        let result = match section_name.as_str() {
                            "HEADER" => self.read_header_section(&mut document),
                            "CLASSES" => self.read_classes_section(&mut document),
                            "TABLES" => self.read_tables_section(
                                &mut document,
                                &mut decoded_source_records,
                            ),
                            "BLOCKS" => self.read_blocks_section(
                                &mut document,
                                &mut decoded_source_records,
                            ),
                            "ENTITIES" => self.read_entities_section(
                                &mut document,
                                &mut decoded_source_records,
                            ),
                            "OBJECTS" => self.read_objects_section(
                                &mut document,
                                &mut decoded_source_records,
                            ),
                            "ACDSDATA" => self.read_acdsdata_section(&mut document),
                            "THUMBNAILIMAGE" => {
                                document.notifications.notify(
                                    crate::notification::NotificationType::NotImplemented,
                                    "THUMBNAILIMAGE section skipped",
                                );
                                self.skip_section()
                            }
                            _ => {
                                // Skip unknown section
                                self.skip_section()
                            }
                        };

                        if carries_records {
                            source_sections = source_sections.saturating_add(1);
                            record_stream_read = true;
                            let decoded =
                                decoded_source_records.saturating_sub(decoded_before);
                            source_records = source_records.saturating_add(decoded);
                        }

                        // In failsafe mode, catch errors and continue
                        if let Err(e) = result {
                            if failsafe {
                                let message = format!("Error reading {} section: {}", section_name, e);
                                document.notifications.notify(
                                    crate::notification::NotificationType::Error,
                                    message.clone(),
                                );
                                let mut diagnostic = self.read_diagnostic(
                                    "section-read-failed",
                                    ReadStage::Section,
                                    message,
                                );
                                diagnostic.section = Some(section_name.clone());
                                push_read_diagnostic(&mut diagnostics, diagnostic);
                                skipped_source_records =
                                    skipped_source_records.saturating_add(1);
                                // Try to skip to the end of the section
                                let _ = self.skip_section();
                            } else {
                                return Err(e);
                            }
                        }
                    }
                }
            } else if pair.code == 0 && pair.value_string == "EOF" {
                stream_completed = true;
                break;
            }
        }

        // Post-read resolution: assign owner handles and update next_handle
        document.resolve_references();

        // Pre-R2004 (R2000/R14) down-saved gradient hatches keep their gradient
        // in the ACAD round-trip metadata (GradientColor1/2ACI EED + an
        // ACAD_XREC_ROUNDTRIP XRecord) rather than a native gradient block, so
        // they read back as flat solid fills. Rebuild them — gated to pre-R2004
        // so a native R2004+ gradient (read directly) always wins over any
        // stale round-trip EED left by an earlier edit.
        if document.version < crate::types::DxfVersion::AC1018 {
            crate::io::dwg::dwg_reader::recover_roundtrip_gradients(&mut document);
            // Pre-R2004 also stores an MTEXT background fill as round-trip EED
            // rather than the native codes; rebuild it (dimension text fills).
            crate::io::dwg::dwg_reader::recover_mtext_bg_roundtrip(&mut document);
        }

        source_records = source_records.saturating_add(skipped_source_records);
        let stats = crate::io::read::ReadStats::from_document(
            &document,
            SourceFormat::Dxf,
            source_sections,
            source_records,
            decoded_source_records,
            skipped_source_records,
            record_stream_read,
            failsafe,
            stream_completed,
            diagnostics,
        );
        Ok(crate::io::read::ReadOutcome::new(document, stats))
    }
    
    /// Read the HEADER section
    fn read_header_section(&mut self, document: &mut CadDocument) -> Result<()> {
        let mut section_reader = SectionReader::new(&mut self.reader);
        section_reader.read_header(document)
    }

    /// Read the CLASSES section
    fn read_classes_section(&mut self, document: &mut CadDocument) -> Result<()> {
        let mut section_reader = SectionReader::new(&mut self.reader);
        section_reader.read_classes(document)
    }

    /// Read the TABLES section
    fn read_tables_section(
        &mut self,
        document: &mut CadDocument,
        decoded_records: &mut usize,
    ) -> Result<()> {
        let mut section_reader = SectionReader::new(&mut self.reader);
        let result = section_reader.read_tables(document);
        *decoded_records = decoded_records.saturating_add(section_reader.decoded_records());
        result
    }

    /// Read the BLOCKS section
    fn read_blocks_section(
        &mut self,
        document: &mut CadDocument,
        decoded_records: &mut usize,
    ) -> Result<()> {
        let mut section_reader = SectionReader::new(&mut self.reader);
        let result = section_reader.read_blocks(document);
        *decoded_records = decoded_records.saturating_add(section_reader.decoded_records());
        result
    }

    /// Read the ENTITIES section
    fn read_entities_section(
        &mut self,
        document: &mut CadDocument,
        decoded_records: &mut usize,
    ) -> Result<()> {
        let mut section_reader = SectionReader::new(&mut self.reader);
        let result = section_reader.read_entities(document);
        *decoded_records = decoded_records.saturating_add(section_reader.decoded_records());
        result
    }

    /// Read the OBJECTS section
    fn read_objects_section(
        &mut self,
        document: &mut CadDocument,
        decoded_records: &mut usize,
    ) -> Result<()> {
        let mut section_reader = SectionReader::new(&mut self.reader);
        let result = section_reader.read_objects(document);
        *decoded_records = decoded_records.saturating_add(section_reader.decoded_records());
        result
    }
    
    /// Read the ACDSDATA section (the AcDb data store).
    ///
    /// From R2013 (AC1027) on, a 3D solid / region / body / surface no longer
    /// carries its ACIS geometry inline in the entity (the `AcDbModelerGeometry`
    /// block is empty); the binary SAB blob lives here instead, in an
    /// `ACDSRECORD` whose `ASM_Data` property is bound to the owning entity by a
    /// 320 soft-pointer handle. The DWG reader gets this from the merged AcDs
    /// stream; the DXF reader used to skip the whole section, leaving every
    /// modeler entity geometry-less. Parse the records, then attach each SAB
    /// blob to its entity so the same downstream SAB → mesh path runs.
    fn read_acdsdata_section(&mut self, document: &mut CadDocument) -> Result<()> {
        // (entity handle, SAB bytes) pairs collected from ASM_Data records.
        let mut blobs: Vec<(u64, Vec<u8>)> = Vec::new();

        // Per-record accumulator, flushed on each record/schema boundary (0-code).
        let mut cur_handle: Option<u64> = None;
        let mut is_asm = false;
        let mut chunks: Vec<u8> = Vec::new();

        while let Some(pair) = self.reader.read_pair()? {
            if pair.code == 0 {
                // Record boundary — flush a complete ASM_Data record.
                if is_asm && !chunks.is_empty() {
                    if let Some(h) = cur_handle {
                        blobs.push((h, std::mem::take(&mut chunks)));
                    }
                }
                cur_handle = None;
                is_asm = false;
                chunks.clear();
                if pair.value_string == "ENDSEC" {
                    break;
                }
                continue;
            }
            match pair.code {
                // AcDbDs::ID soft-pointer to the owning entity.
                320 => {
                    if let Ok(h) = u64::from_str_radix(pair.value_string.trim(), 16) {
                        cur_handle = Some(h);
                    }
                }
                // Property name — the binary payload that follows is ACIS only
                // for the ASM_Data property (Thumbnail_Data etc. are skipped).
                2 => is_asm = pair.value_string == "ASM_Data",
                // Binary chunk (hex-encoded); only kept once inside ASM_Data.
                310 if is_asm => {
                    let hex = pair.value_string.trim().as_bytes();
                    let mut i = 0;
                    while i + 1 < hex.len() {
                        let hi = (hex[i] as char).to_digit(16);
                        let lo = (hex[i + 1] as char).to_digit(16);
                        if let (Some(hi), Some(lo)) = (hi, lo) {
                            chunks.push((hi * 16 + lo) as u8);
                        }
                        i += 2;
                    }
                }
                _ => {}
            }
        }

        // Attach each SAB blob to its modeler entity.
        for (handle, sab) in blobs {
            let Some(entity) = document.get_entity_mut(Handle::new(handle)) else {
                continue;
            };
            match entity {
                EntityType::Solid3D(e) => {
                    e.acis_data.sab_data = sab;
                    e.acis_data.is_binary = true;
                    e.acis_data.version = AcisVersion::Version2;
                    e.point_of_reference = e
                        .acis_data
                        .geometry_centre()
                        .or_else(|| e.acis_data.placement_origin())
                        .unwrap_or(e.point_of_reference);
                }
                EntityType::Region(e) => {
                    e.acis_data.sab_data = sab;
                    e.acis_data.is_binary = true;
                    e.acis_data.version = AcisVersion::Version2;
                    e.point_of_reference = e
                        .acis_data
                        .geometry_centre()
                        .or_else(|| e.acis_data.placement_origin())
                        .unwrap_or(e.point_of_reference);
                }
                EntityType::Body(e) => {
                    e.acis_data.sab_data = sab;
                    e.acis_data.is_binary = true;
                    e.acis_data.version = AcisVersion::Version2;
                    e.point_of_reference = e
                        .acis_data
                        .geometry_centre()
                        .or_else(|| e.acis_data.placement_origin())
                        .unwrap_or(e.point_of_reference);
                }
                EntityType::Surface(e) => {
                    e.acis_data.sab_data = sab;
                    e.acis_data.is_binary = true;
                    e.acis_data.version = AcisVersion::Version2;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Skip the current section
    fn skip_section(&mut self) -> Result<()> {
        while let Some(pair) = self.reader.read_pair()? {
            if pair.code == 0 && pair.value_string == "ENDSEC" {
                break;
            }
        }
        Ok(())
    }
}
