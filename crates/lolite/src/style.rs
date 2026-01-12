use lolite_macros::MergeProperties;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Copy, Default)]
#[allow(unused)]
pub enum Length {
    #[default]
    Auto,
    Px(f64),
    Em(f64),
    Percent(f64),
}

impl Length {
    pub fn to_px(&self) -> f64 {
        match self {
            Length::Px(value) => *value,
            Length::Auto => 0.0,
            Length::Em(_) => 0.0,      // TODO: Implement em conversion
            Length::Percent(_) => 0.0, // TODO: Implement percentage conversion
        }
    }
}

#[derive(Clone)]
#[allow(unused)]
pub struct Extend {
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
}

impl Default for Extend {
    fn default() -> Self {
        // CSS initial value for margin/padding edges is 0.
        Self {
            top: Length::Px(0.0),
            right: Length::Px(0.0),
            bottom: Length::Px(0.0),
            left: Length::Px(0.0),
        }
    }
}

#[derive(Clone, Default)]
pub struct BorderRadius {
    pub top_left: Length,
    pub top_right: Length,
    #[allow(unused)]
    pub bottom_right: Length,
    #[allow(unused)]
    pub bottom_left: Length,
}

#[derive(Clone, Copy, Default)]
pub enum Display {
    // Block,
    // Inline,
    // InlineBlock,
    #[default]
    Flex,
    // Grid,
}

#[derive(Clone, Copy, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Clone, Copy, Default)]
pub enum FlexWrap {
    #[default]
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy, Default)]
pub enum JustifyContent {
    #[default]
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Default)]
pub enum AlignItems {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

#[derive(Clone, Copy, Default)]
pub enum AlignContent {
    #[default]
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Default)]
pub enum AlignSelf {
    #[default]
    Auto,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub enum BoxSizing {
    #[default]
    ContentBox,
    BorderBox,
}

#[derive(Clone, Default, MergeProperties)]
pub struct Style {
    pub display: Display,
    pub color: Option<Rgba>,
    pub background_color: Option<Rgba>,
    pub border_color: Option<Rgba>,
    pub border_width: Option<Length>,
    pub border_radius: Option<BorderRadius>,
    pub box_sizing: Option<BoxSizing>,
    pub margin: Option<Extend>,
    pub margin_top: Option<Length>,
    pub margin_right: Option<Length>,
    pub margin_bottom: Option<Length>,
    pub margin_left: Option<Length>,
    pub padding: Option<Extend>,
    pub width: Option<Length>,
    pub height: Option<Length>,

    // Text / font properties
    pub font_family: Option<String>,
    pub font_size: Option<Length>,

    // Flexbox container properties
    pub flex_direction: Option<FlexDirection>,
    pub flex_wrap: Option<FlexWrap>,
    pub justify_content: Option<JustifyContent>,
    pub align_items: Option<AlignItems>,
    pub align_content: Option<AlignContent>,
    pub gap: Option<Length>,
    pub row_gap: Option<Length>,
    pub column_gap: Option<Length>,

    // Flexbox item properties
    pub flex_grow: Option<f64>,
    pub flex_shrink: Option<f64>,
    pub flex_basis: Option<Length>,
    pub align_self: Option<AlignSelf>,
    pub order: Option<i32>,
}

pub struct StyleSheet {
    pub rules: Vec<Rule>,
}

impl StyleSheet {
    pub fn new() -> Self {
        Self { rules: vec![] }
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }
}

pub struct Rule {
    pub selector: Selector,
    pub declarations: Vec<Style>,
}

#[derive(Debug, PartialEq)]
pub enum Selector {
    Tag(String),
    Class(String),
}
