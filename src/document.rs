//! Central CAD document structure.
//!
//! [`CadDocument`] is the top-level container that holds everything in a
//! drawing: header variables, tables (layers, line types, text styles, …),
//! entities, non-graphical objects, block definitions, and classes.
//!
//! # Creating a document
//!
//! ```rust
//! use acadrust::CadDocument;
//!
//! // Default version (R2018 / AC1032)
//! let doc = CadDocument::new();
//!
//! // Specific version
//! use acadrust::types::DxfVersion;
//! let doc = CadDocument::with_version(DxfVersion::AC1015); // R2000
//! ```

use crate::classes::DxfClassCollection;
use crate::entities::{EntityCommon, EntityType};
use crate::objects::ObjectType;
use crate::tables::*;
use crate::types::{BoundingBox3D, DxfVersion, Color, Handle, Vector2, Vector3};
use crate::Result;
use std::collections::HashMap;
use std::io::Cursor;

/// DWG header variables containing drawing settings
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HeaderVariables {
    // ==================== Version-specific Flags ====================
    /// REQUIREDVERSIONS (R2013+) - Bit coded required versions
    pub required_versions: i64,
    
    // ==================== Drawing Mode Flags ====================
    /// DIMASO - Associates dimensions with geometry
    pub associate_dimensions: bool,
    /// DIMSHO - Updates dimensions while dragging
    pub update_dimensions_while_dragging: bool,
    /// ORTHOMODE - Orthogonal mode on/off
    pub ortho_mode: bool,
    /// FILLMODE - Fill mode for solids/hatches
    pub fill_mode: bool,
    /// QTEXTMODE - Quick text mode (boxes instead of text)
    pub quick_text_mode: bool,
    /// MIRRTEXT - Mirror text on/off
    pub mirror_text: bool,
    /// REGENMODE - Auto regeneration mode
    pub regen_mode: bool,
    /// LIMCHECK - Limits checking on/off
    pub limit_check: bool,
    /// PLIMCHECK - Paper space limits checking
    pub paper_space_limit_check: bool,
    /// PLINEGEN - Line type pattern generation for polylines
    pub polyline_linetype_generation: bool,
    /// PSLTSCALE - Paper space line type scaling (0=viewport, 1=normal)
    pub paper_space_linetype_scaling: bool,
    /// TILEMODE - Show model space (tile mode)
    pub show_model_space: bool,
    /// USRTIMER - User timer on/off
    pub user_timer: bool,
    /// WORLDVIEW - World view on/off
    pub world_view: bool,
    /// VISRETAIN - Retain xref visibility settings
    pub retain_xref_visibility: bool,
    /// DISPSILH - Silhouette display for 3D objects
    pub display_silhouette: bool,
    /// SPLFRAME - Display spline control polygon
    pub spline_frame: bool,
    /// DELOBJ - Delete source objects for regions/solids
    pub delete_objects: bool,
    /// DRAGMODE - Drag mode (0=off, 1=on request, 2=auto)
    pub drag_mode: i16,
    /// BLIPMODE - Blip mode on/off
    pub blip_mode: bool,
    /// ATTREQ - Attribute entry dialogs
    pub attribute_request: bool,
    /// ATTDIA - Attribute dialog mode
    pub attribute_dialog: bool,
    
    // ==================== Unit Settings ====================
    /// LUNITS - Linear units format (0=Scientific, 1=Decimal, 2=Engineering, 3=Architectural, 4=Fractional)
    pub linear_unit_format: i16,
    /// LUPREC - Linear unit precision (0-8)
    pub linear_unit_precision: i16,
    /// AUNITS - Angular units format (0=Decimal degrees, 1=DMS, 2=Gradians, 3=Radians, 4=Surveyor)
    pub angular_unit_format: i16,
    /// AUPREC - Angular unit precision (0-8)
    pub angular_unit_precision: i16,
    /// INSUNITS - Insertion units (0=Unitless, 1=Inches, 2=Feet, etc.)
    pub insertion_units: i16,
    /// ATTMODE - Attribute display mode (0=off, 1=normal, 2=all)
    pub attribute_visibility: i16,
    /// PDMODE - Point display mode
    pub point_display_mode: i16,
    /// USERI1-5 - User integer variables
    pub user_int1: i16,
    pub user_int2: i16,
    pub user_int3: i16,
    pub user_int4: i16,
    pub user_int5: i16,
    /// COORDS - Coordinate display mode
    pub coords_mode: i16,
    /// OSMODE - Object snap mode bits
    pub object_snap_mode: i32,
    /// PICKSTYLE - Pick style
    pub pick_style: i16,
    /// SPLINETYPE - Spline type (5=quadratic, 6=cubic)
    pub spline_type: i16,
    /// SPLINESEGS - Spline segments for approximation
    pub spline_segments: i16,
    /// SPLINESEGQS - Spline segments for surface fit
    pub spline_segs_surface: i16,
    /// SURFU - Surface U density
    pub surface_u_density: i16,
    /// SURFV - Surface V density
    pub surface_v_density: i16,
    /// SURFTYPE - Surface type
    pub surface_type: i16,
    /// SURFTAB1 - Surface tabulation 1
    pub surface_tab1: i16,
    /// SURFTAB2 - Surface tabulation 2
    pub surface_tab2: i16,
    /// SHADEDGE - Shade edge mode
    pub shade_edge: i16,
    /// SHADEDIF - Shade diffuse percentage
    pub shade_diffuse: i16,
    /// MAXACTVP - Maximum active viewports
    pub max_active_viewports: i16,
    /// ISOLINES - Isolines on surfaces
    pub isolines: i16,
    /// CMLJUST - Multiline justification
    pub multiline_justification: i16,
    /// TEXTQLTY - Text quality for TrueType
    pub text_quality: i16,
    /// SORTENTS - Entity sort flags
    pub sort_entities: i16,
    /// INDEXCTL - Index control flags
    pub index_control: i16,
    /// HIDETEXT - Hide text during HIDE command
    pub hide_text: i16,
    /// XCLIPFRAME - Xref clipping frame visibility
    pub xclip_frame: i16,
    /// HALOGAP - Halo gap percentage
    pub halo_gap: i16,
    /// OBSCOLOR - Obscured line color
    pub obscured_color: i16,
    /// OBSLTYPE - Obscured line type
    pub obscured_linetype: i16,
    /// INTERSECTIONDISPLAY - Intersection polyline display
    pub intersection_display: i16,
    /// INTERSECTIONCOLOR - Intersection polyline color
    pub intersection_color: i16,
    /// DIMASSOC - Dimension associativity (0=no, 1=non-exploded, 2=associative)
    pub dimension_associativity: i16,
    /// PROJECTNAME - Project name
    pub project_name: String,
    
    // ==================== Scale/Size Defaults ====================
    /// LTSCALE - Global linetype scale
    pub linetype_scale: f64,
    /// TEXTSIZE - Default text height
    pub text_height: f64,
    /// TRACEWID - Default trace width
    pub trace_width: f64,
    /// SKETCHINC - Sketch increment
    pub sketch_increment: f64,
    /// THICKNESS - Default thickness
    pub thickness: f64,
    /// PDSIZE - Point display size
    pub point_display_size: f64,
    /// PLINEWID - Default polyline width
    pub polyline_width: f64,
    /// CELTSCALE - Current entity linetype scale
    pub current_entity_linetype_scale: f64,
    /// VIEWTWIST - View twist angle
    pub view_twist: f64,
    /// FILLETRAD - Fillet radius
    pub fillet_radius: f64,
    /// CHAMFERA - Chamfer distance A
    pub chamfer_distance_a: f64,
    /// CHAMFERB - Chamfer distance B
    pub chamfer_distance_b: f64,
    /// CHAMFERC - Chamfer length
    pub chamfer_length: f64,
    /// CHAMFERD - Chamfer angle
    pub chamfer_angle: f64,
    /// ANGBASE - Base angle
    pub angle_base: f64,
    /// ANGDIR - Angular direction (0=counterclockwise, 1=clockwise)
    pub angle_direction: i16,
    /// ELEVATION - Current elevation
    pub elevation: f64,
    /// PELEVATION - Paper space elevation
    pub paper_elevation: f64,
    /// FACETRES - Facet resolution
    pub facet_resolution: f64,
    /// CMLSCALE - Multiline scale
    pub multiline_scale: f64,
    /// USERR1-5 - User real variables
    pub user_real1: f64,
    pub user_real2: f64,
    pub user_real3: f64,
    pub user_real4: f64,
    pub user_real5: f64,
    /// PSVPSCALE - Viewport default view scale factor (R2000+)
    pub viewport_scale_factor: f64,
    /// SHADOWPLANELOCATION - Shadow plane Z location
    pub shadow_plane_location: f64,
    /// LOFTANG1 - Loft angle 1
    pub loft_angle1: f64,
    /// LOFTANG2 - Loft angle 2
    pub loft_angle2: f64,
    /// LOFTMAG1 - Loft magnitude 1
    pub loft_magnitude1: f64,
    /// LOFTMAG2 - Loft magnitude 2
    pub loft_magnitude2: f64,
    /// LOFTPARAM - Loft parameters
    pub loft_param: i16,
    /// LOFTNORMALS - Loft normals mode
    pub loft_normals: i16,
    /// LATITUDE - Geographic latitude
    pub latitude: f64,
    /// LONGITUDE - Geographic longitude
    pub longitude: f64,
    /// NORTHDIRECTION - North direction angle
    pub north_direction: f64,
    /// TIMEZONE - Time zone
    pub timezone: i32,
    /// STEPSPERSEC - Steps per second for walk/fly
    pub steps_per_second: f64,
    /// STEPSIZE - Step size for walk/fly
    pub step_size: f64,
    /// LENSLENGTH - Camera lens length
    pub lens_length: f64,
    /// CAMERAHEIGHT - Camera height
    pub camera_height: f64,
    /// CAMERADISPLAY - Camera display mode
    pub camera_display: bool,
    
    // ==================== Current Entity Settings ====================
    /// CECOLOR - Current entity color
    pub current_entity_color: Color,
    /// CELWEIGHT - Current line weight
    pub current_line_weight: i16,
    /// CEPSNTYPE - Current plot style name type
    pub current_plotstyle_type: i16,
    /// ENDCAPS - Line end cap style
    pub end_caps: i16,
    /// JOINSTYLE - Line join style
    pub join_style: i16,
    /// LWDISPLAY - Lineweight display on/off
    pub lineweight_display: bool,
    /// XEDIT - In-place xref editing
    pub xedit: bool,
    /// EXTNAMES - Extended symbol names (R2000+)
    pub extended_names: bool,
    /// PSTYLEMODE - Plot style mode (0=color, 1=named)
    pub plotstyle_mode: bool,
    /// OLESTARTUP - OLE startup
    pub ole_startup: bool,
    
    // ==================== Dimension Variables ====================
    /// DIMSCALE - Overall dimension scale factor
    pub dim_scale: f64,
    /// DIMASZ - Dimension arrow size
    pub dim_arrow_size: f64,
    /// DIMEXO - Extension line offset
    pub dim_ext_line_offset: f64,
    /// DIMDLI - Dimension line increment
    pub dim_line_increment: f64,
    /// DIMEXE - Extension line extension
    pub dim_ext_line_extension: f64,
    /// DIMRND - Dimension rounding
    pub dim_rounding: f64,
    /// DIMDLE - Dimension line extension
    pub dim_line_extension: f64,
    /// DIMTP - Dimension tolerance plus
    pub dim_tolerance_plus: f64,
    /// DIMTM - Dimension tolerance minus
    pub dim_tolerance_minus: f64,
    /// DIMTXT - Dimension text height
    pub dim_text_height: f64,
    /// DIMCEN - Center mark size
    pub dim_center_mark: f64,
    /// DIMTSZ - Tick size
    pub dim_tick_size: f64,
    /// DIMALTF - Alternate unit scale factor
    pub dim_alt_scale: f64,
    /// DIMLFAC - Linear measurements scale factor
    pub dim_linear_scale: f64,
    /// DIMTVP - Text vertical position
    pub dim_text_vertical_pos: f64,
    /// DIMTFAC - Tolerance text height scale factor
    pub dim_tolerance_scale: f64,
    /// DIMGAP - Dimension line gap
    pub dim_line_gap: f64,
    /// DIMALTRND - Alternate units rounding
    pub dim_alt_rounding: f64,
    /// DIMTOL - Tolerance generation on/off
    pub dim_tolerance: bool,
    /// DIMLIM - Limits generation on/off
    pub dim_limits: bool,
    /// DIMTIH - Text inside horizontal
    pub dim_text_inside_horizontal: bool,
    /// DIMTOH - Text outside horizontal
    pub dim_text_outside_horizontal: bool,
    /// DIMSE1 - Suppress extension line 1
    pub dim_suppress_ext1: bool,
    /// DIMSE2 - Suppress extension line 2
    pub dim_suppress_ext2: bool,
    /// DIMTAD - Text above dimension line
    pub dim_text_above: i16,
    /// DIMZIN - Zero suppression
    pub dim_zero_suppression: i16,
    /// DIMAZIN - Alternate zero suppression
    pub dim_alt_zero_suppression: i16,
    /// DIMALT - Alternate units on/off
    pub dim_alternate_units: bool,
    /// DIMALTD - Alternate decimal places
    pub dim_alt_decimal_places: i16,
    /// DIMTOFL - Force line inside
    pub dim_force_line_inside: bool,
    /// DIMSAH - Separate arrow blocks
    pub dim_separate_arrows: bool,
    /// DIMTIX - Force text inside
    pub dim_force_text_inside: bool,
    /// DIMSOXD - Suppress outside extension dim
    pub dim_suppress_outside_ext: bool,
    /// DIMCLRD - Dimension line color
    pub dim_line_color: Color,
    /// DIMCLRE - Extension line color
    pub dim_ext_line_color: Color,
    /// DIMCLRT - Dimension text color
    pub dim_text_color: Color,
    /// DIMADEC - Angular decimal places
    pub dim_angular_decimal_places: i16,
    /// DIMDEC - Decimal places
    pub dim_decimal_places: i16,
    /// DIMTDEC - Tolerance decimal places
    pub dim_tolerance_decimal_places: i16,
    /// DIMALTU - Alternate units format
    pub dim_alt_units_format: i16,
    /// DIMALTTD - Alternate tolerance decimal places
    pub dim_alt_tolerance_decimal_places: i16,
    /// DIMAUNIT - Angular units format
    pub dim_angular_units: i16,
    /// DIMFRAC - Fraction format
    pub dim_fraction_format: i16,
    /// DIMLUNIT - Linear unit format
    pub dim_linear_unit_format: i16,
    /// DIMDSEP - Decimal separator
    pub dim_decimal_separator: char,
    /// DIMTMOVE - Text movement
    pub dim_text_movement: i16,
    /// DIMJUST - Horizontal text justification
    pub dim_horizontal_justification: i16,
    /// DIMSD1 - Suppress dimension line 1
    pub dim_suppress_line1: bool,
    /// DIMSD2 - Suppress dimension line 2
    pub dim_suppress_line2: bool,
    /// DIMTOLJ - Tolerance vertical justification
    pub dim_tolerance_justification: i16,
    /// DIMTZIN - Tolerance zero suppression
    pub dim_tolerance_zero_suppression: i16,
    /// DIMALTZ - Alternate tolerance zero suppression
    pub dim_alt_tolerance_zero_suppression: i16,
    /// DIMALTTZ - Alternate tolerance zero suppression (tight)
    pub dim_alt_tolerance_zero_tight: i16,
    /// DIMFIT/DIMATFIT - Fit options
    pub dim_fit: i16,
    /// DIMUPT - User positioned text
    pub dim_user_positioned_text: bool,
    /// DIMPOST - Primary units suffix
    pub dim_post: String,
    /// DIMAPOST - Alternate units suffix
    pub dim_alt_post: String,
    /// DIMBLK - Arrow block name
    pub dim_arrow_block: String,
    /// DIMBLK1 - First arrow block name
    pub dim_arrow_block1: String,
    /// DIMBLK2 - Second arrow block name
    pub dim_arrow_block2: String,
    /// DIMLDRBLK - Leader arrow block name
    pub dim_leader_arrow_block: String,
    
    // ==================== Extents and Limits ====================
    /// INSBASE - Model space insertion base point
    pub model_space_insertion_base: Vector3,
    /// EXTMIN - Model space extents min
    pub model_space_extents_min: Vector3,
    /// EXTMAX - Model space extents max
    pub model_space_extents_max: Vector3,
    /// LIMMIN - Model space limits min
    pub model_space_limits_min: Vector2,
    /// LIMMAX - Model space limits max
    pub model_space_limits_max: Vector2,
    
    /// Paper space insertion base point
    pub paper_space_insertion_base: Vector3,
    /// Paper space extents min
    pub paper_space_extents_min: Vector3,
    /// Paper space extents max
    pub paper_space_extents_max: Vector3,
    /// Paper space limits min
    pub paper_space_limits_min: Vector2,
    /// Paper space limits max
    pub paper_space_limits_max: Vector2,
    
    // ==================== UCS Settings ====================
    /// UCSBASE - UCS base name
    pub ucs_base: String,
    /// Model space UCS name
    pub model_space_ucs_name: String,
    /// Paper space UCS name  
    pub paper_space_ucs_name: String,
    /// Model space UCS origin
    pub model_space_ucs_origin: Vector3,
    /// Model space UCS X axis
    pub model_space_ucs_x_axis: Vector3,
    /// Model space UCS Y axis
    pub model_space_ucs_y_axis: Vector3,
    /// Paper space UCS origin
    pub paper_space_ucs_origin: Vector3,
    /// Paper space UCS X axis
    pub paper_space_ucs_x_axis: Vector3,
    /// Paper space UCS Y axis
    pub paper_space_ucs_y_axis: Vector3,
    /// UCSORTHOREF - UCS orthographic reference
    pub ucs_ortho_ref: Handle,
    /// UCSORTHOVIEW - UCS orthographic view type
    pub ucs_ortho_view: i16,
    /// PUCSORTHOREF - Paper space UCS orthographic reference  
    pub paper_ucs_ortho_ref: Handle,
    /// PUCSORTHOVIEW - Paper space UCS orthographic view type
    pub paper_ucs_ortho_view: i16,
    
    // ==================== Handles/References ====================
    /// HANDSEED - Next available handle
    pub handle_seed: u64,
    /// Current layer handle
    pub current_layer_handle: Handle,
    /// Current text style handle
    pub current_text_style_handle: Handle,
    /// Current linetype handle
    pub current_linetype_handle: Handle,
    /// Current dimension style handle
    pub current_dimstyle_handle: Handle,
    /// Current multiline style handle
    pub current_multiline_style_handle: Handle,
    /// Current material handle
    pub current_material_handle: Handle,
    /// Dimension text style handle
    pub dim_text_style_handle: Handle,
    /// Dimension linetype handle
    pub dim_linetype_handle: Handle,
    /// Dimension linetype 1 handle
    pub dim_linetype1_handle: Handle,
    /// Dimension linetype 2 handle
    pub dim_linetype2_handle: Handle,
    /// Dimension arrow block handle
    pub dim_arrow_block_handle: Handle,
    /// Dimension arrow block 1 handle
    pub dim_arrow_block1_handle: Handle,
    /// Dimension arrow block 2 handle
    pub dim_arrow_block2_handle: Handle,
    /// DIMLWD - Dimension line weight
    pub dim_line_weight: i16,
    /// DIMLWE - Extension line weight
    pub dim_ext_line_weight: i16,

    // ==================== Table Control Object Handles ====================
    /// Block table control object
    pub block_control_handle: Handle,
    /// Layer table control object
    pub layer_control_handle: Handle,
    /// Text style table control object
    pub style_control_handle: Handle,
    /// Linetype table control object
    pub linetype_control_handle: Handle,
    /// View table control object
    pub view_control_handle: Handle,
    /// UCS table control object
    pub ucs_control_handle: Handle,
    /// Viewport table control object
    pub vport_control_handle: Handle,
    /// AppId table control object
    pub appid_control_handle: Handle,
    /// Dimension style table control object
    pub dimstyle_control_handle: Handle,
    /// VPEntHdr table control object
    pub vpent_hdr_control_handle: Handle,
    
    // ==================== Dictionary Handles ====================
    /// Named objects dictionary
    pub named_objects_dict_handle: Handle,
    /// ACAD_GROUP dictionary
    pub acad_group_dict_handle: Handle,
    /// ACAD_MLINESTYLE dictionary
    pub acad_mlinestyle_dict_handle: Handle,
    /// ACAD_LAYOUT dictionary (R2000+)
    pub acad_layout_dict_handle: Handle,
    /// ACAD_PLOTSETTINGS dictionary (R2000+)
    pub acad_plotsettings_dict_handle: Handle,
    /// ACAD_PLOTSTYLENAME dictionary (R2000+)
    pub acad_plotstylename_dict_handle: Handle,
    /// ACAD_MATERIAL dictionary (R2007+)
    pub acad_material_dict_handle: Handle,
    /// ACAD_COLOR dictionary (R2007+)
    pub acad_color_dict_handle: Handle,
    /// ACAD_VISUALSTYLE dictionary (R2007+)
    pub acad_visualstyle_dict_handle: Handle,
    
    // ==================== Block Record Handles ====================
    /// *MODEL_SPACE block record
    pub model_space_block_handle: Handle,
    /// *PAPER_SPACE block record
    pub paper_space_block_handle: Handle,
    /// BYLAYER linetype
    pub bylayer_linetype_handle: Handle,
    /// BYBLOCK linetype
    pub byblock_linetype_handle: Handle,
    /// CONTINUOUS linetype
    pub continuous_linetype_handle: Handle,
    
    // ==================== Date/Time ====================
    /// Document creation time (Julian date)
    pub create_date_julian: f64,
    /// Document update time (Julian date)
    pub update_date_julian: f64,
    /// Total editing time in days
    pub total_editing_time: f64,
    /// User elapsed time in days
    pub user_elapsed_time: f64,
    
    // ==================== Metadata ====================
    /// Fingerprint GUID
    pub fingerprint_guid: String,
    /// Version GUID
    pub version_guid: String,
    /// Menu file name
    pub menu_name: String,
    /// DWGCODEPAGE
    pub code_page: String,
    /// LASTSAVEDBY
    pub last_saved_by: String,
    /// HYPERLINKBASE
    pub hyperlink_base: String,
    /// STYLESHEET
    pub stylesheet: String,
    
    // ==================== Misc ====================
    /// MEASUREMENT - Drawing units (0=English, 1=Metric)
    pub measurement: i16,
    /// PROXYGRAPHICS - Show proxy graphics
    pub proxy_graphics: i16,
    /// TREEDEPTH - Tree depth for spatial index
    pub tree_depth: i16,
    /// CMLSTYLE - Current multiline style name
    pub multiline_style: String,
    /// CELTYPE - Current linetype name
    pub current_linetype_name: String,
    /// CLAYER - Current layer name
    pub current_layer_name: String,
    /// TEXTSTYLE - Current text style name
    pub current_text_style_name: String,
    /// DIMSTYLE - Current dimension style name
    pub current_dimstyle_name: String,
}

