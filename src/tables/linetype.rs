//! Line type table entry

use super::TableEntry;
use crate::types::Handle;

/// Line type element (dash, dot, space)
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineTypeElement {
    /// Length of the element (positive = dash, negative = space, 0 = dot)
    pub length: f64,
}

impl LineTypeElement {
    /// Create a dash element
    pub fn dash(length: f64) -> Self {
        LineTypeElement { length: length.abs() }
    }

    /// Create a space element
    pub fn space(length: f64) -> Self {
        LineTypeElement { length: -length.abs() }
    }

    /// Create a dot element
    pub fn dot() -> Self {
        LineTypeElement { length: 0.0 }
    }

    /// Check if this is a dash
    pub fn is_dash(&self) -> bool {
        self.length > 0.0
    }

    /// Check if this is a space
    pub fn is_space(&self) -> bool {
        self.length < 0.0
    }

    /// Check if this is a dot
    pub fn is_dot(&self) -> bool {
        self.length == 0.0
    }
}

/// A line type table entry
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineType {
    /// Unique handle
    pub handle: Handle,
    /// Line type name
    pub name: String,
    /// Description
    pub description: String,
    /// Pattern elements
    pub elements: Vec<LineTypeElement>,
    /// Total pattern length
    pub pattern_length: f64,
    /// Alignment (always 'A' for AutoCAD)
    pub alignment: char,
    /// Whether this linetype is externally dependent on an xref
    pub xref_dependent: bool,
}

impl LineType {
    /// Create a new line type
    pub fn new(name: impl Into<String>) -> Self {
        LineType {
            handle: Handle::NULL,
            name: name.into(),
            description: String::new(),
            elements: Vec::new(),
            pattern_length: 0.0,
            alignment: 'A',
            xref_dependent: false,
        }
    }

    /// Create the standard "Continuous" line type
    pub fn continuous() -> Self {
        LineType {
            handle: Handle::NULL,
            name: "Continuous".to_string(),
            description: "Solid line".to_string(),
            elements: Vec::new(),
            pattern_length: 0.0,
            alignment: 'A',
            xref_dependent: false,
        }
    }

    /// Create the standard "ByLayer" line type
    pub fn by_layer() -> Self {
        LineType {
            handle: Handle::NULL,
            name: "ByLayer".to_string(),
            description: String::new(),
            elements: Vec::new(),
            pattern_length: 0.0,
            alignment: 'A',
            xref_dependent: false,
        }
    }

    /// Create the standard "ByBlock" line type
    pub fn by_block() -> Self {
        LineType {
            handle: Handle::NULL,
            name: "ByBlock".to_string(),
            description: String::new(),
            elements: Vec::new(),
            pattern_length: 0.0,
            alignment: 'A',
            xref_dependent: false,
        }
    }

    /// Create a dashed line type
    pub fn dashed() -> Self {
        let mut lt = LineType::new("Dashed");
        lt.description = "__ __ __ __ __ __".to_string();
        lt.add_element(LineTypeElement::dash(0.5));
        lt.add_element(LineTypeElement::space(0.25));
        lt.pattern_length = 0.75;
        lt
    }

    /// Create a dotted line type
    pub fn dotted() -> Self {
        let mut lt = LineType::new("Dotted");
        lt.description = ". . . . . . . .".to_string();
        lt.add_element(LineTypeElement::dot());
        lt.add_element(LineTypeElement::space(0.25));
        lt.pattern_length = 0.25;
        lt
    }

    /// Add an element to the pattern
    pub fn add_element(&mut self, element: LineTypeElement) {
        self.elements.push(element);
    }

    /// Get the number of elements
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Check if this is a continuous line type
    pub fn is_continuous(&self) -> bool {
        self.elements.is_empty()
    }

    /// Start a fluent builder for a line type.
    pub fn builder(name: impl Into<String>) -> LineTypeBuilder {
        LineTypeBuilder::new(name)
    }
}

/// Fluent builder for [`LineType`].
#[derive(Debug, Clone)]
pub struct LineTypeBuilder {
    line_type: LineType,
}

impl LineTypeBuilder {
    /// Create a new line type builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            line_type: LineType::new(name),
        }
    }

    /// Set description text.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.line_type.description = description.into();
        self
    }

    /// Add a custom pattern element.
    pub fn element(mut self, element: LineTypeElement) -> Self {
        self.line_type.elements.push(element);
        self
    }

    /// Add a dash element.
    pub fn dash(self, length: f64) -> Self {
        self.element(LineTypeElement::dash(length))
    }

    /// Add a space element.
    pub fn space(self, length: f64) -> Self {
        self.element(LineTypeElement::space(length))
    }

    /// Add a dot element.
    pub fn dot(self) -> Self {
        self.element(LineTypeElement::dot())
    }

    /// Set alignment character (typically 'A').
    pub fn alignment(mut self, alignment: char) -> Self {
        self.line_type.alignment = alignment;
        self
    }

    /// Set xref-dependent flag.
    pub fn xref_dependent(mut self, xref_dependent: bool) -> Self {
        self.line_type.xref_dependent = xref_dependent;
        self
    }

    /// Build the configured line type.
    pub fn build(mut self) -> LineType {
        self.line_type.pattern_length = self
            .line_type
            .elements
            .iter()
            .map(|e| e.length.abs())
            .sum();
        self.line_type
    }
}

impl TableEntry for LineType {
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
        matches!(
            self.name.as_str(),
            "Continuous" | "ByLayer" | "ByBlock"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linetype_builder_sets_pattern_and_metadata() {
        let lt = LineType::builder("Center")
            .description("Center line")
            .dash(1.0)
            .space(0.25)
            .dot()
            .alignment('A')
            .xref_dependent(true)
            .build();

        assert_eq!(lt.name, "Center");
        assert_eq!(lt.description, "Center line");
        assert_eq!(lt.elements.len(), 3);
        assert!((lt.pattern_length - 1.25).abs() < 1e-10);
        assert_eq!(lt.alignment, 'A');
        assert!(lt.xref_dependent);
    }

    #[test]
    fn linetype_builder_defaults_match_new() {
        let a = LineType::new("X");
        let b = LineType::builder("X").build();
        assert_eq!(a, b);
    }
}


