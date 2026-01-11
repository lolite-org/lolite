use crate::css_parser::parse_css;
use crate::style::{BoxSizing, Display, Length, Rgba, Selector};

#[test]
fn test_parse_simple_css_document() {
    let css = r#"
        .container {
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: stretch;
            background-color: #f0f0f0;
            width: 300px;
            height: 200px;
            padding: 20px;
            margin: 10px;
        }

        .box {
            background-color: red;
            width: 100px;
            height: 50px;
            margin: 5px;
            flex-grow: 1;
            flex-shrink: 0;
        }

        button {
            background-color: #007bff;
            border-width: 2px;
            border-color: #0056b3;
            padding: 8px;
            margin: 4px;
        }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");

    // Should have 3 rules
    assert_eq!(stylesheet.rules.len(), 3);

    // Test first rule (.container)
    let container_rule = &stylesheet.rules[0];
    assert_eq!(
        container_rule.selector,
        Selector::Class("container".to_string())
    );
    assert!(!container_rule.declarations.is_empty());

    // Check that we have multiple declarations for the container
    let mut found_display = false;
    let mut found_background = false;
    let mut found_width = false;

    for declaration in &container_rule.declarations {
        match declaration.display {
            Display::Flex => found_display = true,
        }
        if declaration.background_color.is_some() {
            found_background = true;
        }
        if declaration.width.is_some() {
            found_width = true;
        }
    }

    assert!(found_display, "Should have found display: flex");
    assert!(found_background, "Should have found background-color");
    assert!(found_width, "Should have found width");

    // Test second rule (.box)
    let box_rule = &stylesheet.rules[1];
    assert_eq!(box_rule.selector, Selector::Class("box".to_string()));
    assert!(!box_rule.declarations.is_empty());

    // Test third rule (button)
    let button_rule = &stylesheet.rules[2];
    assert_eq!(button_rule.selector, Selector::Tag("button".to_string()));
    assert!(!button_rule.declarations.is_empty());
}

#[test]
fn test_parse_flex_properties() {
    let css = r#"
        .flex-container {
            display: flex;
            flex-direction: row;
            justify-content: space-between;
            align-items: center;
            flex-wrap: wrap;
            gap: 15px;
        }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 1);

    let rule = &stylesheet.rules[0];
    assert_eq!(rule.selector, Selector::Class("flex-container".to_string()));

    // Verify we can parse all the flex properties
    assert!(!rule.declarations.is_empty());
}

#[test]
fn test_parse_colors() {
    let css = r#"
        .color-test {
            background-color: red;
            border-color: #ff0000;
        }

        .color-test2 {
            background-color: #f00;
            border-color: #ff000080;
        }

        .transparent {
            background-color: transparent;
        }

        .rounded {
            border-radius: 10px;
        }

        .complex-radius {
            border-radius: 5px 10px 15px 20px;
        }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 5);

    // All rules should parse successfully
    for rule in &stylesheet.rules {
        assert!(!rule.declarations.is_empty());
    }
}

#[test]
fn test_named_colors_exact_values() {
    let css = r#"
        .a { background-color: AliceBlue; }
        .b { background-color: rebeccapurple; }
        .c { background-color: gray; }
        .d { background-color: grey; }
        .e { background-color: transparent; }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 5);

    let get_bg = |idx: usize| -> Rgba {
        stylesheet.rules[idx]
            .declarations
            .iter()
            .find_map(|d| d.background_color)
            .expect("Expected background-color declaration")
    };

    assert_eq!(
        get_bg(0),
        Rgba {
            r: 240,
            g: 248,
            b: 255,
            a: 255
        }
    );
    assert_eq!(
        get_bg(1),
        Rgba {
            r: 102,
            g: 51,
            b: 153,
            a: 255
        }
    );
    assert_eq!(
        get_bg(2),
        Rgba {
            r: 128,
            g: 128,
            b: 128,
            a: 255
        }
    );
    assert_eq!(get_bg(2), get_bg(3));
    assert_eq!(
        get_bg(4),
        Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 0
        }
    );
}

