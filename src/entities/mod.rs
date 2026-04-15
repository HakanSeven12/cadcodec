//! Graphical entity types.
//!
//! This module contains all 41 supported CAD entity types — from simple
//! primitives ([`Line`], [`Circle`], [`Arc`]) through complex objects
//! ([`Hatch`], [`Spline`], [`MultiLeader`], [`Mesh`]).
//!
//! Every entity carries [`EntityCommon`] data (layer, color, line weight,
//! handle, etc.) alongside its type-specific fields.
//!
//! Entities are stored in [`CadDocument`](crate::document::CadDocument) and
//! wrapped in the [`EntityType`] enum for heterogeneous collections.

use crate::types::{BoundingBox3D, Color, Handle, LineWeight, Transform, Transparency, Vector3};

pub mod point;
pub mod line;
pub mod circle;
pub mod arc;
pub mod ellipse;
pub mod polyline;
pub mod polyline3d;
pub mod lwpolyline;
pub mod text;
pub mod mtext;
pub mod spline;
pub mod dimension;
pub mod hatch;
pub mod solid;
pub mod face3d;
pub mod insert;
pub mod block;
pub mod ray;
pub mod xline;
pub mod viewport;
pub mod attribute_definition;
pub mod attribute_entity;
pub mod leader;
pub mod multileader;
pub mod mline;
pub mod mesh;
pub mod raster_image;
pub mod solid3d;
pub mod acis;
pub mod table;
pub mod tolerance;
pub mod polyface_mesh;
pub mod wipeout;
pub mod shape;
pub mod underlay;
pub mod seqend;
pub mod ole2frame;
pub mod polygon_mesh;
pub mod unknown_entity;
pub mod explode;
pub mod translate;
pub mod transform;
pub mod mirror;

pub use point::Point;
pub use line::Line;
pub use circle::Circle;
pub use arc::Arc;
pub use ellipse::Ellipse;
pub use polyline::{Polyline, Polyline2D, Vertex2D, Vertex3D, PolylineFlags, VertexFlags, SmoothSurfaceType};
pub use polyline3d::{Polyline3D, Vertex3DPolyline, Polyline3DFlags};
pub use lwpolyline::{LwPolyline, LwVertex};
pub use text::{Text, TextHorizontalAlignment, TextVerticalAlignment};
pub use mtext::{MText, AttachmentPoint, DrawingDirection};
pub use spline::{Spline, SplineFlags};
pub use dimension::*;
pub use hatch::*;
pub use solid::Solid;
pub use face3d::{Face3D, InvisibleEdgeFlags};
pub use insert::Insert;
pub use block::{Block, BlockEnd};
pub use ray::Ray;
pub use xline::XLine;
pub use viewport::{Viewport, ViewportStatusFlags, ViewportRenderMode, StandardView, GridFlags};
pub use attribute_definition::{AttributeDefinition, AttributeFlags, HorizontalAlignment, VerticalAlignment, MTextFlag};
pub use attribute_entity::AttributeEntity;
pub use leader::{Leader, LeaderPathType, LeaderCreationType, HooklineDirection};
pub use multileader::{
    MultiLeader, MultiLeaderBuilder, MultiLeaderAnnotContext,
    LeaderRoot, LeaderLine, BlockAttribute, StartEndPointPair,
    LeaderContentType, MultiLeaderPathType, TextAttachmentType, TextAngleType,
    BlockContentConnectionType, TextAttachmentDirectionType, TextAttachmentPointType,
    TextAlignmentType, FlowDirectionType, LineSpacingStyle,
    MultiLeaderPropertyOverrideFlags, LeaderLinePropertyOverrideFlags,
};
pub use mline::{
    MLine, MLineBuilder, MLineVertex, MLineSegment,
    MLineStyle, MLineStyleElement, MLineJustification, MLineFlags, MLineStyleFlags,
};
pub use mesh::{Mesh, MeshBuilder, MeshEdge, MeshFace};
pub use raster_image::{
    RasterImage, RasterImageBuilder, ImageDefinition, ClipBoundary,
    ClipMode, ClipType, ImageDisplayFlags, ImageDisplayQuality, ResolutionUnit,
};
pub use solid3d::{
    Solid3D, Region, Body, Wire, Silhouette, AcisData,
    WireType, AcisVersion,
};
pub use table::{
    Table, TableBuilder, TableCell, TableRow, TableColumn,
    CellContent, CellValue, CellStyle, CellBorder, CellRange,
    CellType, CellValueType, ValueUnitType, BorderType,
    TableCellContentType, CellStyleType, BreakFlowDirection,
    CellEdgeFlags, CellStateFlags, CellStylePropertyFlags,
    BorderPropertyFlags, ContentLayoutFlags, BreakOptionFlags,
};
pub use tolerance::{Tolerance, gdt_symbols};
pub use polyface_mesh::{
    PolyfaceMesh, PolyfaceVertex, PolyfaceFace,
    PolyfaceMeshFlags, PolyfaceVertexFlags, PolyfaceSmoothType,
};
pub use wipeout::{
    Wipeout, WipeoutDisplayFlags, WipeoutClipType, WipeoutClipMode,
};
pub use shape::{Shape, standard_shapes, gdt_shapes};
pub use underlay::{
    Underlay, UnderlayDefinition, UnderlayType, UnderlayDisplayFlags,
    PdfUnderlay, DwfUnderlay, DgnUnderlay,
    PdfUnderlayDefinition, DwfUnderlayDefinition, DgnUnderlayDefinition,
};
pub use seqend::Seqend;
pub use ole2frame::{Ole2Frame, OleObjectType};
pub use polygon_mesh::{
    PolygonMesh as PolygonMeshEntity, PolygonMeshVertex, PolygonMeshFlags, SurfaceSmoothType,
};
pub use unknown_entity::UnknownEntity;

