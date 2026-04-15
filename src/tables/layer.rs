//! Layer table entry

use super::TableEntry;
use crate::types::{Color, Handle, LineWeight};

/// Layer flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LayerFlags {
    /// Layer is frozen
    pub frozen: bool,
    /// Layer is locked
    pub locked: bool,
    /// Layer is off (invisible)
    pub off: bool,
    /// Layer is xref-dependent (name contains `|`)
    pub xref_dependent: bool,
}

impl LayerFlags {
    /// Create default layer flags (all false)
    pub fn new() -> Self {
        LayerFlags {
            frozen: false,
            locked: false,
            off: false,
            xref_dependent: false,
        }
    }

    /// Create flags for a standard layer (layer "0")
    pub fn standard() -> Self {
        Self::new()
    }
}

impl Default for LayerFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// A layer table entry
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Layer {
    /// Unique handle
    pub handle: Handle,
    /// Layer name
    pub name: String,
    /// Layer flags
    pub flags: LayerFlags,
    /// Layer color
    pub color: Color,
    /// Line type name
    pub line_type: String,
    /// Line weight
    pub line_weight: LineWeight,
    /// Plot style name
    pub plot_style: String,
    /// Is this layer plottable?
    pub is_plottable: bool,
    /// Material handle
    pub material: Handle,
    /// Plot style handle (R2000+)
    pub plotstyle_handle: Handle,
    /// External reference block record handle (for xref-dependent layers)
    pub xref_block_record_handle: Handle,
}

impl Layer {
    /// Create a new layer with default settings
    pub fn new(name: impl Into<String>) -> Self {
        Layer {
            handle: Handle::NULL,
            name: name.into(),
            flags: LayerFlags::new(),
            color: Color::WHITE,
            line_type: "Continuous".to_string(),
            line_weight: LineWeight::Default,
            plot_style: String::new(),
            is_plottable: true,
            material: Handle::NULL,
            plotstyle_handle: Handle::NULL,
            xref_block_record_handle: Handle::NULL,
        }
    }

    /// Create the standard "0" layer
    pub fn layer_0() -> Self {
        Layer {
            handle: Handle::NULL,
            name: "0".to_string(),
            flags: LayerFlags::standard(),
            color: Color::WHITE,
            line_type: "Continuous".to_string(),
            line_weight: LineWeight::Default,
            plot_style: String::new(),
            is_plottable: true,
            material: Handle::NULL,
            plotstyle_handle: Handle::NULL,
            xref_block_record_handle: Handle::NULL,
        }
    }

    /// Create a layer with a specific color
    pub fn with_color(name: impl Into<String>, color: Color) -> Self {
        Layer {
            color,
            ..Self::new(name)
        }
    }

    /// Set the layer as frozen
    pub fn freeze(&mut self) {
        self.flags.frozen = true;
    }

    /// Set the layer as thawed
    pub fn thaw(&mut self) {
        self.flags.frozen = false;
    }

    /// Check if the layer is frozen
    pub fn is_frozen(&self) -> bool {
        self.flags.frozen
    }

    /// Set the layer as locked
    pub fn lock(&mut self) {
        self.flags.locked = true;
    }

    /// Set the layer as unlocked
    pub fn unlock(&mut self) {
        self.flags.locked = false;
    }

    /// Check if the layer is locked
    pub fn is_locked(&self) -> bool {
        self.flags.locked
    }

    /// Turn the layer off
    pub fn turn_off(&mut self) {
        self.flags.off = true;
    }

    /// Turn the layer on
    pub fn turn_on(&mut self) {
        self.flags.off = false;
    }

    /// Check if the layer is off
    pub fn is_off(&self) -> bool {
        self.flags.off
    }

    /// Check if the layer is visible (not off and not frozen)
    pub fn is_visible(&self) -> bool {
        !self.flags.off && !self.flags.frozen
    }

    /// Start a fluent builder for a layer.
    pub fn builder(name: impl Into<String>) -> LayerBuilder {
        LayerBuilder::new(name)
    }
}

/// Fluent builder for [`Layer`].
#[derive(Debug, Clone)]
pub struct LayerBuilder {
    layer: Layer,
}

impl LayerBuilder {
    /// Create a new layer builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            layer: Layer::new(name),
        }
    }

    /// Set the layer color.
    pub fn color(mut self, color: Color) -> Self {
        self.layer.color = color;
        self
    }

    /// Set the line type name.
    pub fn line_type(mut self, line_type: impl Into<String>) -> Self {
        self.layer.line_type = line_type.into();
        self
    }

    /// Set the line weight.
    pub fn line_weight(mut self, line_weight: LineWeight) -> Self {
        self.layer.line_weight = line_weight;
        self
    }

    /// Set whether this layer is plottable.
    pub fn plottable(mut self, is_plottable: bool) -> Self {
        self.layer.is_plottable = is_plottable;
        self
    }

    /// Set the frozen state.
    pub fn frozen(mut self, frozen: bool) -> Self {
        self.layer.flags.frozen = frozen;
        self
    }

    /// Set the locked state.
    pub fn locked(mut self, locked: bool) -> Self {
        self.layer.flags.locked = locked;
        self
    }

    /// Set the off state.
    pub fn off(mut self, off: bool) -> Self {
        self.layer.flags.off = off;
        self
    }

    /// Set whether the layer is xref-dependent.
    pub fn xref_dependent(mut self, xref_dependent: bool) -> Self {
        self.layer.flags.xref_dependent = xref_dependent;
        self
    }

    /// Set the plot style name.
    pub fn plot_style(mut self, plot_style: impl Into<String>) -> Self {
        self.layer.plot_style = plot_style.into();
        self
    }

    /// Set the material handle.
    pub fn material(mut self, material: Handle) -> Self {
        self.layer.material = material;
        self
    }

    /// Build the configured layer.
    pub fn build(self) -> Layer {
        self.layer
    }
}

impl TableEntry for Layer {
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
        self.name == "0"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_builder_sets_common_properties() {
        let layer = Layer::builder("Walls")
            .color(Color::RED)
            .line_type("Hidden")
            .line_weight(LineWeight::W0_50)
            .plottable(false)
            .frozen(true)
            .locked(true)
            .off(true)
            .xref_dependent(true)
            .plot_style("ByLayer")
            .material(Handle::new(42))
            .build();

        assert_eq!(layer.name, "Walls");
        assert_eq!(layer.color, Color::RED);
        assert_eq!(layer.line_type, "Hidden");
        assert_eq!(layer.line_weight, LineWeight::W0_50);
        assert!(!layer.is_plottable);
        assert!(layer.flags.frozen);
        assert!(layer.flags.locked);
        assert!(layer.flags.off);
        assert!(layer.flags.xref_dependent);
        assert_eq!(layer.plot_style, "ByLayer");
        assert_eq!(layer.material, Handle::new(42));
    }

    #[test]
    fn layer_builder_defaults_match_layer_new() {
        let a = Layer::new("A");
        let b = Layer::builder("A").build();
        assert_eq!(a, b);
    }
}


