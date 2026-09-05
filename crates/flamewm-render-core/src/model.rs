use std::collections::{HashMap, HashSet};

pub const FORMAT_MAGIC: [u8; 4] = *b"RWRB";
pub const FORMAT_VERSION: u16 = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorValue {
    Literal(Color),
    Variable(u16),
}

impl Default for ColorValue {
    fn default() -> Self {
        Self::Literal(Color::TRANSPARENT)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Length {
    Auto,
    Px(f32),
    Percent(f32),
}

impl Default for Length {
    fn default() -> Self {
        Self::Auto
    }
}

impl Length {
    pub fn resolve(self, available: f32) -> Option<f32> {
        match self {
            Self::Auto => None,
            Self::Px(value) => Some(value.max(0.0)),
            Self::Percent(value) => Some((available * value).max(0.0)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    pub fn horizontal(self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(self) -> f32 {
        self.top + self.bottom
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Display {
    None,
    #[default]
    Block,
    Flex,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlignItems {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Flow,
    Absolute,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CursorKind {
    #[default]
    Default,
    Pointer,
    Text,
    Move,
    ResizeHorizontal,
    ResizeVertical,
    ResizeNorthWestSouthEast,
    ResizeNorthEastSouthWest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub display: Display,
    pub flex_direction: FlexDirection,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub position: Position,
    pub width: Length,
    pub height: Length,
    pub min_width: Length,
    pub min_height: Length,
    pub max_width: Length,
    pub max_height: Length,
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
    pub margin: Edges,
    pub padding: Edges,
    pub gap: f32,
    pub flex_grow: f32,
    pub background: ColorValue,
    pub color: ColorValue,
    pub border_color: ColorValue,
    pub border_width: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub font_weight: u16,
    pub opacity: f32,
    pub cursor: CursorKind,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            display: Display::Block,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            position: Position::Flow,
            width: Length::Auto,
            height: Length::Auto,
            min_width: Length::Auto,
            min_height: Length::Auto,
            max_width: Length::Auto,
            max_height: Length::Auto,
            top: Length::Auto,
            right: Length::Auto,
            bottom: Length::Auto,
            left: Length::Auto,
            margin: Edges::ZERO,
            padding: Edges::ZERO,
            gap: 0.0,
            flex_grow: 0.0,
            background: ColorValue::Literal(Color::TRANSPARENT),
            color: ColorValue::Literal(Color::BLACK),
            border_color: ColorValue::Literal(Color::TRANSPARENT),
            border_width: 0.0,
            border_radius: 0.0,
            font_size: 14.0,
            font_weight: 400,
            opacity: 1.0,
            cursor: CursorKind::Default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Element,
    Text,
    Image,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledNode {
    pub kind: NodeKind,
    pub parent: Option<u32>,
    pub first_child: Option<u32>,
    pub next_sibling: Option<u32>,
    pub id: String,
    pub action: String,
    pub text: String,
    pub image: Option<u16>,
    pub style: Style,
    pub hover_style: Option<Style>,
    pub active_style: Option<Style>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorVariable {
    pub name: String,
    pub default: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageAsset {
    pub source: String,
    pub width: u32,
    pub height: u32,
    /// Opaque RGB8 pixels in row-major order. Rasterization is a build-time concern.
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledDocument {
    pub source_fingerprint: u64,
    pub root: u32,
    pub variables: Vec<ColorVariable>,
    pub assets: Vec<ImageAsset>,
    pub nodes: Vec<CompiledNode>,
}

impl CompiledDocument {
    pub fn validate(&self) -> Result<(), String> {
        if self.nodes.is_empty() {
            return Err("compiled document has no nodes".to_string());
        }
        if self.root as usize >= self.nodes.len() {
            return Err("root index is outside node table".to_string());
        }
        if self.assets.len() > u16::MAX as usize {
            return Err("compiled document has too many image assets".to_string());
        }
        for (index, asset) in self.assets.iter().enumerate() {
            if asset.width == 0 || asset.height == 0 {
                return Err(format!("image asset {index} has zero dimensions"));
            }
            let expected = asset
                .width
                .checked_mul(asset.height)
                .and_then(|pixels| pixels.checked_mul(3))
                .ok_or_else(|| format!("image asset {index} dimensions overflow"))?
                as usize;
            if asset.pixels.len() != expected {
                return Err(format!(
                    "image asset {index} has {} bytes; expected {expected}",
                    asset.pixels.len()
                ));
            }
        }
        for (index, node) in self.nodes.iter().enumerate() {
            for (field, link) in [
                ("parent", node.parent),
                ("first_child", node.first_child),
                ("next_sibling", node.next_sibling),
            ] {
                if let Some(link) = link {
                    if link as usize >= self.nodes.len() {
                        return Err(format!("node {index} has invalid {field} index {link}"));
                    }
                }
            }
            if let Some(asset) = node.image {
                if asset as usize >= self.assets.len() {
                    return Err(format!(
                        "node {index} references invalid image asset {asset}"
                    ));
                }
                if node.kind != NodeKind::Image {
                    return Err(format!(
                        "node {index} has image data but is not an image node"
                    ));
                }
            }
            if node.kind == NodeKind::Image && node.image.is_none() {
                return Err(format!("image node {index} has no image asset"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeometryOverride {
    pub left: Option<f32>,
    pub top: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RuntimeStyleOverride {
    pub background: Option<ColorValue>,
    pub border_color: Option<ColorValue>,
    pub opacity: Option<f32>,
    pub flex_direction: Option<FlexDirection>,
    pub font_size: Option<f32>,
    pub font_weight: Option<u16>,
}

#[derive(Clone, Debug)]
pub struct RuntimeDocument {
    pub document: CompiledDocument,
    pub variable_values: Vec<Color>,
    pub text_overrides: HashMap<u32, String>,
    hidden_nodes: HashSet<u32>,
    geometry_overrides: HashMap<u32, GeometryOverride>,
    style_overrides: HashMap<u32, RuntimeStyleOverride>,
    z_index_overrides: HashMap<u32, i32>,
    id_index: HashMap<String, u32>,
    variable_index: HashMap<String, u16>,
    ui_font_family: String,
    ui_font_size_offset: f32,
    ui_font_bold: bool,
    ui_scale: f32,
}

impl RuntimeDocument {
    pub fn new(document: CompiledDocument) -> Result<Self, String> {
        document.validate()?;
        let mut id_index = HashMap::new();
        for (index, node) in document.nodes.iter().enumerate() {
            if !node.id.is_empty() {
                if id_index.insert(node.id.clone(), index as u32).is_some() {
                    return Err(format!("duplicate runtime id '{}'", node.id));
                }
            }
        }
        let mut variable_index = HashMap::new();
        let mut variable_values = Vec::with_capacity(document.variables.len());
        for (index, variable) in document.variables.iter().enumerate() {
            variable_index.insert(variable.name.clone(), index as u16);
            variable_values.push(variable.default);
        }
        Ok(Self {
            document,
            variable_values,
            text_overrides: HashMap::new(),
            hidden_nodes: HashSet::new(),
            geometry_overrides: HashMap::new(),
            style_overrides: HashMap::new(),
            z_index_overrides: HashMap::new(),
            id_index,
            variable_index,
            ui_font_family: "IBM Plex Sans".to_string(),
            ui_font_size_offset: 0.0,
            ui_font_bold: false,
            ui_scale: 1.0,
        })
    }

    pub fn node_by_id(&self, id: &str) -> Option<u32> {
        self.id_index.get(id).copied()
    }

    pub fn set_text(&mut self, id: &str, text: impl Into<String>) -> Result<(), String> {
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        let target = if self.document.nodes[index as usize].kind == NodeKind::Text {
            index
        } else {
            let first = self.document.nodes[index as usize]
                .first_child
                .ok_or_else(|| format!("node '{id}' has no text child"))?;
            let child = &self.document.nodes[first as usize];
            if child.kind != NodeKind::Text || child.next_sibling.is_some() {
                return Err(format!("node '{id}' is not a single-text-node element"));
            }
            first
        };
        self.text_overrides.insert(target, text.into());
        Ok(())
    }

    pub fn set_visible(&mut self, id: &str, visible: bool) -> Result<(), String> {
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        if visible {
            self.hidden_nodes.remove(&index);
        } else {
            self.hidden_nodes.insert(index);
        }
        Ok(())
    }

    pub fn is_visible(&self, index: u32) -> bool {
        !self.hidden_nodes.contains(&index)
    }

    pub fn is_effectively_visible(&self, mut index: u32) -> bool {
        loop {
            if !self.is_visible(index) {
                return false;
            }
            let Some(parent) = self.document.nodes[index as usize].parent else {
                return true;
            };
            index = parent;
        }
    }

    pub fn set_color_variable(&mut self, name: &str, color: Color) -> Result<(), String> {
        let index = self
            .variable_index
            .get(name)
            .copied()
            .ok_or_else(|| format!("no color variable named '{name}'"))?;
        self.variable_values[index as usize] = color;
        Ok(())
    }

    pub fn set_position_px(&mut self, id: &str, left: f32, top: f32) -> Result<(), String> {
        if !left.is_finite() || !top.is_finite() {
            return Err("runtime position must be finite".to_string());
        }
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        let entry = self.geometry_overrides.entry(index).or_default();
        entry.left = Some(left);
        entry.top = Some(top);
        Ok(())
    }

    pub fn set_size_px(&mut self, id: &str, width: f32, height: f32) -> Result<(), String> {
        if !width.is_finite() || !height.is_finite() || width < 0.0 || height < 0.0 {
            return Err("runtime size must be finite and non-negative".to_string());
        }
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        let entry = self.geometry_overrides.entry(index).or_default();
        entry.width = Some(width);
        entry.height = Some(height);
        Ok(())
    }

    pub fn clear_geometry_override(&mut self, id: &str) -> Result<(), String> {
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        self.geometry_overrides.remove(&index);
        Ok(())
    }

    pub fn geometry_override(&self, index: u32) -> Option<GeometryOverride> {
        self.geometry_overrides.get(&index).copied()
    }

    pub fn set_background_color(&mut self, id: &str, color: Color) -> Result<(), String> {
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        self.style_overrides.entry(index).or_default().background =
            Some(ColorValue::Literal(color));
        Ok(())
    }

    pub fn set_border_color(&mut self, id: &str, color: Color) -> Result<(), String> {
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        self.style_overrides.entry(index).or_default().border_color =
            Some(ColorValue::Literal(color));
        Ok(())
    }

    pub fn clear_background_color(&mut self, id: &str) -> Result<(), String> {
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        if let Some(override_style) = self.style_overrides.get_mut(&index) {
            override_style.background = None;
        }
        Ok(())
    }

    pub fn clear_border_color(&mut self, id: &str) -> Result<(), String> {
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        if let Some(override_style) = self.style_overrides.get_mut(&index) {
            override_style.border_color = None;
        }
        Ok(())
    }

    pub fn set_opacity(&mut self, id: &str, opacity: f32) -> Result<(), String> {
        if !opacity.is_finite() {
            return Err("runtime opacity must be finite".to_string());
        }
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        self.style_overrides.entry(index).or_default().opacity = Some(opacity.clamp(0.0, 1.0));
        Ok(())
    }

    pub fn set_flex_direction(&mut self, id: &str, direction: FlexDirection) -> Result<(), String> {
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        self.style_overrides
            .entry(index)
            .or_default()
            .flex_direction = Some(direction);
        Ok(())
    }

    pub fn set_font_size_px(&mut self, id: &str, size: f32) -> Result<(), String> {
        if !size.is_finite() || size <= 0.0 {
            return Err("runtime font size must be finite and positive".to_string());
        }
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        self.style_overrides.entry(index).or_default().font_size = Some(size);
        Ok(())
    }

    pub fn set_font_weight(&mut self, id: &str, weight: u16) -> Result<(), String> {
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        self.style_overrides.entry(index).or_default().font_weight = Some(weight.clamp(100, 900));
        Ok(())
    }

    pub fn set_ui_font(
        &mut self,
        family: impl Into<String>,
        size_offset: f32,
        bold: bool,
    ) -> Result<(), String> {
        let family = family.into();
        if family.trim().is_empty() {
            return Err("UI font family cannot be empty".to_string());
        }
        if !size_offset.is_finite() {
            return Err("UI font size offset must be finite".to_string());
        }
        self.ui_font_family = family;
        self.ui_font_size_offset = size_offset.clamp(-3.0, 6.0);
        self.ui_font_bold = bold;
        Ok(())
    }

    pub fn ui_font_family(&self) -> &str {
        &self.ui_font_family
    }

    pub fn set_ui_scale(&mut self, scale: f32) -> Result<(), String> {
        if !scale.is_finite() || scale <= 0.0 {
            return Err("UI scale must be finite and positive".to_string());
        }
        self.ui_scale = scale.clamp(0.5, 4.0);
        Ok(())
    }

    pub fn ui_scale(&self) -> f32 {
        self.ui_scale
    }

    pub fn set_z_index(&mut self, id: &str, z: i32) -> Result<(), String> {
        let index = self
            .node_by_id(id)
            .ok_or_else(|| format!("no node with id '{id}'"))?;
        self.z_index_overrides.insert(index, z);
        Ok(())
    }

    pub fn effective_z_index(&self, mut index: u32) -> i32 {
        loop {
            if let Some(z) = self.z_index_overrides.get(&index) {
                return *z;
            }
            let Some(parent) = self.document.nodes[index as usize].parent else {
                return 0;
            };
            index = parent;
        }
    }

    pub fn runtime_style(&self, index: u32, interaction: InteractionState) -> Style {
        let node = &self.document.nodes[index as usize];
        let mut style = interaction.style_for(index, node).clone();
        if !self.is_visible(index) {
            style.display = Display::None;
        }
        if let Some(geometry) = self.geometry_override(index) {
            if let Some(left) = geometry.left {
                style.left = Length::Px(left);
            }
            if let Some(top) = geometry.top {
                style.top = Length::Px(top);
            }
            if let Some(width) = geometry.width {
                style.width = Length::Px(width);
            }
            if let Some(height) = geometry.height {
                style.height = Length::Px(height);
            }
        }
        if let Some(override_style) = self.style_overrides.get(&index) {
            if let Some(background) = override_style.background {
                style.background = background;
            }
            if let Some(border_color) = override_style.border_color {
                style.border_color = border_color;
            }
            if let Some(opacity) = override_style.opacity {
                style.opacity = opacity;
            }
            if let Some(direction) = override_style.flex_direction {
                style.flex_direction = direction;
            }
            if let Some(font_size) = override_style.font_size {
                style.font_size = font_size;
            }
            if let Some(font_weight) = override_style.font_weight {
                style.font_weight = font_weight;
            }
        }
        style.font_size = (style.font_size + self.ui_font_size_offset).max(6.0);
        if self.ui_font_bold && style.font_weight < 600 {
            style.font_weight = 600;
        }
        style
    }

    pub fn text_for(&self, index: u32) -> &str {
        self.text_overrides
            .get(&index)
            .map(String::as_str)
            .unwrap_or_else(|| self.document.nodes[index as usize].text.as_str())
    }

    pub fn resolve_color(&self, value: ColorValue) -> Color {
        match value {
            ColorValue::Literal(color) => color,
            ColorValue::Variable(index) => self
                .variable_values
                .get(index as usize)
                .copied()
                .unwrap_or(Color::TRANSPARENT),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InteractionState {
    pub hover: Option<u32>,
    pub active: Option<u32>,
}

impl InteractionState {
    pub fn style_for<'a>(&self, index: u32, node: &'a CompiledNode) -> &'a Style {
        if self.active == Some(index) {
            if let Some(style) = node.active_style.as_ref() {
                return style;
            }
        }
        if self.hover == Some(index) {
            if let Some(style) = node.hover_style.as_ref() {
                return style;
            }
        }
        &node.style
    }
}