/// Base trait for all CAD entities
pub trait Entity {
    /// Get the entity's unique handle
    fn handle(&self) -> Handle;

    /// Set the entity's handle
    fn set_handle(&mut self, handle: Handle);

    /// Get the entity's layer name
    fn layer(&self) -> &str;

    /// Set the entity's layer name
    fn set_layer(&mut self, layer: String);

    /// Get the entity's color
    fn color(&self) -> Color;

    /// Set the entity's color
    fn set_color(&mut self, color: Color);

    /// Get the entity's line weight
    fn line_weight(&self) -> LineWeight;

    /// Set the entity's line weight
    fn set_line_weight(&mut self, weight: LineWeight);

    /// Get the entity's transparency
    fn transparency(&self) -> Transparency;

    /// Set the entity's transparency
    fn set_transparency(&mut self, transparency: Transparency);

    /// Check if the entity is invisible
    fn is_invisible(&self) -> bool;

    /// Set the entity's visibility
    fn set_invisible(&mut self, invisible: bool);

    /// Get the bounding box of the entity
    fn bounding_box(&self) -> BoundingBox3D;

    /// Transform the entity by a translation vector
    fn translate(&mut self, offset: Vector3);

    /// Get the entity type name
    fn entity_type(&self) -> &'static str;
    
    /// Apply a general transform to the entity
    /// 
    /// This is the main transformation method. Default implementation
    /// only supports translation for backward compatibility.
    fn apply_transform(&mut self, transform: &Transform) {
        // Default: extract translation and apply
        let origin = Vector3::ZERO;
        let translated = transform.apply(origin);
        self.translate(translated);
    }
    
    /// Apply rotation around an axis
    fn apply_rotation(&mut self, axis: Vector3, angle: f64) {
        self.apply_transform(&Transform::from_rotation(axis, angle));
    }
    
    /// Apply uniform scaling
    fn apply_scaling(&mut self, scale: f64) {
        self.apply_transform(&Transform::from_scale(scale));
    }
    
    /// Apply non-uniform scaling
    fn apply_scaling_xyz(&mut self, scale: Vector3) {
        self.apply_transform(&Transform::from_scaling(scale));
    }
    
    /// Apply scaling with a specific origin point
    fn apply_scaling_with_origin(&mut self, scale: Vector3, origin: Vector3) {
        self.apply_transform(&Transform::from_scaling_with_origin(scale, origin));
    }
    
    /// Apply a mirror transform with entity-specific corrections
    ///
    /// Override this for entities that need post-processing after mirroring
    /// (e.g., arc angle swaps, bulge negation, face winding reversal).
    fn apply_mirror(&mut self, transform: &Transform) {
        self.apply_transform(transform);
    }
    
    /// Mirror the entity across the YZ plane (negate X coordinates)
    fn mirror_x(&mut self) {
        self.apply_mirror(&Transform::from_mirror_x());
    }
    
    /// Mirror the entity across the XZ plane (negate Y coordinates)
    fn mirror_y(&mut self) {
        self.apply_mirror(&Transform::from_mirror_y());
    }
    
    /// Mirror the entity across the XY plane (negate Z coordinates)
    fn mirror_z(&mut self) {
        self.apply_mirror(&Transform::from_mirror_z());
    }
    
    /// Mirror the entity across a line defined by two points (in the XY plane)
    fn mirror_about_line(&mut self, p1: Vector3, p2: Vector3) {
        self.apply_mirror(&Transform::from_mirror_line(p1, p2));
    }
    
    /// Mirror the entity across an arbitrary plane
    fn mirror_about_plane(&mut self, point: Vector3, normal: Vector3) {
        self.apply_mirror(&Transform::from_mirror_plane(point, normal));
    }
}

