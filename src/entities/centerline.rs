//! Persistent association metadata for centre-line entities.
//!
//! A centre line is rendered and exchanged as an ordinary line while this
//! compact XDATA payload retains the source geometry needed to regenerate it.

use crate::types::{Handle, Vector3};
use crate::xdata::{ExtendedData, ExtendedDataRecord, XDataValue};

/// Registered application name used by centre-line association records.
pub const CENTERLINE_XDATA_APPLICATION: &str = "OCS_CENTERLINE";
const SIGNATURE: &str = "CENTERLINE_ASSOCIATION";
const VERSION: i16 = 1;

/// Kind of source geometry referenced by a centre line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CenterLineSourceKind {
    Line,
    LwPolylineSegment,
    Polyline2DSegment,
}

impl CenterLineSourceKind {
    fn code(self) -> i16 {
        match self {
            Self::Line => 0,
            Self::LwPolylineSegment => 1,
            Self::Polyline2DSegment => 2,
        }
    }

    fn from_code(code: i16) -> Option<Self> {
        match code {
            0 => Some(Self::Line),
            1 => Some(Self::LwPolylineSegment),
            2 => Some(Self::Polyline2DSegment),
            _ => None,
        }
    }
}

/// One selected source line or linear polyline segment.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CenterLineSource {
    pub handle: Handle,
    pub kind: CenterLineSourceKind,
    pub segment_index: i32,
    pub pick_point: Vector3,
}

/// Complete, versioned metadata required to regenerate a centre line.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CenterLineAssociation {
    pub first: CenterLineSource,
    pub second: CenterLineSource,
    pub plane_origin: Vector3,
    pub plane_x: Vector3,
    pub plane_y: Vector3,
    pub start_extension: f64,
    pub end_extension: f64,
    pub start_length_adjustment: f64,
    pub end_length_adjustment: f64,
    pub associated: bool,
}

impl CenterLineAssociation {
    /// Decode association metadata, rejecting incomplete or future payloads.
    pub fn read(data: &ExtendedData) -> Option<Self> {
        let values = &data.get_record(CENTERLINE_XDATA_APPLICATION)?.values;
        let [
            XDataValue::String(signature),
            XDataValue::Integer16(version),
            XDataValue::Handle(first_handle),
            XDataValue::Integer16(first_kind),
            XDataValue::Integer32(first_segment),
            XDataValue::Point3D(first_pick),
            XDataValue::Handle(second_handle),
            XDataValue::Integer16(second_kind),
            XDataValue::Integer32(second_segment),
            XDataValue::Point3D(second_pick),
            XDataValue::Point3D(plane_origin),
            XDataValue::Direction3D(plane_x),
            XDataValue::Direction3D(plane_y),
            XDataValue::Distance(start_extension),
            XDataValue::Distance(end_extension),
            XDataValue::Distance(start_length_adjustment),
            XDataValue::Distance(end_length_adjustment),
            XDataValue::Integer16(flags),
        ] = values.as_slice()
        else {
            return None;
        };
        if signature != SIGNATURE || *version != VERSION {
            return None;
        }
        Some(Self {
            first: CenterLineSource {
                handle: *first_handle,
                kind: CenterLineSourceKind::from_code(*first_kind)?,
                segment_index: *first_segment,
                pick_point: *first_pick,
            },
            second: CenterLineSource {
                handle: *second_handle,
                kind: CenterLineSourceKind::from_code(*second_kind)?,
                segment_index: *second_segment,
                pick_point: *second_pick,
            },
            plane_origin: *plane_origin,
            plane_x: *plane_x,
            plane_y: *plane_y,
            start_extension: *start_extension,
            end_extension: *end_extension,
            start_length_adjustment: *start_length_adjustment,
            end_length_adjustment: *end_length_adjustment,
            associated: flags & 1 != 0,
        })
    }

    /// Replace the association payload without disturbing unrelated XDATA.
    pub fn write(&self, data: &mut ExtendedData) {
        let mut record = ExtendedDataRecord::new(CENTERLINE_XDATA_APPLICATION);
        record.values = vec![
            XDataValue::String(SIGNATURE.to_owned()),
            XDataValue::Integer16(VERSION),
            XDataValue::Handle(self.first.handle),
            XDataValue::Integer16(self.first.kind.code()),
            XDataValue::Integer32(self.first.segment_index),
            XDataValue::Point3D(self.first.pick_point),
            XDataValue::Handle(self.second.handle),
            XDataValue::Integer16(self.second.kind.code()),
            XDataValue::Integer32(self.second.segment_index),
            XDataValue::Point3D(self.second.pick_point),
            XDataValue::Point3D(self.plane_origin),
            XDataValue::Direction3D(self.plane_x),
            XDataValue::Direction3D(self.plane_y),
            XDataValue::Distance(self.start_extension),
            XDataValue::Distance(self.end_extension),
            XDataValue::Distance(self.start_length_adjustment),
            XDataValue::Distance(self.end_length_adjustment),
            XDataValue::Integer16(i16::from(self.associated)),
        ];
        data.upsert_record(record);
    }

    /// Remove only centre-line metadata, leaving other applications intact.
    pub fn remove(data: &mut ExtendedData) {
        data.remove_record(CENTERLINE_XDATA_APPLICATION);
    }
}