#[test]
fn test_rgb_rgba_comma_syntax() {
    let css = r#"
        .a { background-color: rgb(255, 0, 128); }
        .b { background-color: rgb(100%, 0%, 0%); }
        .c { background-color: rgba(255, 0, 0, 0.5); }
        .d { background-color: rgba(0%, 50%, 100%, 25%); }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 4);

    let get_bg = |idx: usize| -> crate::style::Rgba {
        stylesheet.rules[idx]
            .declarations
            .iter()
            .find_map(|d| d.background_color)
            .expect("Expected background-color declaration")
    };

    assert_eq!(
        get_bg(0),
        crate::style::Rgba {
            r: 255,
            g: 0,
            b: 128,
            a: 255
        }
    );
    assert_eq!(
        get_bg(1),
        crate::style::Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        }
    );
    assert_eq!(
        get_bg(2),
        crate::style::Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 128
        }
    );
    assert_eq!(
        get_bg(3),
        crate::style::Rgba {
            r: 0,
            g: 128,
            b: 255,
            a: 64
        }
    );
}

#[test]
fn test_hsl_hsla_parsing_stylesheet() {
    let css = r#"
        .a { background-color: hsl(0 100% 50%); }
        .b { background-color: hsl(240 100% 50% / 50%); }
        .c { background-color: hsl(120, 100%, 50%); }
        .d { background-color: hsla(240, 100%, 50%, 0.5); }
        .e { background-color: hsl(0.5turn 100% 50%); }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 5);

    let get_bg = |idx: usize| -> Rgba {
        stylesheet.rules[idx]
            .declarations
            .iter()
            .find_map(|d| d.background_color)
            .expect("Expected background-color declaration")
    };

    assert_eq!(
        get_bg(0),
        Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        }
    );
    assert_eq!(
        get_bg(1),
        Rgba {
            r: 0,
            g: 0,
            b: 255,
            a: 128
        }
    );
    assert_eq!(
        get_bg(2),
        Rgba {
            r: 0,
            g: 255,
            b: 0,
            a: 255
        }
    );
    assert_eq!(
        get_bg(3),
        Rgba {
            r: 0,
            g: 0,
            b: 255,
            a: 128
        }
    );
    // 0.5turn == 180deg -> cyan
    assert_eq!(
        get_bg(4),
        Rgba {
            r: 0,
            g: 255,
            b: 255,
            a: 255
        }
    );
}

#[test]
fn test_hsl_none_components_stylesheet() {
    let css = r#"
        .hue_none { background-color: hsl(none 100% 50%); }
        .sat_none { background-color: hsl(120 none 50%); }
        .light_none { background-color: hsl(120 100% none); }
        .alpha_none { background-color: hsl(0 100% 50% / none); }
        .all_none { background-color: hsla(none none none / none); }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 5);

    let get_bg = |idx: usize| -> Option<Rgba> {
        stylesheet.rules[idx]
            .declarations
            .iter()
            .find_map(|d| d.background_color)
    };

    // hue none -> treated as 0deg (red at 100%/50%)
    assert_eq!(
        get_bg(0),
        Some(Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        })
    );

    // saturation none -> treated as 0 (grayscale at 50% lightness)
    assert_eq!(
        get_bg(1),
        Some(Rgba {
            r: 128,
            g: 128,
            b: 128,
            a: 255
        })
    );

    // lightness none -> treated as 0 (black)
    assert_eq!(
        get_bg(2),
        Some(Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255
        })
    );

    // alpha none -> treated as 1 (opaque)
    assert_eq!(
        get_bg(3),
        Some(Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        })
    );

    // all none -> treated as black, opaque
    assert_eq!(
        get_bg(4),
        Some(Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255
        })
    );
}

#[test]
fn test_hsl_legacy_rejects_none_in_stylesheet() {
    let css = r#"
        .bad { background-color: hsl(0, none, 50%); }
        .ok { background-color: hsl(0 100% 50%); }
    "#;

    // Should not panic; invalid declaration should be skipped.
    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 2);

    let bad_bg = stylesheet.rules[0]
        .declarations
        .iter()
        .find_map(|d| d.background_color);
    assert!(bad_bg.is_none(), "Expected invalid hsl() to be skipped");

    let ok_bg = stylesheet.rules[1]
        .declarations
        .iter()
        .find_map(|d| d.background_color)
        .expect("Expected background-color declaration");
    assert_eq!(
        ok_bg,
        Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        }
    );
}