/// Common entity data shared by all entities
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityCommon {
    /// Unique handle
    pub handle: Handle,
    /// Layer name
    pub layer: String,
    /// Color
    pub color: Color,
    /// Line weight
    pub line_weight: LineWeight,
    /// Linetype name (empty string = "ByLayer")
    pub linetype: String,
    /// Linetype scale factor (default 1.0)
    pub linetype_scale: f64,
    /// Transparency
    pub transparency: Transparency,
    /// Visibility flag
    pub invisible: bool,
    /// Extended data (XDATA)
    pub extended_data: crate::xdata::ExtendedData,
    /// Raw entity graphic data bytes (stored for DWG round-trip; None otherwise).
    #[cfg_attr(feature = "serde", serde(skip))]
    pub graphic_data: Option<Vec<u8>>,
    /// Reactor handles — objects attached as reactors ({ACAD_REACTORS})
    pub reactors: Vec<Handle>,
    /// Extended dictionary handle ({ACAD_XDICTIONARY}) — hard-owner handle to a Dictionary
    pub xdictionary_handle: Option<Handle>,
    /// Owner handle (soft pointer, code 330)
    pub owner_handle: Handle,

    // ── DWG round-trip fields (not exposed via DXF) ──
    /// Material flags (BB: 00=bylayer, 01=byblock, 10=reserved, 11=handle) — R2007+
    #[cfg_attr(feature = "serde", serde(skip))]
    pub material_flags: u8,
    /// Material handle (only valid when material_flags == 0b11) — R2007+
    #[cfg_attr(feature = "serde", serde(skip))]
    pub material_handle: Option<Handle>,
    /// Shadow flags (RC) — R2007+
    #[cfg_attr(feature = "serde", serde(skip))]
    pub shadow_flags: u8,
    /// Plotstyle flags (BB: 00=bylayer, 01=byblock, 10=reserved, 11=handle) — R2000+
    #[cfg_attr(feature = "serde", serde(skip))]
    pub plotstyle_flags: u8,
    /// Plotstyle handle (only valid when plotstyle_flags == 0b11) — R2000+
    #[cfg_attr(feature = "serde", serde(skip))]
    pub plotstyle_handle: Option<Handle>,
    /// Entity mode (0=owned, 1=paper, 2=model) — raw DWG value for round-trip
    #[cfg_attr(feature = "serde", serde(skip))]
    pub entity_mode: Option<u8>,
}

impl EntityCommon {
    /// Create new common entity data with defaults
    pub fn new() -> Self {
        EntityCommon {
            handle: Handle::NULL,
            layer: "0".to_string(),
            color: Color::ByLayer,
            line_weight: LineWeight::ByLayer,
            linetype: String::new(),
            linetype_scale: 1.0,
            transparency: Transparency::OPAQUE,
            invisible: false,
            extended_data: crate::xdata::ExtendedData::new(),
            graphic_data: None,
            reactors: Vec::new(),
            xdictionary_handle: None,
            owner_handle: Handle::NULL,
            material_flags: 0,
            material_handle: None,
            shadow_flags: 0,
            plotstyle_flags: 0,
            plotstyle_handle: None,
            entity_mode: None,
        }
    }

    /// Create with a specific layer
    pub fn with_layer(layer: impl Into<String>) -> Self {
        EntityCommon {
            layer: layer.into(),
            ..Self::new()
        }
    }

    /// Check whether a linetype name is set (not empty and not "ByLayer")
    pub fn has_linetype(&self) -> bool {
        !self.linetype.is_empty() && self.linetype != "ByLayer"
    }
}

impl Default for EntityCommon {
    fn default() -> Self {
        Self::new()
    }
}