impl Default for HeaderVariables {
    fn default() -> Self {
        Self {
            // Version-specific flags
            required_versions: 0,
            
            // Drawing mode flags
            associate_dimensions: true,
            update_dimensions_while_dragging: true,
            ortho_mode: false,
            fill_mode: true,
            quick_text_mode: false,
            mirror_text: false,
            regen_mode: true,
            limit_check: false,
            paper_space_limit_check: false,
            polyline_linetype_generation: false,
            paper_space_linetype_scaling: true,
            show_model_space: true,
            user_timer: true,
            world_view: true,
            retain_xref_visibility: true,
            display_silhouette: false,
            spline_frame: false,
            delete_objects: true,
            drag_mode: 2,
            blip_mode: false,
            attribute_request: true,
            attribute_dialog: true,
            
            // Unit settings
            linear_unit_format: 2,  // Decimal
            linear_unit_precision: 4,
            angular_unit_format: 0, // Decimal degrees
            angular_unit_precision: 0,
            insertion_units: 0,     // Unitless
            attribute_visibility: 1,
            point_display_mode: 0,
            user_int1: 0, user_int2: 0, user_int3: 0, user_int4: 0, user_int5: 0,
            coords_mode: 2,
            object_snap_mode: 0,
            pick_style: 1,
            spline_type: 6,
            spline_segments: 8,
            spline_segs_surface: 6,
            surface_u_density: 6,
            surface_v_density: 6,
            surface_type: 6,
            surface_tab1: 6,
            surface_tab2: 6,
            shade_edge: 3,
            shade_diffuse: 70,
            max_active_viewports: 64,
            isolines: 4,
            multiline_justification: 0,
            text_quality: 50,
            sort_entities: 127,
            index_control: 0,
            hide_text: 1,
            xclip_frame: 0,
            halo_gap: 0,
            obscured_color: 257,
            obscured_linetype: 0,
            intersection_display: 0,
            intersection_color: 257,
            dimension_associativity: 2,
            project_name: String::new(),
            
            // Scale/size defaults
            linetype_scale: 1.0,
            text_height: 2.5,
            trace_width: 0.05,
            sketch_increment: 0.1,
            thickness: 0.0,
            point_display_size: 0.0,
            polyline_width: 0.0,
            current_entity_linetype_scale: 1.0,
            view_twist: 0.0,
            fillet_radius: 0.0,
            chamfer_distance_a: 0.0,
            chamfer_distance_b: 0.0,
            chamfer_length: 0.0,
            chamfer_angle: 0.0,
            angle_base: 0.0,
            angle_direction: 0,
            elevation: 0.0,
            paper_elevation: 0.0,
            facet_resolution: 0.5,
            multiline_scale: 1.0,
            user_real1: 0.0, user_real2: 0.0, user_real3: 0.0, user_real4: 0.0, user_real5: 0.0,
            viewport_scale_factor: 0.0,
            shadow_plane_location: 0.0,
            loft_angle1: std::f64::consts::FRAC_PI_2,
            loft_angle2: std::f64::consts::FRAC_PI_2,
            loft_magnitude1: 0.0,
            loft_magnitude2: 0.0,
            loft_param: 7,
            loft_normals: 1,
            latitude: 37.795,
            longitude: -122.394,
            north_direction: 0.0,
            timezone: -8000,
            steps_per_second: 2.0,
            step_size: 6.0,
            lens_length: 50.0,
            camera_height: 0.0,
            camera_display: false,
            
            // Current entity settings
            current_entity_color: Color::ByLayer,
            current_line_weight: -1, // ByLayer
            current_plotstyle_type: 0,
            end_caps: 0,
            join_style: 0,
            lineweight_display: false,
            xedit: true,
            extended_names: true,
            plotstyle_mode: true,
            ole_startup: false,
            
            // Dimension variables
            dim_scale: 1.0,
            dim_arrow_size: 0.18,
            dim_ext_line_offset: 0.0625,
            dim_line_increment: 0.38,
            dim_ext_line_extension: 0.18,
            dim_rounding: 0.0,
            dim_line_extension: 0.0,
            dim_tolerance_plus: 0.0,
            dim_tolerance_minus: 0.0,
            dim_text_height: 0.18,
            dim_center_mark: 0.09,
            dim_tick_size: 0.0,
            dim_alt_scale: 25.4,
            dim_linear_scale: 1.0,
            dim_text_vertical_pos: 0.0,
            dim_tolerance_scale: 1.0,
            dim_line_gap: 0.09,
            dim_alt_rounding: 0.0,
            dim_tolerance: false,
            dim_limits: false,
            dim_text_inside_horizontal: true,
            dim_text_outside_horizontal: true,
            dim_suppress_ext1: false,
            dim_suppress_ext2: false,
            dim_text_above: 0,
            dim_zero_suppression: 0,
            dim_alt_zero_suppression: 0,
            dim_alternate_units: false,
            dim_alt_decimal_places: 2,
            dim_force_line_inside: false,
            dim_separate_arrows: false,
            dim_force_text_inside: false,
            dim_suppress_outside_ext: false,
            dim_line_color: Color::ByBlock,
            dim_ext_line_color: Color::ByBlock,
            dim_text_color: Color::ByBlock,
            dim_angular_decimal_places: 0,
            dim_decimal_places: 4,
            dim_tolerance_decimal_places: 4,
            dim_alt_units_format: 2,
            dim_alt_tolerance_decimal_places: 4,
            dim_angular_units: 0,
            dim_fraction_format: 0,
            dim_linear_unit_format: 2,
            dim_decimal_separator: '.',
            dim_text_movement: 0,
            dim_horizontal_justification: 0,
            dim_suppress_line1: false,
            dim_suppress_line2: false,
            dim_tolerance_justification: 1,
            dim_tolerance_zero_suppression: 0,
            dim_alt_tolerance_zero_suppression: 0,
            dim_alt_tolerance_zero_tight: 0,
            dim_fit: 3,
            dim_user_positioned_text: false,
            dim_post: String::new(),
            dim_alt_post: String::new(),
            dim_arrow_block: String::new(),
            dim_arrow_block1: String::new(),
            dim_arrow_block2: String::new(),
            dim_leader_arrow_block: String::new(),
            
            // Extents and limits - Model space
            model_space_insertion_base: Vector3::ZERO,
            model_space_extents_min: Vector3::new(1e20, 1e20, 1e20),
            model_space_extents_max: Vector3::new(-1e20, -1e20, -1e20),
            model_space_limits_min: Vector2::new(0.0, 0.0),
            model_space_limits_max: Vector2::new(12.0, 9.0),
            
            // Extents and limits - Paper space
            paper_space_insertion_base: Vector3::ZERO,
            paper_space_extents_min: Vector3::new(1e20, 1e20, 1e20),
            paper_space_extents_max: Vector3::new(-1e20, -1e20, -1e20),
            paper_space_limits_min: Vector2::new(0.0, 0.0),
            paper_space_limits_max: Vector2::new(12.0, 9.0),
            
            // UCS settings
            ucs_base: String::new(),
            model_space_ucs_name: String::new(),
            paper_space_ucs_name: String::new(),
            model_space_ucs_origin: Vector3::ZERO,
            model_space_ucs_x_axis: Vector3::new(1.0, 0.0, 0.0),
            model_space_ucs_y_axis: Vector3::new(0.0, 1.0, 0.0),
            paper_space_ucs_origin: Vector3::ZERO,
            paper_space_ucs_x_axis: Vector3::new(1.0, 0.0, 0.0),
            paper_space_ucs_y_axis: Vector3::new(0.0, 1.0, 0.0),
            ucs_ortho_ref: Handle::NULL,
            ucs_ortho_view: 0,
            paper_ucs_ortho_ref: Handle::NULL,
            paper_ucs_ortho_view: 0,
            
            // Handles
            handle_seed: 1,
            current_layer_handle: Handle::NULL,
            current_text_style_handle: Handle::NULL,
            current_linetype_handle: Handle::NULL,
            current_dimstyle_handle: Handle::NULL,
            current_multiline_style_handle: Handle::NULL,
            current_material_handle: Handle::NULL,
            dim_text_style_handle: Handle::NULL,
            dim_linetype_handle: Handle::NULL,
            dim_linetype1_handle: Handle::NULL,
            dim_linetype2_handle: Handle::NULL,
            dim_arrow_block_handle: Handle::NULL,
            dim_arrow_block1_handle: Handle::NULL,
            dim_arrow_block2_handle: Handle::NULL,
            dim_line_weight: -2,      // ByBlock
            dim_ext_line_weight: -2,  // ByBlock
            
            // Table control handles
            block_control_handle: Handle::NULL,
            layer_control_handle: Handle::NULL,
            style_control_handle: Handle::NULL,
            linetype_control_handle: Handle::NULL,
            view_control_handle: Handle::NULL,
            ucs_control_handle: Handle::NULL,
            vport_control_handle: Handle::NULL,
            appid_control_handle: Handle::NULL,
            dimstyle_control_handle: Handle::NULL,
            vpent_hdr_control_handle: Handle::NULL,
            
            // Dictionary handles
            named_objects_dict_handle: Handle::NULL,
            acad_group_dict_handle: Handle::NULL,
            acad_mlinestyle_dict_handle: Handle::NULL,
            acad_layout_dict_handle: Handle::NULL,
            acad_plotsettings_dict_handle: Handle::NULL,
            acad_plotstylename_dict_handle: Handle::NULL,
            acad_material_dict_handle: Handle::NULL,
            acad_color_dict_handle: Handle::NULL,
            acad_visualstyle_dict_handle: Handle::NULL,
            
            // Block record handles
            model_space_block_handle: Handle::NULL,
            paper_space_block_handle: Handle::NULL,
            bylayer_linetype_handle: Handle::NULL,
            byblock_linetype_handle: Handle::NULL,
            continuous_linetype_handle: Handle::NULL,
            
            // Date/time
            create_date_julian: 0.0,
            update_date_julian: 0.0,
            total_editing_time: 0.0,
            user_elapsed_time: 0.0,
            
            // Metadata
            fingerprint_guid: String::new(),
            version_guid: String::new(),
            menu_name: String::new(),
            code_page: String::from("ANSI_1252"),
            last_saved_by: String::new(),
            hyperlink_base: String::new(),
            stylesheet: String::new(),
            
            // Misc
            measurement: 0,
            proxy_graphics: 1,
            tree_depth: 3020,
            multiline_style: String::from("Standard"),
            current_linetype_name: String::from("ByLayer"),
            current_layer_name: String::from("0"),
            current_text_style_name: String::from("Standard"),
            current_dimstyle_name: String::from("Standard"),
        }
    }
}

/// Information about an attached external reference.
///
/// Returned by [`CadDocument::xref_info`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrefInfo<'a> {
    /// Block name used by INSERT entities to place this xref.
    pub block_name: &'a str,
    /// File path to the referenced drawing.
    pub file_path: &'a str,
    /// `true` for an overlay xref, `false` for a standard attach.
    pub is_overlay: bool,
    /// Handle of the xref's block record.
    pub handle: Handle,
}

