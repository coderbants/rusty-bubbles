//! Cleanroom Rust port of upstream Go source file: `textarea/textarea_test.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! TextArea rendering, navigation, word handling, dynamic height and
//! max-content-height behavior. The upstream tests are white-box; where the
//! Rust port keeps internals private, assertions are adapted to the public
//! API (`cursor_position()`, `set_cursor_position()`,
//! `set_scroll_y_offset()`, `total_visual_lines()`).
//!
//! Divergences from the upstream suite (all documented at the call site):
//! - `Update(nil)` has no Rust equivalent; an inert message
//!   (`update_nil()`) is used instead — it matches no update case and only
//!   runs the unconditional tail (`recalculate_height`, viewport content
//!   refresh, `reposition_view`), exactly like the Go nil message.
//! - `TestVerticalScrolling` drives the private viewport through
//!   `cursor_up()`/`cursor_down()` (which scroll the viewport to keep the
//!   cursor visible) instead of `viewport.GotoTop()`/`ScrollDown(1)`.

mod common;

use rusty_bubbles::textarea;
use rusty_bubbletea::key::{Key, KeyMod, KeyPressMsg};
use rusty_bubbletea::model::Msg;
use rusty_bubbletea::paste::PasteMsg;

/// Port of the upstream `newTextArea()` helper.
fn new_text_area() -> textarea::Model {
    let mut m = textarea::new();
    m.prompt = "> ".to_string();
    m.placeholder = "Hello, World!".to_string();
    m.focus();
    m
}

/// Port of the upstream `newDynamicTextArea(minH, maxH)` helper.
fn new_dynamic_text_area(min_h: usize, max_h: usize) -> textarea::Model {
    let mut ta = textarea::new();
    ta.prompt = String::new();
    ta.show_line_numbers = false;
    ta.dynamic_height = true;
    ta.min_height = min_h;
    ta.max_height = max_h;
    ta.set_width(20);
    ta.focus();
    ta
}

/// Port of the upstream `keyPress(key rune)` helper.
fn key_press(c: char) -> Box<dyn Msg> {
    Box::new(KeyPressMsg(Key::new(c, &c.to_string(), KeyMod::default())))
}

/// A key press message for a special key (no text), e.g. `tea.KeyDown`.
fn key_code(code: char) -> Box<dyn Msg> {
    Box::new(KeyPressMsg(Key::new(code, "", KeyMod::default())))
}

/// Port of the upstream `sendString(m, str)` helper.
fn send_string(m: &mut textarea::Model, s: &str) {
    for c in s.chars() {
        m.update(&*key_press(c));
    }
}

/// Port of the upstream `Update(nil)` call: an inert message that matches no
/// update case, running only the unconditional tail (height recalculation,
/// viewport content refresh and repositioning). The `Msg` trait has a
/// blanket impl, so this needs no manual impl.
#[derive(Debug)]
struct InertMsg;

fn update_nil(m: &mut textarea::Model) {
    m.update(&InertMsg);
}

/// Port of the upstream `stripString(str)` helper: strip ANSI sequences,
/// trim trailing whitespace per line and drop empty lines.
fn strip_string(s: &str) -> String {
    let s = rusty_x_ansi::util::strip(s);
    let mut lines = vec![];
    for l in s.split('\n') {
        let trimmed = l.trim_end();
        if !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }
    }
    lines.join("\n")
}

#[test]
fn test_vertical_scrolling() {
    let mut m = new_text_area();
    m.prompt = String::new();
    m.show_line_numbers = false;
    m.set_height(1);
    m.set_width(20);
    m.char_limit = 100;

    update_nil(&mut m);

    let input = "This is a really long line that should wrap around the text area.";

    for c in input.chars() {
        m.update(&*key_press(c));
    }

    let view = m.view();

    // The view should contain the end of "line" of the input.
    assert!(
        view.contains("the text area."),
        "Text area did not render the input:\n{view}"
    );

    // But we should be able to scroll to see the next line.
    // Let's scroll to the top for each line to view the full input.
    // (Upstream drives the private viewport with GotoTop()/ScrollDown(1);
    // here cursor_up()/cursor_down() move the cursor and scroll the
    // viewport to keep it visible, so each wrapped line must appear in the
    // view after the corresponding movement.)
    let lines = [
        "This is a really",
        "long line that",
        "should wrap around",
        "the text area.",
    ];

    // GotoTop: move the cursor up to the first wrapped line.
    for _ in 0..3 {
        m.cursor_up();
    }
    for line in lines {
        // (The cursor renders ANSI around its character here, since this
        // adaptation moves the cursor onto the checked line — unlike
        // upstream, where the cursor stays on the last line — so the view
        // is stripped before the substring check.)
        let view = rusty_x_ansi::util::strip(&m.view());
        assert!(
            view.contains(line),
            "Text area did not render the correct scrolled input:\n{view}"
        );
        m.cursor_down();
    }
}

#[test]
fn test_word_wrap_overflowing() {
    // An interesting edge case is when the user enters many words that fill
    // up the text area and then goes back up and inserts a few words which
    // causes a cascading wrap and causes an overflow of the last line.
    //
    // In this case, we should not let the user insert more words if, after
    // the entire wrap is complete, the last line is overflowing.
    let mut m = new_text_area();

    m.set_height(3);
    m.set_width(20);
    m.char_limit = 500;

    update_nil(&mut m);

    let input = "Testing Testing Testing Testing Testing Testing Testing Testing";

    for c in input.chars() {
        m.update(&*key_press(c));
        let _ = m.view();
    }

    // We have essentially filled the text area with input.
    // Let's see if we can cause wrapping to overflow the last line.
    m.set_cursor_position(0, 0);

    let input = "Testing";

    for c in input.chars() {
        m.update(&*key_press(c));
        let _ = m.view();
    }

    let last_line_width = m.line_info().width;
    assert!(
        last_line_width <= 20,
        "last line width {} exceeds 20:\n{}",
        last_line_width,
        m.view()
    );
}

#[test]
fn test_value_soft_wrap() {
    let mut m = new_text_area();
    m.set_width(16);
    m.set_height(10);
    m.char_limit = 500;

    update_nil(&mut m);

    let input = "Testing Testing Testing Testing Testing Testing Testing Testing";

    for c in input.chars() {
        m.update(&*key_press(c));
        let _ = m.view();
    }

    let value = m.value();
    assert_eq!(
        value, input,
        "The text area does not have the correct value"
    );
}

#[test]
fn test_set_value() {
    let mut m = new_text_area();
    m.set_value(&["Foo", "Bar", "Baz"].join("\n"));

    let (row, col) = m.cursor_position();
    assert!(
        row == 2 && col == 3,
        "Cursor Should be on row 2 column 3 after inserting 2 new lines (got row {row}, col {col})"
    );

    let value = m.value();
    assert_eq!(value, "Foo\nBar\nBaz", "Value should be Foo\nBar\nBaz");

    // SetValue should reset text area
    m.set_value("Test");
    let value = m.value();
    assert_eq!(
        value, "Test",
        "Text area was not reset when SetValue() was called"
    );
}

