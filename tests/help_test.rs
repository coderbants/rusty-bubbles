//! Cleanroom Rust port of upstream Go source file: `help/help_test.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! Full-help view rendering across several widths, compared against golden
//! files (ported from the upstream `x/exp/golden` helper).

mod common;

use charming_bubbles::help;
use charming_bubbles::key;
use charming_x_ansi::util;

#[test]
fn test_full_help() {
    let mut m = help::new();
    m.full_separator = " | ".to_string();
    let kb: Vec<Vec<key::Binding>> = vec![
        vec![key::new_binding(vec![
            key::with_keys(&["x"]),
            key::with_help("enter", "continue"),
        ])],
        vec![
            key::new_binding(vec![key::with_keys(&["x"]), key::with_help("esc", "back")]),
            key::new_binding(vec![key::with_keys(&["x"]), key::with_help("?", "help")]),
        ],
        vec![
            key::new_binding(vec![key::with_keys(&["x"]), key::with_help("H", "home")]),
            key::new_binding(vec![
                key::with_keys(&["x"]),
                key::with_help("ctrl+c", "quit"),
            ]),
            key::new_binding(vec![
                key::with_keys(&["x"]),
                key::with_help("ctrl+l", "log"),
            ]),
        ],
    ];

    for w in [20usize, 30, 40] {
        m.set_width(w);
        let s = m.full_help_view(&kb);
        let s = util::strip(&s);
        common::assert_golden(&s, &format!("TestFullHelp/full_help_{w}_width.golden"));
    }
}