/// Enumeration of all entity types for type-safe storage
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EntityType {
    /// Point entity
    Point(Point),
    /// Line entity
    Line(Line),
    /// Circle entity
    Circle(Circle),
    /// Arc entity
    Arc(Arc),
    /// Ellipse entity
    Ellipse(Ellipse),
    /// 3D Polyline entity
    Polyline(Polyline),
    /// 2D Polyline entity (heavy polyline)
    Polyline2D(Polyline2D),
    /// 3D Polyline entity (new style)
    Polyline3D(Polyline3D),
    /// Lightweight polyline entity
    LwPolyline(LwPolyline),
    /// Text entity
    Text(Text),
    /// Multi-line text entity
    MText(MText),
    /// Spline entity
    Spline(Spline),
    /// Dimension entity
    Dimension(Dimension),
    /// Hatch entity
    Hatch(Hatch),
    /// Solid entity
    Solid(Solid),
    /// 3D Face entity
    Face3D(Face3D),
    /// Insert entity (block reference)
    Insert(Insert),
    /// Block entity (block definition start)
    Block(Block),
    /// BlockEnd entity (block definition end)
    BlockEnd(BlockEnd),
    /// Ray entity (semi-infinite line)
    Ray(Ray),
    /// XLine entity (construction line, infinite)
    XLine(XLine),
    /// Viewport entity (paper space viewport)
    Viewport(Viewport),
    /// Attribute definition entity
    AttributeDefinition(AttributeDefinition),
    /// Attribute entity (block attribute instance)
    AttributeEntity(AttributeEntity),
    /// Leader entity
    Leader(Leader),
    /// MultiLeader entity
    MultiLeader(MultiLeader),
    /// MLine (multiline) entity
    MLine(MLine),
    /// Mesh entity
    Mesh(Mesh),
    /// RasterImage entity
    RasterImage(RasterImage),
    /// Solid3D entity
    Solid3D(Solid3D),
    /// Region entity
    Region(Region),
    /// Body entity
    Body(Body),
    /// Table entity
    Table(Table),
    /// Tolerance entity (geometric tolerancing)
    Tolerance(Tolerance),
    /// PolyfaceMesh entity
    PolyfaceMesh(PolyfaceMesh),
    /// Wipeout entity
    Wipeout(Wipeout),
    /// Shape entity
    Shape(Shape),
    /// Underlay entity (PDF, DWF, DGN)
    Underlay(Underlay),
    /// End-of-sequence marker
    Seqend(Seqend),
    /// OLE2 embedded object
    Ole2Frame(Ole2Frame),
    /// Polygon mesh (3D surface mesh)
    PolygonMesh(PolygonMeshEntity),
    /// Unknown / unsupported entity type (common fields only)
    Unknown(UnknownEntity),
}

impl EntityType {
    /// Get a reference to the entity trait object
    pub fn as_entity(&self) -> &dyn Entity {
        match self {
            EntityType::Point(e) => e,
            EntityType::Line(e) => e,
            EntityType::Circle(e) => e,
            EntityType::Arc(e) => e,
            EntityType::Ellipse(e) => e,
            EntityType::Polyline(e) => e,
            EntityType::Polyline2D(e) => e,
            EntityType::Polyline3D(e) => e,
            EntityType::LwPolyline(e) => e,
            EntityType::Text(e) => e,
            EntityType::MText(e) => e,
            EntityType::Spline(e) => e,
            EntityType::Dimension(e) => e,
            EntityType::Hatch(e) => e,
            EntityType::Solid(e) => e,
            EntityType::Face3D(e) => e,
            EntityType::Insert(e) => e,
            EntityType::Block(e) => e,
            EntityType::BlockEnd(e) => e,
            EntityType::Ray(e) => e,
            EntityType::XLine(e) => e,
            EntityType::Viewport(e) => e,
            EntityType::AttributeDefinition(e) => e,
            EntityType::AttributeEntity(e) => e,
            EntityType::Leader(e) => e,
            EntityType::MultiLeader(e) => e,
            EntityType::MLine(e) => e,
            EntityType::Mesh(e) => e,
            EntityType::RasterImage(e) => e,
            EntityType::Solid3D(e) => e,
            EntityType::Region(e) => e,
            EntityType::Body(e) => e,
            EntityType::Table(e) => e,
            EntityType::Tolerance(e) => e,
            EntityType::PolyfaceMesh(e) => e,
            EntityType::Wipeout(e) => e,
            EntityType::Shape(e) => e,
            EntityType::Underlay(e) => e,
            EntityType::Seqend(e) => e,
            EntityType::Ole2Frame(e) => e,
            EntityType::PolygonMesh(e) => e,
            EntityType::Unknown(e) => e,
        }
    }

