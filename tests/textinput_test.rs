//! Cleanroom Rust port of upstream Go source file: `textinput/textinput_test.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! Text input suggestion and view-slicing behavior. The upstream test calls
//! the private `updateSuggestions`/`nextSuggestion` helpers directly; here
//! the same flow is driven through the public `update` with the suggestion
//! key, which is what the upstream `Update` handler does.

use rusty_bubbles::textinput;
use rusty_bubbletea::key::{Key, KeyMod, KeyPressMsg};
use rusty_bubbletea::model::Msg;

fn suggestion_next() -> Box<dyn Msg> {
    Box::new(KeyPressMsg(Key::new(
        rusty_bubbletea::key::KEY_DOWN,
        "",
        KeyMod::default(),
    )))
}

#[test]
fn test_current_suggestion() {
    let mut m = textinput::new();
    m.show_suggestions = true;

    let suggestion = m.current_suggestion();
    assert_eq!(
        suggestion, "",
        "expected no current suggestion but was {suggestion}"
    );

    m.set_suggestions(&[
        "test1".to_string(),
        "test2".to_string(),
        "test3".to_string(),
    ]);
    let suggestion = m.current_suggestion();
    assert_eq!(
        suggestion, "",
        "expected no current suggestion but was {suggestion}"
    );

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
    assert_eq!(
        suggestion, "test2",
        "expected first suggestion but was {suggestion}"
    );

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

#[test]
fn test_textinput_editing_and_navigation() {
    let mut m = textinput::new();
    m.focus();
    m.set_value("hello world");
    assert_eq!(m.value(), "hello world");
    assert_eq!(m.position(), 11);

    // Cursor navigation
    m.cursor_start();
    assert_eq!(m.position(), 0);
    m.cursor_end();
    assert_eq!(m.position(), 11);

    // Word backward / forward via key update
    m.set_cursor(11);
    m.update(&KeyPressMsg(Key::new('b', "alt+b", KeyMod::default())));
    assert_eq!(m.position(), 6);

    m.update(&KeyPressMsg(Key::new('f', "alt+f", KeyMod::default())));
    assert_eq!(m.position(), 11);

    // Delete character backward via key update
    m.update(&KeyPressMsg(Key::new(
        rusty_bubbletea::key::KEY_BACKSPACE,
        "backspace",
        KeyMod::default(),
    )));
    assert_eq!(m.value(), "hello worl");

    // Password echo mode
    m.echo_mode = textinput::EchoMode::EchoPassword;
    m.echo_character = '*';
    // Word deletion backward & forward
    m.echo_mode = textinput::EchoMode::EchoNormal;
    m.set_value("first second third");
    m.set_cursor(12);
    m.update(&KeyPressMsg(Key::new('w', "ctrl+w", KeyMod::default())));
    assert_eq!(m.value(), "first  third");

    m.set_value("first second third");
    m.set_cursor(6);
    m.update(&KeyPressMsg(Key::new('d', "alt+d", KeyMod::default())));
    assert_eq!(m.value(), "first  third");

    // Line delete after & before cursor
    m.set_value("alpha beta gamma");
    m.set_cursor(6);
    m.update(&KeyPressMsg(Key::new('k', "ctrl+k", KeyMod::default())));
    assert_eq!(m.value(), "alpha ");

    m.update(&KeyPressMsg(Key::new('u', "ctrl+u", KeyMod::default())));
    assert_eq!(m.value(), "");

    // Delete character forward
    m.set_value("testing");
    m.set_cursor(0);
    m.update(&KeyPressMsg(Key::new('d', "ctrl+d", KeyMod::default())));
    assert_eq!(m.value(), "esting");

    // Paste messages
    m.update(&rusty_bubbletea::paste::PasteMsg {
        content: " 123".to_string(),
    });
    assert_eq!(m.value(), " 123esting");

    // EchoMode::EchoNone
    m.echo_mode = textinput::EchoMode::EchoNone;
    assert!(rusty_x_ansi::util::strip(&m.view()).starts_with("> "));

    // Validation
    m.validate = Some(Box::new(|val| {
        if val.len() < 3 {
            Err("too short".to_string())
        } else {
            Ok(())
        }
    }));
    m.set_value("ab");
    assert_eq!(m.err, Some("too short".to_string()));
    m.set_value("abc");
    assert_eq!(m.err, None);
}
