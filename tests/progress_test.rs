//! Cleanroom Rust port of upstream Go source file: `progress/progress_test.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! Progress-bar blending and scaling tests, compared against golden files
//! (ported from the upstream `x/exp/golden` helper).

mod common;

use charming_bubbles::progress;
use charming_bubbles::progress::{with_color_func, with_colors, with_fill_characters, with_scaled, without_percentage};
use charming_lipgloss::Color;

#[test]
fn test_blend() {
    let cases: Vec<(&str, Vec<progress::Option>, usize, f64)> = vec![
        (
            "10w-red-to-green-50perc",
            vec![
                with_colors(&[Color::parse("#FF0000"), Color::parse("#00FF00")]),
                with_scaled(false),
                without_percentage(),
            ],
            10,
            0.5,
        ),
        (
            "10w-red-to-green-50perc-full-block",
            vec![
                with_colors(&[Color::parse("#FF0000"), Color::parse("#00FF00")]),
                with_fill_characters('█', progress::DEFAULT_EMPTY_CHAR_BLOCK),
                without_percentage(),
            ],
            10,
            0.5,
        ),
        (
            "30w-red-to-green-100perc",
            vec![
                with_colors(&[Color::parse("#FF0000"), Color::parse("#00FF00")]),
                with_scaled(false),
                without_percentage(),
            ],
            30,
            1.0,
        ),
        (
            "10w-red-to-green-scaled-50perc",
            vec![
                with_colors(&[Color::parse("#FF0000"), Color::parse("#00FF00")]),
                with_scaled(true),
                without_percentage(),
            ],
            10,
            0.5,
        ),
        (
            "30w-red-to-green-scaled-100perc",
            vec![
                with_colors(&[Color::parse("#FF0000"), Color::parse("#00FF00")]),
                with_scaled(true),
                without_percentage(),
            ],
            30,
            1.0,
        ),
        (
            "30w-colorfunc-rgb-100perc",
            vec![
                with_color_func(Box::new(|_, current: f64| -> Color {
                    if current <= 0.3 {
                        Color::parse("#FF0000")
                    } else if current <= 0.7 {
                        Color::parse("#00FF00")
                    } else {
                        Color::parse("#0000FF")
                    }
                })),
                without_percentage(),
            ],
            30,
            1.0,
        ),
    ];

    for (name, options, width, percent) in cases {
        let mut p = progress::new(options);
        p.set_width(width);
        common::assert_golden(&p.view_as(percent), &format!("TestBlend/{name}.golden"));
    }
}