    /// Get a mutable reference to the entity trait object
    pub fn as_entity_mut(&mut self) -> &mut dyn Entity {
        match self {
            EntityType::Point(e) => e,
            EntityType::Line(e) => e,
            EntityType::Circle(e) => e,
            EntityType::Arc(e) => e,
            EntityType::Ellipse(e) => e,
            EntityType::Polyline(e) => e,
            EntityType::Polyline2D(e) => e,
            EntityType::Polyline3D(e) => e,
            EntityType::LwPolyline(e) => e,
            EntityType::MText(e) => e,
            EntityType::Text(e) => e,
            EntityType::Spline(e) => e,
            EntityType::Dimension(e) => e,
            EntityType::Hatch(e) => e,
            EntityType::Solid(e) => e,
            EntityType::Face3D(e) => e,
            EntityType::Insert(e) => e,
            EntityType::Block(e) => e,
            EntityType::BlockEnd(e) => e,
            EntityType::Ray(e) => e,
            EntityType::XLine(e) => e,
            EntityType::Viewport(e) => e,
            EntityType::AttributeDefinition(e) => e,
            EntityType::AttributeEntity(e) => e,
            EntityType::Leader(e) => e,
            EntityType::MultiLeader(e) => e,
            EntityType::MLine(e) => e,
            EntityType::Mesh(e) => e,
            EntityType::RasterImage(e) => e,
            EntityType::Solid3D(e) => e,
            EntityType::Region(e) => e,
            EntityType::Body(e) => e,
            EntityType::Table(e) => e,
            EntityType::Tolerance(e) => e,
            EntityType::PolyfaceMesh(e) => e,
            EntityType::Wipeout(e) => e,
            EntityType::Shape(e) => e,
            EntityType::Underlay(e) => e,
            EntityType::Seqend(e) => e,
            EntityType::Ole2Frame(e) => e,
            EntityType::PolygonMesh(e) => e,
            EntityType::Unknown(e) => e,
        }
    }

    /// Get a reference to the entity's common data
    pub fn common(&self) -> &EntityCommon {
        match self {
            EntityType::Point(e) => &e.common,
            EntityType::Line(e) => &e.common,
            EntityType::Circle(e) => &e.common,
            EntityType::Arc(e) => &e.common,
            EntityType::Ellipse(e) => &e.common,
            EntityType::Polyline(e) => &e.common,
            EntityType::Polyline2D(e) => &e.common,
            EntityType::Polyline3D(e) => &e.common,
            EntityType::LwPolyline(e) => &e.common,
            EntityType::Text(e) => &e.common,
            EntityType::MText(e) => &e.common,
            EntityType::Spline(e) => &e.common,
            EntityType::Dimension(e) => &e.base().common,
            EntityType::Hatch(e) => &e.common,
            EntityType::Solid(e) => &e.common,
            EntityType::Face3D(e) => &e.common,
            EntityType::Insert(e) => &e.common,
            EntityType::Block(e) => &e.common,
            EntityType::BlockEnd(e) => &e.common,
            EntityType::Ray(e) => &e.common,
            EntityType::XLine(e) => &e.common,
            EntityType::Viewport(e) => &e.common,
            EntityType::AttributeDefinition(e) => &e.common,
            EntityType::AttributeEntity(e) => &e.common,
            EntityType::Leader(e) => &e.common,
            EntityType::MultiLeader(e) => &e.common,
            EntityType::MLine(e) => &e.common,
            EntityType::Mesh(e) => &e.common,
            EntityType::RasterImage(e) => &e.common,
            EntityType::Solid3D(e) => &e.common,
            EntityType::Region(e) => &e.common,
            EntityType::Body(e) => &e.common,
            EntityType::Table(e) => &e.common,
            EntityType::Tolerance(e) => &e.common,
            EntityType::PolyfaceMesh(e) => &e.common,
            EntityType::Wipeout(e) => &e.common,
            EntityType::Shape(e) => &e.common,
            EntityType::Underlay(e) => &e.common,
            EntityType::Seqend(e) => &e.common,
            EntityType::Ole2Frame(e) => &e.common,
            EntityType::PolygonMesh(e) => &e.common,
            EntityType::Unknown(e) => &e.common,
        }
    }

    /// Get a mutable reference to the entity's common data
    pub fn common_mut(&mut self) -> &mut EntityCommon {
        match self {
            EntityType::Point(e) => &mut e.common,
            EntityType::Line(e) => &mut e.common,
            EntityType::Circle(e) => &mut e.common,
            EntityType::Arc(e) => &mut e.common,
            EntityType::Ellipse(e) => &mut e.common,
            EntityType::Polyline(e) => &mut e.common,
            EntityType::Polyline2D(e) => &mut e.common,
            EntityType::Polyline3D(e) => &mut e.common,
            EntityType::LwPolyline(e) => &mut e.common,
            EntityType::Text(e) => &mut e.common,
            EntityType::MText(e) => &mut e.common,
            EntityType::Spline(e) => &mut e.common,
            EntityType::Dimension(e) => &mut e.base_mut().common,
            EntityType::Hatch(e) => &mut e.common,
            EntityType::Solid(e) => &mut e.common,
            EntityType::Face3D(e) => &mut e.common,
            EntityType::Insert(e) => &mut e.common,
            EntityType::Block(e) => &mut e.common,
            EntityType::BlockEnd(e) => &mut e.common,
            EntityType::Ray(e) => &mut e.common,
            EntityType::XLine(e) => &mut e.common,
            EntityType::Viewport(e) => &mut e.common,
            EntityType::AttributeDefinition(e) => &mut e.common,
            EntityType::AttributeEntity(e) => &mut e.common,
            EntityType::Leader(e) => &mut e.common,
            EntityType::MultiLeader(e) => &mut e.common,
            EntityType::MLine(e) => &mut e.common,
            EntityType::Mesh(e) => &mut e.common,
            EntityType::RasterImage(e) => &mut e.common,
            EntityType::Solid3D(e) => &mut e.common,
            EntityType::Region(e) => &mut e.common,
            EntityType::Body(e) => &mut e.common,
            EntityType::Table(e) => &mut e.common,
            EntityType::Tolerance(e) => &mut e.common,
            EntityType::PolyfaceMesh(e) => &mut e.common,
            EntityType::Wipeout(e) => &mut e.common,
            EntityType::Shape(e) => &mut e.common,
            EntityType::Underlay(e) => &mut e.common,
            EntityType::Seqend(e) => &mut e.common,
            EntityType::Ole2Frame(e) => &mut e.common,
            EntityType::PolygonMesh(e) => &mut e.common,
            EntityType::Unknown(e) => &mut e.common,
        }
    }