/// Severity level for a [`ValidationIssue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A problem that may cause data loss or incorrect behaviour.
    Error,
    /// A potential issue that should be reviewed.
    Warning,
}

/// A single issue found by [`CadDocument::validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// How severe the issue is.
    pub severity: Severity,
    /// Human-readable description of the issue.
    pub message: String,
}

/// A CAD document containing all drawing data
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CadDocument {
    /// Document version
    pub version: DxfVersion,
    
    /// AutoCAD maintenance release version (from file header byte 0x0B).
    ///
    /// Used to determine encoding variations within a major DWG version.
    /// For AC1024 (R2010), maintenance > 3 triggers an extra 4-byte RL field
    /// in the Classes and Header sections.  Preserved during roundtrip.
    pub maintenance_version: u8,
    
    /// Header variables containing drawing settings
    pub header: HeaderVariables,
    
    /// Layer table
    pub layers: Table<Layer>,
    
    /// Line type table
    pub line_types: Table<LineType>,
    
    /// Text style table
    pub text_styles: Table<TextStyle>,
    
    /// Block record table
    pub block_records: Table<BlockRecord>,
    
    /// Dimension style table
    pub dim_styles: Table<DimStyle>,
    
    /// Application ID table
    pub app_ids: Table<AppId>,
    
    /// View table
    pub views: Table<View>,
    
    /// Viewport table
    pub vports: Table<VPort>,
    
    /// UCS table
    pub ucss: Table<Ucs>,
    
    /// DXF class definitions (CLASSES section)
    pub classes: DxfClassCollection,

    /// Notifications collected during the last read/write operation
    pub notifications: crate::notification::NotificationCollection,

    /// Document mutation events (entity add/remove/modify, undo/redo markers).
    pub events: crate::notification::DocumentEventCollection,

    /// All entities in the document (contiguous storage for cache locality).
    pub(crate) entities: Vec<EntityType>,

    /// Handle → index mapping for O(1) entity lookup by handle.
    pub(crate) entity_index: HashMap<Handle, usize>,

    /// All objects in the document (indexed by handle)
    pub objects: HashMap<Handle, ObjectType>,

    /// Raw EED blobs per handle — populated during DWG read, consumed during DWG write.
    /// Keyed by the object/table-entry handle. Not serialized.
    pub(crate) eed_by_handle: HashMap<Handle, Vec<(u64, Vec<u8>)>>,

    /// Non-entity object xdictionary handles — populated during DWG read, consumed during DWG write.
    pub(crate) xdic_by_handle: HashMap<Handle, Handle>,

    /// Non-entity object reactors — populated during DWG read, consumed during DWG write.
    pub(crate) reactors_by_handle: HashMap<Handle, Vec<Handle>>,

    /// Original BLOCK_HEADER entity handles from the DWG binary — includes sub-entity handles
    /// (vertices, faces, SEQENDs). Keyed by BlockRecord handle. Used by the writer to produce
    /// correct owned_object_count without re-expanding from the document model.
    pub(crate) block_entity_handles: HashMap<Handle, Vec<Handle>>,

    /// Next handle to assign
    next_handle: u64,
}

impl CadDocument {
    /// Create a new empty CAD document
    pub fn new() -> Self {
        let mut doc = CadDocument {
            version: DxfVersion::AC1032, // DXF 2018
            maintenance_version: 0,
            header: HeaderVariables::default(),
            layers: Table::new(),
            line_types: Table::new(),
            text_styles: Table::new(),
            block_records: Table::new(),
            dim_styles: Table::new(),
            app_ids: Table::new(),
            views: Table::new(),
            vports: Table::new(),
            ucss: Table::new(),
            classes: DxfClassCollection::new(),
            notifications: crate::notification::NotificationCollection::new(),
            events: crate::notification::DocumentEventCollection::new(),
            entities: Vec::new(),
            entity_index: HashMap::new(),
            objects: HashMap::new(),
            eed_by_handle: HashMap::new(),
            xdic_by_handle: HashMap::new(),
            reactors_by_handle: HashMap::new(),
            block_entity_handles: HashMap::new(),
            // Start handle allocation above reserved table handles (0x1-0xA)
            // Table handles are well-known fixed values used by AutoCAD
            next_handle: 0x10,
        };
        
        // Initialize with standard entries
        doc.initialize_defaults();
        doc
    }

    /// Create a document with a specific version
    pub fn with_version(version: DxfVersion) -> Self {
        let mut doc = Self::new();
        doc.version = version;
        doc
    }

    /// Initialize default tables with standard entries
    fn initialize_defaults(&mut self) {
        // Allocate table control handles first (these are well-known handles in DWG)
        self.header.block_control_handle = Handle::new(0x01);
        self.header.layer_control_handle = Handle::new(0x02);
        self.header.style_control_handle = Handle::new(0x03);
        self.header.linetype_control_handle = Handle::new(0x05);
        self.header.view_control_handle = Handle::new(0x06);
        self.header.ucs_control_handle = Handle::new(0x07);
        self.header.vport_control_handle = Handle::new(0x08);
        self.header.appid_control_handle = Handle::new(0x09);
        self.header.dimstyle_control_handle = Handle::new(0x0A);
        self.header.vpent_hdr_control_handle = Handle::new(0x0B);
        self.header.named_objects_dict_handle = Handle::new(0x0C);

        // Assign allocated table control handles TO the Table objects so the
        // object writer uses the same handles the header section references.
        // Without this, Table<T>.handle() returns Handle::NULL and every
        // table control is written with handle 0, not registered in the
        // handle map, and unreachable by readers → "invalid data" for all objects.
        self.block_records.set_handle(self.header.block_control_handle);
        self.layers.set_handle(self.header.layer_control_handle);
        self.text_styles.set_handle(self.header.style_control_handle);
        self.line_types.set_handle(self.header.linetype_control_handle);
        self.views.set_handle(self.header.view_control_handle);
        self.ucss.set_handle(self.header.ucs_control_handle);
        self.vports.set_handle(self.header.vport_control_handle);
        self.app_ids.set_handle(self.header.appid_control_handle);
        self.dim_styles.set_handle(self.header.dimstyle_control_handle);

        // Add standard layer "0"
        let mut layer0 = Layer::layer_0();
        layer0.set_handle(self.allocate_handle());
        // Store the layer handle for CLAYER
        self.header.current_layer_handle = layer0.handle;
        self.layers.add(layer0).ok();

        // Add standard line types
        let mut continuous = LineType::continuous();
        continuous.set_handle(self.allocate_handle());
        self.header.continuous_linetype_handle = continuous.handle;
        self.line_types.add(continuous).ok();

        let mut by_layer = LineType::by_layer();
        by_layer.set_handle(self.allocate_handle());
        self.header.bylayer_linetype_handle = by_layer.handle;
        self.header.current_linetype_handle = by_layer.handle; // Default linetype is ByLayer
        self.line_types.add(by_layer).ok();

        let mut by_block = LineType::by_block();
        by_block.set_handle(self.allocate_handle());
        self.header.byblock_linetype_handle = by_block.handle;
        self.line_types.add(by_block).ok();

        // Add standard text style
        let mut standard_style = TextStyle::standard();
        standard_style.set_handle(self.allocate_handle());
        self.header.current_text_style_handle = standard_style.handle;
        self.text_styles.add(standard_style).ok();

        // Add model space and paper space blocks
        let mut model_space = BlockRecord::model_space();
        model_space.set_handle(self.allocate_handle());
        model_space.block_entity_handle = self.allocate_handle();
        model_space.block_end_handle = self.allocate_handle();
        self.header.model_space_block_handle = model_space.handle;
        self.block_records.add(model_space).ok();

        let mut paper_space = BlockRecord::paper_space();
        paper_space.set_handle(self.allocate_handle());
        paper_space.block_entity_handle = self.allocate_handle();
        paper_space.block_end_handle = self.allocate_handle();
        self.header.paper_space_block_handle = paper_space.handle;
        self.block_records.add(paper_space).ok();

        // Add standard dimension style
        let mut standard_dimstyle = DimStyle::standard();
        standard_dimstyle.set_handle(self.allocate_handle());
        // DIMTXSTY must reference the Standard text style
        standard_dimstyle.dimtxsty_handle = self.header.current_text_style_handle;
        self.header.current_dimstyle_handle = standard_dimstyle.handle;
        // Header dim text style handle must also point to Standard
        self.header.dim_text_style_handle = self.header.current_text_style_handle;
        // Dim linetype handles: reference ByBlock linetype for R2007+
        self.header.dim_linetype_handle = self.header.byblock_linetype_handle;
        self.header.dim_linetype1_handle = self.header.byblock_linetype_handle;
        self.header.dim_linetype2_handle = self.header.byblock_linetype_handle;
        self.dim_styles.add(standard_dimstyle).ok();

        // Add standard application ID
        let mut acad = AppId::acad();
        acad.set_handle(self.allocate_handle());
        self.app_ids.add(acad).ok();

        // Add standard viewport
        let mut active_vport = VPort::active();
        active_vport.set_handle(self.allocate_handle());
        self.vports.add(active_vport).ok();
        
        // ── Standard dictionary objects (required for DWG format) ────
        // Allocate handles for core dictionaries
        self.header.acad_group_dict_handle = self.allocate_handle();
        self.header.acad_mlinestyle_dict_handle = self.allocate_handle();
        self.header.acad_layout_dict_handle = self.allocate_handle();
        self.header.acad_plotsettings_dict_handle = self.allocate_handle();
        self.header.acad_plotstylename_dict_handle = self.allocate_handle();
        // R2004+/R2007+ dictionaries (AutoCAD requires these even if empty)
        self.header.acad_material_dict_handle = self.allocate_handle();
        self.header.acad_color_dict_handle = self.allocate_handle();
        self.header.acad_visualstyle_dict_handle = self.allocate_handle();

        // Allocate handles for objects that live inside dictionaries
        let mlinestyle_std_handle = self.allocate_handle();
        let model_layout_handle = self.allocate_handle();
        let paper_layout_handle = self.allocate_handle();
        let plotstylename_placeholder_handle = self.allocate_handle();

        // Store the current MLineStyle handle in the header (for CMLSTYLE)
        self.header.current_multiline_style_handle = mlinestyle_std_handle;

        // Link block records to their layouts
        if let Some(ms) = self.block_records.get_mut("*Model_Space") {
            ms.layout = model_layout_handle;
        }
        if let Some(ps) = self.block_records.get_mut("*Paper_Space") {
            ps.layout = paper_layout_handle;
        }

        // -- Root dictionary (NAMED_OBJECTS_DICTIONARY) --
        let root_dict_handle = self.header.named_objects_dict_handle;
        let mut root_dict = crate::objects::Dictionary::new();
        root_dict.handle = root_dict_handle;
        root_dict.owner = Handle::NULL; // owned by document
        root_dict.add_entry("ACAD_GROUP", self.header.acad_group_dict_handle);
        root_dict.add_entry("ACAD_MLINESTYLE", self.header.acad_mlinestyle_dict_handle);
        root_dict.add_entry("ACAD_LAYOUT", self.header.acad_layout_dict_handle);
        root_dict.add_entry("ACAD_PLOTSETTINGS", self.header.acad_plotsettings_dict_handle);
        root_dict.add_entry("ACAD_PLOTSTYLENAME", self.header.acad_plotstylename_dict_handle);
        root_dict.add_entry("ACAD_MATERIAL", self.header.acad_material_dict_handle);
        root_dict.add_entry("ACAD_COLOR", self.header.acad_color_dict_handle);
        root_dict.add_entry("ACAD_VISUALSTYLE", self.header.acad_visualstyle_dict_handle);
        self.objects.insert(root_dict_handle, ObjectType::Dictionary(root_dict));

        // -- ACAD_GROUP dictionary (empty) --
        let mut group_dict = crate::objects::Dictionary::new();
        group_dict.handle = self.header.acad_group_dict_handle;
        group_dict.owner = root_dict_handle;
        self.objects.insert(group_dict.handle, ObjectType::Dictionary(group_dict));

        // -- ACAD_MLINESTYLE dictionary (contains "Standard") --
        let mut mlinestyle_dict = crate::objects::Dictionary::new();
        mlinestyle_dict.handle = self.header.acad_mlinestyle_dict_handle;
        mlinestyle_dict.owner = root_dict_handle;
        mlinestyle_dict.add_entry("Standard", mlinestyle_std_handle);
        self.objects.insert(mlinestyle_dict.handle, ObjectType::Dictionary(mlinestyle_dict));

        // -- MLineStyle Standard object --
        let mut mlinestyle_std = crate::objects::MLineStyle::standard();
        mlinestyle_std.handle = mlinestyle_std_handle;
        mlinestyle_std.owner = self.header.acad_mlinestyle_dict_handle;
        self.objects.insert(mlinestyle_std_handle, ObjectType::MLineStyle(mlinestyle_std));

        // -- ACAD_LAYOUT dictionary (Model + Layout1) --
        let mut layout_dict = crate::objects::Dictionary::new();
        layout_dict.handle = self.header.acad_layout_dict_handle;
        layout_dict.owner = root_dict_handle;
        layout_dict.add_entry("Model", model_layout_handle);
        layout_dict.add_entry("Layout1", paper_layout_handle);
        self.objects.insert(layout_dict.handle, ObjectType::Dictionary(layout_dict));

        // -- Layout: Model --
        let mut model_layout = crate::objects::Layout::new("Model");
        model_layout.handle = model_layout_handle;
        model_layout.owner = self.header.acad_layout_dict_handle;
        model_layout.tab_order = 0;
        model_layout.flags = 1; // model space
        model_layout.block_record = self.header.model_space_block_handle;
        self.objects.insert(model_layout_handle, ObjectType::Layout(model_layout));

        // -- Layout: Layout1 (paper space) --
        let mut paper_layout = crate::objects::Layout::new("Layout1");
        paper_layout.handle = paper_layout_handle;
        paper_layout.owner = self.header.acad_layout_dict_handle;
        paper_layout.tab_order = 1;
        paper_layout.block_record = self.header.paper_space_block_handle;

        self.objects.insert(paper_layout_handle, ObjectType::Layout(paper_layout));

        // -- ACAD_PLOTSETTINGS dictionary (empty) --
        let mut plotsettings_dict = crate::objects::Dictionary::new();
        plotsettings_dict.handle = self.header.acad_plotsettings_dict_handle;
        plotsettings_dict.owner = root_dict_handle;
        self.objects.insert(plotsettings_dict.handle, ObjectType::Dictionary(plotsettings_dict));

        // -- ACAD_MATERIAL dictionary (empty, required R2004+) --
        let mut material_dict = crate::objects::Dictionary::new();
        material_dict.handle = self.header.acad_material_dict_handle;
        material_dict.owner = root_dict_handle;
        self.objects.insert(material_dict.handle, ObjectType::Dictionary(material_dict));

        // -- ACAD_COLOR dictionary (empty, required R2004+) --
        let mut color_dict = crate::objects::Dictionary::new();
        color_dict.handle = self.header.acad_color_dict_handle;
        color_dict.owner = root_dict_handle;
        self.objects.insert(color_dict.handle, ObjectType::Dictionary(color_dict));

        // -- ACAD_VISUALSTYLE dictionary (empty, required R2007+) --
        let mut visualstyle_dict = crate::objects::Dictionary::new();
        visualstyle_dict.handle = self.header.acad_visualstyle_dict_handle;
        visualstyle_dict.owner = root_dict_handle;
        self.objects.insert(visualstyle_dict.handle, ObjectType::Dictionary(visualstyle_dict));

        // -- ACAD_PLOTSTYLENAME dictionary (DictionaryWithDefault with PlaceHolder) --
        let mut plotstyle_dict = crate::objects::DictionaryWithDefault::new();
        plotstyle_dict.handle = self.header.acad_plotstylename_dict_handle;
        plotstyle_dict.owner = root_dict_handle;
        plotstyle_dict.default_handle = plotstylename_placeholder_handle;
        plotstyle_dict.entries.push(("Normal".to_string(), plotstylename_placeholder_handle));
        self.objects.insert(plotstyle_dict.handle, ObjectType::DictionaryWithDefault(plotstyle_dict));

        // -- PlaceHolder for ACAD_PLOTSTYLENAME "Normal" --
        let mut placeholder = crate::objects::PlaceHolder::new();
        placeholder.handle = plotstylename_placeholder_handle;
        placeholder.owner = self.header.acad_plotstylename_dict_handle;
        self.objects.insert(plotstylename_placeholder_handle, ObjectType::PlaceHolder(placeholder));

        // Register standard DXF classes required by the DWG format.
        // For pre-R2004, "unlisted" object types (LAYOUT, PLOTSETTINGS, etc.)
        // need a class entry so the writer can emit the class number instead of
        // the R2004+ fixed type code.
        use crate::classes::{DxfClass, ProxyFlags};
        let standard_classes = [
            DxfClass {
                dxf_name: "ACDBDICTIONARYWDFLT".to_string(),
                cpp_class_name: "AcDbDictionaryWithDefault".to_string(),
                application_name: "ObjectDBX Classes".to_string(),
                proxy_flags: ProxyFlags::NONE,
                instance_count: 0,
                was_zombie: false,
                is_an_entity: false,
                class_number: 0, // will be assigned (500+)
                item_class_id: 0x1F3,
            },
            DxfClass {
                dxf_name: "DICTIONARYVAR".to_string(),
                cpp_class_name: "AcDbDictionaryVar".to_string(),
                application_name: "ObjectDBX Classes".to_string(),
                proxy_flags: ProxyFlags::NONE,
                instance_count: 0,
                was_zombie: false,
                is_an_entity: false,
                class_number: 0,
                item_class_id: 0x1F3,
            },
            DxfClass {
                dxf_name: "LAYOUT".to_string(),
                cpp_class_name: "AcDbLayout".to_string(),
                application_name: "ObjectDBX Classes".to_string(),
                proxy_flags: ProxyFlags::NONE,
                instance_count: 0,
                was_zombie: false,
                is_an_entity: false,
                class_number: 0,
                item_class_id: 0x1F3,
            },
            DxfClass {
                dxf_name: "ACDBPLACEHOLDER".to_string(),
                cpp_class_name: "AcDbPlaceHolder".to_string(),
                application_name: "ObjectDBX Classes".to_string(),
                proxy_flags: ProxyFlags::NONE,
                instance_count: 0,
                was_zombie: false,
                is_an_entity: false,
                class_number: 0,
                item_class_id: 0x1F3,
            },
            DxfClass {
                dxf_name: "PLOTSETTINGS".to_string(),
                cpp_class_name: "AcDbPlotSettings".to_string(),
                application_name: "ObjectDBX Classes".to_string(),
                proxy_flags: ProxyFlags::NONE,
                instance_count: 0,
                was_zombie: false,
                is_an_entity: false,
                class_number: 0,
                item_class_id: 0x1F3,
            },
            DxfClass {
                dxf_name: "SCALE".to_string(),
                cpp_class_name: "AcDbScale".to_string(),
                application_name: "ObjectDBX Classes".to_string(),
                proxy_flags: ProxyFlags::NONE,
                instance_count: 0,
                was_zombie: false,
                is_an_entity: false,
                class_number: 0,
                item_class_id: 0x1F3,
            },
        ];
        for cls in standard_classes {
            self.classes.add_or_update(cls);
        }

        // Register default DXF classes for all entity/object types.
        // Unlisted types like MESH, MULTILEADER, IMAGE need class entries
        // so the writer emits the correct 500+ type code instead of a
        // wrong fixed code.
        self.classes.update_defaults();
    }

