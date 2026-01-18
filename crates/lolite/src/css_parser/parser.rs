use crate::style::{
    AlignContent, AlignItems, AlignSelf, BorderRadius, BoxSizing, Directional, Display,
    FlexDirection, FlexWrap, JustifyContent, Rule, Selector, Style, StyleSheet,
};
use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, Parser, ParserInput, ParserState,
    QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser,
};

/// Parse a CSS string into a StyleSheet
pub fn parse_css(css: &str) -> Result<StyleSheet, String> {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);

    let mut stylesheet = StyleSheet::new();
    let mut css_parser = CssParser::new();

    let rules = StyleSheetParser::new(&mut parser, &mut css_parser);

    for rule in rules {
        match rule {
            Ok(parsed_rule) => {
                stylesheet.add_rule(parsed_rule);
            }
            Err(err) => {
                eprintln!("CSS parsing error: {:?}", err);
            }
        }
    }

    Ok(stylesheet)
}

/// CSS Parser implementation
pub struct CssParser {
    // We can add state here if needed
}

impl CssParser {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'i> QualifiedRuleParser<'i> for CssParser {
    type Prelude = Selector;
    type QualifiedRule = Rule;
    type Error = ();

    fn parse_prelude<'t>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i, Self::Error>> {
        // Parse selector - for now we'll support simple class and tag selectors
        if input.try_parse(|input| input.expect_delim('.')).is_ok() {
            let class_name = input.expect_ident()?;
            Ok(Selector::Class(class_name.to_string()))
        } else {
            let name = input.expect_ident()?;
            Ok(Selector::Tag(name.to_string()))
        }
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _start: &ParserState,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, ParseError<'i, Self::Error>> {
        let mut declarations = Vec::new();
        let mut declaration_parser = StyleDeclarationParser::new();

        let parser = RuleBodyParser::new(input, &mut declaration_parser);
        for item in parser {
            match item {
                Ok(declaration) => declarations.push(declaration),
                Err(err) => {
                    eprintln!("Declaration parsing error: {:?}", err);
                }
            }
        }

        Ok(Rule {
            selector: prelude,
            declarations,
        })
    }
}

impl<'i> AtRuleParser<'i> for CssParser {
    type Prelude = ();
    type AtRule = Rule;
    type Error = ();
}

/// Declaration parser for style properties
pub struct StyleDeclarationParser {
    // State can be added here if needed
}

impl StyleDeclarationParser {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'i> DeclarationParser<'i> for StyleDeclarationParser {
    type Declaration = Style;
    type Error = ();

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
        _declaration_start: &ParserState,
    ) -> Result<Self::Declaration, ParseError<'i, Self::Error>> {
        let mut style = Style::default();

