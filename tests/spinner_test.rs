//! Cleanroom Rust port of upstream Go source file: `spinner/spinner_test.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! Spinner factory and preset equality tests.

use charming_bubbles::spinner;

fn assert_equal_spinner(exp: &spinner::Spinner, got: &spinner::Spinner) {
    assert_eq!(
        exp.fps,
        got.fps,
        "expecting {} FPS, got {}",
        exp.fps.as_millis(),
        got.fps.as_millis()
    );
    assert_eq!(
        exp.frames.len(),
        got.frames.len(),
        "expecting {} frames, got {}",
        exp.frames.len(),
        got.frames.len()
    );
    for (i, e) in exp.frames.iter().enumerate() {
        let g = &got.frames[i];
        assert_eq!(e, g, "expecting frame index {i} with value {e:?}, got {g:?}");
    }
}

#[test]
fn test_spinner_new() {
    let s = spinner::new(vec![]);
    assert_equal_spinner(&spinner::line(), &s.spinner);

    let custom_spinner = spinner::Spinner {
        frames: vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()],
        fps: std::time::Duration::from_millis(16),
    };
    let s = spinner::new(vec![spinner::with_spinner(custom_spinner.clone())]);
    assert_equal_spinner(&custom_spinner, &s.spinner);

    let tests: Vec<(&str, spinner::Spinner)> = vec![
        ("Line", spinner::line()),
        ("Dot", spinner::dot()),
        ("MiniDot", spinner::mini_dot()),
        ("Jump", spinner::jump()),
        ("Pulse", spinner::pulse()),
        ("Points", spinner::points()),
        ("Globe", spinner::globe()),
        ("Moon", spinner::moon()),
        ("Monkey", spinner::monkey()),
    ];
    for (_name, s_spin) in tests {
        assert_equal_spinner(&spinner::new(vec![spinner::with_spinner(s_spin.clone())]).spinner, &s_spin);
    }
}