    /// Allocate a new unique handle
    pub fn allocate_handle(&mut self) -> Handle {
        let handle = Handle::new(self.next_handle);
        self.next_handle += 1;
        // Keep HANDSEED in sync — DWG header requires this to be ≥ next_handle
        self.header.handle_seed = self.next_handle;
        handle
    }

    /// Get the next handle value (without allocating)
    pub fn next_handle(&self) -> u64 {
        self.next_handle
    }

    /// Add an entity to the document (model space).
    ///
    /// The entity is stored in both the flat entity map (used by the DXF
    /// writer) and the *Model_Space block record (used by the DWG writer).
    pub fn add_entity(&mut self, mut entity: EntityType) -> Result<Handle> {
        // Allocate a handle if the entity doesn't have one
        let handle = if entity.common().handle.is_null() {
            let h = self.allocate_handle();
            entity.as_entity_mut().set_handle(h);
            h
        } else {
            let h = entity.common().handle;
            // Ensure the handle counter stays above this handle so
            // future allocations (e.g., vertex sub-entities) don't
            // collide with it.
            if h.value() >= self.next_handle {
                self.next_handle = h.value() + 1;
                self.header.handle_seed = self.next_handle;
            }
            h
        };

        // Set owner to *Model_Space block record if not already set
        let ms_handle = self.header.model_space_block_handle;
        if entity.common().owner_handle.is_null() && !ms_handle.is_null() {
            entity.common_mut().owner_handle = ms_handle;
        }

        // AttributeEntity is a sub-entity owned by INSERT, not a direct
        // block-record child.  Never add it to entity_handles.
        // Block/BlockEnd are structural markers with separate handle fields.
        let is_excluded = matches!(&entity, EntityType::AttributeEntity(_) | EntityType::Block(_) | EntityType::BlockEnd(_));

        // Route entity handle to the correct block record based on owner handle.
        let owner = entity.common().owner_handle;
        let mut added_to_block = false;
        if !is_excluded && !owner.is_null() {
            for br in self.block_records.iter_mut() {
                if br.handle == owner {
                    br.entity_handles.push(handle);
                    added_to_block = true;
                    break;
                }
            }
        }
        // Fallback: add to *Model_Space if owner didn't match any block record
        if !is_excluded && !added_to_block {
            if let Some(ms) = self.block_records.get_mut("*Model_Space") {
                ms.entity_handles.push(handle);
                // Fix the entity's owner so the writer can determine
                // entity_mode correctly (model-space = 2).
                entity.common_mut().owner_handle = ms.handle;
            }
        }

        // Store in the flat entity map (DXF writer reads from here)
        let idx = self.entities.len();
        self.entities.push(entity);
        self.entity_index.insert(handle, idx);
        self.events.entity_added(handle);
        Ok(handle)
    }

    /// Get an entity by handle
    pub fn get_entity(&self, handle: Handle) -> Option<&EntityType> {
        self.entity_index.get(&handle).map(|&idx| &self.entities[idx])
    }

    /// Get a mutable entity by handle
    pub fn get_entity_mut(&mut self, handle: Handle) -> Option<&mut EntityType> {
        let idx = *self.entity_index.get(&handle)?;
        Some(&mut self.entities[idx])
    }

    /// Explode an entity into simpler primitives, allocating valid handles.
    ///
    /// Each resulting entity receives a unique handle from the document's
    /// handle allocator and inherits the original entity's owner handle.
    /// The caller can then add the returned entities to the document via
    /// [`add_entity`](Self::add_entity) or use them directly.
    ///
    /// Returns an empty `Vec` for atomic entities that cannot be decomposed.
    pub fn explode_entity(&mut self, entity: &EntityType) -> Vec<EntityType> {
        let mut parts = entity.explode();
        let owner = entity.common().owner_handle;
        for part in &mut parts {
            let h = self.allocate_handle();
            part.as_entity_mut().set_handle(h);
            if !owner.is_null() && part.common().owner_handle.is_null() {
                part.common_mut().owner_handle = owner;
            }
        }
        parts
    }

    /// Add an entity to the default paper space (`*Paper_Space` / "Layout1").
    ///
    /// This sets the entity's owner to the `*Paper_Space` block record and
    /// stores it there.  Viewports must be placed in paper space to be
    /// visible in a layout.
    ///
    /// For documents with multiple layouts, use
    /// [`add_entity_to_layout`](Self::add_entity_to_layout) instead.
    pub fn add_paper_space_entity(&mut self, entity: EntityType) -> Result<Handle> {
        self.add_entity_to_block(entity, "*Paper_Space")
    }

    /// Add an entity to a named layout.
    ///
    /// Looks up the [`Layout`](crate::objects::Layout) object by name (e.g.
    /// `"Layout1"`, `"Layout2"`) and adds the entity to the layout's
    /// backing block record.  Returns an error if the layout is not found.
    ///
    /// # Example
    /// ```ignore
    /// use acadrust::entities::{Viewport, EntityType};
    ///
    /// let vp = Viewport::new();
    /// document.add_entity_to_layout(EntityType::Viewport(vp), "Layout1")?;
    /// ```
    pub fn add_entity_to_layout(
        &mut self,
        entity: EntityType,
        layout_name: &str,
    ) -> Result<Handle> {
        // Find the Layout object by name to get its block_record handle
        let block_handle = self
            .objects
            .values()
            .find_map(|obj| match obj {
                ObjectType::Layout(layout) if layout.name == layout_name => {
                    Some(layout.block_record)
                }
                _ => None,
            })
            .ok_or_else(|| {
                crate::error::DxfError::Custom(format!(
                    "Layout '{}' not found",
                    layout_name
                ))
            })?;

        // Find the block record name for this handle
        let block_name = self
            .block_records
            .iter()
            .find(|br| br.handle == block_handle)
            .map(|br| br.name().to_string())
            .ok_or_else(|| {
                crate::error::DxfError::Custom(format!(
                    "Block record for layout '{}' not found",
                    layout_name
                ))
            })?;

        self.add_entity_to_block(entity, &block_name)
    }

    /// Add an entity to a named block record.
    ///
    /// Sets the entity's owner handle and routes it to the specified block
    /// record.  Used internally by [`add_entity`](Self::add_entity),
    /// [`add_paper_space_entity`](Self::add_paper_space_entity), and
    /// [`add_entity_to_layout`](Self::add_entity_to_layout).
    fn add_entity_to_block(
        &mut self,
        mut entity: EntityType,
        block_name: &str,
    ) -> Result<Handle> {
        // Allocate a handle if the entity doesn't have one
        let handle = if entity.common().handle.is_null() {
            let h = self.allocate_handle();
            entity.as_entity_mut().set_handle(h);
            h
        } else {
            let h = entity.common().handle;
            if h.value() >= self.next_handle {
                self.next_handle = h.value() + 1;
                self.header.handle_seed = self.next_handle;
            }
            h
        };

        // Set owner to the target block record
        if let Some(br) = self.block_records.get(block_name) {
            entity.common_mut().owner_handle = br.handle;
        }

        // Route entity handle to the block record
        let owner = entity.common().owner_handle;
        let mut added_to_block = false;
        if !owner.is_null() {
            for br in self.block_records.iter_mut() {
                if br.handle == owner {
                    br.entity_handles.push(handle);
                    added_to_block = true;
                    break;
                }
            }
        }
        if !added_to_block {
            if let Some(target) = self.block_records.get_mut(block_name) {
                target.entity_handles.push(handle);
            }
        }

        // Store in the flat entity map
        let idx = self.entities.len();
        self.entities.push(entity);
        self.entity_index.insert(handle, idx);
        self.events.entity_added(handle);
        Ok(handle)
    }

    /// Remove an entity by handle
    pub fn remove_entity(&mut self, handle: Handle) -> Option<EntityType> {
        let idx = self.entity_index.remove(&handle)?;
        let entity = self.entities.swap_remove(idx);
        // If the swap moved an element, update its index
        if idx < self.entities.len() {
            let moved_handle = self.entities[idx].common().handle;
            self.entity_index.insert(moved_handle, idx);
        }
        self.events.entity_removed(handle);
        Some(entity)
    }

    /// Add a new paper space layout to the document.
    ///
    /// Creates the backing `*Paper_Space<N>` block record, a [`Layout`]
    /// object, and registers both in the ACAD_LAYOUT dictionary.  Returns
    /// the layout handle.
    ///
    /// # Example
    /// ```ignore
    /// let layout_handle = document.add_layout("Layout2")?;
    /// // Then add entities to it:
    /// document.add_entity_to_layout(EntityType::Viewport(vp), "Layout2")?;
    /// ```
    pub fn add_layout(&mut self, name: &str) -> Result<Handle> {
        // Check for duplicate layout name
        let already_exists = self.objects.values().any(|obj| {
            matches!(obj, ObjectType::Layout(l) if l.name == name)
        });
        if already_exists {
            return Err(crate::error::DxfError::Custom(format!(
                "Layout '{}' already exists",
                name
            )));
        }

        // Determine the next *Paper_Space block name.
        // AutoCAD uses: *Paper_Space, *Paper_Space0, *Paper_Space1, …
        let ps_count = self
            .block_records
            .iter()
            .filter(|br| br.is_paper_space())
            .count();
        let block_name = if ps_count == 0 {
            "*Paper_Space".to_string()
        } else {
            format!("*Paper_Space{}", ps_count - 1)
        };

        // Create the block record
        let mut block_record = BlockRecord::new(&block_name);
        block_record.set_handle(self.allocate_handle());
        block_record.block_entity_handle = self.allocate_handle();
        block_record.block_end_handle = self.allocate_handle();
        let br_handle = block_record.handle;

        // Create the Layout object
        let layout_handle = self.allocate_handle();
        let mut layout = crate::objects::Layout::new(name);
        layout.handle = layout_handle;
        layout.owner = self.header.acad_layout_dict_handle;
        layout.tab_order = ps_count as i16 + 1;
        layout.block_record = br_handle;

        // Link block record → layout
        block_record.layout = layout_handle;
        self.block_records.add(block_record).map_err(|e| {
            crate::error::DxfError::Custom(e)
        })?;

        // Create the overall paper space viewport (ID=1) for this layout.
        // Every paper space layout requires this entity.
        let mut overall_vp = crate::entities::Viewport::new();
        overall_vp.id = 1;
        overall_vp.status = crate::entities::ViewportStatusFlags::default_on();
        let overall_vp_handle = self.allocate_handle();
        overall_vp.common.handle = overall_vp_handle;
        overall_vp.common.owner_handle = br_handle;
        layout.viewport = overall_vp_handle;

        if let Some(br) = self.block_records.get_mut(&block_name) {
            br.entity_handles.push(overall_vp_handle);
        }
        let idx = self.entities.len();
        self.entities.push(EntityType::Viewport(overall_vp));
        self.entity_index.insert(overall_vp_handle, idx);

        // Register in ACAD_LAYOUT dictionary
        if let Some(ObjectType::Dictionary(dict)) =
            self.objects.get_mut(&self.header.acad_layout_dict_handle)
        {
            dict.add_entry(name, layout_handle);
        }

        // Store the Layout object
        self.objects.insert(layout_handle, ObjectType::Layout(layout));

        Ok(layout_handle)
    }

    // ════════════════════════════════════════════════════════════════════
    // Layer & Table Management API — ensure_* helpers
    // ════════════════════════════════════════════════════════════════════

    /// Ensure a layer with the given name exists, creating it with default
    /// settings if it does not.
    ///
    /// Returns the layer's `Handle`.  If the layer already exists its
    /// properties are left unchanged.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// let h1 = doc.ensure_layer("Walls");
    /// let h2 = doc.ensure_layer("Walls");
    /// assert_eq!(h1, h2);
    /// ```
    pub fn ensure_layer(&mut self, name: &str) -> Handle {
        if let Some(existing) = self.layers.get(name) {
            return existing.handle;
        }
        let mut layer = Layer::new(name);
        layer.set_handle(self.allocate_handle());
        let handle = layer.handle;
        self.layers.add(layer).ok();
        handle
    }

    /// Ensure a layer exists with a specific color and linetype, creating
    /// it if it does not.
    ///
    /// If the layer already exists its properties are **not** modified —
    /// only the existing handle is returned.  This avoids accidentally
    /// overwriting settings loaded from a DWG/DXF file.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::types::Color;
    ///
    /// let mut doc = CadDocument::new();
    /// let h = doc.ensure_layer_with("Electrical", Color::CYAN, "DASHED");
    /// assert!(doc.layers.get("Electrical").is_some());
    /// ```
    pub fn ensure_layer_with(&mut self, name: &str, color: Color, linetype: &str) -> Handle {
        if let Some(existing) = self.layers.get(name) {
            return existing.handle;
        }
        let mut layer = Layer::new(name);
        layer.color = color;
        layer.line_type = linetype.to_string();
        layer.set_handle(self.allocate_handle());
        let handle = layer.handle;
        self.layers.add(layer).ok();
        handle
    }

    /// Ensure a linetype with the given name exists, creating it with a
    /// continuous (solid) pattern if it does not.
    ///
    /// Returns the linetype's `Handle`.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// let h = doc.ensure_linetype("CENTER");
    /// assert!(doc.line_types.get("CENTER").is_some());
    /// ```
    pub fn ensure_linetype(&mut self, name: &str) -> Handle {
        if let Some(existing) = self.line_types.get(name) {
            return existing.handle();
        }
        let mut lt = LineType::new(name);
        lt.set_handle(self.allocate_handle());
        let handle = lt.handle();
        self.line_types.add(lt).ok();
        handle
    }

    /// Ensure a text style with the given name exists, creating it with
    /// default settings if it does not.
    ///
    /// Returns the text style's `Handle`.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// let h = doc.ensure_text_style("Notes");
    /// assert!(doc.text_styles.get("Notes").is_some());
    /// ```
    pub fn ensure_text_style(&mut self, name: &str) -> Handle {
        if let Some(existing) = self.text_styles.get(name) {
            return existing.handle();
        }
        let mut style = TextStyle::new(name);
        style.set_handle(self.allocate_handle());
        let handle = style.handle();
        self.text_styles.add(style).ok();
        handle
    }

    /// Ensure a dimension style with the given name exists, creating it
    /// with default settings if it does not.
    ///
    /// Returns the dimension style's `Handle`.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// let h = doc.ensure_dim_style("Metric");
    /// assert!(doc.dim_styles.get("Metric").is_some());
    /// ```
    pub fn ensure_dim_style(&mut self, name: &str) -> Handle {
        if let Some(existing) = self.dim_styles.get(name) {
            return existing.handle();
        }
        let mut ds = DimStyle::new(name);
        ds.set_handle(self.allocate_handle());
        let handle = ds.handle();
        self.dim_styles.add(ds).ok();
        handle
    }

