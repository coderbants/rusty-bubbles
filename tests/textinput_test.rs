//! Cleanroom Rust port of upstream Go source file: `textinput/textinput_test.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! Text input suggestion and view-slicing behavior. The upstream test calls
//! the private `updateSuggestions`/`nextSuggestion` helpers directly; here
//! the same flow is driven through the public `update` with the suggestion
//! key, which is what the upstream `Update` handler does.

use charming_bubbles::textinput;
use charming_bubbletea::key::{Key, KeyMod, KeyPressMsg};
use charming_bubbletea::model::Msg;

fn suggestion_next() -> Box<dyn Msg> {
    Box::new(KeyPressMsg(Key::new(
        charming_bubbletea::key::KEY_DOWN,
        "",
        KeyMod::default(),
    )))
}

#[test]
fn test_current_suggestion() {
    let mut m = textinput::new();
    m.show_suggestions = true;

    let suggestion = m.current_suggestion();
    assert_eq!(suggestion, "", "expected no current suggestion but was {suggestion}");

    m.set_suggestions(&["test1".to_string(), "test2".to_string(), "test3".to_string()]);
    let suggestion = m.current_suggestion();
    assert_eq!(suggestion, "", "expected no current suggestion but was {suggestion}");

    m.set_value("test");
    // Drive the suggestion flow like the upstream Update handler: focus and
    // press the "next suggestion" key (down / ctrl+n). Update recomputes the
    // matched suggestions and advances to the next one.
    m.focus();
    // First press populates the matched suggestions (and resets the index),
    // the second advances to the next suggestion — mirroring the upstream
    // `updateSuggestions` + `nextSuggestion` sequence.
    m.update(&*suggestion_next());
    m.update(&*suggestion_next());
    let suggestion = m.current_suggestion();
    assert_eq!(suggestion, "test2", "expected first suggestion but was {suggestion}");

    m.blur();
    let view = m.view();
    assert!(
        !view.ends_with("test2"),
        "suggestions should not be rendered when input isn't focused. expected \"> test\" but got \"{view}\""
    );
}

#[test]
fn test_slicing_outside_cap() {
    let mut m = textinput::new();
    m.placeholder = "作業ディレクトリを指定してください".to_string();
    m.set_width(32);
    let _ = m.view(); // ensure no panic
}

#[test]
fn test_chinese_placeholder() {
    // Upstream skips this as flaky (the returned view seems incorrect).
    let mut m = textinput::new();
    m.placeholder = "输入消息...".to_string();
    m.set_width(20);
    let _ = m.view(); // ensure no panic
}

#[test]
fn test_placeholder_truncate() {
    // Upstream skips this as flaky (the returned view seems incorrect).
    let mut m = textinput::new();
    m.placeholder = "A very long placeholder, or maybe not so much".to_string();
    m.set_width(10);
    let _ = m.view(); // ensure no panic
}