    /// Compute the axis-aligned bounding box for this entity.
    ///
    /// Delegates to the [`Entity::bounding_box()`] trait method.
    pub fn bounding_box(&self) -> BoundingBox3D {
        self.as_entity().bounding_box()
    }

    // ── Convenience type-check and downcast helpers ──────────────────

    /// Check whether this entity is of a given concrete type.
    ///
    /// ```
    /// use acadrust::entities::{EntityType, Circle, EntityVariant};
    /// let et = EntityType::Circle(Circle::new());
    /// assert!(et.is::<Circle>());
    /// ```
    pub fn is<T: EntityVariant>(&self) -> bool {
        T::from_entity_type(self).is_some()
    }

    /// Try to downcast to a concrete entity type (immutable).
    ///
    /// ```
    /// use acadrust::entities::{EntityType, Circle, EntityVariant};
    /// let et = EntityType::Circle(Circle::new());
    /// assert!(et.downcast_ref::<Circle>().is_some());
    /// ```
    pub fn downcast_ref<T: EntityVariant>(&self) -> Option<&T> {
        T::from_entity_type(self)
    }

    /// Try to downcast to a concrete entity type (mutable).
    pub fn downcast_mut<T: EntityVariant>(&mut self) -> Option<&mut T> {
        T::from_entity_type_mut(self)
    }
}

macro_rules! impl_entity_type_helpers {
    ($($fn_is:ident, $fn_as:ident, $fn_as_mut:ident, $variant:ident, $ty:ty);* $(;)?) => {
        impl EntityType {
        $(
            /// Returns `true` if this entity is the corresponding variant.
            pub fn $fn_is(&self) -> bool {
                matches!(self, EntityType::$variant(_))
            }

            /// Try to get an immutable reference to the inner type.
            pub fn $fn_as(&self) -> Option<&$ty> {
                match self {
                    EntityType::$variant(inner) => Some(inner),
                    _ => None,
                }
            }

            /// Try to get a mutable reference to the inner type.
            pub fn $fn_as_mut(&mut self) -> Option<&mut $ty> {
                match self {
                    EntityType::$variant(inner) => Some(inner),
                    _ => None,
                }
            }
        )*
        }
    };
}