    /// Ensure an application ID with the given name exists, creating it
    /// if it does not.
    ///
    /// Returns the AppId's `Handle`.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// let h = doc.ensure_app_id("MyApp");
    /// assert!(doc.app_ids.get("MyApp").is_some());
    /// ```
    pub fn ensure_app_id(&mut self, name: &str) -> Handle {
        if let Some(existing) = self.app_ids.get(name) {
            return existing.handle();
        }
        let mut appid = AppId::new(name);
        appid.set_handle(self.allocate_handle());
        let handle = appid.handle();
        self.app_ids.add(appid).ok();
        handle
    }

    // ════════════════════════════════════════════════════════════════════
    // Layer-aware entity helpers
    // ════════════════════════════════════════════════════════════════════

    /// Add an entity to the document, assigning it to the given layer.
    ///
    /// The layer is created with default settings if it does not already
    /// exist.  The entity is placed in model space (same as
    /// [`add_entity`](Self::add_entity)).
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    /// use acadrust::types::Vector3;
    ///
    /// let mut doc = CadDocument::new();
    /// let line = Line::from_coords(0.0, 0.0, 0.0, 10.0, 10.0, 0.0);
    /// let handle = doc.add_entity_on_layer(EntityType::Line(line), "Walls").unwrap();
    ///
    /// let entity = doc.get_entity(handle).unwrap();
    /// assert_eq!(entity.common().layer, "Walls");
    /// assert!(doc.layers.get("Walls").is_some());
    /// ```
    pub fn add_entity_on_layer(
        &mut self,
        mut entity: EntityType,
        layer_name: &str,
    ) -> Result<Handle> {
        self.ensure_layer(layer_name);
        entity.common_mut().layer = layer_name.to_string();
        self.add_entity(entity)
    }