#[test]
fn test_insert_string() {
    let mut m = new_text_area();

    // Insert some text
    let input = "foo baz";

    for c in input.chars() {
        m.update(&*key_press(c));
    }

    // Put cursor in the middle of the text
    m.set_cursor_position(0, 4);

    m.insert_string("bar ");

    let value = m.value();
    assert_eq!(
        value, "foo bar baz",
        "Expected insert string to insert bar between foo and baz"
    );
}

#[test]
fn test_can_handle_emoji() {
    let mut m = new_text_area();
    let input = "🧋";

    for c in input.chars() {
        m.update(&*key_press(c));
    }

    let value = m.value();
    assert_eq!(value, input, "Expected emoji to be inserted");

    let input = "🧋🧋🧋";

    m.set_value(input);

    let value = m.value();
    assert_eq!(value, input, "Expected emoji to be inserted");

    let (_, col) = m.cursor_position();
    assert_eq!(col, 3, "Expected cursor to be on the third character");

    let char_offset = m.line_info().char_offset;
    assert_eq!(
        char_offset, 6,
        "Expected cursor to be on the sixth character"
    );
}

#[test]
fn test_vertical_navigation_keeps_cursor_horizontal_position() {
    let mut m = new_text_area();
    m.set_width(20);

    m.set_value(&["你好你好", "Hello"].join("\n"));

    m.set_cursor_position(0, 2);

    // 你好|你好
    // Hell|o
    // 1234|

    // Let's imagine our cursor is on the first line where the pipe is.
    // We press the down arrow to get to the next line.
    // The issue is that if we keep the cursor on the same column, the
    // cursor will jump to after the `e`.
    //
    // 你好|你好
    // He|llo
    //
    // But this is wrong because visually we were at the 4th character due
    // to the first line containing double-width runes.
    // We want to keep the cursor on the same visual column.
    //
    // 你好|你好
    // Hell|o
    //
    // This test ensures that the cursor is kept on the same visual column
    // by ensuring that the column offset goes from 2 -> 4.

    let line_info = m.line_info();
    assert!(
        line_info.char_offset == 4 && line_info.column_offset == 2,
        "Expected cursor to be on the fourth character because there are two double width runes on the first line (char_offset {}, column_offset {})",
        line_info.char_offset,
        line_info.column_offset
    );

    m.update(&*key_code(rusty_bubbletea::key::KEY_DOWN));

    let line_info = m.line_info();
    assert!(
        line_info.char_offset == 4 && line_info.column_offset == 4,
        "Expected cursor to be on the fourth character because we came down from the first line (char_offset {}, column_offset {})",
        line_info.char_offset,
        line_info.column_offset
    );
}

#[test]
fn test_vertical_navigation_should_remember_position_while_traversing() {
    let mut m = new_text_area();
    m.set_width(40);

    // Let's imagine we have a text area with the following content:
    //
    // Hello
    // World
    // This is a long line.
    //
    // If we are at the end of the last line and go up, we should be at the
    // end of the second line.
    // And, if we go up again we should be at the end of the first line.
    // But, if we go back down twice, we should be at the end of the last
    // line again and not the fifth (length of second line) character of
    // the last line.
    //
    // In other words, we should remember the last horizontal position
    // while traversing vertically.

    m.set_value(&["Hello", "World", "This is a long line."].join("\n"));

    // We are at the end of the last line.
    let (row, col) = m.cursor_position();
    assert!(
        col == 20 && row == 2,
        "Expected cursor to be on the 20th character of the last line (got row {row}, col {col})"
    );

    // Let's go up.
    m.update(&*key_code(rusty_bubbletea::key::KEY_UP));

    // We should be at the end of the second line.
    let (row, col) = m.cursor_position();
    assert!(
        col == 5 && row == 1,
        "Expected cursor to be on the 5th character of the second line (got row {row}, col {col})"
    );

    // And, again.
    m.update(&*key_code(rusty_bubbletea::key::KEY_UP));

    // We should be at the end of the first line.
    let (row, col) = m.cursor_position();
    assert!(
        col == 5 && row == 0,
        "Expected cursor to be on the 5th character of the first line (got row {row}, col {col})"
    );

    // Let's go down, twice.
    m.update(&*key_code(rusty_bubbletea::key::KEY_DOWN));
    m.update(&*key_code(rusty_bubbletea::key::KEY_DOWN));

    // We should be at the end of the last line.
    let (row, col) = m.cursor_position();
    assert!(
        col == 20 && row == 2,
        "Expected cursor to be on the 20th character of the last line (got row {row}, col {col})"
    );

    // Now, for correct behavior, if we move right or left, we should
    // forget (reset) the saved horizontal position. Since we assume the
    // user wants to keep the cursor where it is horizontally. This is how
    // most text areas work.

    m.update(&*key_code(rusty_bubbletea::key::KEY_UP));
    m.update(&*key_code(rusty_bubbletea::key::KEY_LEFT));

    let (row, col) = m.cursor_position();
    assert!(
        col == 4 && row == 1,
        "Expected cursor to be on the 5th character of the second line (got row {row}, col {col})"
    );

    // Going down now should keep us at the 4th column since we moved left
    // and reset the horizontal position saved state.
    m.update(&*key_code(rusty_bubbletea::key::KEY_DOWN));
    let (row, col) = m.cursor_position();
    assert!(
        col == 4 && row == 2,
        "Expected cursor to be on the 4th character of the last line (got row {row}, col {col})"
    );
}