impl_entity_type_helpers! {
    is_point, as_point, as_point_mut, Point, Point;
    is_line, as_line, as_line_mut, Line, Line;
    is_circle, as_circle, as_circle_mut, Circle, Circle;
    is_arc, as_arc, as_arc_mut, Arc, Arc;
    is_ellipse, as_ellipse, as_ellipse_mut, Ellipse, Ellipse;
    is_polyline, as_polyline, as_polyline_mut, Polyline, Polyline;
    is_polyline2d, as_polyline2d, as_polyline2d_mut, Polyline2D, Polyline2D;
    is_polyline3d, as_polyline3d, as_polyline3d_mut, Polyline3D, Polyline3D;
    is_lwpolyline, as_lwpolyline, as_lwpolyline_mut, LwPolyline, LwPolyline;
    is_text, as_text, as_text_mut, Text, Text;
    is_mtext, as_mtext, as_mtext_mut, MText, MText;
    is_spline, as_spline, as_spline_mut, Spline, Spline;
    is_dimension, as_dimension, as_dimension_mut, Dimension, Dimension;
    is_hatch, as_hatch, as_hatch_mut, Hatch, Hatch;
    is_solid, as_solid, as_solid_mut, Solid, Solid;
    is_face3d, as_face3d, as_face3d_mut, Face3D, Face3D;
    is_insert, as_insert, as_insert_mut, Insert, Insert;
    is_block, as_block, as_block_mut, Block, Block;
    is_block_end, as_block_end, as_block_end_mut, BlockEnd, BlockEnd;
    is_ray, as_ray, as_ray_mut, Ray, Ray;
    is_xline, as_xline, as_xline_mut, XLine, XLine;
    is_viewport, as_viewport, as_viewport_mut, Viewport, Viewport;
    is_attribute_definition, as_attribute_definition, as_attribute_definition_mut, AttributeDefinition, AttributeDefinition;
    is_attribute_entity, as_attribute_entity, as_attribute_entity_mut, AttributeEntity, AttributeEntity;
    is_leader, as_leader, as_leader_mut, Leader, Leader;
    is_multileader, as_multileader, as_multileader_mut, MultiLeader, MultiLeader;
    is_mline, as_mline, as_mline_mut, MLine, MLine;
    is_mesh, as_mesh, as_mesh_mut, Mesh, Mesh;
    is_raster_image, as_raster_image, as_raster_image_mut, RasterImage, RasterImage;
    is_solid3d, as_solid3d, as_solid3d_mut, Solid3D, Solid3D;
    is_region, as_region, as_region_mut, Region, Region;
    is_body, as_body, as_body_mut, Body, Body;
    is_table, as_table, as_table_mut, Table, Table;
    is_tolerance, as_tolerance, as_tolerance_mut, Tolerance, Tolerance;
    is_polyface_mesh, as_polyface_mesh, as_polyface_mesh_mut, PolyfaceMesh, PolyfaceMesh;
    is_wipeout, as_wipeout, as_wipeout_mut, Wipeout, Wipeout;
    is_shape, as_shape, as_shape_mut, Shape, Shape;
    is_underlay, as_underlay, as_underlay_mut, Underlay, Underlay;
    is_seqend, as_seqend, as_seqend_mut, Seqend, Seqend;
    is_ole2_frame, as_ole2_frame, as_ole2_frame_mut, Ole2Frame, Ole2Frame;
    is_polygon_mesh, as_polygon_mesh, as_polygon_mesh_mut, PolygonMesh, PolygonMeshEntity;
    is_unknown, as_unknown, as_unknown_mut, Unknown, UnknownEntity;
}

/// Trait for concrete entity types that can be extracted from an [`EntityType`] variant.
///
/// This is implemented for every inner entity type (e.g. `Circle`, `Line`, `Arc`, …)
/// so that generic code can filter entities by type:
///
/// ```
/// use acadrust::entities::{EntityType, EntityVariant, Circle};
///
/// let et = EntityType::Circle(Circle::new());
/// assert!(Circle::from_entity_type(&et).is_some());
/// ```
pub trait EntityVariant: Sized {
    /// Try to extract an immutable reference from an [`EntityType`].
    fn from_entity_type(e: &EntityType) -> Option<&Self>;
    /// Try to extract a mutable reference from an [`EntityType`].
    fn from_entity_type_mut(e: &mut EntityType) -> Option<&mut Self>;
}

macro_rules! impl_entity_variant {
    ($variant:ident, $ty:ty) => {
        impl EntityVariant for $ty {
            fn from_entity_type(e: &EntityType) -> Option<&Self> {
                match e {
                    EntityType::$variant(inner) => Some(inner),
                    _ => None,
                }
            }
            fn from_entity_type_mut(e: &mut EntityType) -> Option<&mut Self> {
                match e {
                    EntityType::$variant(inner) => Some(inner),
                    _ => None,
                }
            }
        }
    };
}