    /// Change an existing entity's layer.
    ///
    /// The target layer is created with default settings if it does not
    /// already exist.  Returns an error if the entity handle is not found.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    ///
    /// let mut doc = CadDocument::new();
    /// let line = Line::from_coords(0.0, 0.0, 0.0, 10.0, 10.0, 0.0);
    /// let handle = doc.add_entity(EntityType::Line(line)).unwrap();
    ///
    /// doc.set_entity_layer(handle, "Dimensions").unwrap();
    /// assert_eq!(doc.get_entity(handle).unwrap().common().layer, "Dimensions");
    /// ```
    pub fn set_entity_layer(
        &mut self,
        handle: Handle,
        layer_name: &str,
    ) -> Result<()> {
        self.ensure_layer(layer_name);
        let entity = self.get_entity_mut(handle).ok_or_else(|| {
            crate::error::DxfError::Custom(format!(
                "Entity with handle {:#X} not found",
                handle.value()
            ))
        })?;
        entity.common_mut().layer = layer_name.to_string();
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // Entity query / filtering helpers
    // ════════════════════════════════════════════════════════════════════

    /// Iterate over all entities on a given layer (case-insensitive).
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    ///
    /// let mut doc = CadDocument::new();
    /// let line = Line::from_coords(0.0, 0.0, 0.0, 10.0, 10.0, 0.0);
    /// doc.add_entity_on_layer(EntityType::Line(line), "Walls").unwrap();
    ///
    /// let count = doc.entities_on_layer("walls").count();
    /// assert_eq!(count, 1);
    /// ```
    pub fn entities_on_layer<'a>(&'a self, layer_name: &'a str) -> impl Iterator<Item = &'a EntityType> {
        let upper = layer_name.to_uppercase();
        self.entities.iter().filter(move |e| e.common().layer.to_uppercase() == upper)
    }

    /// Iterate mutably over all entities on a given layer (case-insensitive).
    pub fn entities_on_layer_mut<'a>(&'a mut self, layer_name: &'a str) -> impl Iterator<Item = &'a mut EntityType> {
        let upper = layer_name.to_uppercase();
        self.entities.iter_mut().filter(move |e| e.common().layer.to_uppercase() == upper)
    }

    /// Iterate over entities whose bounding box intersects the given region.
    ///
    /// This is a brute-force scan — suitable for interactive use, not for
    /// spatial indexing of millions of entities.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    /// use acadrust::types::{BoundingBox3D, Vector3};
    ///
    /// let mut doc = CadDocument::new();
    /// doc.add_entity(EntityType::Line(
    ///     Line::from_coords(0.0, 0.0, 0.0, 5.0, 5.0, 0.0),
    /// )).unwrap();
    /// doc.add_entity(EntityType::Line(
    ///     Line::from_coords(100.0, 100.0, 0.0, 200.0, 200.0, 0.0),
    /// )).unwrap();
    ///
    /// let region = BoundingBox3D::new(
    ///     Vector3::new(-1.0, -1.0, -1.0),
    ///     Vector3::new(10.0, 10.0, 1.0),
    /// );
    /// assert_eq!(doc.entities_in_bounding_box(&region).count(), 1);
    /// ```
    pub fn entities_in_bounding_box<'a>(
        &'a self,
        region: &'a crate::types::BoundingBox3D,
    ) -> impl Iterator<Item = &'a EntityType> {
        self.entities
            .iter()
            .filter(move |e| e.bounding_box().intersects(region))
    }

    /// Compute the combined bounding box of all entities in the document.
    ///
    /// Returns `None` if the document has no entities.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    ///
    /// let mut doc = CadDocument::new();
    /// doc.add_entity(EntityType::Line(
    ///     Line::from_coords(0.0, 0.0, 0.0, 10.0, 5.0, 0.0),
    /// )).unwrap();
    /// let bbox = doc.extents().unwrap();
    /// assert_eq!(bbox.width(), 10.0);
    /// ```
    pub fn extents(&self) -> Option<crate::types::BoundingBox3D> {
        let mut iter = self.entities.iter();
        let first = iter.next()?;
        let mut result = first.bounding_box();
        for e in iter {
            result = result.merge(&e.bounding_box());
        }
        Some(result)
    }

    /// Get the number of entities
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Iterate over all entities
    pub fn entities(&self) -> impl Iterator<Item = &EntityType> {
        self.entities.iter()
    }

    /// Iterate over all entities mutably
    pub fn entities_mut(&mut self) -> impl Iterator<Item = &mut EntityType> {
        self.entities.iter_mut()
    }

    /// Iterate over entities of a specific concrete type.
    ///
    /// Uses the [`EntityVariant`] trait to filter entities by their variant
    /// and yield references to the inner concrete type.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Circle, Line};
    ///
    /// let mut doc = CadDocument::new();
    /// doc.add_entity(EntityType::Circle(Circle::new())).unwrap();
    /// doc.add_entity(EntityType::Line(Line::new())).unwrap();
    /// doc.add_entity(EntityType::Circle(Circle::from_center_radius(
    ///     acadrust::types::Vector3::new(5.0, 5.0, 0.0), 2.0,
    /// ))).unwrap();
    ///
    /// let circles: Vec<&Circle> = doc.entities_of_type::<Circle>().collect();
    /// assert_eq!(circles.len(), 2);
    /// ```
    pub fn entities_of_type<'a, T: crate::entities::EntityVariant + 'a>(
        &'a self,
    ) -> impl Iterator<Item = &'a T> + 'a {
        self.entities.iter().filter_map(T::from_entity_type)
    }

    /// Iterate mutably over entities of a specific concrete type.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Circle};
    ///
    /// let mut doc = CadDocument::new();
    /// doc.add_entity(EntityType::Circle(Circle::new())).unwrap();
    ///
    /// for c in doc.entities_of_type_mut::<Circle>() {
    ///     c.radius = 10.0;
    /// }
    /// assert_eq!(doc.entities_of_type::<Circle>().next().unwrap().radius, 10.0);
    /// ```
    pub fn entities_of_type_mut<'a, T: crate::entities::EntityVariant + 'a>(
        &'a mut self,
    ) -> impl Iterator<Item = &'a mut T> + 'a {
        self.entities.iter_mut().filter_map(T::from_entity_type_mut)
    }

    /// Search for entities that contain the given text (case-insensitive).
    ///
    /// Examines [`Text`](crate::entities::Text) and [`MText`](crate::entities::MText)
    /// entities and returns those whose `value` field contains `query`.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Text, MText};
    /// use acadrust::types::Vector3;
    ///
    /// let mut doc = CadDocument::new();
    /// let mut t = Text::new();
    /// t.value = "Hello World".to_string();
    /// doc.add_entity(EntityType::Text(t)).unwrap();
    /// let mut mt = MText::new();
    /// mt.value = "Goodbye".to_string();
    /// doc.add_entity(EntityType::MText(mt)).unwrap();
    ///
    /// let found: Vec<_> = doc.entities_with_text("hello").collect();
    /// assert_eq!(found.len(), 1);
    /// ```
    pub fn entities_with_text<'a>(&'a self, query: &'a str) -> impl Iterator<Item = &'a EntityType> + 'a {
        let q = query.to_lowercase();
        self.entities.iter().filter(move |e| {
            match e {
                EntityType::Text(t) => t.value.to_lowercase().contains(&q),
                EntityType::MText(t) => t.value.to_lowercase().contains(&q),
                _ => false,
            }
        })
    }

    /// Return all text values found in the document.
    ///
    /// Collects the `value` field from every `Text` and `MText` entity.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Text};
    ///
    /// let mut doc = CadDocument::new();
    /// let mut t = Text::new();
    /// t.value = "Note A".to_string();
    /// doc.add_entity(EntityType::Text(t)).unwrap();
    /// assert_eq!(doc.text_values().len(), 1);
    /// ```
    pub fn text_values(&self) -> Vec<&str> {
        self.entities.iter().filter_map(|e| {
            match e {
                EntityType::Text(t) => Some(t.value.as_str()),
                EntityType::MText(t) => Some(t.value.as_str()),
                _ => None,
            }
        }).collect()
    }

    /// Return a count of entities grouped by their type name.
    ///
    /// The keys are the strings returned by [`Entity::entity_type()`], e.g.
    /// `"LINE"`, `"CIRCLE"`, `"ARC"`.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line, Circle};
    ///
    /// let mut doc = CadDocument::new();
    /// doc.add_entity(EntityType::Line(Line::new())).unwrap();
    /// doc.add_entity(EntityType::Line(Line::new())).unwrap();
    /// doc.add_entity(EntityType::Circle(Circle::new())).unwrap();
    ///
    /// let counts = doc.entity_type_counts();
    /// assert_eq!(counts["LINE"], 2);
    /// assert_eq!(counts["CIRCLE"], 1);
    /// ```
    pub fn entity_type_counts(&self) -> HashMap<&str, usize> {
        let mut counts = HashMap::new();
        for e in &self.entities {
            *counts.entry(e.as_entity().entity_type()).or_insert(0) += 1;
        }
        counts
    }

    /// Open a DWG or DXF file, auto-detecting the format from the file extension.
    ///
    /// Supported extensions: `.dwg`, `.dxf` (case-insensitive).
    ///
    /// # Example
    /// ```rust,no_run
    /// use acadrust::CadDocument;
    ///
    /// let doc = CadDocument::from_file("drawing.dwg").unwrap();
    /// let doc = CadDocument::from_file("drawing.dxf").unwrap();
    /// ```
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> crate::Result<Self> {
        let path = path.as_ref();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "dwg" => {
                let mut reader = crate::io::dwg::DwgReader::from_file(path)?;
                reader.read()
            }
            "dxf" => {
                let reader = crate::io::dxf::DxfReader::from_file(path)?;
                reader.read()
            }
            _ => Err(crate::error::DxfError::InvalidFormat(format!(
                "Unsupported file extension: '{}'. Use .dwg or .dxf",
                ext,
            ))),
        }
    }

    /// Save the document to a DWG or DXF file, auto-detecting the format
    /// from the file extension.
    ///
    /// # Example
    /// ```rust,no_run
    /// use acadrust::CadDocument;
    ///
    /// let doc = CadDocument::new();
    /// doc.save("output.dxf").unwrap();
    /// doc.save("output.dwg").unwrap();
    /// ```
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> crate::Result<()> {
        let path = path.as_ref();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "dwg" => crate::io::dwg::DwgWriter::write_to_file(path, self),
            "dxf" => {
                let writer = crate::io::dxf::DxfWriter::new(self);
                writer.write_to_file(path)
            }
            _ => Err(crate::error::DxfError::InvalidFormat(format!(
                "Unsupported file extension for save: '{}'. Use .dwg or .dxf",
                ext,
            ))),
        }
    }

    /// Open a DWG or DXF document from in-memory bytes.
    ///
    /// The method tries DWG first, then falls back to DXF.
    pub fn from_bytes(bytes: &[u8]) -> crate::Result<Self> {
        if bytes.is_empty() {
            return Err(crate::error::DxfError::InvalidFormat(
                "Input bytes are empty".to_string(),
            ));
        }

        let mut dwg_reader = crate::io::dwg::DwgReader::from_stream(Cursor::new(bytes.to_vec()));
        match dwg_reader.read() {
            Ok(doc) => Ok(doc),
            Err(dwg_err) => {
                let dxf_result = crate::io::dxf::DxfReader::from_reader(Cursor::new(bytes.to_vec()))
                    .and_then(|r| r.read());
                match dxf_result {
                    Ok(doc) => Ok(doc),
                    Err(dxf_err) => Err(crate::error::DxfError::InvalidFormat(format!(
                        "Unable to parse bytes as DWG or DXF. DWG error: {}; DXF error: {}",
                        dwg_err, dxf_err,
                    ))),
                }
            }
        }
    }

    /// Serialize the document to DWG bytes.
    pub fn to_bytes(&self) -> crate::Result<Vec<u8>> {
        crate::io::dwg::DwgWriter::write_to_vec(self)
    }

    /// Serialize the document to DXF bytes.
    ///
    /// Set `binary = true` for binary DXF output.
    pub fn to_dxf_bytes(&self, binary: bool) -> crate::Result<Vec<u8>> {
        let mut writer = crate::io::dxf::DxfWriter::new(self);
        writer.set_binary(binary);
        writer.write_to_vec()
    }

    /// Borrow the collected document mutation events.
    pub fn events(&self) -> &crate::notification::DocumentEventCollection {
        &self.events
    }

    /// Drain and return all currently collected mutation events.
    pub fn drain_events(&mut self) -> Vec<crate::notification::DocumentEvent> {
        self.events.drain()
    }

    /// Clear all currently collected mutation events.
    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    /// Move (translate) an entity by the given offset.
    ///
    /// Returns `false` if the handle was not found.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    /// use acadrust::types::Vector3;
    ///
    /// let mut doc = CadDocument::new();
    /// let h = doc.add_entity(EntityType::Line(Line::from_coords(0.0,0.0,0.0,1.0,0.0,0.0))).unwrap();
    /// assert!(doc.move_entity(h, Vector3::new(10.0, 0.0, 0.0)));
    /// ```
    pub fn move_entity(&mut self, handle: crate::types::Handle, offset: Vector3) -> bool {
        if let Some(&idx) = self.entity_index.get(&handle) {
            self.entities[idx].as_entity_mut().translate(offset);
            self.events.entity_modified(handle, "move");
            true
        } else {
            false
        }
    }

    /// Rotate an entity around a center point by the given angle (in radians, Z-axis).
    ///
    /// Returns `false` if the handle was not found.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    /// use acadrust::types::Vector3;
    ///
    /// let mut doc = CadDocument::new();
    /// let h = doc.add_entity(EntityType::Line(Line::from_coords(1.0,0.0,0.0,2.0,0.0,0.0))).unwrap();
    /// doc.rotate_entity(h, Vector3::ZERO, std::f64::consts::FRAC_PI_2);
    /// ```
    pub fn rotate_entity(
        &mut self,
        handle: crate::types::Handle,
        center: Vector3,
        angle: f64,
    ) -> bool {
        if let Some(&idx) = self.entity_index.get(&handle) {
            // translate to origin, rotate around Z, translate back
            let t_to_origin = crate::types::Transform::from_translation(-center);
            let rotation = crate::types::Transform::from_rotation(Vector3::UNIT_Z, angle);
            let t_back = crate::types::Transform::from_translation(center);
            let combined = t_to_origin.then(&rotation).then(&t_back);
            self.entities[idx].as_entity_mut().apply_transform(&combined);
            self.events.entity_modified(handle, "rotate");
            true
        } else {
            false
        }
    }

    /// Scale an entity from a base point by the given factor.
    ///
    /// Returns `false` if the handle was not found.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Circle};
    /// use acadrust::types::Vector3;
    ///
    /// let mut doc = CadDocument::new();
    /// let h = doc.add_entity(EntityType::Circle(Circle::from_center_radius(
    ///     Vector3::new(5.0, 5.0, 0.0), 1.0,
    /// ))).unwrap();
    /// doc.scale_entity(h, Vector3::ZERO, 2.0);
    /// ```
    pub fn scale_entity(
        &mut self,
        handle: crate::types::Handle,
        base_point: Vector3,
        factor: f64,
    ) -> bool {
        if let Some(&idx) = self.entity_index.get(&handle) {
            let transform = crate::types::Transform::from_scaling_with_origin(
                Vector3::new(factor, factor, factor),
                base_point,
            );
            self.entities[idx].as_entity_mut().apply_transform(&transform);
            self.events.entity_modified(handle, "scale");
            true
        } else {
            false
        }
    }

    /// Compute an approximate distance between two entities by handle.
    ///
    /// Uses axis-aligned bounding boxes of both entities.
    pub fn distance_between_handles(&self, a: Handle, b: Handle) -> Option<f64> {
        let ea = self.get_entity(a)?;
        let eb = self.get_entity(b)?;
        Some(Self::distance_between_entities(ea, eb))
    }

    /// Compute an approximate distance between two entities.
    ///
    /// Uses axis-aligned bounding boxes of both entities.
    pub fn distance_between_entities(a: &EntityType, b: &EntityType) -> f64 {
        Self::bbox_distance(a.bounding_box(), b.bounding_box())
    }

    /// Return the minimum approximate distance from one entity to all others.
    pub fn nearest_distance_from(&self, handle: Handle) -> Option<f64> {
        let e = self.get_entity(handle)?;
        let mut best = f64::INFINITY;
        for other in &self.entities {
            if other.common().handle == handle {
                continue;
            }
            let d = Self::distance_between_entities(e, other);
            if d < best {
                best = d;
            }
        }
        if best.is_finite() {
            Some(best)
        } else {
            None
        }
    }

    fn bbox_distance(a: BoundingBox3D, b: BoundingBox3D) -> f64 {
        fn axis_dist(a_min: f64, a_max: f64, b_min: f64, b_max: f64) -> f64 {
            if a_max < b_min {
                b_min - a_max
            } else if b_max < a_min {
                a_min - b_max
            } else {
                0.0
            }
        }

        let dx = axis_dist(a.min.x, a.max.x, b.min.x, b.max.x);
        let dy = axis_dist(a.min.y, a.max.y, b.min.y, b.max.y);
        let dz = axis_dist(a.min.z, a.max.z, b.min.z, b.max.z);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Filter entities using an arbitrary predicate.
    ///
    /// This is the most flexible query method — use it when the built-in
    /// filters (`entities_on_layer`, `entities_of_type`, etc.) are not
    /// sufficient.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    /// use acadrust::types::Color;
    ///
    /// let mut doc = CadDocument::new();
    /// let mut l = Line::from_coords(0.0,0.0,0.0, 1.0,0.0,0.0);
    /// l.common.color = Color::RED;
    /// doc.add_entity(EntityType::Line(l)).unwrap();
    /// doc.add_entity(EntityType::Line(Line::new())).unwrap();
    ///
    /// let red: Vec<_> = doc.entities_where(|e| e.common().color == Color::RED).collect();
    /// assert_eq!(red.len(), 1);
    /// ```
    pub fn entities_where<F>(&self, predicate: F) -> impl Iterator<Item = &EntityType>
    where
        F: Fn(&EntityType) -> bool,
    {
        self.entities.iter().filter(move |e| predicate(e))
    }

    /// Collect handles of entities that match a predicate.
    ///
    /// Convenient for building a [`SelectionSet`](crate::api::selection::SelectionSet)
    /// from a query.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    ///
    /// let mut doc = CadDocument::new();
    /// doc.add_entity(EntityType::Line(Line::from_coords(0.0,0.0,0.0,5.0,0.0,0.0))).unwrap();
    /// doc.add_entity(EntityType::Line(Line::from_coords(0.0,0.0,0.0,1.0,0.0,0.0))).unwrap();
    ///
    /// let on_zero: Vec<_> = doc.select_where(|e| e.common().layer == "0");
    /// assert_eq!(on_zero.len(), 2);
    /// ```
    pub fn select_where<F>(&self, predicate: F) -> Vec<Handle>
    where
        F: Fn(&EntityType) -> bool,
    {
        self.entities
            .iter()
            .filter(|e| predicate(e))
            .map(|e| e.common().handle)
            .collect()
    }

    // ════════════════════════════════════════════════════════════════════
    // Layout Management API
    // ════════════════════════════════════════════════════════════════════

    /// Return all layouts sorted by tab order.
    ///
    /// The first element is always the *Model* layout (tab order 0),
    /// followed by paper-space layouts in tab order.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let doc = CadDocument::new();
    /// let layouts = doc.layouts();
    /// assert!(layouts.iter().any(|l| l.name == "Model"));
    /// ```
    pub fn layouts(&self) -> Vec<&crate::objects::Layout> {
        let mut result: Vec<&crate::objects::Layout> = self
            .objects
            .values()
            .filter_map(|obj| match obj {
                ObjectType::Layout(l) => Some(l),
                _ => None,
            })
            .collect();
        result.sort_by_key(|l| l.tab_order);
        result
    }

    /// Look up a layout by name.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let doc = CadDocument::new();
    /// assert!(doc.get_layout("Model").is_some());
    /// assert!(doc.get_layout("NonExistent").is_none());
    /// ```
    pub fn get_layout(&self, name: &str) -> Option<&crate::objects::Layout> {
        self.objects.values().find_map(|obj| match obj {
            ObjectType::Layout(l) if l.name == name => Some(l),
            _ => None,
        })
    }

    /// Look up a layout by name (mutable).
    pub fn get_layout_mut(&mut self, name: &str) -> Option<&mut crate::objects::Layout> {
        self.objects.values_mut().find_map(|obj| match obj {
            ObjectType::Layout(l) if l.name == name => Some(l),
            _ => None,
        })
    }

    /// Rename a paper-space layout.
    ///
    /// The *Model* layout cannot be renamed.  Returns an error if the
    /// old name is not found or the new name already exists.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// doc.add_layout("Sheet1").unwrap();
    /// doc.rename_layout("Sheet1", "A1-Plan").unwrap();
    /// assert!(doc.get_layout("A1-Plan").is_some());
    /// assert!(doc.get_layout("Sheet1").is_none());
    /// ```
    pub fn rename_layout(&mut self, old_name: &str, new_name: &str) -> Result<()> {
        if old_name == "Model" {
            return Err(crate::error::DxfError::Custom(
                "Cannot rename the Model layout".to_string(),
            ));
        }
        // Check target name doesn't already exist
        let new_exists = self.objects.values().any(|obj| {
            matches!(obj, ObjectType::Layout(l) if l.name == new_name)
        });
        if new_exists {
            return Err(crate::error::DxfError::Custom(format!(
                "Layout '{}' already exists",
                new_name
            )));
        }
        // Find the layout handle
        let layout_handle = self
            .objects
            .values()
            .find_map(|obj| match obj {
                ObjectType::Layout(l) if l.name == old_name => Some(l.handle),
                _ => None,
            })
            .ok_or_else(|| {
                crate::error::DxfError::Custom(format!(
                    "Layout '{}' not found",
                    old_name
                ))
            })?;
        // Rename the layout object
        if let Some(ObjectType::Layout(layout)) = self.objects.get_mut(&layout_handle) {
            layout.name = new_name.to_string();
        }
        // Update ACAD_LAYOUT dictionary
        if let Some(ObjectType::Dictionary(dict)) =
            self.objects.get_mut(&self.header.acad_layout_dict_handle)
        {
            if let Some(pos) = dict.entries.iter().position(|(k, _)| k == old_name) {
                let (_, handle) = dict.entries.remove(pos);
                dict.entries.push((new_name.to_string(), handle));
            }
        }
        Ok(())
    }

    /// Remove a paper-space layout and all its entities.
    ///
    /// The *Model* layout cannot be removed.  This also removes the backing
    /// `*Paper_Space` block record and de-registers the layout from the
    /// ACAD_LAYOUT dictionary.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// doc.add_layout("Temporary").unwrap();
    /// assert!(doc.get_layout("Temporary").is_some());
    /// doc.remove_layout("Temporary").unwrap();
    /// assert!(doc.get_layout("Temporary").is_none());
    /// ```
    pub fn remove_layout(&mut self, name: &str) -> Result<()> {
        if name == "Model" {
            return Err(crate::error::DxfError::Custom(
                "Cannot remove the Model layout".to_string(),
            ));
        }
        // Find layout and its block record handle
        let (layout_handle, br_handle) = self
            .objects
            .values()
            .find_map(|obj| match obj {
                ObjectType::Layout(l) if l.name == name => {
                    Some((l.handle, l.block_record))
                }
                _ => None,
            })
            .ok_or_else(|| {
                crate::error::DxfError::Custom(format!(
                    "Layout '{}' not found",
                    name
                ))
            })?;

        // Collect entity handles owned by this block record
        let entity_handles: Vec<Handle> = self
            .block_records
            .iter()
            .find(|br| br.handle == br_handle)
            .map(|br| br.entity_handles.clone())
            .unwrap_or_default();

        // Remove all entities in this layout
        for eh in entity_handles {
            self.remove_entity(eh);
        }

        // Remove the block record
        let block_name: Option<String> = self
            .block_records
            .iter()
            .find(|br| br.handle == br_handle)
            .map(|br| br.name().to_string());
        if let Some(bn) = block_name {
            self.block_records.remove(&bn);
        }

        // Remove from ACAD_LAYOUT dictionary
        if let Some(ObjectType::Dictionary(dict)) =
            self.objects.get_mut(&self.header.acad_layout_dict_handle)
        {
            dict.entries.retain(|(k, _)| k != name);
        }

        // Remove layout object
        self.objects.remove(&layout_handle);
        Ok(())
    }

    // ════════════════════════════════════════════════════════════════════
    // Entity Copying
    // ════════════════════════════════════════════════════════════════════

    /// Clone an entity and add the copy to the same block/layout.
    ///
    /// The copy receives a new handle; all other properties (layer, color,
    /// geometry, …) are duplicated from the original.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    ///
    /// let mut doc = CadDocument::new();
    /// let line = Line::from_coords(0.0, 0.0, 0.0, 10.0, 10.0, 0.0);
    /// let h1 = doc.add_entity(EntityType::Line(line)).unwrap();
    /// let h2 = doc.copy_entity(h1).unwrap();
    /// assert_ne!(h1, h2);
    /// assert_eq!(doc.entity_count(), 2);
    /// ```
    pub fn copy_entity(&mut self, handle: Handle) -> Result<Handle> {
        let original = self
            .get_entity(handle)
            .ok_or_else(|| {
                crate::error::DxfError::Custom(format!(
                    "Entity with handle {:#X} not found",
                    handle.value()
                ))
            })?
            .clone();
        let mut copy = original;
        // Allocate a fresh handle; clear reactors and xdictionary since
        // those belong to the original entity.
        let new_handle = self.allocate_handle();
        copy.common_mut().handle = new_handle;
        copy.common_mut().reactors.clear();
        copy.common_mut().xdictionary_handle = None;

        // Re-route into the same block record as the original
        let owner = copy.common().owner_handle;
        let is_excluded = matches!(
            &copy,
            EntityType::AttributeEntity(_)
                | EntityType::Block(_)
                | EntityType::BlockEnd(_)
        );
        if !is_excluded && !owner.is_null() {
            for br in self.block_records.iter_mut() {
                if br.handle == owner {
                    br.entity_handles.push(new_handle);
                    break;
                }
            }
        }
        let idx = self.entities.len();
        self.entities.push(copy);
        self.entity_index.insert(new_handle, idx);
        Ok(new_handle)
    }

    /// Clone an entity into a different layout.
    ///
    /// The copy receives a new handle and its owner is set to the target
    /// layout's block record.  The target layout is created if it does
    /// not exist (via [`add_layout`](Self::add_layout)).
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    ///
    /// let mut doc = CadDocument::new();
    /// let line = Line::from_coords(0.0, 0.0, 0.0, 5.0, 5.0, 0.0);
    /// let h1 = doc.add_entity(EntityType::Line(line)).unwrap();
    /// doc.add_layout("Layout2").unwrap();
    /// let h2 = doc.copy_entity_to_layout(h1, "Layout2").unwrap();
    /// assert_ne!(h1, h2);
    /// ```
    pub fn copy_entity_to_layout(
        &mut self,
        handle: Handle,
        layout_name: &str,
    ) -> Result<Handle> {
        let original = self
            .get_entity(handle)
            .ok_or_else(|| {
                crate::error::DxfError::Custom(format!(
                    "Entity with handle {:#X} not found",
                    handle.value()
                ))
            })?
            .clone();
        let mut copy = original;
        let new_handle = self.allocate_handle();
        copy.common_mut().handle = new_handle;
        copy.common_mut().reactors.clear();
        copy.common_mut().xdictionary_handle = None;
        // Reset owner — add_entity_to_layout will set it correctly
        copy.common_mut().owner_handle = Handle::NULL;
        self.add_entity_to_layout(copy, layout_name)
    }

    // ════════════════════════════════════════════════════════════════════
    // Group & Batch Operations API
    // ════════════════════════════════════════════════════════════════════

    /// Create a named entity group and register it in the ACAD_GROUP
    /// dictionary.
    ///
    /// Returns an error if a group with the same name already exists.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// let gh = doc.create_group("Walls").unwrap();
    /// assert!(doc.get_group("Walls").is_some());
    /// ```
    pub fn create_group(&mut self, name: &str) -> Result<Handle> {
        // Check for duplicate
        if self.get_group(name).is_some() {
            return Err(crate::error::DxfError::Custom(format!(
                "Group '{}' already exists",
                name
            )));
        }
        let mut group = crate::objects::Group::new(name);
        group.handle = self.allocate_handle();
        group.owner = self.header.acad_group_dict_handle;
        let gh = group.handle;

        // Register in ACAD_GROUP dictionary
        if let Some(ObjectType::Dictionary(dict)) =
            self.objects.get_mut(&self.header.acad_group_dict_handle)
        {
            dict.add_entry(name, gh);
        }
        self.objects.insert(gh, ObjectType::Group(group));
        Ok(gh)
    }

    /// Create a named group pre-populated with a description and entity
    /// handles.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    ///
    /// let mut doc = CadDocument::new();
    /// let h1 = doc.add_entity(EntityType::Line(Line::from_coords(0.0,0.0,0.0, 1.0,1.0,0.0))).unwrap();
    /// let h2 = doc.add_entity(EntityType::Line(Line::from_coords(1.0,1.0,0.0, 2.0,2.0,0.0))).unwrap();
    /// let gh = doc.create_group_with("Frame", "Outer frame lines", &[h1, h2]).unwrap();
    /// assert_eq!(doc.get_group("Frame").unwrap().len(), 2);
    /// ```
    pub fn create_group_with(
        &mut self,
        name: &str,
        description: &str,
        handles: &[Handle],
    ) -> Result<Handle> {
        let gh = self.create_group(name)?;
        if let Some(ObjectType::Group(group)) = self.objects.get_mut(&gh) {
            group.description = description.to_string();
            group.add_entities(handles.iter().copied());
        }
        Ok(gh)
    }

    /// Look up a group by name.
    pub fn get_group(&self, name: &str) -> Option<&crate::objects::Group> {
        self.objects.values().find_map(|obj| match obj {
            ObjectType::Group(g) if g.name == name => Some(g),
            _ => None,
        })
    }

    /// Look up a group by name (mutable).
    pub fn get_group_mut(&mut self, name: &str) -> Option<&mut crate::objects::Group> {
        self.objects.values_mut().find_map(|obj| match obj {
            ObjectType::Group(g) if g.name == name => Some(g),
            _ => None,
        })
    }

    /// Return all groups in the document.
    pub fn groups(&self) -> Vec<&crate::objects::Group> {
        self.objects
            .values()
            .filter_map(|obj| match obj {
                ObjectType::Group(g) => Some(g),
                _ => None,
            })
            .collect()
    }

    /// Remove a group from the document.
    ///
    /// This only removes the group object — the member entities are **not**
    /// deleted.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// doc.create_group("Temp").unwrap();
    /// assert!(doc.get_group("Temp").is_some());
    /// doc.remove_group("Temp").unwrap();
    /// assert!(doc.get_group("Temp").is_none());
    /// ```
    pub fn remove_group(&mut self, name: &str) -> Result<()> {
        let group_handle = self
            .objects
            .values()
            .find_map(|obj| match obj {
                ObjectType::Group(g) if g.name == name => Some(g.handle),
                _ => None,
            })
            .ok_or_else(|| {
                crate::error::DxfError::Custom(format!(
                    "Group '{}' not found",
                    name
                ))
            })?;

        // Remove from ACAD_GROUP dictionary
        if let Some(ObjectType::Dictionary(dict)) =
            self.objects.get_mut(&self.header.acad_group_dict_handle)
        {
            dict.entries.retain(|(k, _)| k != name);
        }
        self.objects.remove(&group_handle);
        Ok(())
    }

    /// Apply a mutation to every entity whose handle is in the given slice.
    ///
    /// Handles that do not resolve to an entity are silently skipped.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    /// use acadrust::types::Color;
    ///
    /// let mut doc = CadDocument::new();
    /// let h1 = doc.add_entity(EntityType::Line(Line::from_coords(0.0,0.0,0.0, 1.0,0.0,0.0))).unwrap();
    /// let h2 = doc.add_entity(EntityType::Line(Line::from_coords(0.0,0.0,0.0, 0.0,1.0,0.0))).unwrap();
    /// doc.modify_entities(&[h1, h2], |e| {
    ///     e.common_mut().color = Color::RED;
    /// });
    /// assert_eq!(doc.get_entity(h1).unwrap().common().color, Color::RED);
    /// assert_eq!(doc.get_entity(h2).unwrap().common().color, Color::RED);
    /// ```
    pub fn modify_entities<F>(&mut self, handles: &[Handle], mut f: F)
    where
        F: FnMut(&mut EntityType),
    {
        for &h in handles {
            if let Some(&idx) = self.entity_index.get(&h) {
                f(&mut self.entities[idx]);
            }
        }
    }

    // ════════════════════════════════════════════════════════════════════
    // Xref (External Reference) API
    // ════════════════════════════════════════════════════════════════════

    /// Attach an external drawing as a reference (xref).
    ///
    /// Creates a `BlockRecord` with the `is_xref` flag and the given file
    /// path, together with the required `Block` / `BlockEnd` structural
    /// entities.  The xref is *not* resolved (loaded) — it only records
    /// the reference in the document so that a DWG/DXF writer emits the
    /// correct data and the host CAD application can resolve it.
    ///
    /// Use `overlay = true` for an xref overlay (not nested into further
    /// xrefs) or `false` for a standard attach.
    ///
    /// Returns the block-record handle.  To place the xref in model space
    /// create an [`Insert`](crate::entities::Insert) referencing the
    /// block name.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// let br_handle = doc.attach_xref("site_plan", "C:/drawings/site_plan.dwg", false).unwrap();
    /// assert!(doc.block_records.get("site_plan").is_some());
    /// ```
    pub fn attach_xref(
        &mut self,
        block_name: &str,
        file_path: &str,
        overlay: bool,
    ) -> Result<Handle> {
        // Reject if a block with this name already exists
        if self.block_records.get(block_name).is_some() {
            return Err(crate::error::DxfError::Custom(format!(
                "Block record '{}' already exists",
                block_name
            )));
        }

        // Create the xref block record
        let mut br = BlockRecord::new(block_name);
        br.set_handle(self.allocate_handle());
        br.flags.is_xref = true;
        br.flags.is_xref_overlay = overlay;
        br.xref_path = file_path.to_string();

        // Create structural Block/BlockEnd entities
        let block_handle = self.allocate_handle();
        let block_end_handle = self.allocate_handle();
        br.block_entity_handle = block_handle;
        br.block_end_handle = block_end_handle;

        let br_handle = br.handle;
        self.block_records
            .add(br)
            .map_err(|e| crate::error::DxfError::Custom(e))?;

        // Store Block entity
        let mut block = crate::entities::Block::new(block_name, Vector3::default());
        block.xref_path = file_path.to_string();
        block.common.handle = block_handle;
        block.common.owner_handle = br_handle;
        let idx = self.entities.len();
        self.entities.push(EntityType::Block(block));
        self.entity_index.insert(block_handle, idx);

        // Store BlockEnd entity
        let mut block_end = crate::entities::BlockEnd::new();
        block_end.common.handle = block_end_handle;
        block_end.common.owner_handle = br_handle;
        let idx = self.entities.len();
        self.entities.push(EntityType::BlockEnd(block_end));
        self.entity_index.insert(block_end_handle, idx);

        Ok(br_handle)
    }

    /// Detach (remove) an external reference by block name.
    ///
    /// Removes the xref block record, its structural `Block`/`BlockEnd`
    /// entities, **and** any `Insert` entities that reference it.
    ///
    /// Returns an error if the block is not an xref.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// doc.attach_xref("site", "site.dwg", false).unwrap();
    /// doc.detach_xref("site").unwrap();
    /// assert!(doc.block_records.get("site").is_none());
    /// ```
    pub fn detach_xref(&mut self, block_name: &str) -> Result<()> {
        // Verify this is indeed an xref
        let br = self
            .block_records
            .get(block_name)
            .ok_or_else(|| {
                crate::error::DxfError::Custom(format!(
                    "Block record '{}' not found",
                    block_name
                ))
            })?;
        if !br.flags.is_xref {
            return Err(crate::error::DxfError::Custom(format!(
                "Block '{}' is not an xref",
                block_name
            )));
        }

        // Collect handles to remove: Block, BlockEnd, owned entities
        let mut to_remove: Vec<Handle> = Vec::new();
        to_remove.push(br.block_entity_handle);
        to_remove.push(br.block_end_handle);
        to_remove.extend(br.entity_handles.iter().copied());

        // Also remove any Insert entities that reference this block
        let inserts: Vec<Handle> = self
            .entities
            .iter()
            .filter_map(|e| match e {
                EntityType::Insert(ins) if ins.block_name == block_name => {
                    Some(ins.common.handle)
                }
                _ => None,
            })
            .collect();
        to_remove.extend(inserts);

        for h in to_remove {
            if !h.is_null() {
                self.remove_entity(h);
            }
        }

        // Remove the block record
        self.block_records.remove(block_name);
        Ok(())
    }

    /// List all xref block records in the document.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// doc.attach_xref("floor1", "floor1.dwg", false).unwrap();
    /// doc.attach_xref("floor2", "floor2.dwg", true).unwrap();
    /// let xrefs = doc.xrefs();
    /// assert_eq!(xrefs.len(), 2);
    /// ```
    pub fn xrefs(&self) -> Vec<&BlockRecord> {
        self.block_records
            .iter()
            .filter(|br| br.flags.is_xref)
            .collect()
    }

    /// Return xref info for a block name, or `None` if it is not an xref.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let mut doc = CadDocument::new();
    /// doc.attach_xref("plan", "plan.dwg", false).unwrap();
    /// let info = doc.xref_info("plan").unwrap();
    /// assert_eq!(info.file_path, "plan.dwg");
    /// assert!(!info.is_overlay);
    /// ```
    pub fn xref_info(&self, block_name: &str) -> Option<XrefInfo<'_>> {
        let br = self.block_records.get(block_name)?;
        if !br.flags.is_xref {
            return None;
        }
        Some(XrefInfo {
            block_name: &br.name,
            file_path: &br.xref_path,
            is_overlay: br.flags.is_xref_overlay,
            handle: br.handle,
        })
    }

    // ════════════════════════════════════════════════════════════════════
    // Block Instance API
    // ════════════════════════════════════════════════════════════════════

    /// Insert a block reference into model space.
    ///
    /// The block must already exist (created via
    /// [`BlockBuilder`](crate::api::block::BlockBuilder) or loaded from a
    /// file).  Returns the handle of the new
    /// [`Insert`](crate::entities::Insert) entity.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::api::block::BlockBuilder;
    /// use acadrust::entities::{EntityType, Circle};
    /// use acadrust::types::Vector3;
    ///
    /// let mut doc = CadDocument::new();
    /// BlockBuilder::new("Bolt")
    ///     .entity(EntityType::Circle(Circle::from_center_radius(Vector3::ZERO, 3.0)))
    ///     .build(&mut doc)
    ///     .unwrap();
    ///
    /// let h = doc.insert_block("Bolt", Vector3::new(10.0, 20.0, 0.0)).unwrap();
    /// assert_eq!(doc.get_entity(h).unwrap().common().layer, "0");
    /// ```
    pub fn insert_block(
        &mut self,
        block_name: &str,
        position: Vector3,
    ) -> Result<Handle> {
        if self.block_records.get(block_name).is_none() {
            return Err(crate::error::DxfError::Custom(format!(
                "Block '{}' not found",
                block_name
            )));
        }
        let insert = crate::entities::Insert::new(block_name, position);
        self.add_entity(EntityType::Insert(insert))
    }

    /// Insert a block reference with explicit scale and rotation.
    ///
    /// `scale` is applied uniformly to X, Y, and Z.  `rotation` is in
    /// radians.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::api::block::BlockBuilder;
    /// use acadrust::entities::{EntityType, Circle};
    /// use acadrust::types::Vector3;
    /// use std::f64::consts::FRAC_PI_2;
    ///
    /// let mut doc = CadDocument::new();
    /// BlockBuilder::new("Arrow")
    ///     .entity(EntityType::Circle(Circle::from_center_radius(Vector3::ZERO, 1.0)))
    ///     .build(&mut doc)
    ///     .unwrap();
    ///
    /// let h = doc.insert_block_with(
    ///     "Arrow",
    ///     Vector3::new(0.0, 0.0, 0.0),
    ///     2.0,
    ///     FRAC_PI_2,
    /// ).unwrap();
    /// ```
    pub fn insert_block_with(
        &mut self,
        block_name: &str,
        position: Vector3,
        scale: f64,
        rotation: f64,
    ) -> Result<Handle> {
        if self.block_records.get(block_name).is_none() {
            return Err(crate::error::DxfError::Custom(format!(
                "Block '{}' not found",
                block_name
            )));
        }
        let mut insert = crate::entities::Insert::new(block_name, position);
        insert.set_x_scale(scale);
        insert.set_y_scale(scale);
        insert.set_z_scale(scale);
        insert.rotation = rotation;
        self.add_entity(EntityType::Insert(insert))
    }

    /// Resolve handle references after reading a DXF file.
    ///
    /// This performs a simplified version of ACadSharp's two-phase build:
    ///
    /// 1. Assigns owner handles on model-space entities (owner = model space
    ///    block record handle) when the entity has no owner set.
    /// 2. Assigns owner handles on block-owned entities (owner = the block
    ///    record handle) when the entity has no owner set.
    /// 3. Updates `next_handle` to be above the maximum handle seen in the
    ///    document so that subsequent `allocate_handle()` calls produce unique
    ///    values.
    ///
    /// Call this once after loading (the DXF reader calls it automatically).
    pub fn resolve_references(&mut self) {
        // --- 1. Find the max handle in use across the whole document ---
        let mut max_handle: u64 = self.next_handle;

        // Check entities
        for entity in self.entities.iter() {
            let h = entity.common().handle.value();
            if h >= max_handle {
                max_handle = h + 1;
            }
        }

        // Check objects
        for (handle, _) in &self.objects {
            let h = handle.value();
            if h >= max_handle {
                max_handle = h + 1;
            }
        }

        // Check block record handles
        for br in self.block_records.iter() {
            let h = br.handle.value();
            if h >= max_handle {
                max_handle = h + 1;
            }
            for eh in &br.entity_handles {
                let h = eh.value();
                if h >= max_handle {
                    max_handle = h + 1;
                }
            }
        }

        self.next_handle = max_handle;

        // --- 1b. Resolve table handle collisions ---
        // Collect ALL handles used by entries, entities, and objects so we can
        // detect when a table control handle collides with ANY of them.
        let mut used_handles = std::collections::HashSet::new();
        for e in self.layers.iter()       { if !e.handle().is_null() { used_handles.insert(e.handle().value()); } }
        for e in self.line_types.iter()    { if !e.handle().is_null() { used_handles.insert(e.handle().value()); } }
        for e in self.text_styles.iter()   { if !e.handle().is_null() { used_handles.insert(e.handle().value()); } }
        for e in self.vports.iter()        { if !e.handle().is_null() { used_handles.insert(e.handle().value()); } }
        for e in self.views.iter()         { if !e.handle().is_null() { used_handles.insert(e.handle().value()); } }
        for e in self.ucss.iter()          { if !e.handle().is_null() { used_handles.insert(e.handle().value()); } }
        for e in self.app_ids.iter()       { if !e.handle().is_null() { used_handles.insert(e.handle().value()); } }
        for e in self.dim_styles.iter()    { if !e.handle().is_null() { used_handles.insert(e.handle().value()); } }
        for e in self.block_records.iter() { if !e.handle().is_null() { used_handles.insert(e.handle().value()); } }
        for e in self.entities.iter()      { let h = e.common().handle.value(); if h > 0 { used_handles.insert(h); } }
        for (h, _) in &self.objects        { let v = h.value(); if v > 0 { used_handles.insert(v); } }

        // Reassign any table control handle that collides with a used handle
        if used_handles.contains(&self.vports.handle().value()) {
            let h = Handle::new(self.next_handle); self.next_handle += 1;
            self.vports.set_handle(h); self.header.vport_control_handle = h;
        }
        if used_handles.contains(&self.line_types.handle().value()) {
            let h = Handle::new(self.next_handle); self.next_handle += 1;
            self.line_types.set_handle(h); self.header.linetype_control_handle = h;
        }
        if used_handles.contains(&self.layers.handle().value()) {
            let h = Handle::new(self.next_handle); self.next_handle += 1;
            self.layers.set_handle(h); self.header.layer_control_handle = h;
        }
        if used_handles.contains(&self.text_styles.handle().value()) {
            let h = Handle::new(self.next_handle); self.next_handle += 1;
            self.text_styles.set_handle(h); self.header.style_control_handle = h;
        }
        if used_handles.contains(&self.views.handle().value()) {
            let h = Handle::new(self.next_handle); self.next_handle += 1;
            self.views.set_handle(h); self.header.view_control_handle = h;
        }
        if used_handles.contains(&self.ucss.handle().value()) {
            let h = Handle::new(self.next_handle); self.next_handle += 1;
            self.ucss.set_handle(h); self.header.ucs_control_handle = h;
        }
        if used_handles.contains(&self.app_ids.handle().value()) {
            let h = Handle::new(self.next_handle); self.next_handle += 1;
            self.app_ids.set_handle(h); self.header.appid_control_handle = h;
        }
        if used_handles.contains(&self.dim_styles.handle().value()) {
            let h = Handle::new(self.next_handle); self.next_handle += 1;
            self.dim_styles.set_handle(h); self.header.dimstyle_control_handle = h;
        }
        if used_handles.contains(&self.block_records.handle().value()) {
            let h = Handle::new(self.next_handle); self.next_handle += 1;
            self.block_records.set_handle(h); self.header.block_control_handle = h;
        }

        // --- 1c. Resolve block entity/end handle collisions ---
        // block_entity_handle and block_end_handle are pre-allocated during
        // initialize_defaults() and may collide with entry/entity handles
        // read from the file.
        for br in self.block_records.iter_mut() {
            if !br.block_entity_handle.is_null()
                && used_handles.contains(&br.block_entity_handle.value())
            {
                let h = Handle::new(self.next_handle); self.next_handle += 1;
                br.block_entity_handle = h;
            }
            if !br.block_end_handle.is_null()
                && used_handles.contains(&br.block_end_handle.value())
            {
                let h = Handle::new(self.next_handle); self.next_handle += 1;
                br.block_end_handle = h;
            }
        }

        // --- 1d. Resolve object handle collisions ---
        // Dictionary and other objects created by initialize_defaults() may
        // have handles that collide with file-sourced handles.
        let mut remap: Vec<(Handle, Handle)> = Vec::new();
        let obj_handles: Vec<Handle> = self.objects.keys().copied().collect();
        for old_h in obj_handles {
            if used_handles.contains(&old_h.value()) {
                let new_h = Handle::new(self.next_handle); self.next_handle += 1;
                remap.push((old_h, new_h));
            }
        }
        for (old_h, new_h) in &remap {
            if let Some(mut obj) = self.objects.remove(old_h) {
                // Update the object's own handle field
                match &mut obj {
                    ObjectType::Dictionary(d) => d.handle = *new_h,
                    ObjectType::Layout(l) => l.handle = *new_h,
                    ObjectType::MLineStyle(m) => m.handle = *new_h,
                    ObjectType::PlaceHolder(p) => p.handle = *new_h,
                    ObjectType::DictionaryWithDefault(d) => d.handle = *new_h,
                    _ => {}
                }
                self.objects.insert(*new_h, obj);
            }
        }
        // Update cross-references: dictionary entries and owner handles
        if !remap.is_empty() {
            let remap_map: std::collections::HashMap<u64, Handle> =
                remap.iter().map(|(o, n)| (o.value(), *n)).collect();

            // Update dictionary entry values that reference remapped handles
            for (_, obj) in self.objects.iter_mut() {
                match obj {
                    ObjectType::Dictionary(d) => {
                        if let Some(new_owner) = remap_map.get(&d.owner.value()) {
                            d.owner = *new_owner;
                        }
                        for (_, entry_handle) in d.entries.iter_mut() {
                            if let Some(new_h) = remap_map.get(&entry_handle.value()) {
                                *entry_handle = *new_h;
                            }
                        }
                    }
                    ObjectType::Layout(l) => {
                        if let Some(new_owner) = remap_map.get(&l.owner.value()) {
                            l.owner = *new_owner;
                        }
                    }
                    ObjectType::MLineStyle(m) => {
                        if let Some(new_owner) = remap_map.get(&m.owner.value()) {
                            m.owner = *new_owner;
                        }
                    }
                    ObjectType::PlaceHolder(p) => {
                        if let Some(new_owner) = remap_map.get(&p.owner.value()) {
                            p.owner = *new_owner;
                        }
                    }
                    ObjectType::DictionaryWithDefault(d) => {
                        if let Some(new_owner) = remap_map.get(&d.owner.value()) {
                            d.owner = *new_owner;
                        }
                        for (_, entry_handle) in d.entries.iter_mut() {
                            if let Some(new_h) = remap_map.get(&entry_handle.value()) {
                                *entry_handle = *new_h;
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Update header handles that reference remapped objects
            let header_handles = [
                &mut self.header.named_objects_dict_handle,
                &mut self.header.acad_group_dict_handle,
                &mut self.header.acad_mlinestyle_dict_handle,
                &mut self.header.acad_layout_dict_handle,
                &mut self.header.acad_plotsettings_dict_handle,
                &mut self.header.acad_plotstylename_dict_handle,
                &mut self.header.acad_material_dict_handle,
                &mut self.header.acad_color_dict_handle,
                &mut self.header.acad_visualstyle_dict_handle,
                &mut self.header.current_multiline_style_handle,
            ];
            for handle in header_handles {
                if let Some(new_h) = remap_map.get(&handle.value()) {
                    *handle = *new_h;
                }
            }

            // Update block record layout references
            for br in self.block_records.iter_mut() {
                if let Some(new_h) = remap_map.get(&br.layout.value()) {
                    br.layout = *new_h;
                }
            }
        }
        let model_handle = self.header.model_space_block_handle;
        let paper_handle = self.header.paper_space_block_handle;

        // Model-space entities (document.entities) — use model space as default owner
        for entity in self.entities.iter_mut() {
            let common = match entity {
                EntityType::Dimension(d) => {
                    let base = d.base_mut();
                    &mut base.common
                }
                _ => {
                    // For all other entity types, use as_entity_mut().set_handle pattern
                    // but we need &mut EntityCommon directly — use a helper
                    get_common_mut(entity)
                }
            };
            if common.owner_handle.is_null() {
                common.owner_handle = model_handle;
            }
        }

        // Block record entities — set owner handle on entities looked up from entity map
        for br in self.block_records.iter() {
            let br_handle = br.handle;
            for eh in &br.entity_handles {
                if let Some(&idx) = self.entity_index.get(eh) {
                    let entity = &mut self.entities[idx];
                    let common = match entity {
                        EntityType::Dimension(d) => {
                            let base = d.base_mut();
                            &mut base.common
                        }
                        _ => get_common_mut(entity),
                    };
                    if common.owner_handle.is_null() {
                        common.owner_handle = br_handle;
                    }
                }
            }
        }

        // Paper-space entities — if an entity's owner is the paper space block,
        // the entity is already correctly assigned by the reader.
        // We just skip further assignment here.

        let _ = paper_handle; // suppress unused warning; future: paper space logic
    }

    // ════════════════════════════════════════════════════════════════════
    // Entity Relationship Queries
    // ════════════════════════════════════════════════════════════════════

    /// Return the entities that belong to a named block definition.
    ///
    /// Resolves each handle stored in [`BlockRecord::entity_handles`] and
    /// returns references to the matching entities.  Handles that do not
    /// resolve to an entity in the document are silently skipped.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::api::block::BlockBuilder;
    /// use acadrust::entities::{EntityType, Line};
    ///
    /// let mut doc = CadDocument::new();
    /// BlockBuilder::new("Frame")
    ///     .entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 10.0, 0.0, 0.0)))
    ///     .entity(EntityType::Line(Line::from_coords(10.0, 0.0, 0.0, 10.0, 10.0, 0.0)))
    ///     .build(&mut doc).unwrap();
    ///
    /// assert_eq!(doc.block_entities("Frame").count(), 2);
    /// assert_eq!(doc.block_entities("NonExistent").count(), 0);
    /// ```
    pub fn block_entities<'a>(&'a self, block_name: &str) -> impl Iterator<Item = &'a EntityType> {
        let handles: Vec<Handle> = self
            .block_records
            .get(block_name)
            .map(|br| br.entity_handles.clone())
            .unwrap_or_default();
        handles.into_iter().filter_map(move |h| self.get_entity(h))
    }

    /// Return all [`Insert`] entities that reference a given block name.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::api::block::BlockBuilder;
    /// use acadrust::entities::{EntityType, Circle, Insert};
    /// use acadrust::types::Vector3;
    ///
    /// let mut doc = CadDocument::new();
    /// BlockBuilder::new("Bolt")
    ///     .entity(EntityType::Circle(Circle::from_center_radius(Vector3::ZERO, 2.0)))
    ///     .build(&mut doc).unwrap();
    /// doc.insert_block("Bolt", Vector3::new(10.0, 0.0, 0.0)).unwrap();
    /// doc.insert_block("Bolt", Vector3::new(20.0, 0.0, 0.0)).unwrap();
    ///
    /// assert_eq!(doc.inserts_of_block("Bolt").count(), 2);
    /// ```
    pub fn inserts_of_block<'a>(
        &'a self,
        block_name: &'a str,
    ) -> impl Iterator<Item = &'a crate::entities::Insert> {
        self.entities_of_type::<crate::entities::Insert>()
            .filter(move |ins| ins.block_name == block_name)
    }

    /// Return the name of the block record that owns a given entity.
    ///
    /// Returns `None` if the entity's owner handle doesn't match any block
    /// record, or if the handle is null.
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    /// use acadrust::entities::{EntityType, Line};
    ///
    /// let mut doc = CadDocument::new();
    /// let h = doc.add_entity(EntityType::Line(Line::new())).unwrap();
    /// // Model-space entities are owned by the "*Model_Space" block record
    /// assert!(doc.entity_owner(h).is_some());
    /// ```
    pub fn entity_owner(&self, handle: Handle) -> Option<&str> {
        let entity = self.get_entity(handle)?;
        let owner = entity.common().owner_handle;
        if owner.is_null() {
            return None;
        }
        self.block_records
            .iter()
            .find(|br| br.handle == owner)
            .map(|br| br.name.as_str())
    }

    // ════════════════════════════════════════════════════════════════════
    // Document Validation / Audit
    // ════════════════════════════════════════════════════════════════════

    /// Validate the document and return a list of issues found.
    ///
    /// Checks performed:
    /// - Entities referencing non-existent layers
    /// - Entities with null handles
    /// - Block records referencing entity handles that don't exist
    /// - Duplicate handles in the entity index
    ///
    /// # Example
    /// ```rust
    /// use acadrust::CadDocument;
    ///
    /// let doc = CadDocument::new();
    /// let issues = doc.validate();
    /// assert!(issues.is_empty());
    /// ```
    pub fn validate(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check for entities with null handles
        for (i, entity) in self.entities.iter().enumerate() {
            let common = entity.common();
            if common.handle.is_null() {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    message: format!("Entity at index {} has a null handle", i),
                });
            }

            // Check layer existence
            if !common.layer.is_empty() && self.layers.get(&common.layer).is_none() {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    message: format!(
                        "Entity {:?} references non-existent layer \"{}\"",
                        common.handle, common.layer
                    ),
                });
            }
        }

        // Check block records for dangling entity handles
        for br in self.block_records.iter() {
            for eh in &br.entity_handles {
                if !eh.is_null() && self.entity_index.get(eh).is_none() {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        message: format!(
                            "Block record \"{}\" references entity handle {:?} which does not exist",
                            br.name, eh
                        ),
                    });
                }
            }
        }

        // Check for handle conflicts (entity_index size vs entities vec size)
        if self.entity_index.len() != self.entities.len() {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                message: format!(
                    "Entity index has {} entries but entities vec has {} entries (possible duplicate handles)",
                    self.entity_index.len(),
                    self.entities.len()
                ),
            });
        }

        issues
    }
}

