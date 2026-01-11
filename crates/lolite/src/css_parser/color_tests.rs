use crate::css_parser::parse_css;
use crate::style::Rgba;

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
fn test_text_color_property_parsing() {
    let css = r#"
        .a { color: red; }
        .b { color: rgba(0, 255, 0, 0.5); }
    "#;

    let stylesheet = parse_css(css).expect("Failed to parse CSS");
    assert_eq!(stylesheet.rules.len(), 2);

    let get_color = |idx: usize| -> Rgba {
        stylesheet.rules[idx]
            .declarations
            .iter()
            .find_map(|d| d.color)
            .expect("Expected color declaration")
    };

    assert_eq!(
        get_color(0),
        Rgba {
            r: 255,
            g: 0,
            b: 0,
            a: 255
        }
    );

    assert_eq!(
        get_color(1),
        Rgba {
            r: 0,
            g: 255,
            b: 0,
            a: 128
        }
    );
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
