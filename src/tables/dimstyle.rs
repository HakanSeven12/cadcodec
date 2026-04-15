//! Dimension style table entry

use super::TableEntry;
use crate::types::Handle;

/// A dimension style table entry — maps to ACadSharp's DimensionStyle
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DimStyle {
    /// Unique handle
    pub handle: Handle,
    /// Style name
    pub name: String,

    // ─── Dimension line ───
    /// Dimension line color (DIMCLRD, code 176)
    pub dimclrd: i16,
    /// Dimension line extension (DIMDLE, code 46)
    pub dimdle: f64,
    /// Dimension line increment for continuation (DIMDLI, code 43)
    pub dimdli: f64,
    /// Dimension line gap (DIMGAP, code 147)
    pub dimgap: f64,
    /// Dimension line weight (DIMLWD, code 371)
    pub dimlwd: i16,
    /// Suppress first dimension line (DIMSD1, code 281)
    pub dimsd1: bool,
    /// Suppress second dimension line (DIMSD2, code 282)
    pub dimsd2: bool,

    // ─── Extension line ───
    /// Extension line color (DIMCLRE, code 177)
    pub dimclre: i16,
    /// Extension line extension (DIMEXE, code 44)
    pub dimexe: f64,
    /// Extension line offset (DIMEXO, code 42)
    pub dimexo: f64,
    /// Extension line weight (DIMLWE, code 372)
    pub dimlwe: i16,
    /// Suppress first extension line (DIMSE1, code 75)
    pub dimse1: bool,
    /// Suppress second extension line (DIMSE2, code 76)
    pub dimse2: bool,
    /// Fixed extension line length (DIMFXL, code 49)
    pub dimfxl: f64,
    /// Fixed extension line length on (DIMFXLON, code 290)
    pub dimfxlon: bool,

    // ─── Arrows ───
    /// Arrow size (DIMASZ, code 41)
    pub dimasz: f64,
    /// Arrow block handle (DIMBLK, code 342)
    pub dimblk: Handle,
    /// First arrow block handle (DIMBLK1, code 343)
    pub dimblk1: Handle,
    /// Second arrow block handle (DIMBLK2, code 344)
    pub dimblk2: Handle,
    /// Leader arrow block handle (DIMLDRBLK, code 341)
    pub dimldrblk: Handle,
    /// Separate arrow blocks (DIMSAH, code 173)
    pub dimsah: bool,
    /// Center mark size (DIMCEN, code 141)
    pub dimcen: f64,
    /// Tick size (DIMTSZ, code 142)
    pub dimtsz: f64,

    // ─── Text ───
    /// Dimension text color (DIMCLRT, code 178)
    pub dimclrt: i16,
    /// Text height (DIMTXT, code 140)
    pub dimtxt: f64,
    /// Text horizontal alignment / justification (DIMJUST, code 280)
    pub dimjust: i16,
    /// Text vertical alignment (DIMTAD, code 77)
    pub dimtad: i16,
    /// Text vertical position (DIMTVP, code 145)
    pub dimtvp: f64,
    /// Text inside horizontal (DIMTIH, code 73)
    pub dimtih: bool,
    /// Text outside horizontal (DIMTOH, code 74)
    pub dimtoh: bool,
    /// Text inside extensions (DIMTIX, code 174)
    pub dimtix: bool,
    /// Suppress outside extensions (DIMSOXD, code 175)
    pub dimsoxd: bool,
    /// Text background fill mode (DIMTFILL, code 69)
    pub dimtfill: i16,
    /// Text background fill color (DIMTFILLCLR, code 70)
    pub dimtfillclr: i16,
    /// Text movement (DIMTMOVE, code 279)
    pub dimtmove: i16,
    /// Text direction (DIMTXTDIRECTION, code 295)
    pub dimtxtdirection: bool,
    /// Text style handle (DIMTXSTY, code 340)
    pub dimtxsty_handle: Handle,
    /// Text style name
    pub dimtxsty: String,

    // ─── Scale / units ───
    /// Dimension scale factor (DIMSCALE, code 40)
    pub dimscale: f64,
    /// Linear scale factor (DIMLFAC, code 144)
    pub dimlfac: f64,
    /// Linear unit format (DIMLUNIT, code 277)
    pub dimlunit: i16,
    /// Decimal places (DIMDEC, code 271)
    pub dimdec: i16,
    /// Rounding (DIMRND, code 45)
    pub dimrnd: f64,
    /// Decimal separator (DIMDSEP, code 278)
    pub dimdsep: i16,
    /// Zero handling (DIMZIN, code 78)
    pub dimzin: i16,

    // ─── Angular ───
    /// Angular unit format (DIMAUNIT, code 275)
    pub dimaunit: i16,
    /// Angular decimal places (DIMADEC, code 179)
    pub dimadec: i16,
    /// Angular zero handling (DIMAZIN, code 79)
    pub dimazin: i16,

    // ─── Alternate units ───
    /// Alternate unit dimensioning on (DIMALT, code 170)
    pub dimalt: bool,
    /// Alternate unit scale factor (DIMALTF, code 143)
    pub dimaltf: f64,
    /// Alternate unit decimal places (DIMALTD, code 171)
    pub dimaltd: i16,
    /// Alternate unit format (DIMALTU, code 273)
    pub dimaltu: i16,
    /// Alternate unit tolerance decimal places (DIMALTTD, code 274)
    pub dimalttd: i16,
    /// Alternate unit rounding (DIMALTRND, code 148)
    pub dimaltrnd: f64,
    /// Alternate unit suffix (DIMAPOST, code 4)
    pub dimapost: String,
    /// Alternate unit zero handling (DIMALTZ, code 285)
    pub dimaltz: i16,
    /// Alternate unit tolerance zero handling (DIMALTTZ, code 286)
    pub dimalttz: i16,

    // ─── Tolerances ───
    /// Generate tolerances (DIMTOL, code 71)
    pub dimtol: bool,
    /// Limits generation (DIMLIM, code 72)
    pub dimlim: bool,
    /// Plus tolerance (DIMTP, code 47)
    pub dimtp: f64,
    /// Minus tolerance (DIMTM, code 48)
    pub dimtm: f64,
    /// Tolerance decimal places (DIMTDEC, code 272)
    pub dimtdec: i16,
    /// Tolerance scale factor (DIMTFAC, code 146)
    pub dimtfac: f64,
    /// Tolerance alignment (DIMTOLJ, code 283)
    pub dimtolj: i16,
    /// Tolerance zero handling (DIMTZIN, code 284)
    pub dimtzin: i16,

    // ─── Fit ───
    /// Text/arrow fit (DIMATFIT, code 289)
    pub dimatfit: i16,
    /// Text outside flow line (DIMTOFL, code 172)
    pub dimtofl: bool,
    /// Cursor update (DIMUPT, code 288)
    pub dimupt: bool,
    /// Dimension fit (DIMFIT, code 287) — obsolete 
    pub dimfit: i16,

    // ─── Formatting ───
    /// Postfix (DIMPOST, code 3)
    pub dimpost: String,
    /// Fraction format (DIMFRAC, code 276)
    pub dimfrac: i16,
    /// Arc length symbol position (DIMARCSYM, code 90)
    pub dimarcsym: i16,
    /// Jogged radius angle (DIMJOGANG, code 50)
    pub dimjogang: f64,

    // ─── Linetype handles ───
    /// Dimension line linetype handle (code 345)
    pub dimltex_handle: Handle,
    /// Extension line 1 linetype handle (code 346)
    pub dimltex1_handle: Handle,
    /// Extension line 2 linetype handle (code 347)
    pub dimltex2_handle: Handle,

    /// Obsolete DIMUNIT (code 270)
    pub dimunit: i16,
}

