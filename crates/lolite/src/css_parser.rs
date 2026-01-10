use crate::named_colors;
use crate::style::{
    AlignContent, AlignItems, AlignSelf, BorderRadius, BoxSizing, Display, Extend, FlexDirection,
    FlexWrap, JustifyContent, Length, Rgba, Rule, Selector, Style, StyleSheet,
};
use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, ParseError, Parser, ParserInput, ParserState,
    QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser, Token,
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

    fn parse_color_value<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Rgba, ParseError<'i, ()>> {
        let token = input.next()?;
        match token {
            Token::Ident(name) => named_colors::named_color(name.as_ref())
                .ok_or_else(|| input.new_error_for_next_token()),
            Token::Hash(hex) | Token::IDHash(hex) => {
                // Parse hex colors like #ff0000
                parse_hex_color(&hex).map_err(|_| input.new_error_for_next_token())
            }
            _ => Err(input.new_error_for_next_token()),
        }
    }

    fn parse_length_value<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Length, ParseError<'i, ()>> {
        let token = input.next()?;
        match token {
            Token::Dimension { value, unit, .. } => match unit.as_ref() {
                "px" => Ok(Length::Px(*value as f64)),
                "em" => Ok(Length::Em(*value as f64)),
                "%" => Ok(Length::Percent(*value as f64)),
                _ => Err(input.new_error_for_next_token()),
            },
            Token::Number { value, .. } => {
                // Numbers without units are treated as pixels
                Ok(Length::Px(*value as f64))
            }
            Token::Percentage { unit_value, .. } => {
                // Handle percentage values
                Ok(Length::Percent(*unit_value as f64 * 100.0))
            }
            Token::Ident(name) => match name.as_ref() {
                "auto" => Ok(Length::Auto),
                _ => Err(input.new_error_for_next_token()),
            },
            _ => Err(input.new_error_for_next_token()),
        }
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
            "background" => {
                // only support color for now
                style.background_color = Some(self.parse_color_value(input)?);
            }
            "background-color" => {
                style.background_color = Some(self.parse_color_value(input)?);
            }
            "border-color" => {
                style.border_color = Some(self.parse_color_value(input)?);
            }
            "border-width" => {
                style.border_width = Some(self.parse_length_value(input)?);
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

                style.margin = Some(Extend {
                    top: first,
                    right: second,
                    bottom: third,
                    left: fourth,
                });

                // Mirror into per-side fields so later declarations like `margin-left`
                // can override a single side via Style::merge.
                if let Some(m) = style.margin.as_ref() {
                    style.margin_top = Some(m.top);
                    style.margin_right = Some(m.right);
                    style.margin_bottom = Some(m.bottom);
                    style.margin_left = Some(m.left);
                }
            }
            "margin-top" => {
                style.margin_top = Some(self.parse_length_value(input)?);
            }
            "margin-right" => {
                style.margin_right = Some(self.parse_length_value(input)?);
            }
            "margin-bottom" => {
                style.margin_bottom = Some(self.parse_length_value(input)?);
            }
            "margin-left" => {
                style.margin_left = Some(self.parse_length_value(input)?);
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

                style.padding = Some(Extend {
                    top: first,
                    right: second,
                    bottom: third,
                    left: fourth,
                });
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
                style.gap = Some(self.parse_length_value(input)?);
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

/// Parse a hex color string into Rgba
fn parse_hex_color(hex: &str) -> Result<Rgba, &'static str> {
    let hex = hex.trim_start_matches('#');

    match hex.len() {
        3 => {
            // #rgb -> #rrggbb
            let r =
                u8::from_str_radix(&hex[0..1].repeat(2), 16).map_err(|_| "Invalid hex digit")?;
            let g =
                u8::from_str_radix(&hex[1..2].repeat(2), 16).map_err(|_| "Invalid hex digit")?;
            let b =
                u8::from_str_radix(&hex[2..3].repeat(2), 16).map_err(|_| "Invalid hex digit")?;
            Ok(Rgba { r, g, b, a: 255 })
        }
        6 => {
            // #rrggbb
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex digit")?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex digit")?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex digit")?;
            Ok(Rgba { r, g, b, a: 255 })
        }
        8 => {
            // #rrggbbaa
            let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex digit")?;
            let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex digit")?;
            let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex digit")?;
            let a = u8::from_str_radix(&hex[6..8], 16).map_err(|_| "Invalid hex digit")?;
            Ok(Rgba { r, g, b, a })
        }
        _ => Err("Invalid hex color length"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(
            parse_hex_color("ff0000").unwrap(),
            Rgba {
                r: 255,
                g: 0,
                b: 0,
                a: 255
            }
        );
        assert_eq!(
            parse_hex_color("f00").unwrap(),
            Rgba {
                r: 255,
                g: 0,
                b: 0,
                a: 255
            }
        );
        assert_eq!(
            parse_hex_color("ff000080").unwrap(),
            Rgba {
                r: 255,
                g: 0,
                b: 0,
                a: 128
            }
        );
    }

    #[test]
    fn test_parse_simple_css() {
        let css = r#"
            .container {
                display: flex;
                background-color: red;
                width: 100px;
                height: 200px;
            }
        "#;

        let stylesheet = parse_css(css).unwrap();
        assert_eq!(stylesheet.rules.len(), 1);

        let rule = &stylesheet.rules[0];
        assert_eq!(rule.selector, Selector::Class("container".to_string()));
        assert!(!rule.declarations.is_empty());
    }

    #[test]
    fn test_parse_border_radius() {
        let css = r#"
            .rounded {
                border-radius: 10px;
                background-color: blue;
            }

            .complex-rounded {
                border-radius: 5px 10px 15px 20px;
                border-width: 2px;
                border-color: #333;
            }
        "#;

        let stylesheet = parse_css(css).unwrap();
        assert_eq!(stylesheet.rules.len(), 2);

        // Check first rule has border-radius
        let rule1 = &stylesheet.rules[0];
        assert_eq!(rule1.selector, Selector::Class("rounded".to_string()));
        let has_border_radius = rule1
            .declarations
            .iter()
            .any(|decl| decl.border_radius.is_some());
        assert!(has_border_radius, "Should have border-radius property");

        // Check second rule has border-radius with multiple values
        let rule2 = &stylesheet.rules[1];
        assert_eq!(
            rule2.selector,
            Selector::Class("complex-rounded".to_string())
        );
        let has_border_radius = rule2
            .declarations
            .iter()
            .any(|decl| decl.border_radius.is_some());
        assert!(
            has_border_radius,
            "Should have complex border-radius property"
        );
    }
}