/// Helper to get a mutable reference to EntityCommon for non-Dimension entities.
fn get_common_mut(entity: &mut EntityType) -> &mut EntityCommon {
    match entity {
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
        EntityType::Dimension(d) => &mut d.base_mut().common,
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

impl Default for CadDocument {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::*;

    // ── Text search tests ────────────────────────────────────────────

    #[test]
    fn test_entities_with_text_case_insensitive() {
        let mut doc = CadDocument::new();
        let mut t = Text::new();
        t.value = "Hello World".into();
        doc.add_entity(EntityType::Text(t)).unwrap();
        let mut mt = MText::new();
        mt.value = "Goodbye".into();
        doc.add_entity(EntityType::MText(mt)).unwrap();

        let found: Vec<_> = doc.entities_with_text("hello").collect();
        assert_eq!(found.len(), 1);

        let found: Vec<_> = doc.entities_with_text("GOODBYE").collect();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn test_entities_with_text_partial_match() {
        let mut doc = CadDocument::new();
        let mut t = Text::new();
        t.value = "ROOM 101".into();
        doc.add_entity(EntityType::Text(t)).unwrap();
        let mut t2 = Text::new();
        t2.value = "ROOM 202".into();
        doc.add_entity(EntityType::Text(t2)).unwrap();

        assert_eq!(doc.entities_with_text("room").count(), 2);
        assert_eq!(doc.entities_with_text("101").count(), 1);
        assert_eq!(doc.entities_with_text("xyz").count(), 0);
    }

    #[test]
    fn test_entities_with_text_no_text_entities() {
        let mut doc = CadDocument::new();
        doc.add_entity(EntityType::Line(Line::new())).unwrap();
        assert_eq!(doc.entities_with_text("anything").count(), 0);
    }

    #[test]
    fn test_text_values() {
        let mut doc = CadDocument::new();
        let mut t = Text::new();
        t.value = "A".into();
        doc.add_entity(EntityType::Text(t)).unwrap();
        let mut mt = MText::new();
        mt.value = "B".into();
        doc.add_entity(EntityType::MText(mt)).unwrap();
        doc.add_entity(EntityType::Line(Line::new())).unwrap();

        let values = doc.text_values();
        assert_eq!(values.len(), 2);
        assert!(values.contains(&"A"));
        assert!(values.contains(&"B"));
    }

    // ── Entity type counts tests ─────────────────────────────────────

    #[test]
    fn test_entity_type_counts_empty() {
        let doc = CadDocument::new();
        assert!(doc.entity_type_counts().is_empty());
    }

    #[test]
    fn test_entity_type_counts_mixed() {
        let mut doc = CadDocument::new();
        doc.add_entity(EntityType::Line(Line::new())).unwrap();
        doc.add_entity(EntityType::Line(Line::new())).unwrap();
        doc.add_entity(EntityType::Circle(Circle::new())).unwrap();
        doc.add_entity(EntityType::Arc(Arc::new())).unwrap();

        let counts = doc.entity_type_counts();
        assert_eq!(counts["LINE"], 2);
        assert_eq!(counts["CIRCLE"], 1);
        assert_eq!(counts["ARC"], 1);
        assert_eq!(counts.len(), 3);
    }

    // ── from_file / save tests ───────────────────────────────────────

    #[test]
    fn test_from_file_bad_extension() {
        let result = CadDocument::from_file("test.xyz");
        assert!(result.is_err());
    }

    #[test]
    fn test_save_bad_extension() {
        let doc = CadDocument::new();
        let result = doc.save("test.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_from_file_nonexistent() {
        let result = CadDocument::from_file("nonexistent.dxf");
        assert!(result.is_err());
        let result = CadDocument::from_file("nonexistent.dwg");
        assert!(result.is_err());
    }

    // ── Document-level transform tests ───────────────────────────────

    #[test]
    fn test_move_entity() {
        let mut doc = CadDocument::new();
        let h = doc
            .add_entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 1.0, 0.0, 0.0)))
            .unwrap();
        assert!(doc.move_entity(h, Vector3::new(10.0, 20.0, 0.0)));

        let line = doc.entities_of_type::<Line>().next().unwrap();
        assert!((line.start.x - 10.0).abs() < 1e-10);
        assert!((line.start.y - 20.0).abs() < 1e-10);
        assert!((line.end.x - 11.0).abs() < 1e-10);
    }

    #[test]
    fn test_move_entity_not_found() {
        let mut doc = CadDocument::new();
        assert!(!doc.move_entity(Handle::new(999), Vector3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_rotate_entity() {
        use std::f64::consts::FRAC_PI_2;
        let mut doc = CadDocument::new();
        let h = doc
            .add_entity(EntityType::Line(Line::from_coords(1.0, 0.0, 0.0, 2.0, 0.0, 0.0)))
            .unwrap();
        doc.rotate_entity(h, Vector3::ZERO, FRAC_PI_2);

        let line = doc.entities_of_type::<Line>().next().unwrap();
        // After 90° CCW rotation: (1,0)→(0,1), (2,0)→(0,2)
        assert!(line.start.x.abs() < 1e-8);
        assert!((line.start.y - 1.0).abs() < 1e-8);
    }

    #[test]
    fn test_rotate_entity_not_found() {
        let mut doc = CadDocument::new();
        assert!(!doc.rotate_entity(Handle::new(999), Vector3::ZERO, 1.0));
    }

    #[test]
    fn test_scale_entity() {
        let mut doc = CadDocument::new();
        let h = doc
            .add_entity(EntityType::Circle(Circle::from_center_radius(
                Vector3::new(5.0, 5.0, 0.0),
                1.0,
            )))
            .unwrap();
        doc.scale_entity(h, Vector3::ZERO, 2.0);

        let circle = doc.entities_of_type::<Circle>().next().unwrap();
        assert!((circle.center.x - 10.0).abs() < 1e-8);
        assert!((circle.center.y - 10.0).abs() < 1e-8);
    }

    #[test]
    fn test_scale_entity_not_found() {
        let mut doc = CadDocument::new();
        assert!(!doc.scale_entity(Handle::new(999), Vector3::ZERO, 2.0));
    }

    // ── In-memory bytes IO tests ─────────────────────────────────────

    #[test]
    fn test_to_dxf_bytes_and_from_bytes_roundtrip() {
        let mut doc = CadDocument::new();
        doc.add_entity(EntityType::Line(Line::from_coords(
            0.0, 0.0, 0.0, 10.0, 0.0, 0.0,
        )))
        .unwrap();

        let bytes = doc.to_dxf_bytes(false).expect("serialize DXF bytes");
        assert!(!bytes.is_empty());

        let parsed = CadDocument::from_bytes(&bytes).expect("parse bytes as DXF");
        assert_eq!(parsed.entities().count(), 1);
    }

    #[test]
    fn test_to_dwg_bytes_non_empty() {
        let mut doc = CadDocument::new();
        doc.add_entity(EntityType::Line(Line::from_coords(
            0.0, 0.0, 0.0, 5.0, 0.0, 0.0,
        )))
        .unwrap();

        let bytes = doc.to_bytes().expect("serialize DWG bytes");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_from_bytes_empty_input_fails() {
        let err = CadDocument::from_bytes(&[]).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.to_lowercase().contains("empty"));
    }

    // ── Mutation event tests ─────────────────────────────────────────

    #[test]
    fn test_document_events_for_add_remove() {
        let mut doc = CadDocument::new();
        let h = doc.add_entity(EntityType::Line(Line::new())).unwrap();

        let events = doc.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].handle, Some(h));
        assert_eq!(events[0].event_type, crate::notification::DocumentEventType::EntityAdded);

        doc.remove_entity(h).unwrap();
        let events = doc.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].handle, Some(h));
        assert_eq!(events[0].event_type, crate::notification::DocumentEventType::EntityRemoved);
    }

    #[test]
    fn test_document_events_for_transforms() {
        let mut doc = CadDocument::new();
        let h = doc
            .add_entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 1.0, 0.0, 0.0)))
            .unwrap();
        doc.clear_events();

        assert!(doc.move_entity(h, Vector3::new(1.0, 2.0, 0.0)));
        assert!(doc.rotate_entity(h, Vector3::ZERO, std::f64::consts::FRAC_PI_2));
        assert!(doc.scale_entity(h, Vector3::ZERO, 2.0));

        let events = doc.drain_events();
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| e.handle == Some(h)));
        assert_eq!(
            events[0].event_type,
            crate::notification::DocumentEventType::EntityModified
        );
        assert_eq!(events[0].message.as_deref(), Some("move"));
        assert_eq!(events[1].message.as_deref(), Some("rotate"));
        assert_eq!(events[2].message.as_deref(), Some("scale"));

        doc.clear_events();
        assert!(doc.events().is_empty());
    }

    // ── Entity distance tests ────────────────────────────────────────

    #[test]
    fn test_distance_between_handles() {
        let mut doc = CadDocument::new();
        let a = doc
            .add_entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 1.0, 0.0, 0.0)))
            .unwrap();
        let b = doc
            .add_entity(EntityType::Line(Line::from_coords(4.0, 0.0, 0.0, 5.0, 0.0, 0.0)))
            .unwrap();

        let d = doc.distance_between_handles(a, b).unwrap();
        assert!((d - 3.0).abs() < 1e-9, "distance was {}", d);
    }

    #[test]
    fn test_distance_between_handles_missing_returns_none() {
        let doc = CadDocument::new();
        assert!(doc
            .distance_between_handles(Handle::new(1), Handle::new(2))
            .is_none());
    }

    #[test]
    fn test_nearest_distance_from() {
        let mut doc = CadDocument::new();
        let a = doc
            .add_entity(EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 1.0, 0.0, 0.0)))
            .unwrap();
        let _b = doc
            .add_entity(EntityType::Line(Line::from_coords(4.0, 0.0, 0.0, 5.0, 0.0, 0.0)))
            .unwrap();
        let _c = doc
            .add_entity(EntityType::Line(Line::from_coords(2.0, 0.0, 0.0, 3.0, 0.0, 0.0)))
            .unwrap();

        let d = doc.nearest_distance_from(a).unwrap();
        assert!((d - 1.0).abs() < 1e-9, "nearest distance was {}", d);
    }
}