impl DimStyle {
    /// Create a new dimension style
    pub fn new(name: impl Into<String>) -> Self {
        DimStyle {
            handle: Handle::NULL,
            name: name.into(),
            // Dimension line
            dimclrd: 0,
            dimdle: 0.0,
            dimdli: 3.75,
            dimgap: 0.625,
            dimlwd: -2, // ByBlock
            dimsd1: false,
            dimsd2: false,
            // Extension line
            dimclre: 0,
            dimexe: 1.25,
            dimexo: 0.625,
            dimlwe: -2, // ByBlock
            dimse1: false,
            dimse2: false,
            dimfxl: 1.0,
            dimfxlon: false,
            // Arrows
            dimasz: 0.18,
            dimblk: Handle::NULL,
            dimblk1: Handle::NULL,
            dimblk2: Handle::NULL,
            dimldrblk: Handle::NULL,
            dimsah: true,
            dimcen: 0.09,
            dimtsz: 0.0,
            // Text
            dimclrt: 0,
            dimtxt: 0.18,
            dimjust: 0,
            dimtad: 1,    // Above
            dimtvp: 0.0,
            dimtih: false,
            dimtoh: false,
            dimtix: false,
            dimsoxd: false,
            dimtfill: 0,
            dimtfillclr: 0,
            dimtmove: 0,
            dimtxtdirection: false,
            dimtxsty_handle: Handle::NULL,
            dimtxsty: "Standard".to_string(),
            // Scale/units
            dimscale: 1.0,
            dimlfac: 1.0,
            dimlunit: 2,
            dimdec: 2,
            dimrnd: 0.0,
            dimdsep: 46, // '.'
            dimzin: 8,
            // Angular
            dimaunit: 0,
            dimadec: 0,
            dimazin: 0,
            // Alternate units
            dimalt: false,
            dimaltf: 25.4,
            dimaltd: 3,
            dimaltu: 2,
            dimalttd: 3,
            dimaltrnd: 0.0,
            dimapost: String::new(),
            dimaltz: 0,
            dimalttz: 0,
            // Tolerances
            dimtol: false,
            dimlim: false,
            dimtp: 0.0,
            dimtm: 0.0,
            dimtdec: 2,
            dimtfac: 1.0,
            dimtolj: 0,
            dimtzin: 8,
            // Fit
            dimatfit: 3,
            dimtofl: false,
            dimupt: false,
            dimfit: 0,
            // Formatting
            dimpost: "<>".to_string(),
            dimfrac: 0,
            dimarcsym: 0,
            dimjogang: std::f64::consts::FRAC_PI_4,
            // Linetype handles
            dimltex_handle: Handle::NULL,
            dimltex1_handle: Handle::NULL,
            dimltex2_handle: Handle::NULL,
            // Obsolete
            dimunit: 2,
        }
    }

