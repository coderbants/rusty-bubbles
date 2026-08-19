//! Cleanroom Rust port of upstream Go source file: `paginator/paginator_test.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! Paginator navigation tests, including regression coverage for rapid
//! consecutive key presses in both directions.

use rusty_bubbles::paginator::{self, Model, Type};
use rusty_bubbletea::key::{Key, KeyMod, KeyPressMsg};
use rusty_bubbletea::model::Msg;

fn key_press(key: char) -> Box<dyn Msg> {
    Box::new(KeyPressMsg(Key::new(
        key,
        &key.to_string(),
        KeyMod::default(),
    )))
}

fn arrow(name: &str) -> Box<dyn Msg> {
    let code = match name {
        "right" => rusty_bubbletea::key::KEY_RIGHT,
        "left" => rusty_bubbletea::key::KEY_LEFT,
        _ => panic!("bad arrow"),
    };
    Box::new(KeyPressMsg(Key::new(code, "", KeyMod::default())))
}

#[test]
fn test_paginator_arrows_mixed_directions() {
    // right x3 then left must land on page 3 (items 21-30).
    let mut m: Model = paginator::new(vec![]);
    m.per_page = 10;
    m.set_total_pages(100);

    for _ in 0..3 {
        m.update(&*arrow("right"));
    }
    assert_eq!(m.page, 3, "right x3 should be page 3");

    m.update(&*arrow("left"));
    assert_eq!(m.page, 2, "left after right x3 should be page 2");

    // More stress: 20 rights clamp at the last page, then left goes back one.
    for _ in 0..20 {
        m.update(&*arrow("right"));
    }
    assert_eq!(
        m.page,
        m.total_pages - 1,
        "right x20 clamps at the last page"
    );

    m.update(&*arrow("left"));
    assert_eq!(
        m.page,
        m.total_pages - 2,
        "left after the clamp goes back one"
    );
}

#[test]
fn test_paginator_prev_at_start_is_noop() {
    let mut m: Model = paginator::new(vec![]);
    m.per_page = 10;
    m.set_total_pages(100);

    m.update(&*arrow("left"));
    assert_eq!(m.page, 0, "left at page 0 is a no-op");
}

#[test]
fn test_paginator_lh_keys() {
    // The example's advertised h/l keys behave like the arrows. From page 0
    // (1-based page 1), three 'l' presses advance three pages and 'h' goes
    // back one: lllh lands on 0-based page 2 (1-based page 3).
    let mut m: Model = paginator::new(vec![]);
    m.per_page = 10;
    m.set_total_pages(100);

    m.update(&*key_press('l'));
    m.update(&*key_press('l'));
    m.update(&*key_press('l'));
    m.update(&*key_press('h'));
    assert_eq!(m.page, 2, "lllh should be 0-based page 2 (1-based page 3)");

    // Four 'l' then 'h': llllh -> 0-based page 3 (1-based page 4).
    m.update(&*key_press('l'));
    assert_eq!(m.page, 3, "llllh should be 0-based page 3 (1-based page 4)");
}

#[test]
fn test_paginator_dots() {
    let mut m: Model = paginator::new(vec![
        paginator::with_total_pages(5),
        paginator::with_per_page(10),
    ]);
    m.total_pages = 5;
    m.type_ = Type::Dots;
    m.active_dot = "*".to_string();
    m.inactive_dot = ".".to_string();
    m.arabic_format = "%d of %d".to_string();

    assert_eq!(m.view(), "*....");
    assert!(m.on_first_page());
    assert!(!m.on_last_page());
    assert_eq!(m.items_on_page(45), 10);

    m.next_page();
    assert_eq!(m.view(), ".*...");
    assert!(!m.on_first_page());
    m.prev_page();
    assert_eq!(m.view(), "*....");

    // Arabic view
    m.type_ = Type::Arabic;
    assert_eq!(m.view(), "1 of 5");
}