#[test]
fn test_hwb_parsing_stylesheet() {
    let css = r#"
        .a { background-color: hwb(0 0% 0%); }
        .b { background-color: hwb(120 0% 0%); }
        .c { background-color: hwb(240 0% 0% / 0.5); }
        .d { background-color: hwb(0 100% 0%); }
        .e { background-color: hwb(0 0% 100%); }
        .f { background-color: hwb(45 40% 80%); }
        .g { background-color: hwb(0.5turn 0% 0%); }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 7);

    let get_bg = |idx: usize| -> Option<Rgba> {
        stylesheet.rules[idx]
            .declarations
            .iter()
            .find_map(|d| d.background_color)
    };

    assert_eq!(
        get_bg(0),
        Some(Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        })
    );
    assert_eq!(
        get_bg(1),
        Some(Rgba {
            r: 0,
            g: 255,
            b: 0,
            a: 255
        })
    );
    assert_eq!(
        get_bg(2),
        Some(Rgba {
            r: 0,
            g: 0,
            b: 255,
            a: 128
        })
    );
    assert_eq!(
        get_bg(3),
        Some(Rgba {
            r: 255,
            g: 255,
            b: 255,
            a: 255
        })
    );
    assert_eq!(
        get_bg(4),
        Some(Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255
        })
    );
    // 45deg, 40% white + 80% black -> achromatic gray: 40 / (40 + 80) = 1/3
    assert_eq!(
        get_bg(5),
        Some(Rgba {
            r: 85,
            g: 85,
            b: 85,
            a: 255
        })
    );
    // 0.5turn == 180deg -> cyan
    assert_eq!(
        get_bg(6),
        Some(Rgba {
            r: 0,
            g: 255,
            b: 255,
            a: 255
        })
    );
}

#[test]
fn test_hwb_none_components_stylesheet() {
    let css = r#"
        .hue_none { background-color: hwb(none 0% 0%); }
        .white_none { background-color: hwb(0 none 100%); }
        .black_none { background-color: hwb(0 100% none); }
        .alpha_none { background-color: hwb(240 0% 0% / none); }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 4);

    let get_bg = |idx: usize| -> Option<Rgba> {
        stylesheet.rules[idx]
            .declarations
            .iter()
            .find_map(|d| d.background_color)
    };

    assert_eq!(
        get_bg(0),
        Some(Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        })
    );
    assert_eq!(
        get_bg(1),
        Some(Rgba {
            r: 0,
            g: 0,
            b: 0,
            a: 255
        })
    );
    assert_eq!(
        get_bg(2),
        Some(Rgba {
            r: 255,
            g: 255,
            b: 255,
            a: 255
        })
    );
    assert_eq!(
        get_bg(3),
        Some(Rgba {
            r: 0,
            g: 0,
            b: 255,
            a: 255
        })
    );
}

#[test]
fn test_hwb_rejects_commas_in_stylesheet() {
    let css = r#"
        .bad { background-color: hwb(0, 0%, 0%); }
        .ok { background-color: hwb(0 0% 0%); }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 2);

    let bad_bg = stylesheet.rules[0]
        .declarations
        .iter()
        .find_map(|d| d.background_color);
    assert!(
        bad_bg.is_none(),
        "Expected comma-separated hwb() to be skipped"
    );

    let ok_bg = stylesheet.rules[1]
        .declarations
        .iter()
        .find_map(|d| d.background_color)
        .expect("Expected background-color declaration");
    assert_eq!(
        ok_bg,
        Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        }
    );
}

#[test]
fn test_parse_lengths() {
    let css = r#"
        .length-test {
            width: 100px;
            height: 50em;
            margin: 10px 20px 30px 40px;
            padding: 5%;
        }

        .auto-test {
            width: auto;
            height: auto;
        }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 2);

    // All rules should parse successfully
    for rule in &stylesheet.rules {
        assert!(!rule.declarations.is_empty());
    }
}

#[test]
fn test_parse_invalid_css_gracefully() {
    let css = r#"
        .valid {
            display: flex;
            background-color: red;
        }

        .invalid {
            unknown-property: some-value;
            display: invalid-value;
        }

        .also-valid {
            width: 100px;
        }
    "#;

    // Should not panic, even with invalid properties
    let result = parse_css(css);
    assert!(result.is_ok());

    let stylesheet = result.unwrap();
    // Should still parse the valid rules
    assert!(!stylesheet.rules.is_empty());
}