#[test]
fn test_word() {
    let mut m = new_text_area();

    m.set_height(3);
    m.set_width(20);
    m.char_limit = 500;

    update_nil(&mut m);

    {
        // "regular input"
        let input = "Word1 Word2 Word3 Word4";
        for c in input.chars() {
            m.update(&*key_press(c));
            let _ = m.view();
        }

        let expect = "Word4";
        let word = m.word();
        assert_eq!(
            word, expect,
            "Expected last word to be '{expect}', got '{word}'"
        );
    }

    {
        // "navigate"
        let keys: Vec<Box<dyn Msg>> = vec![
            Box::new(KeyPressMsg(Key::new(
                rusty_bubbletea::key::KEY_LEFT,
                "alt+left",
                KeyMod::ALT,
            ))),
            Box::new(KeyPressMsg(Key::new(
                rusty_bubbletea::key::KEY_LEFT,
                "alt+left",
                KeyMod::ALT,
            ))),
            Box::new(KeyPressMsg(Key::new(
                rusty_bubbletea::key::KEY_RIGHT,
                "right",
                KeyMod::default(),
            ))),
        ];
        for k in keys {
            m.update(&*k);
            let _ = m.view();
        }

        let expect = "Word3";
        let word = m.word();
        assert_eq!(
            word, expect,
            "Expected last word to be '{expect}', got '{word}'"
        );
    }

    {
        // "delete"
        let keys: Vec<Box<dyn Msg>> = vec![
            Box::new(KeyPressMsg(Key::new(
                rusty_bubbletea::key::KEY_END,
                "end",
                KeyMod::default(),
            ))),
            Box::new(KeyPressMsg(Key::new(
                rusty_bubbletea::key::KEY_BACKSPACE,
                "alt+backspace",
                KeyMod::ALT,
            ))),
            Box::new(KeyPressMsg(Key::new(
                rusty_bubbletea::key::KEY_BACKSPACE,
                "alt+backspace",
                KeyMod::ALT,
            ))),
            Box::new(KeyPressMsg(Key::new(
                rusty_bubbletea::key::KEY_BACKSPACE,
                "backspace",
                KeyMod::default(),
            ))),
        ];
        for k in keys {
            m.update(&*k);
            let _ = m.view();
        }

        let expect = "Word2";
        let word = m.word();
        assert_eq!(
            word, expect,
            "Expected last word to be '{expect}', got '{word}'"
        );
    }
}

#[test]
fn test_dynamic_height_default_unchanged() {
    let mut ta = new_text_area();
    ta.set_height(6);
    ta.set_width(40);

    for c in "hello\nworld\n".chars() {
        ta.update(&*key_press(c));
    }

    assert_eq!(
        ta.height(),
        6,
        "expected static height 6, got {}",
        ta.height()
    );
}

#[test]
fn test_dynamic_height_grows_on_newline() {
    let mut ta = new_dynamic_text_area(1, 20);

    ta.update(&*key_press('a'));
    assert_eq!(
        ta.height(),
        1,
        "expected height 1 after single char, got {}",
        ta.height()
    );

    ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    assert_eq!(
        ta.height(),
        2,
        "expected height 2 after first newline, got {}",
        ta.height()
    );

    ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    assert_eq!(
        ta.height(),
        3,
        "expected height 3 after second newline, got {}",
        ta.height()
    );
}

#[test]
fn test_dynamic_height_grows_on_soft_wrap() {
    let mut ta = new_dynamic_text_area(1, 20);
    // width=20, so typing >20 chars should cause a soft wrap
    let input = "abcdefghijklmnopqrstuvwxyz";
    for c in input.chars() {
        ta.update(&*key_press(c));
    }

    assert!(
        ta.height() >= 2,
        "expected height >= 2 after soft wrap, got {}",
        ta.height()
    );
}

#[test]
fn test_dynamic_height_shrinks_on_line_deletion() {
    let mut ta = new_dynamic_text_area(1, 20);

    ta.update(&*key_press('a'));
    ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    ta.update(&*key_press('b'));
    ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    ta.update(&*key_press('c'));

    assert_eq!(
        ta.height(),
        3,
        "expected height 3 before deletion, got {}",
        ta.height()
    );

    // Backspace at start of line 3 merges with line 2
    ta.cursor_start();
    ta.update(&*key_code(rusty_bubbletea::key::KEY_BACKSPACE));

    assert_eq!(
        ta.height(),
        2,
        "expected height 2 after line merge, got {}",
        ta.height()
    );
}

#[test]
fn test_dynamic_height_respects_min_height() {
    let mut ta = new_dynamic_text_area(5, 20);

    ta.update(&*key_press('a'));

    assert_eq!(ta.height(), 5, "expected min height 5, got {}", ta.height());
}

#[test]
fn test_dynamic_height_respects_max_height() {
    let mut ta = new_dynamic_text_area(1, 5);

    for _ in 0..10 {
        ta.update(&*key_press('x'));
        ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    }

    assert_eq!(ta.height(), 5, "expected max height 5, got {}", ta.height());
}

#[test]
fn test_dynamic_height_grows_on_paste() {
    let mut ta = new_dynamic_text_area(1, 20);

    ta.update(&PasteMsg {
        content: "line1\nline2\nline3\nline4\nline5".to_string(),
    });

    assert_eq!(
        ta.height(),
        5,
        "expected height 5 after pasting 5 lines, got {}",
        ta.height()
    );
}

#[test]
fn test_dynamic_height_recalculates_on_set_width() {
    let mut ta = new_dynamic_text_area(1, 50);
    ta.set_width(40);

    // Insert a line that fits in 40 cols but wraps in 10 cols
    ta.set_value("abcdefghijklmnopqrstuvwxyz");

    assert_eq!(
        ta.height(),
        1,
        "expected height 1 at width 40, got {}",
        ta.height()
    );

    ta.set_width(10);

    assert!(
        ta.height() >= 3,
        "expected height >= 3 after narrowing to width 10, got {}",
        ta.height()
    );
}

#[test]
fn test_dynamic_height_recalculates_on_set_value() {
    let mut ta = new_dynamic_text_area(1, 20);

    ta.set_value("a\nb\nc\nd\ne");

    assert_eq!(
        ta.height(),
        5,
        "expected height 5 after SetValue with 5 lines, got {}",
        ta.height()
    );
}

#[test]
fn test_dynamic_height_cursor_position_after_grow() {
    let mut ta = new_dynamic_text_area(1, 20);

    for i in 0..5 {
        ta.update(&*key_press(char::from(b'a' + i)));
        ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    }
    ta.update(&*key_press('f'));

    // Cursor should be on the last line (row 5, 0-indexed)
    assert_eq!(ta.line(), 5, "expected cursor on row 5, got {}", ta.line());

    // Cursor visual line should be within the viewport
    let cursor_line = ta.cursor_line_number();
    let min_visible = ta.scroll_y_offset();
    let max_visible = min_visible + ta.height() - 1;
    assert!(
        cursor_line >= min_visible && cursor_line <= max_visible,
        "cursor line {cursor_line} outside viewport [{min_visible}, {max_visible}]"
    );
}

#[test]
fn test_dynamic_height_cursor_position_after_shrink() {
    let mut ta = new_dynamic_text_area(1, 20);

    for i in 0..5 {
        ta.update(&*key_press(char::from(b'a' + i)));
        ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    }
    ta.update(&*key_press('f'));

    assert_eq!(
        ta.height(),
        6,
        "expected height 6 before shrink, got {}",
        ta.height()
    );

    // Delete lines by backspacing
    for _ in 0..4 {
        ta.update(&*key_code(rusty_bubbletea::key::KEY_BACKSPACE));
    }

    let cursor_line = ta.cursor_line_number();
    let min_visible = ta.scroll_y_offset();
    let max_visible = min_visible + ta.height() - 1;
    assert!(
        cursor_line >= min_visible && cursor_line <= max_visible,
        "cursor line {cursor_line} outside viewport [{min_visible}, {max_visible}] after shrink"
    );
}