    /// Create the standard dimension style
    pub fn standard() -> Self {
        Self::new("Standard")
    }

    /// Start a fluent builder for a dimension style.
    pub fn builder(name: impl Into<String>) -> DimStyleBuilder {
        DimStyleBuilder::new(name)
    }
}

/// Fluent builder for [`DimStyle`].
#[derive(Debug, Clone)]
pub struct DimStyleBuilder {
    style: DimStyle,
}

impl DimStyleBuilder {
    /// Create a new dimension style builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            style: DimStyle::new(name),
        }
    }

    /// Set dimension line color.
    pub fn dim_line_color(mut self, color: i16) -> Self {
        self.style.dimclrd = color;
        self
    }

    /// Set extension line color.
    pub fn ext_line_color(mut self, color: i16) -> Self {
        self.style.dimclre = color;
        self
    }

    /// Set dimension text color.
    pub fn text_color(mut self, color: i16) -> Self {
        self.style.dimclrt = color;
        self
    }

    /// Set dimension text height.
    pub fn text_height(mut self, height: f64) -> Self {
        self.style.dimtxt = height;
        self
    }

    /// Set arrow size.
    pub fn arrow_size(mut self, size: f64) -> Self {
        self.style.dimasz = size;
        self
    }

    /// Set overall dimension scale.
    pub fn scale(mut self, scale: f64) -> Self {
        self.style.dimscale = scale;
        self
    }

    /// Set linear measurement scale factor.
    pub fn linear_scale(mut self, factor: f64) -> Self {
        self.style.dimlfac = factor;
        self
    }

    /// Set linear unit format code.
    pub fn unit_format(mut self, format: i16) -> Self {
        self.style.dimlunit = format;
        self
    }

    /// Set decimal place precision.
    pub fn decimal_places(mut self, places: i16) -> Self {
        self.style.dimdec = places;
        self
    }

    /// Set text style by name.
    pub fn text_style(mut self, style_name: impl Into<String>) -> Self {
        self.style.dimtxsty = style_name.into();
        self
    }

    /// Set text style handle.
    pub fn text_style_handle(mut self, handle: Handle) -> Self {
        self.style.dimtxsty_handle = handle;
        self
    }

    /// Set tolerance mode and values.
    pub fn tolerance(mut self, enabled: bool, plus: f64, minus: f64) -> Self {
        self.style.dimtol = enabled;
        self.style.dimtp = plus;
        self.style.dimtm = minus;
        self
    }

    /// Set limits mode.
    pub fn limits(mut self, enabled: bool) -> Self {
        self.style.dimlim = enabled;
        self
    }

    /// Configure alternate units.
    pub fn alternate_units(mut self, enabled: bool, factor: f64, decimals: i16) -> Self {
        self.style.dimalt = enabled;
        self.style.dimaltf = factor;
        self.style.dimaltd = decimals;
        self
    }

    /// Set fit mode value (`DIMATFIT`).
    pub fn fit_mode(mut self, fit_mode: i16) -> Self {
        self.style.dimatfit = fit_mode;
        self
    }

    /// Build the configured dimension style.
    pub fn build(self) -> DimStyle {
        self.style
    }
}