        match name.as_ref() {
            "display" => {
                let ident = input.expect_ident()?;
                match ident.as_ref() {
                    "flex" => style.display = Display::Flex,
                    _ => return Err(input.new_error_for_next_token()),
                }
            }
            "color" => {
                style.color = Some(self.parse_color_value(input)?);
            }
            "background" => {
                // only support color for now
                style.background_color = Some(self.parse_color_value(input)?);
            }
            "background-color" => {
                style.background_color = Some(self.parse_color_value(input)?);
            }
            "border-color" => {
                style.border_color = Directional::set_all(Some(self.parse_color_value(input)?));
            }
            "border-width" => {
                style.border_width = Directional::set_all(Some(self.parse_length_value(input)?));
            }
            "border-style" => {
                // Parse a single <line-style> value.
                let v = self
                    .try_parse_line_style(input)?
                    .ok_or_else(|| input.new_error_for_next_token())?;
                style.border_style = Directional::set_all(Some(v));
            }
            "border" => {
                self.parse_border_shorthand(input, &mut style)?;
            }
            "border-radius" => {
                let first = self.parse_length_value(input)?;
                let second = input
                    .try_parse(|input| self.parse_length_value(input))
                    .unwrap_or(first.clone());
                let third = input
                    .try_parse(|input| self.parse_length_value(input))
                    .unwrap_or(first.clone());
                let fourth = input
                    .try_parse(|input| self.parse_length_value(input))
                    .unwrap_or(second.clone());

                style.border_radius = Some(BorderRadius {
                    top_left: first,
                    top_right: second,
                    bottom_right: third,
                    bottom_left: fourth,
                });
            }
            "box-sizing" => {
                let ident = input.expect_ident()?;
                style.box_sizing = Some(match ident.as_ref() {
                    "content-box" => BoxSizing::ContentBox,
                    "border-box" => BoxSizing::BorderBox,
                    _ => return Err(input.new_error_for_next_token()),
                });
            }
            "width" => {
                style.width = Some(self.parse_length_value(input)?);
            }
            "height" => {
                style.height = Some(self.parse_length_value(input)?);
            }
            "margin" => {
                // NOTE: We currently implement the physical margin shorthands/sides:
                // - `margin` (shorthand)
                // - `margin-top/right/bottom/left`
                // We do NOT yet implement logical properties like `margin-inline`,
                // `margin-block`, `margin-inline-start/end`, etc.
                let first = self.parse_length_value(input)?;
                let second = input
                    .try_parse(|input| self.parse_length_value(input))
                    .unwrap_or(first.clone());
                let third = input
                    .try_parse(|input| self.parse_length_value(input))
                    .unwrap_or(first.clone());
                let fourth = input
                    .try_parse(|input| self.parse_length_value(input))
                    .unwrap_or(second.clone());

                // Emit per-side declarations so later `margin-left` etc. can override
                // a single side via Style::merge.
                style.margin = Directional {
                    top: Some(first),
                    right: Some(second),
                    bottom: Some(third),
                    left: Some(fourth),
                };
            }
            "margin-top" => {
                style.margin.top = Some(self.parse_length_value(input)?);
            }
            "margin-right" => {
                style.margin.right = Some(self.parse_length_value(input)?);
            }
            "margin-bottom" => {
                style.margin.bottom = Some(self.parse_length_value(input)?);
            }
            "margin-left" => {
                style.margin.left = Some(self.parse_length_value(input)?);
            }
            "padding" => {
                let first = self.parse_length_value(input)?;
                let second = input
                    .try_parse(|input| self.parse_length_value(input))
                    .unwrap_or(first.clone());
                let third = input
                    .try_parse(|input| self.parse_length_value(input))
                    .unwrap_or(first.clone());
                let fourth = input
                    .try_parse(|input| self.parse_length_value(input))
                    .unwrap_or(second.clone());

                // Emit per-side declarations so later `padding-left` etc. can override
                // a single side via Style::merge.
                style.padding = Directional {
                    top: Some(first),
                    right: Some(second),
                    bottom: Some(third),
                    left: Some(fourth),
                };
            }
            "padding-top" => {
                style.padding.top = Some(self.parse_length_value(input)?);
            }
            "padding-right" => {
                style.padding.right = Some(self.parse_length_value(input)?);
            }
            "padding-bottom" => {
                style.padding.bottom = Some(self.parse_length_value(input)?);
            }
            "padding-left" => {
                style.padding.left = Some(self.parse_length_value(input)?);
            }
            "flex" => {
                let value = input.expect_number()?;
                style.flex_grow = Some(value as f64);
            }
            "flex-direction" => {
                let ident = input.expect_ident()?;
                style.flex_direction = Some(match ident.as_ref() {
                    "row" => FlexDirection::Row,
                    "row-reverse" => FlexDirection::RowReverse,
                    "column" => FlexDirection::Column,
                    "column-reverse" => FlexDirection::ColumnReverse,
                    _ => return Err(input.new_error_for_next_token()),
                });
            }
            "flex-wrap" => {
                let ident = input.expect_ident()?;
                style.flex_wrap = Some(match ident.as_ref() {
                    "nowrap" => FlexWrap::NoWrap,
                    "wrap" => FlexWrap::Wrap,
                    "wrap-reverse" => FlexWrap::WrapReverse,
                    _ => return Err(input.new_error_for_next_token()),
                });
            }
            "justify-content" => {
                let ident = input.expect_ident()?;
                style.justify_content = Some(match ident.as_ref() {
                    "flex-start" => JustifyContent::FlexStart,
                    "flex-end" => JustifyContent::FlexEnd,
                    "center" => JustifyContent::Center,
                    "space-between" => JustifyContent::SpaceBetween,
                    "space-around" => JustifyContent::SpaceAround,
                    "space-evenly" => JustifyContent::SpaceEvenly,
                    _ => return Err(input.new_error_for_next_token()),
                });
            }
            "align-items" => {
                let ident = input.expect_ident()?;
                style.align_items = Some(match ident.as_ref() {
                    "stretch" => AlignItems::Stretch,
                    "flex-start" => AlignItems::FlexStart,
                    "flex-end" => AlignItems::FlexEnd,
                    "center" => AlignItems::Center,
                    "baseline" => AlignItems::Baseline,
                    _ => return Err(input.new_error_for_next_token()),
                });
            }
            "align-content" => {
                let ident = input.expect_ident()?;
                style.align_content = Some(match ident.as_ref() {
                    "stretch" => AlignContent::Stretch,
                    "flex-start" => AlignContent::FlexStart,
                    "flex-end" => AlignContent::FlexEnd,
                    "center" => AlignContent::Center,
                    "space-between" => AlignContent::SpaceBetween,
                    "space-around" => AlignContent::SpaceAround,
                    "space-evenly" => AlignContent::SpaceEvenly,
                    _ => return Err(input.new_error_for_next_token()),
                });
            }
            "align-self" => {
                let ident = input.expect_ident()?;
                style.align_self = Some(match ident.as_ref() {
                    "auto" => AlignSelf::Auto,
                    "flex-start" => AlignSelf::FlexStart,
                    "flex-end" => AlignSelf::FlexEnd,
                    "center" => AlignSelf::Center,
                    "baseline" => AlignSelf::Baseline,
                    "stretch" => AlignSelf::Stretch,
                    _ => return Err(input.new_error_for_next_token()),
                });
            }
            "flex-grow" => {
                let value = input.expect_number()?;
                style.flex_grow = Some(value as f64);
            }
            "flex-shrink" => {
                let value = input.expect_number()?;
                style.flex_shrink = Some(value as f64);
            }
            "flex-basis" => {
                style.flex_basis = Some(self.parse_length_value(input)?);
            }
            "order" => {
                let value = input.expect_number()?;
                style.order = Some(value as i32);
            }
            "gap" => {
                let gap = self.parse_length_value(input)?;
                style.row_gap = Some(gap);
                style.column_gap = Some(gap);
            }
            "row-gap" => {
                style.row_gap = Some(self.parse_length_value(input)?);
            }
            "column-gap" => {
                style.column_gap = Some(self.parse_length_value(input)?);
            }
            _ => {
                // Skip unknown properties
                return Err(input.new_error_for_next_token());
            }
        }

        Ok(style)
    }
}

impl<'i> AtRuleParser<'i> for StyleDeclarationParser {
    type Prelude = ();
    type AtRule = Style;
    type Error = ();
}

impl<'i> QualifiedRuleParser<'i> for StyleDeclarationParser {
    type Prelude = ();
    type QualifiedRule = Style;
    type Error = ();
}

impl<'i> RuleBodyItemParser<'i, Style, ()> for StyleDeclarationParser {
    fn parse_qualified(&self) -> bool {
        false
    }

    fn parse_declarations(&self) -> bool {
        true
    }
}