#[test]
fn test_complex_selectors() {
    let css = r#"
        .main-container {
            display: flex;
            justify-content: space-evenly;
            align-content: space-around;
        }

        div {
            background-color: blue;
        }

        .sidebar {
            flex-basis: 200px;
            align-self: flex-end;
        }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 3);

    assert_eq!(
        stylesheet.rules[0].selector,
        Selector::Class("main-container".to_string())
    );
    assert_eq!(
        stylesheet.rules[1].selector,
        Selector::Tag("div".to_string())
    );
    assert_eq!(
        stylesheet.rules[2].selector,
        Selector::Class("sidebar".to_string())
    );
}

#[test]
fn test_empty_css() {
    let css = "";
    let stylesheet = parse_css(css).expect("Failed to parse empty CSS");
    assert_eq!(stylesheet.rules.len(), 0);
}

#[test]
fn test_whitespace_only_css() {
    let css = "   \n\t  \n  ";
    let stylesheet = parse_css(css).expect("Failed to parse whitespace CSS");
    assert_eq!(stylesheet.rules.len(), 0);
}

#[test]
fn test_parse_box_sizing() {
    let css = r#"
        .a { box-sizing: border-box; }
        .b { box-sizing: content-box; }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 2);

    let a = &stylesheet.rules[0];
    assert_eq!(a.selector, Selector::Class("a".to_string()));
    assert!(a
        .declarations
        .iter()
        .any(|d| d.box_sizing == Some(BoxSizing::BorderBox)));

    let b = &stylesheet.rules[1];
    assert_eq!(b.selector, Selector::Class("b".to_string()));
    assert!(b
        .declarations
        .iter()
        .any(|d| d.box_sizing == Some(BoxSizing::ContentBox)));
}

#[test]
fn test_single_rule_css() {
    let css = ".single { display: flex; }";
    let stylesheet = parse_css(css).expect("Failed to parse single rule CSS");
    assert_eq!(stylesheet.rules.len(), 1);
    assert_eq!(
        stylesheet.rules[0].selector,
        Selector::Class("single".to_string())
    );
}

#[test]
fn test_border_radius_parsing() {
    let css = r#"
        .simple-radius {
            border-radius: 10px;
        }

        .complex-radius {
            border-radius: 5px 10px 15px 20px;
        }

        .mixed-units {
            border-radius: 1em 50% 2px;
        }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse border-radius CSS");
    assert_eq!(stylesheet.rules.len(), 3);

    // All rules should parse successfully
    for rule in &stylesheet.rules {
        assert!(!rule.declarations.is_empty());
        // Check that at least one declaration has border_radius
        let has_border_radius = rule
            .declarations
            .iter()
            .any(|decl| decl.border_radius.is_some());
        assert!(has_border_radius, "Rule should have border-radius property");
    }
}

#[test]
fn test_parse_margin_sides() {
    let css = r#"
        .m {
            margin-left: 20px;
            margin-top: 10px;
            margin-right: 5px;
            margin-bottom: 15px;
        }

        .auto_left {
            margin-left: auto;
        }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 2);

    let m_rule = &stylesheet.rules[0];
    assert_eq!(m_rule.selector, Selector::Class("m".to_string()));
    assert!(m_rule
        .declarations
        .iter()
        .any(|d| matches!(d.margin_left, Some(Length::Px(20.0)))));
    assert!(m_rule
        .declarations
        .iter()
        .any(|d| matches!(d.margin_top, Some(Length::Px(10.0)))));
    assert!(m_rule
        .declarations
        .iter()
        .any(|d| matches!(d.margin_right, Some(Length::Px(5.0)))));
    assert!(m_rule
        .declarations
        .iter()
        .any(|d| matches!(d.margin_bottom, Some(Length::Px(15.0)))));

    let auto_rule = &stylesheet.rules[1];
    assert_eq!(auto_rule.selector, Selector::Class("auto_left".to_string()));
    assert!(auto_rule
        .declarations
        .iter()
        .any(|d| matches!(d.margin_left, Some(Length::Auto))));
}