impl TableEntry for DimStyle {
    fn handle(&self) -> Handle {
        self.handle
    }

    fn set_handle(&mut self, handle: Handle) {
        self.handle = handle;
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn is_standard(&self) -> bool {
        self.name == "Standard"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimstyle_builder_sets_common_fields() {
        let style = DimStyle::builder("ISO-25")
            .dim_line_color(1)
            .ext_line_color(2)
            .text_color(3)
            .text_height(2.5)
            .arrow_size(1.25)
            .scale(2.0)
            .linear_scale(0.5)
            .unit_format(4)
            .decimal_places(4)
            .text_style("Annotative")
            .text_style_handle(Handle::new(77))
            .tolerance(true, 0.02, 0.01)
            .limits(true)
            .alternate_units(true, 25.4, 2)
            .fit_mode(1)
            .build();

        assert_eq!(style.name, "ISO-25");
        assert_eq!(style.dimclrd, 1);
        assert_eq!(style.dimclre, 2);
        assert_eq!(style.dimclrt, 3);
        assert_eq!(style.dimtxt, 2.5);
        assert_eq!(style.dimasz, 1.25);
        assert_eq!(style.dimscale, 2.0);
        assert_eq!(style.dimlfac, 0.5);
        assert_eq!(style.dimlunit, 4);
        assert_eq!(style.dimdec, 4);
        assert_eq!(style.dimtxsty, "Annotative");
        assert_eq!(style.dimtxsty_handle, Handle::new(77));
        assert!(style.dimtol);
        assert_eq!(style.dimtp, 0.02);
        assert_eq!(style.dimtm, 0.01);
        assert!(style.dimlim);
        assert!(style.dimalt);
        assert_eq!(style.dimaltf, 25.4);
        assert_eq!(style.dimaltd, 2);
        assert_eq!(style.dimatfit, 1);
    }

    #[test]
    fn dimstyle_builder_defaults_match_new() {
        let a = DimStyle::new("X");
        let b = DimStyle::builder("X").build();
        assert_eq!(a, b);
    }
}