impl_entity_variant!(Point, Point);
impl_entity_variant!(Line, Line);
impl_entity_variant!(Circle, Circle);
impl_entity_variant!(Arc, Arc);
impl_entity_variant!(Ellipse, Ellipse);
impl_entity_variant!(Polyline, Polyline);
impl_entity_variant!(Polyline2D, Polyline2D);
impl_entity_variant!(Polyline3D, Polyline3D);
impl_entity_variant!(LwPolyline, LwPolyline);
impl_entity_variant!(Text, Text);
impl_entity_variant!(MText, MText);
impl_entity_variant!(Spline, Spline);
impl_entity_variant!(Dimension, Dimension);
impl_entity_variant!(Hatch, Hatch);
impl_entity_variant!(Solid, Solid);
impl_entity_variant!(Face3D, Face3D);
impl_entity_variant!(Insert, Insert);
impl_entity_variant!(Block, Block);
impl_entity_variant!(BlockEnd, BlockEnd);
impl_entity_variant!(Ray, Ray);
impl_entity_variant!(XLine, XLine);
impl_entity_variant!(Viewport, Viewport);
impl_entity_variant!(AttributeDefinition, AttributeDefinition);
impl_entity_variant!(AttributeEntity, AttributeEntity);
impl_entity_variant!(Leader, Leader);
impl_entity_variant!(MultiLeader, MultiLeader);
impl_entity_variant!(MLine, MLine);
impl_entity_variant!(Mesh, Mesh);
impl_entity_variant!(RasterImage, RasterImage);
impl_entity_variant!(Solid3D, Solid3D);
impl_entity_variant!(Region, Region);
impl_entity_variant!(Body, Body);
impl_entity_variant!(Table, Table);
impl_entity_variant!(Tolerance, Tolerance);
impl_entity_variant!(PolyfaceMesh, PolyfaceMesh);
impl_entity_variant!(Wipeout, Wipeout);
impl_entity_variant!(Shape, Shape);
impl_entity_variant!(Underlay, Underlay);
impl_entity_variant!(Seqend, Seqend);
impl_entity_variant!(Ole2Frame, Ole2Frame);
impl_entity_variant!(PolygonMesh, PolygonMeshEntity);
impl_entity_variant!(Unknown, UnknownEntity);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_line() {
        let et = EntityType::Line(Line::new());
        assert!(et.is_line());
        assert!(!et.is_circle());
        assert!(!et.is_arc());
    }

    #[test]
    fn test_is_circle() {
        let et = EntityType::Circle(Circle::new());
        assert!(et.is_circle());
        assert!(!et.is_line());
    }

    #[test]
    fn test_as_line() {
        let et = EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 1.0, 0.0, 0.0));
        let l = et.as_line().unwrap();
        assert!((l.end.x - 1.0).abs() < 1e-10);
        assert!(et.as_circle().is_none());
    }

    #[test]
    fn test_as_circle_mut() {
        let mut et = EntityType::Circle(Circle::new());
        et.as_circle_mut().unwrap().radius = 42.0;
        assert!((et.as_circle().unwrap().radius - 42.0).abs() < 1e-10);
    }

    #[test]
    fn test_is_generic() {
        let et = EntityType::Arc(Arc::new());
        assert!(et.is::<Arc>());
        assert!(!et.is::<Circle>());
    }

    #[test]
    fn test_downcast_ref() {
        let et = EntityType::Spline(Spline::new());
        assert!(et.downcast_ref::<Spline>().is_some());
        assert!(et.downcast_ref::<Line>().is_none());
    }

    #[test]
    fn test_downcast_mut() {
        let mut et = EntityType::Hatch(Hatch::new());
        assert!(et.downcast_mut::<Hatch>().is_some());
        assert!(et.downcast_mut::<Circle>().is_none());
    }

    #[test]
    fn test_all_type_checks() {
        // Spot check several variants
        assert!(EntityType::Point(Point::new()).is_point());
        assert!(EntityType::Arc(Arc::new()).is_arc());
        assert!(EntityType::Ellipse(Ellipse::new()).is_ellipse());
        assert!(EntityType::Spline(Spline::new()).is_spline());
        assert!(EntityType::Text(Text::new()).is_text());
        assert!(EntityType::MText(MText::new()).is_mtext());
        assert!(EntityType::Insert(Insert::new("B", Vector3::ZERO)).is_insert());
        assert!(EntityType::Dimension(Dimension::Linear(
            crate::entities::dimension::DimensionLinear::new(Vector3::ZERO, Vector3::new(1.0, 0.0, 0.0)),
        )).is_dimension());
        assert!(EntityType::Leader(Leader::new()).is_leader());
        assert!(EntityType::Mesh(Mesh::new()).is_mesh());
    }

    #[test]
    fn test_extended_type_helpers() {
        assert!(EntityType::Polyline3D(Polyline3D::new()).is_polyline3d());
        assert!(EntityType::BlockEnd(BlockEnd::new()).is_block_end());
        assert!(
            EntityType::AttributeDefinition(AttributeDefinition::new(
                "TAG".to_string(),
                "Prompt".to_string(),
                "Default".to_string(),
            ))
            .is_attribute_definition()
        );
        assert!(
            EntityType::AttributeEntity(AttributeEntity::new(
                "TAG".to_string(),
                "Value".to_string(),
            ))
            .is_attribute_entity()
        );
        assert!(EntityType::PolyfaceMesh(PolyfaceMesh::new()).is_polyface_mesh());
        assert!(EntityType::Shape(Shape::new()).is_shape());
        assert!(EntityType::Underlay(Underlay::new(UnderlayType::Pdf)).is_underlay());
        assert!(EntityType::Seqend(Seqend::new()).is_seqend());
        assert!(EntityType::Ole2Frame(Ole2Frame::new()).is_ole2_frame());
        assert!(EntityType::PolygonMesh(PolygonMeshEntity::new()).is_polygon_mesh());
        assert!(EntityType::Unknown(UnknownEntity::new("FOO")).is_unknown());
    }
}
