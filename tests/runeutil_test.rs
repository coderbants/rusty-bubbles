//! Cleanroom Rust port of upstream Go source file: `internal/runeutil/runeutil_test.go`
//! Upstream Target Tag / Version: `v2.1.0`

use rusty_bubbles::internal::runeutil::{self, Sanitizer};

#[test]
fn test_sanitize() {
    let td: Vec<(&str, &str)> = vec![
        ("", ""),
        ("x", "x"),
        ("\n", "XX"),
        ("\na\n", "XXaXX"),
        ("\n\n", "XXXX"),
        ("\t", ""),
        ("hello", "hello"),
        ("hel\nlo", "helXXlo"),
        ("hel\rlo", "helXXlo"),
        ("hel\tlo", "hello"),
        ("he\n\nl\tlo", "heXXXXllo"),
        ("he\tl\n\nlo", "helXXXXlo"),
        ("hel\x1blo", "hello"),
    ];

    for (input, output) in td {
        let runes: Vec<char> = input.chars().collect();
        let s = runeutil::new_sanitizer(vec![
            runeutil::replace_newlines("XX"),
            runeutil::replace_tabs(""),
        ]);
        let result = s.sanitize(&runes);
        let rs: String = result.iter().collect();
        assert_eq!(rs, output, "input: {:?}", input);
    }
}
