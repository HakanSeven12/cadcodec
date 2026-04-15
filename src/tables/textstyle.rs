//! Text style table entry

use super::TableEntry;
use crate::types::Handle;

/// Text generation flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextGenerationFlags {
    /// Text is backward (mirrored in X)
    pub backward: bool,
    /// Text is upside down (mirrored in Y)
    pub upside_down: bool,
}

impl TextGenerationFlags {
    /// Create default flags
    pub fn new() -> Self {
        TextGenerationFlags {
            backward: false,
            upside_down: false,
        }
    }
}

impl Default for TextGenerationFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// A text style table entry
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextStyle {
    /// Unique handle
    pub handle: Handle,
    /// Style name
    pub name: String,
    /// Text generation flags
    pub flags: TextGenerationFlags,
    /// Fixed text height (0 = variable)
    pub height: f64,
    /// Width factor
    pub width_factor: f64,
    /// Oblique angle in radians
    pub oblique_angle: f64,
    /// Last height used (code 42, default 2.5)
    pub last_height: f64,
    /// Primary font file name
    pub font_file: String,
    /// Big font file name (for Asian languages)
    pub big_font_file: String,
    /// True Type font name
    pub true_type_font: String,
    /// Whether this style is xref-dependent
    pub xref_dependent: bool,
}

impl TextStyle {
    /// Create a new text style
    pub fn new(name: impl Into<String>) -> Self {
        TextStyle {
            handle: Handle::NULL,
            name: name.into(),
            flags: TextGenerationFlags::new(),
            height: 0.0,
            width_factor: 1.0,
            oblique_angle: 0.0,
            last_height: 2.5,
            font_file: "txt".to_string(),
            big_font_file: String::new(),
            true_type_font: String::new(),
            xref_dependent: false,
        }
    }

    /// Create the standard "Standard" text style
    pub fn standard() -> Self {
        TextStyle {
            handle: Handle::NULL,
            name: "Standard".to_string(),
            flags: TextGenerationFlags::new(),
            height: 0.0,
            width_factor: 1.0,
            oblique_angle: 0.0,
            last_height: 2.5,
            font_file: "txt".to_string(),
            big_font_file: String::new(),
            true_type_font: String::new(),
            xref_dependent: false,
        }
    }

    /// Create a text style with a TrueType font
    pub fn with_truetype(name: impl Into<String>, font: impl Into<String>) -> Self {
        TextStyle {
            true_type_font: font.into(),
            ..Self::new(name)
        }
    }

    /// Get the effective last height (returns last_height, or default 2.5 if 0)
    pub fn effective_last_height(&self) -> f64 {
        if self.last_height > 0.0 {
            self.last_height
        } else {
            2.5
        }
    }

    /// Set the text as backward (mirrored in X)
    pub fn set_backward(&mut self, backward: bool) {
        self.flags.backward = backward;
    }

    /// Set the text as upside down (mirrored in Y)
    pub fn set_upside_down(&mut self, upside_down: bool) {
        self.flags.upside_down = upside_down;
    }

    /// Check if text is backward
    pub fn is_backward(&self) -> bool {
        self.flags.backward
    }

    /// Check if text is upside down
    pub fn is_upside_down(&self) -> bool {
        self.flags.upside_down
    }

    /// Check if this style has a fixed height
    pub fn has_fixed_height(&self) -> bool {
        self.height > 0.0
    }

    /// Start a fluent builder for a text style.
    pub fn builder(name: impl Into<String>) -> TextStyleBuilder {
        TextStyleBuilder::new(name)
    }
}

/// Fluent builder for [`TextStyle`].
#[derive(Debug, Clone)]
pub struct TextStyleBuilder {
    style: TextStyle,
}

impl TextStyleBuilder {
    /// Create a new text style builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            style: TextStyle::new(name),
        }
    }

    /// Set the primary shape/font file name.
    pub fn font_file(mut self, font_file: impl Into<String>) -> Self {
        self.style.font_file = font_file.into();
        self
    }

    /// Set the TrueType font name.
    pub fn truetype(mut self, true_type_font: impl Into<String>) -> Self {
        self.style.true_type_font = true_type_font.into();
        self
    }

    /// Set the big font file name.
    pub fn big_font(mut self, big_font_file: impl Into<String>) -> Self {
        self.style.big_font_file = big_font_file.into();
        self
    }

    /// Set fixed text height.
    pub fn height(mut self, height: f64) -> Self {
        self.style.height = height;
        self
    }

    /// Set width factor.
    pub fn width_factor(mut self, width_factor: f64) -> Self {
        self.style.width_factor = width_factor;
        self
    }

    /// Set oblique angle in radians.
    pub fn oblique_angle(mut self, oblique_angle: f64) -> Self {
        self.style.oblique_angle = oblique_angle;
        self
    }

    /// Set last used text height.
    pub fn last_height(mut self, last_height: f64) -> Self {
        self.style.last_height = last_height;
        self
    }

    /// Set mirrored-in-X generation flag.
    pub fn backward(mut self, backward: bool) -> Self {
        self.style.flags.backward = backward;
        self
    }

    /// Set mirrored-in-Y generation flag.
    pub fn upside_down(mut self, upside_down: bool) -> Self {
        self.style.flags.upside_down = upside_down;
        self
    }

    /// Set xref-dependent flag.
    pub fn xref_dependent(mut self, xref_dependent: bool) -> Self {
        self.style.xref_dependent = xref_dependent;
        self
    }

    /// Build the configured text style.
    pub fn build(self) -> TextStyle {
        self.style
    }
}

impl TableEntry for TextStyle {
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
    fn test_textstyle_creation() {
        let style = TextStyle::new("MyStyle");
        assert_eq!(style.name, "MyStyle");
        assert_eq!(style.width_factor, 1.0);
        assert!(!style.has_fixed_height());
    }

    #[test]
    fn test_textstyle_standard() {
        let style = TextStyle::standard();
        assert_eq!(style.name, "Standard");
        assert!(style.is_standard());
    }

    #[test]
    fn test_textstyle_flags() {
        let mut style = TextStyle::new("Test");
        assert!(!style.is_backward());
        assert!(!style.is_upside_down());
        
        style.set_backward(true);
        assert!(style.is_backward());
        
        style.set_upside_down(true);
        assert!(style.is_upside_down());
    }

    #[test]
    fn test_textstyle_builder_sets_properties() {
        let style = TextStyle::builder("Anno")
            .font_file("romans.shx")
            .truetype("Arial")
            .big_font("gbcbig.shx")
            .height(2.5)
            .width_factor(0.8)
            .oblique_angle(0.2)
            .last_height(3.0)
            .backward(true)
            .upside_down(true)
            .xref_dependent(true)
            .build();

        assert_eq!(style.name, "Anno");
        assert_eq!(style.font_file, "romans.shx");
        assert_eq!(style.true_type_font, "Arial");
        assert_eq!(style.big_font_file, "gbcbig.shx");
        assert_eq!(style.height, 2.5);
        assert_eq!(style.width_factor, 0.8);
        assert_eq!(style.oblique_angle, 0.2);
        assert_eq!(style.last_height, 3.0);
        assert!(style.flags.backward);
        assert!(style.flags.upside_down);
        assert!(style.xref_dependent);
    }

    #[test]
    fn test_textstyle_builder_defaults_match_new() {
        let a = TextStyle::new("A");
        let b = TextStyle::builder("A").build();
        assert_eq!(a, b);
    }
}


