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

    fn clamp_u8(value: f32) -> u8 {
        value.clamp(0.0, 255.0).round() as u8
    }

    fn parse_rgb_channel<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<u8, ParseError<'i, ()>> {
        let token = input.next()?;
        match token {
            Token::Number { value, .. } => Ok(Self::clamp_u8(*value as f32)),
            Token::Percentage { unit_value, .. } => {
                Ok(Self::clamp_u8((*unit_value as f32) * 255.0))
            }
            _ => Err(input.new_error_for_next_token()),
        }
    }

    fn parse_alpha_channel<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<u8, ParseError<'i, ()>> {
        let token = input.next()?;
        let alpha_0_1 = match token {
            Token::Number { value, .. } => *value as f32,
            Token::Percentage { unit_value, .. } => *unit_value as f32,
            _ => return Err(input.new_error_for_next_token()),
        };

        Ok((alpha_0_1.clamp(0.0, 1.0) * 255.0).round() as u8)
    }

    fn normalize_hue_degrees(hue_degrees: f32) -> f32 {
        if !hue_degrees.is_finite() {
            return 0.0;
        }
        let mut hue = hue_degrees % 360.0;
        if hue < 0.0 {
            hue += 360.0;
        }
        hue
    }

    /// Parse a CSS `<angle>` value and return degrees normalized to [0, 360).
    ///
    /// Supported forms:
    /// - `<number>` (interpreted as degrees)
    /// - `<angle>` units: `deg`, `rad`, `grad`, `turn`
    fn parse_angle_degrees<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<f32, ParseError<'i, ()>> {
        let token = input.next()?;
        let degrees = match token {
            Token::Number { value, .. } => *value as f32,
            Token::Dimension { value, unit, .. } => match unit.as_ref() {
                "deg" => *value as f32,
                "grad" => (*value as f32) * 0.9,
                "rad" => (*value as f32) * (180.0 / std::f32::consts::PI),
                "turn" => (*value as f32) * 360.0,
                _ => return Err(input.new_error_for_next_token()),
            },
            _ => return Err(input.new_error_for_next_token()),
        };

        Ok(Self::normalize_hue_degrees(degrees))
    }

    fn parse_hue_value<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<f32, ParseError<'i, ()>> {
        // Modern syntax allows 'none' (missing component). We can't represent
        // missing here, so treat it as 0deg.
        if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
            return Ok(0.0);
        }

        self.parse_angle_degrees(input)
    }

    fn parse_hsl_percent_or_number<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<f32, ParseError<'i, ()>> {
        let token = input.next()?;
        match token {
            Token::Percentage { unit_value, .. } => Ok((*unit_value as f32) * 100.0),
            Token::Number { value, .. } => Ok(*value as f32),
            Token::Ident(name) if name.eq_ignore_ascii_case("none") => Ok(0.0),
            _ => Err(input.new_error_for_next_token()),
        }
    }

    fn parse_hwb_percent_or_number<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<f32, ParseError<'i, ()>> {
        // HWB W and B allow mixed <percentage>/<number> in modern syntax, and also allow 'none'.
        self.parse_hsl_percent_or_number(input)
    }

    fn parse_percentage<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<f32, ParseError<'i, ()>> {
        let token = input.next()?;
        match token {
            Token::Percentage { unit_value, .. } => Ok((*unit_value as f32) * 100.0),
            _ => Err(input.new_error_for_next_token()),
        }
    }

    fn parse_alpha_value_u8<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<u8, ParseError<'i, ()>> {
        // <alpha-value> = <number> | <percentage> in Level 4.
        // Modern syntax also allows 'none'.
        let token = input.next()?;
        let alpha_0_1 = match token {
            Token::Number { value, .. } => *value as f32,
            Token::Percentage { unit_value, .. } => *unit_value as f32,
            Token::Ident(name) if name.eq_ignore_ascii_case("none") => 1.0,
            _ => return Err(input.new_error_for_next_token()),
        };
        Ok((alpha_0_1.clamp(0.0, 1.0) * 255.0).round() as u8)
    }

    fn hsl_to_rgb_u8(
        hue_degrees: f32,
        saturation_0_100: f32,
        lightness_0_100: f32,
    ) -> (u8, u8, u8) {
        // Conversion algorithm based on CSS Color 4 §7.1 (sample implementation).
        let hue = hue_degrees;
        let sat = (saturation_0_100.max(0.0).min(100.0)) / 100.0;
        let light = (lightness_0_100.max(0.0).min(100.0)) / 100.0;

        fn f(n: f32, hue: f32, sat: f32, light: f32) -> f32 {
            let k = (n + hue / 30.0) % 12.0;
            let a = sat * light.min(1.0 - light);
            let m = (k - 3.0).min(9.0 - k).min(1.0);
            light - a * (-1.0_f32).max(m)
        }

        let r = Self::clamp_u8(f(0.0, hue, sat, light) * 255.0);
        let g = Self::clamp_u8(f(8.0, hue, sat, light) * 255.0);
        let b = Self::clamp_u8(f(4.0, hue, sat, light) * 255.0);
        (r, g, b)
    }

    fn parse_hsl_color<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Rgba, ParseError<'i, ()>> {
        // Supports both legacy comma-separated and modern space-separated syntax.
        // See CSS Color 4 §7.
        let hue = self.parse_hue_value(input)?;

        // Legacy: hsl(<hue>, <percentage>, <percentage>[, <alpha>]?)
        if input.try_parse(|i| i.expect_comma()).is_ok() {
            let sat = self.parse_percentage(input)?;
            input.expect_comma()?;
            let light = self.parse_percentage(input)?;

            let a = input
                .try_parse(|i| {
                    i.expect_comma()?;
                    self.parse_alpha_value_u8(i)
                })
                .unwrap_or(255);

            // Historical behavior: negative saturation clamped to 0 at parse time.
            let sat = sat.max(0.0);
            let (r, g, b) = Self::hsl_to_rgb_u8(hue, sat, light);
            return Ok(Rgba { r, g, b, a });
        }

        // Modern: hsl(<hue> <sat> <light> [ / <alpha> ]?)
        let sat = self.parse_hsl_percent_or_number(input)?.max(0.0);
        let light = self.parse_hsl_percent_or_number(input)?;

        let a = if input.try_parse(|i| i.expect_delim('/')).is_ok() {
            self.parse_alpha_value_u8(input)?
        } else {
            255
        };

        let (r, g, b) = Self::hsl_to_rgb_u8(hue, sat, light);
        Ok(Rgba { r, g, b, a })
    }

    fn hwb_to_rgb_u8(hue_degrees: f32, white_0_100: f32, black_0_100: f32) -> (u8, u8, u8) {
        // Conversion algorithm based on CSS Color 4 §8.1 (sample implementation).
        // Note: values outside [0,100] are not invalid; we only clamp negative values.
        let hue = hue_degrees;
        let white = (white_0_100.max(0.0)) / 100.0;
        let black = (black_0_100.max(0.0)) / 100.0;

        if white + black >= 1.0 {
            let gray = white / (white + black);
            let v = Self::clamp_u8(gray * 255.0);
            return (v, v, v);
        }

        // Base color is the fully saturated hue at 50% lightness.
        let (base_r, base_g, base_b) = Self::hsl_to_rgb_u8(hue, 100.0, 50.0);
        let mut r = (base_r as f32) / 255.0;
        let mut g = (base_g as f32) / 255.0;
        let mut b = (base_b as f32) / 255.0;

        let factor = 1.0 - white - black;
        r = r * factor + white;
        g = g * factor + white;
        b = b * factor + white;

        (
            Self::clamp_u8(r * 255.0),
            Self::clamp_u8(g * 255.0),
            Self::clamp_u8(b * 255.0),
        )
    }

    fn parse_hwb_color<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Rgba, ParseError<'i, ()>> {
        // CSS Color 4 §8: hwb() does NOT support legacy comma-separated syntax.
        // Using commas inside hwb() is an error.
        let hue = self.parse_hue_value(input)?;

        if input.try_parse(|i| i.expect_comma()).is_ok() {
            return Err(input.new_error_for_next_token());
        }

        let white = self.parse_hwb_percent_or_number(input)?;
        let black = self.parse_hwb_percent_or_number(input)?;

        let a = if input.try_parse(|i| i.expect_delim('/')).is_ok() {
            self.parse_alpha_value_u8(input)?
        } else {
            255
        };

        let (r, g, b) = Self::hwb_to_rgb_u8(hue, white, black);
        Ok(Rgba { r, g, b, a })
    }

    fn parse_color_value<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Rgba, ParseError<'i, ()>> {
        let token = input.next()?;
        match token {
            Token::Ident(name) => named_colors::named_color(name.as_ref())
                .ok_or_else(|| input.new_error_for_next_token()),
            Token::Function(name) => {
                let func = name.as_ref();
                if func.eq_ignore_ascii_case("rgb") {
                    input.parse_nested_block(|input| {
                        let r = self.parse_rgb_channel(input)?;
                        input.expect_comma()?;
                        let g = self.parse_rgb_channel(input)?;
                        input.expect_comma()?;
                        let b = self.parse_rgb_channel(input)?;
                        Ok(Rgba { r, g, b, a: 255 })
                    })
                } else if func.eq_ignore_ascii_case("rgba") {
                    input.parse_nested_block(|input| {
                        let r = self.parse_rgb_channel(input)?;
                        input.expect_comma()?;
                        let g = self.parse_rgb_channel(input)?;
                        input.expect_comma()?;
                        let b = self.parse_rgb_channel(input)?;
                        input.expect_comma()?;
                        let a = self.parse_alpha_channel(input)?;
                        Ok(Rgba { r, g, b, a })
                    })
                } else if func.eq_ignore_ascii_case("hsl") || func.eq_ignore_ascii_case("hsla") {
                    input.parse_nested_block(|input| self.parse_hsl_color(input))
                } else if func.eq_ignore_ascii_case("hwb") {
                    input.parse_nested_block(|input| self.parse_hwb_color(input))
                } else {
                    Err(input.new_error_for_next_token())
                }
            }
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