#[test]
fn test_dynamic_height_cursor_position_after_paste() {
    let mut ta = new_dynamic_text_area(1, 20);

    ta.update(&PasteMsg {
        content: "line1\nline2\nline3\nline4\nline5".to_string(),
    });

    // Cursor should be at the end of the last pasted line
    assert_eq!(ta.line(), 4, "expected cursor on row 4, got {}", ta.line());

    let cursor_line = ta.cursor_line_number();
    let min_visible = ta.scroll_y_offset();
    let max_visible = min_visible + ta.height() - 1;
    assert!(
        cursor_line >= min_visible && cursor_line <= max_visible,
        "cursor line {cursor_line} outside viewport [{min_visible}, {max_visible}] after paste"
    );
}

#[test]
fn test_max_content_height_scrolls_beyond_max_height() {
    let mut ta = new_dynamic_text_area(1, 5);
    ta.max_content_height = 10;

    for _ in 0..8 {
        ta.update(&*key_press('x'));
        ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    }

    assert_eq!(
        ta.height(),
        5,
        "expected visible height capped at 5, got {}",
        ta.height()
    );

    assert_eq!(
        ta.line_count(),
        9,
        "expected 9 logical lines, got {}",
        ta.line_count()
    );
}

#[test]
fn test_max_content_height_blocks_at_limit() {
    let mut ta = textarea::new();
    ta.prompt = String::new();
    ta.show_line_numbers = false;
    ta.max_content_height = 5;
    ta.set_width(20);
    ta.focus();

    update_nil(&mut ta);

    for _ in 0..10 {
        ta.update(&*key_press('x'));
        ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    }

    assert!(
        ta.total_visual_lines() <= 5,
        "expected total visual lines <= 5, got {}",
        ta.total_visual_lines()
    );
}

#[test]
fn test_max_content_height_backward_compat() {
    let mut ta = textarea::new();
    ta.prompt = String::new();
    ta.show_line_numbers = false;
    ta.max_height = 10;
    ta.set_width(20);
    ta.focus();

    update_nil(&mut ta);

    for _ in 0..15 {
        ta.update(&*key_press('x'));
        ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    }

    assert!(
        ta.line_count() <= 10,
        "expected logical line count <= 10 (legacy behavior), got {}",
        ta.line_count()
    );
}

#[test]
fn test_max_content_height_without_dynamic_height() {
    let mut ta = textarea::new();
    ta.prompt = String::new();
    ta.show_line_numbers = false;
    ta.max_content_height = 5;
    ta.set_height(3);
    ta.set_width(20);
    ta.focus();

    update_nil(&mut ta);

    for _ in 0..10 {
        ta.update(&*key_press('x'));
        ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    }

    assert_eq!(
        ta.height(),
        3,
        "expected fixed height 3, got {}",
        ta.height()
    );

    assert!(
        ta.total_visual_lines() <= 5,
        "expected content capped at 5 visual lines, got {}",
        ta.total_visual_lines()
    );
}

#[test]
fn test_max_content_height_cursor_visible_while_scrolling() {
    let mut ta = new_dynamic_text_area(1, 5);
    ta.max_content_height = 10;

    for _ in 0..8 {
        ta.update(&*key_press('x'));
        ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    }
    ta.update(&*key_press('y'));

    let cursor_line = ta.cursor_line_number();
    let min_visible = ta.scroll_y_offset();
    let max_visible = min_visible + ta.height() - 1;
    assert!(
        cursor_line >= min_visible && cursor_line <= max_visible,
        "cursor line {cursor_line} outside viewport [{min_visible}, {max_visible}] while scrolling"
    );
}

#[test]
fn test_max_content_height_paste_capped() {
    let mut ta = textarea::new();
    ta.prompt = String::new();
    ta.show_line_numbers = false;
    ta.max_content_height = 5;
    ta.set_width(20);
    ta.focus();

    update_nil(&mut ta);

    ta.update(&PasteMsg {
        content: "1\n2\n3\n4\n5\n6\n7\n8\n9\n10".to_string(),
    });

    assert!(
        ta.total_visual_lines() <= 5,
        "expected paste capped at 5 visual lines, got {}",
        ta.total_visual_lines()
    );
}

#[test]
fn test_dynamic_height_shrinks_when_scrolled_and_lines_deleted() {
    let mut ta = new_dynamic_text_area(1, 5);
    ta.max_content_height = 10;

    // Type 8 lines so we exceed MaxHeight (5) and start scrolling
    for _ in 0..7 {
        ta.update(&*key_press('x'));
        ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    }
    ta.update(&*key_press('x'));

    assert_eq!(
        ta.height(),
        5,
        "expected height 5 (capped at MaxHeight), got {}",
        ta.height()
    );
    assert_eq!(
        ta.line_count(),
        8,
        "expected 8 lines, got {}",
        ta.line_count()
    );

    // Now delete lines from the bottom by selecting all on current line and
    // backspacing
    while ta.line_count() > 4 {
        ta.cursor_end();
        let (row, _) = ta.cursor_position();
        while !ta.value().split('\n').nth(row).unwrap_or("").is_empty() {
            ta.update(&*key_code(rusty_bubbletea::key::KEY_BACKSPACE));
        }
        ta.update(&*key_code(rusty_bubbletea::key::KEY_BACKSPACE)); // merge with previous line
    }

    // Now we have 4 lines, which is less than MaxHeight (5).
    // Height should shrink to 4.
    assert_eq!(
        ta.height(),
        4,
        "expected height to shrink to 4 (matching content), got {}",
        ta.height()
    );
    assert_eq!(
        ta.scroll_y_offset(),
        0,
        "expected yOffset 0 after shrinking, got {}",
        ta.scroll_y_offset()
    );
}

#[test]
fn test_dynamic_height_shrinks_when_scrolled_no_max_content() {
    // DynamicHeight with MaxHeight but no MaxContentHeight
    let mut ta = new_dynamic_text_area(1, 99);

    // Type 8 lines
    for _ in 0..7 {
        ta.update(&*key_press('x'));
        ta.update(&*key_code(rusty_bubbletea::key::KEY_ENTER));
    }
    ta.update(&*key_press('x'));

    assert_eq!(ta.height(), 8, "expected height 8, got {}", ta.height());

    // Manually set a smaller MaxHeight to simulate scrolling scenario
    // (upstream then calls Update(nil) to trigger a recalculate; the same
    // recalculation runs on the first update below.)
    ta.max_height = 5;

    // Now delete lines from the bottom
    while ta.line_count() > 3 {
        ta.cursor_end();
        let (row, _) = ta.cursor_position();
        while !ta.value().split('\n').nth(row).unwrap_or("").is_empty() {
            ta.update(&*key_code(rusty_bubbletea::key::KEY_BACKSPACE));
        }
        ta.update(&*key_code(rusty_bubbletea::key::KEY_BACKSPACE));
    }

    assert_eq!(
        ta.height(),
        3,
        "expected height to shrink to 3 (matching content), got {}",
        ta.height()
    );
    assert_eq!(
        ta.scroll_y_offset(),
        0,
        "expected yOffset 0 after shrinking, got {}",
        ta.scroll_y_offset()
    );
}

