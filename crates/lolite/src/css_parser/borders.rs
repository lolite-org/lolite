use super::parser::StyleDeclarationParser;
use crate::style::{BorderStyle, Directional, Length, Radius, Style};
use cssparser::{ParseError, Parser};

impl StyleDeclarationParser {
    pub(crate) fn try_parse_line_width<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Option<Length>, ParseError<'i, ()>> {
        // <line-width> = <length> | thin | medium | thick
        if let Ok(len) = input.try_parse(|i| self.parse_length_value(i)) {
            return Ok(Some(len));
        }

        if input.try_parse(|i| i.expect_ident_matching("thin")).is_ok() {
            return Ok(Some(Length::Px(1.0)));
        }
        if input
            .try_parse(|i| i.expect_ident_matching("medium"))
            .is_ok()
        {
            return Ok(Some(Length::Px(3.0)));
        }
        if input
            .try_parse(|i| i.expect_ident_matching("thick"))
            .is_ok()
        {
            return Ok(Some(Length::Px(5.0)));
        }

        Ok(None)
    }

    pub(crate) fn try_parse_line_style<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Option<BorderStyle>, ParseError<'i, ()>> {
        // <line-style> = none | hidden | solid | dotted | dashed | double | groove | ridge | inset | outset
        if input.try_parse(|i| i.expect_ident_matching("none")).is_ok() {
            return Ok(Some(BorderStyle::None));
        }
        if input
            .try_parse(|i| i.expect_ident_matching("hidden"))
            .is_ok()
        {
            return Ok(Some(BorderStyle::Hidden));
        }
        if input
            .try_parse(|i| i.expect_ident_matching("solid"))
            .is_ok()
        {
            return Ok(Some(BorderStyle::Solid));
        }
        if input
            .try_parse(|i| i.expect_ident_matching("dotted"))
            .is_ok()
        {
            return Ok(Some(BorderStyle::Dotted));
        }
        if input
            .try_parse(|i| i.expect_ident_matching("dashed"))
            .is_ok()
        {
            return Ok(Some(BorderStyle::Dashed));
        }
        if input
            .try_parse(|i| i.expect_ident_matching("double"))
            .is_ok()
        {
            return Ok(Some(BorderStyle::Double));
        }
        if input
            .try_parse(|i| i.expect_ident_matching("groove"))
            .is_ok()
        {
            return Ok(Some(BorderStyle::Groove));
        }
        if input
            .try_parse(|i| i.expect_ident_matching("ridge"))
            .is_ok()
        {
            return Ok(Some(BorderStyle::Ridge));
        }
        if input
            .try_parse(|i| i.expect_ident_matching("inset"))
            .is_ok()
        {
            return Ok(Some(BorderStyle::Inset));
        }
        if input
            .try_parse(|i| i.expect_ident_matching("outset"))
            .is_ok()
        {
            return Ok(Some(BorderStyle::Outset));
        }

        Ok(None)
    }

    pub(crate) fn parse_border_shorthand<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
        style: &mut Style,
    ) -> Result<(), ParseError<'i, ()>> {
        // border: <line-width> || <line-style> || <color>
        let mut saw_width = false;
        let mut saw_color = false;
        let mut saw_style = false;
        while !input.is_exhausted() {
            if let Some(width) = self.try_parse_line_width(input)? {
                if saw_width {
                    return Err(input.new_error_for_next_token());
                }
                saw_width = true;
                style.border_width = Directional::set_all(Some(width));
                continue;
            }

            if let Ok(color) = input.try_parse(|i| self.parse_color_value(i)) {
                if saw_color {
                    return Err(input.new_error_for_next_token());
                }
                saw_color = true;
                style.border_color = Directional::set_all(Some(color));
                continue;
            }

            if let Some(border_style) = self.try_parse_line_style(input)? {
                if saw_style {
                    return Err(input.new_error_for_next_token());
                }
                saw_style = true;
                style.border_style = Directional::set_all(Some(border_style));
                continue;
            }

            // Unknown token
            return Err(input.new_error_for_next_token());
        }

        Ok(())
    }

    fn parse_border_radius_value<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Radius, ParseError<'i, ()>> {
        let x = self.parse_length_value(input)?;
        let y = input
            .try_parse(|input| self.parse_length_value(input))
            .unwrap_or(x);
        Ok(Radius { x, y })
    }

    fn parse_border_radius_1_to_4<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<(Length, Length, Length, Length), ParseError<'i, ()>> {
        // Matches CSS 1-4 expansion order: top-left, top-right, bottom-right, bottom-left.
        let first = self.parse_length_value(input)?;
        let second = input
            .try_parse(|input| self.parse_length_value(input))
            .unwrap_or(first);
        let third = input
            .try_parse(|input| self.parse_length_value(input))
            .unwrap_or(first);
        let fourth = input
            .try_parse(|input| self.parse_length_value(input))
            .unwrap_or(second);

        Ok((first, second, third, fourth))
    }

    pub(crate) fn parse_border_radius_shorthand<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
        style: &mut Style,
    ) -> Result<(), ParseError<'i, ()>> {
        // border-radius: <length-percentage>{1,4} [ / <length-percentage>{1,4} ]?
        // Values before '/' are the horizontal radii; values after '/' are the vertical radii.
        // If '/' is omitted, vertical radii equal horizontal radii.

        let (tl_x, tr_x, br_x, bl_x) = self.parse_border_radius_1_to_4(input)?;

        let (tl_y, tr_y, br_y, bl_y) = if input.try_parse(|i| i.expect_delim('/')).is_ok() {
            self.parse_border_radius_1_to_4(input)?
        } else {
            (tl_x, tr_x, br_x, bl_x)
        };

        if !input.is_exhausted() {
            return Err(input.new_error_for_next_token());
        }

        style.border_radius.top_left = Some(Radius { x: tl_x, y: tl_y });
        style.border_radius.top_right = Some(Radius { x: tr_x, y: tr_y });
        style.border_radius.bottom_right = Some(Radius { x: br_x, y: br_y });
        style.border_radius.bottom_left = Some(Radius { x: bl_x, y: bl_y });

        Ok(())
    }

    pub(crate) fn parse_border_corner_radius<'i, 't>(
        &mut self,
        input: &mut Parser<'i, 't>,
        corner: &mut Option<Radius>,
    ) -> Result<(), ParseError<'i, ()>> {
        // border-*-radius: <length-percentage [0,∞]>{1,2}
        let radius = self.parse_border_radius_value(input)?;
        if !input.is_exhausted() {
            return Err(input.new_error_for_next_token());
        }
        *corner = Some(radius);
        Ok(())
    }
}