/// Port of the upstream `TestView` table. Each case mirrors the upstream
/// `modelFunc` closures; expected views are compared after stripping ANSI
/// sequences exactly like the upstream `stripString`.
#[test]
fn test_view() {
    struct Want {
        view: &'static str,
        cursor_row: usize,
        cursor_col: usize,
    }

    /// A model setup callback used by the upstream test cases.
    type ModelFunc = Option<Box<dyn Fn(&mut textarea::Model)>>;

    struct Case {
        name: &'static str,
        model_func: ModelFunc,
        want: Want,
    }

    // Mirror of the upstream `s.Focused.Base = Border(NormalBorder())`
    // style setup used by the "set width with style" subtests.
    fn bordered(m: &mut textarea::Model) {
        let mut s = m.styles().clone();
        s.focused.base = rusty_lipgloss::new_style().border(
            rusty_lipgloss::border::Border::normal(),
            &[true, true, true, true],
        );
        m.set_styles(s);
        m.focus();
    }

    // Mirror of the upstream page-up/page-down subtests' setup: fill the
    // textarea with 10 lines, position the cursor and scroll the viewport,
    // then move by a page. (The upstream `viewport.SetContent(m.view())`
    // step is emulated by a no-op update which refreshes the viewport
    // content; the upstream `Update(nil)` is not representable in Rust.)
    fn paged(m: &mut textarea::Model, height: usize, row: usize, y_offset: usize) {
        m.show_line_numbers = true;
        m.set_height(height);
        m.set_width(20);

        let lines: Vec<String> = (1..=10).map(|i| format!("Line {i}")).collect();
        m.set_value(&lines.join("\n"));
        m.set_cursor_position(row, 0);
        m.update(&PasteMsg {
            content: String::new(),
        });
        m.set_scroll_y_offset(y_offset);
    }

    let cases: Vec<Case> = vec![
        Case {
            name: "placeholder",
            model_func: None,
            want: Want {
                view: "\n\
                        >   1 Hello, World!\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "single line",
            model_func: Some(Box::new(|m| {
                m.set_value("the first line");
            })),
            want: Want {
                view: "\n\
                        >   1 the first line\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 14,
            },
        },
        Case {
            name: "multiple lines",
            model_func: Some(Box::new(|m| {
                m.set_value("the first line\nthe second line\nthe third line");
            })),
            want: Want {
                view: "\n\
                        >   1 the first line\n\
                        >   2 the second line\n\
                        >   3 the third line\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 2,
                cursor_col: 14,
            },
        },
        Case {
            name: "single line without line numbers",
            model_func: Some(Box::new(|m| {
                m.set_value("the first line");
                m.show_line_numbers = false;
            })),
            want: Want {
                view: "\n\
                        > the first line\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 14,
            },
        },
        Case {
            name: "multipline lines without line numbers",
            model_func: Some(Box::new(|m| {
                m.set_value("the first line\nthe second line\nthe third line");
                m.show_line_numbers = false;
            })),
            want: Want {
                view: "\n\
                        > the first line\n\
                        > the second line\n\
                        > the third line\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 2,
                cursor_col: 14,
            },
        },
        Case {
            name: "single line and custom end of buffer character",
            model_func: Some(Box::new(|m| {
                m.set_value("the first line");
                m.end_of_buffer_character = '*';
            })),
            want: Want {
                view: "\n\
                        >   1 the first line\n\
                        > *\n\
                        > *\n\
                        > *\n\
                        > *\n\
                        > *\n",
                cursor_row: 0,
                cursor_col: 14,
            },
        },
        Case {
            name: "multiple lines and custom end of buffer character",
            model_func: Some(Box::new(|m| {
                m.set_value("the first line\nthe second line\nthe third line");
                m.end_of_buffer_character = '*';
            })),
            want: Want {
                view: "\n\
                        >   1 the first line\n\
                        >   2 the second line\n\
                        >   3 the third line\n\
                        > *\n\
                        > *\n\
                        > *\n",
                cursor_row: 2,
                cursor_col: 14,
            },
        },
        Case {
            name: "single line without line numbers and custom end of buffer character",
            model_func: Some(Box::new(|m| {
                m.set_value("the first line");
                m.show_line_numbers = false;
                m.end_of_buffer_character = '*';
            })),
            want: Want {
                view: "\n\
                        > the first line\n\
                        > *\n\
                        > *\n\
                        > *\n\
                        > *\n\
                        > *\n",
                cursor_row: 0,
                cursor_col: 14,
            },
        },
        Case {
            name: "multiple lines without line numbers and custom end of buffer character",
            model_func: Some(Box::new(|m| {
                m.set_value("the first line\nthe second line\nthe third line");
                m.show_line_numbers = false;
                m.end_of_buffer_character = '*';
            })),
            want: Want {
                view: "\n\
                        > the first line\n\
                        > the second line\n\
                        > the third line\n\
                        > *\n\
                        > *\n\
                        > *\n",
                cursor_row: 2,
                cursor_col: 14,
            },
        },
        Case {
            name: "single line and custom prompt",
            model_func: Some(Box::new(|m| {
                m.set_value("the first line");
                m.prompt = "* ".to_string();
            })),
            want: Want {
                view: "\n\
                        *   1 the first line\n\
                        *\n\
                        *\n\
                        *\n\
                        *\n\
                        *\n",
                cursor_row: 0,
                cursor_col: 14,
            },
        },
        Case {
            name: "multiple lines and custom prompt",
            model_func: Some(Box::new(|m| {
                m.set_value("the first line\nthe second line\nthe third line");
                m.prompt = "* ".to_string();
            })),
            want: Want {
                view: "\n\
                        *   1 the first line\n\
                        *   2 the second line\n\
                        *   3 the third line\n\
                        *\n\
                        *\n\
                        *\n",
                cursor_row: 2,
                cursor_col: 14,
            },
        },
        Case {
            name: "type single line",
            model_func: Some(Box::new(|m| {
                send_string(m, "foo");
            })),
            want: Want {
                view: "\n\
                        >   1 foo\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 3,
            },
        },
        Case {
            name: "type multiple lines",
            model_func: Some(Box::new(|m| {
                send_string(m, "foo\nbar\nbaz");
            })),
            want: Want {
                view: "\n\
                        >   1 foo\n\
                        >   2 bar\n\
                        >   3 baz\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 2,
                cursor_col: 3,
            },
        },
        Case {
            name: "softwrap",
            model_func: Some(Box::new(|m| {
                m.show_line_numbers = false;
                m.prompt = String::new();
                m.set_width(5);

                send_string(m, "foo bar baz");
            })),
            want: Want {
                view: "\n\
                        foo\n\
                        bar\n\
                        baz\n\
                        \n\
                        \n\
                        \n",
                cursor_row: 2,
                cursor_col: 3,
            },
        },
        Case {
            name: "single line character limit",
            model_func: Some(Box::new(|m| {
                m.char_limit = 7;

                send_string(m, "foo bar baz");
            })),
            want: Want {
                view: "\n\
                        >   1 foo bar\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 7,
            },
        },
        Case {
            name: "multiple lines character limit",
            model_func: Some(Box::new(|m| {
                m.char_limit = 19;

                send_string(m, "foo bar baz\nfoo bar baz");
            })),
            want: Want {
                view: "\n\
                        >   1 foo bar baz\n\
                        >   2 foo bar\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 1,
                cursor_col: 7,
            },
        },
        Case {
            name: "set width",
            model_func: Some(Box::new(|m| {
                m.set_width(10);

                send_string(m, "12");
            })),
            want: Want {
                view: "\n\
                        >   1 12\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 2,
            },
        },
        Case {
            name: "set width max length text minus one",
            model_func: Some(Box::new(|m| {
                m.set_width(10);

                send_string(m, "123");
            })),
            want: Want {
                view: "\n\
                        >   1 123\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 3,
            },
        },
        Case {
            name: "set width max length text",
            model_func: Some(Box::new(|m| {
                m.set_width(10);

                send_string(m, "1234");
            })),
            want: Want {
                view: "\n\
                        >   1 1234\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 1,
                cursor_col: 0,
            },
        },
        Case {
            name: "set width max length text plus one",
            model_func: Some(Box::new(|m| {
                m.set_width(10);

                send_string(m, "12345");
            })),
            want: Want {
                view: "\n\
                        >   1 1234\n\
                        >     5\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 1,
                cursor_col: 1,
            },
        },
        Case {
            name: "set width set max width minus one",
            model_func: Some(Box::new(|m| {
                m.max_width = 10;
                m.set_width(11);

                send_string(m, "123");
            })),
            want: Want {
                view: "\n\
                        >   1 123\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 3,
            },
        },
        Case {
            name: "set width set max width",
            model_func: Some(Box::new(|m| {
                m.max_width = 10;
                m.set_width(11);

                send_string(m, "1234");
            })),
            want: Want {
                view: "\n\
                        >   1 1234\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 1,
                cursor_col: 0,
            },
        },
        Case {
            name: "set width set max width plus one",
            model_func: Some(Box::new(|m| {
                m.max_width = 10;
                m.set_width(11);

                send_string(m, "12345");
            })),
            want: Want {
                view: "\n\
                        >   1 1234\n\
                        >     5\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 1,
                cursor_col: 1,
            },
        },
        Case {
            name: "set width min width minus one",
            model_func: Some(Box::new(|m| {
                m.set_width(6);

                send_string(m, "123");
            })),
            want: Want {
                view: "\n\
                        >   1 1\n\
                        >     2\n\
                        >     3\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 3,
                cursor_col: 0,
            },
        },
        Case {
            name: "set width min width",
            model_func: Some(Box::new(|m| {
                m.set_width(7);

                send_string(m, "123");
            })),
            want: Want {
                view: "\n\
                        >   1 1\n\
                        >     2\n\
                        >     3\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 3,
                cursor_col: 0,
            },
        },
        Case {
            name: "set width min width no line numbers",
            model_func: Some(Box::new(|m| {
                m.show_line_numbers = false;
                m.set_width(0);

                send_string(m, "123");
            })),
            want: Want {
                view: "\n\
                        > 1\n\
                        > 2\n\
                        > 3\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 3,
                cursor_col: 0,
            },
        },
        Case {
            name: "set width min width no line numbers no prompt",
            model_func: Some(Box::new(|m| {
                m.show_line_numbers = false;
                m.prompt = String::new();
                m.set_width(0);

                send_string(m, "123");
            })),
            want: Want {
                view: "\n\
                        1\n\
                        2\n\
                        3\n\
                        \n\
                        \n\
                        \n",
                cursor_row: 3,
                cursor_col: 0,
            },
        },
        Case {
            name: "set width min width plus one",
            model_func: Some(Box::new(|m| {
                m.set_width(8);

                send_string(m, "123");
            })),
            want: Want {
                view: "\n\
                        >   1 12\n\
                        >     3\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 1,
                cursor_col: 1,
            },
        },
        Case {
            name: "set width without line numbers max length text minus one",
            model_func: Some(Box::new(|m| {
                m.show_line_numbers = false;
                m.set_width(6);

                send_string(m, "123");
            })),
            want: Want {
                view: "\n\
                        > 123\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 3,
            },
        },
        Case {
            name: "set width without line numbers max length text",
            model_func: Some(Box::new(|m| {
                m.show_line_numbers = false;
                m.set_width(6);

                send_string(m, "1234");
            })),
            want: Want {
                view: "\n\
                        > 1234\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 1,
                cursor_col: 0,
            },
        },
        Case {
            name: "set width without line numbers max length text plus one",
            model_func: Some(Box::new(|m| {
                m.show_line_numbers = false;
                m.set_width(6);

                send_string(m, "12345");
            })),
            want: Want {
                view: "\n\
                        > 1234\n\
                        > 5\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 1,
                cursor_col: 1,
            },
        },
        Case {
            name: "set width with style",
            model_func: Some(Box::new(|m| {
                bordered(m);

                m.set_width(12);

                send_string(m, "1");
            })),
            want: Want {
                view: "\n\
                        ┌──────────┐\n\
                        │>   1 1   │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        └──────────┘\n",
                cursor_row: 0,
                cursor_col: 1,
            },
        },
        Case {
            name: "set width with style max width minus one",
            model_func: Some(Box::new(|m| {
                bordered(m);

                m.set_width(12);

                send_string(m, "123");
            })),
            want: Want {
                view: "\n\
                        ┌──────────┐\n\
                        │>   1 123 │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        └──────────┘\n",
                cursor_row: 0,
                cursor_col: 3,
            },
        },
        Case {
            name: "set width with style max width",
            model_func: Some(Box::new(|m| {
                bordered(m);

                m.set_width(12);

                send_string(m, "1234");
            })),
            want: Want {
                view: "\n\
                        ┌──────────┐\n\
                        │>   1 1234│\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        └──────────┘\n",
                cursor_row: 1,
                cursor_col: 0,
            },
        },
        Case {
            name: "set width with style max width plus one",
            model_func: Some(Box::new(|m| {
                bordered(m);

                m.set_width(12);

                send_string(m, "12345");
            })),
            want: Want {
                view: "\n\
                        ┌──────────┐\n\
                        │>   1 1234│\n\
                        │>     5   │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        └──────────┘\n",
                cursor_row: 1,
                cursor_col: 1,
            },
        },
        Case {
            name: "set width without line numbers with style",
            model_func: Some(Box::new(|m| {
                bordered(m);

                m.show_line_numbers = false;
                m.set_width(12);

                send_string(m, "123456");
            })),
            want: Want {
                view: "\n\
                        ┌──────────┐\n\
                        │> 123456  │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        └──────────┘\n",
                cursor_row: 0,
                cursor_col: 6,
            },
        },
        Case {
            name: "set width without line numbers with style max width minus one",
            model_func: Some(Box::new(|m| {
                bordered(m);

                m.show_line_numbers = false;
                m.set_width(12);

                send_string(m, "1234567");
            })),
            want: Want {
                view: "\n\
                        ┌──────────┐\n\
                        │> 1234567 │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        └──────────┘\n",
                cursor_row: 0,
                cursor_col: 7,
            },
        },
        Case {
            name: "set width without line numbers with style max width",
            model_func: Some(Box::new(|m| {
                bordered(m);

                m.show_line_numbers = false;
                m.set_width(12);

                send_string(m, "12345678");
            })),
            want: Want {
                view: "\n\
                        ┌──────────┐\n\
                        │> 12345678│\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        └──────────┘\n",
                cursor_row: 1,
                cursor_col: 0,
            },
        },
        Case {
            name: "set width without line numbers with style max width plus one",
            model_func: Some(Box::new(|m| {
                bordered(m);

                m.show_line_numbers = false;
                m.set_width(12);

                send_string(m, "123456789");
            })),
            want: Want {
                view: "\n\
                        ┌──────────┐\n\
                        │> 12345678│\n\
                        │> 9       │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        │>         │\n\
                        └──────────┘\n",
                cursor_row: 1,
                cursor_col: 1,
            },
        },
        Case {
            name: "placeholder min width",
            model_func: Some(Box::new(|m| {
                m.set_width(0);
            })),
            want: Want {
                view: "\n\
                        >   1 H\n\
                        >     e\n\
                        >     l\n\
                        >     l\n\
                        >     o\n\
                        >     ,\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder single line",
            model_func: Some(Box::new(|m| {
                m.placeholder = "placeholder the first line".to_string();
                m.show_line_numbers = false;
            })),
            want: Want {
                view: "\n\
                        > placeholder the first line\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder multiple lines",
            model_func: Some(Box::new(|m| {
                m.placeholder = "placeholder the first line\nplaceholder the second line\nplaceholder the third line".to_string();
                m.show_line_numbers = false;
            })),
            want: Want {
                view: "\n\
                        > placeholder the first line\n\
                        > placeholder the second line\n\
                        > placeholder the third line\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder single line with line numbers",
            model_func: Some(Box::new(|m| {
                m.placeholder = "placeholder the first line".to_string();
                m.show_line_numbers = true;
            })),
            want: Want {
                view: "\n\
                        >   1 placeholder the first line\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder multiple lines with line numbers",
            model_func: Some(Box::new(|m| {
                m.placeholder = "placeholder the first line\nplaceholder the second line\nplaceholder the third line".to_string();
                m.show_line_numbers = true;
            })),
            want: Want {
                view: "\n\
                        >   1 placeholder the first line\n\
                        >     placeholder the second line\n\
                        >     placeholder the third line\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder single line with end of buffer character",
            model_func: Some(Box::new(|m| {
                m.placeholder = "placeholder the first line".to_string();
                m.show_line_numbers = false;
                m.end_of_buffer_character = '*';
            })),
            want: Want {
                view: "\n\
                        > placeholder the first line\n\
                        > *\n\
                        > *\n\
                        > *\n\
                        > *\n\
                        > *\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder multiple lines with with end of buffer character",
            model_func: Some(Box::new(|m| {
                m.placeholder = "placeholder the first line\nplaceholder the second line\nplaceholder the third line".to_string();
                m.show_line_numbers = false;
                m.end_of_buffer_character = '*';
            })),
            want: Want {
                view: "\n\
                        > placeholder the first line\n\
                        > placeholder the second line\n\
                        > placeholder the third line\n\
                        > *\n\
                        > *\n\
                        > *\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder single line with line numbers and end of buffer character",
            model_func: Some(Box::new(|m| {
                m.placeholder = "placeholder the first line".to_string();
                m.show_line_numbers = true;
                m.end_of_buffer_character = '*';
            })),
            want: Want {
                view: "\n\
                        >   1 placeholder the first line\n\
                        > *\n\
                        > *\n\
                        > *\n\
                        > *\n\
                        > *\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder multiple lines with line numbers and end of buffer character",
            model_func: Some(Box::new(|m| {
                m.placeholder = "placeholder the first line\nplaceholder the second line\nplaceholder the third line".to_string();
                m.show_line_numbers = true;
                m.end_of_buffer_character = '*';
            })),
            want: Want {
                view: "\n\
                        >   1 placeholder the first line\n\
                        >     placeholder the second line\n\
                        >     placeholder the third line\n\
                        > *\n\
                        > *\n\
                        > *\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder single line that is longer than max width",
            model_func: Some(Box::new(|m| {
                m.placeholder = "placeholder the first line that is longer than the max width".to_string();
                m.set_width(40);
                m.show_line_numbers = false;
            })),
            want: Want {
                view: "\n\
                        > placeholder the first line that is\n\
                        > longer than the max width\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder multiple lines that are longer than max width",
            model_func: Some(Box::new(|m| {
                m.placeholder = "placeholder the first line that is longer than the max width\nplaceholder the second line that is longer than the max width".to_string();
                m.show_line_numbers = false;
                m.set_width(40);
            })),
            want: Want {
                view: "\n\
                        > placeholder the first line that is\n\
                        > longer than the max width\n\
                        > placeholder the second line that is\n\
                        > longer than the max width\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder single line that is longer than max width with line numbers",
            model_func: Some(Box::new(|m| {
                m.placeholder = "placeholder the first line that is longer than the max width".to_string();
                m.show_line_numbers = true;
                m.set_width(40);
            })),
            want: Want {
                view: "\n\
                        >   1 placeholder the first line that is\n\
                        >     longer than the max width\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder multiple lines that are longer than max width with line numbers",
            model_func: Some(Box::new(|m| {
                m.placeholder = "placeholder the first line that is longer than the max width\nplaceholder the second line that is longer than the max width".to_string();
                m.show_line_numbers = true;
                m.set_width(40);
            })),
            want: Want {
                view: "\n\
                        >   1 placeholder the first line that is\n\
                        >     longer than the max width\n\
                        >     placeholder the second line that\n\
                        >     is longer than the max width\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder single line that is longer than max width at limit",
            model_func: Some(Box::new(|m| {
                m.placeholder = "123456789012345678".to_string();
                m.show_line_numbers = false;
                m.set_width(20);
            })),
            want: Want {
                view: "\n\
                        > 123456789012345678\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder single line that is longer than max width at limit plus one",
            model_func: Some(Box::new(|m| {
                m.placeholder = "1234567890123456789".to_string();
                m.show_line_numbers = false;
                m.set_width(20);
            })),
            want: Want {
                view: "\n\
                        > 123456789012345678\n\
                        > 9\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder single line that is longer than max width with line numbers at limit",
            model_func: Some(Box::new(|m| {
                m.placeholder = "12345678901234".to_string();
                m.show_line_numbers = true;
                m.set_width(20);
            })),
            want: Want {
                view: "\n\
                        >   1 12345678901234\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder single line that is longer than max width with line numbers at limit plus one",
            model_func: Some(Box::new(|m| {
                m.placeholder = "123456789012345".to_string();
                m.show_line_numbers = true;
                m.set_width(20);
            })),
            want: Want {
                view: "\n\
                        >   1 12345678901234\n\
                        >     5\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder multiple lines that are longer than max width at limit",
            model_func: Some(Box::new(|m| {
                m.placeholder = "123456789012345678\n123456789012345678".to_string();
                m.show_line_numbers = false;
                m.set_width(20);
            })),
            want: Want {
                view: "\n\
                        > 123456789012345678\n\
                        > 123456789012345678\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder multiple lines that are longer than max width at limit plus one",
            model_func: Some(Box::new(|m| {
                m.placeholder = "1234567890123456789\n1234567890123456789".to_string();
                m.show_line_numbers = false;
                m.set_width(20);
            })),
            want: Want {
                view: "\n\
                        > 123456789012345678\n\
                        > 9\n\
                        > 123456789012345678\n\
                        > 9\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder multiple lines that are longer than max width with line numbers at limit",
            model_func: Some(Box::new(|m| {
                m.placeholder = "12345678901234\n12345678901234".to_string();
                m.show_line_numbers = true;
                m.set_width(20);
            })),
            want: Want {
                view: "\n\
                        >   1 12345678901234\n\
                        >     12345678901234\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder multiple lines that are longer than max width with line numbers at limit plus one",
            model_func: Some(Box::new(|m| {
                m.placeholder = "123456789012345\n123456789012345".to_string();
                m.show_line_numbers = true;
                m.set_width(20);
            })),
            want: Want {
                view: "\n\
                        >   1 12345678901234\n\
                        >     5\n\
                        >     12345678901234\n\
                        >     5\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "placeholder chinese character",
            model_func: Some(Box::new(|m| {
                m.placeholder = "输入消息...".to_string();
                m.show_line_numbers = true;
                m.set_width(20);
            })),
            want: Want {
                view: "\n\
                        >   1 输入消息...\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n\
                        >\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "page up moves to beginning when near top",
            model_func: Some(Box::new(|m| {
                paged(m, 4, 3, 0);
                m.page_up();
            })),
            want: Want {
                view: "\n\
                        >   1 Line 1\n\
                        >   2 Line 2\n\
                        >   3 Line 3\n\
                        >   4 Line 4\n",
                cursor_row: 0,
                cursor_col: 0,
            },
        },
        Case {
            name: "page up snaps to first visible line when not on it",
            model_func: Some(Box::new(|m| {
                paged(m, 4, 5, 3);
                m.page_up();
            })),
            want: Want {
                view: "\n\
                        >   4 Line 4\n\
                        >   5 Line 5\n\
                        >   6 Line 6\n\
                        >   7 Line 7\n",
                cursor_row: 3,
                cursor_col: 0,
            },
        },
        Case {
            name: "page up moves up by full page when on first visible line",
            model_func: Some(Box::new(|m| {
                paged(m, 3, 5, 5);
                m.page_up();
            })),
            want: Want {
                view: "\n\
                        >   3 Line 3\n\
                        >   4 Line 4\n\
                        >   5 Line 5\n",
                cursor_row: 2,
                cursor_col: 0,
            },
        },
        Case {
            name: "page down moves to end when near bottom",
            model_func: Some(Box::new(|m| {
                paged(m, 3, 8, 7);
                m.page_down();
            })),
            want: Want {
                view: "\n\
                        >   8 Line 8\n\
                        >   9 Line 9\n\
                        >  10 Line 10\n",
                cursor_row: 9,
                cursor_col: 0,
            },
        },
        Case {
            name: "page down snaps to last visible line when not on it",
            model_func: Some(Box::new(|m| {
                paged(m, 3, 3, 3);
                m.page_down();
            })),
            want: Want {
                view: "\n\
                        >   4 Line 4\n\
                        >   5 Line 5\n\
                        >   6 Line 6\n",
                cursor_row: 5,
                cursor_col: 0,
            },
        },
        Case {
            name: "page down moves down by full page when on last visible line",
            model_func: Some(Box::new(|m| {
                paged(m, 3, 4, 2);
                m.page_down();
            })),
            want: Want {
                view: "\n\
                        >   6 Line 6\n\
                        >   7 Line 7\n\
                        >   8 Line 8\n",
                cursor_row: 7,
                cursor_col: 0,
            },
        },
    ];

    for tt in cases {
        let mut m = new_text_area();

        if let Some(f) = &tt.model_func {
            f(&mut m);
        }

        let view = strip_string(&m.view());
        let want_view = strip_string(&common::heredoc(tt.want.view));

        assert_eq!(want_view, view, "subtest: {}", tt.name);

        let cursor_row = m.cursor_line_number();
        let cursor_col = m.line_info().column_offset;
        assert!(
            tt.want.cursor_row == cursor_row && tt.want.cursor_col == cursor_col,
            "subtest: {} — Want cursor at row: {}, col: {} Got: row: {} col: {}",
            tt.name,
            tt.want.cursor_row,
            tt.want.cursor_col,
            cursor_row,
            cursor_col
        );
    }
}

#[test]
fn test_textarea_words_and_deletion_methods() {
    let mut ta = new_text_area();
    ta.set_value("hello world\nfoo bar baz");
    assert_eq!(ta.line_count(), 2);
    assert!(ta.length() > 0);

    // Word navigation
    ta.set_cursor_position(0, 0);
    ta.update(&KeyPressMsg(Key::new('f', "alt+f", KeyMod::default())));
    assert_eq!(ta.cursor_position(), (0, 5));

    ta.update(&KeyPressMsg(Key::new('b', "alt+b", KeyMod::default())));
    assert_eq!(ta.cursor_position(), (0, 0));

    // Delete word right
    ta.set_cursor_position(0, 0);
    ta.update(&KeyPressMsg(Key::new('d', "alt+d", KeyMod::default())));
    assert_eq!(ta.value(), " world\nfoo bar baz");

    // Reset
    ta.reset();
    assert_eq!(ta.value(), "");
    assert_eq!(ta.cursor_position(), (0, 0));
}
