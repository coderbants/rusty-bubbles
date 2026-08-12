//! Cleanroom Rust port of upstream Go source file: `cursor/cursor_test.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! Regression coverage for the blink command: the upstream test exercises
//! the cursor's `Blink` command concurrently to catch a data race on the
//! blink tag. The Rust port captures the tag by value in the command, so
//! the race cannot occur; the test is preserved as a concurrency smoke
//! test.

use charming_bubbles::cursor;
use std::sync::{Arc, Mutex};

#[test]
fn blink_cmd_data_race() {
    let mut m = cursor::new();
    let speed = m.blink_speed;
    let cmd = m.blink().expect("blink should return a command");
    let m = Arc::new(Mutex::new(m));

    let t1 = std::thread::spawn(move || {
        std::thread::sleep(speed * 3);
        // Run the original blink command.
        cmd();
    });
    let t2 = {
        let m = m.clone();
        std::thread::spawn(move || {
            std::thread::sleep(speed * 2);
            // Re-blink while the original command is still pending.
            let mut m = m.lock().unwrap();
            m.blink();
        })
    };
    t1.join().expect("blink command thread");
    t2.join().expect("re-blink thread");

    // A final blink after everything settles should still work.
    let mut m = m.lock().unwrap();
    assert!(m.blink().is_some(), "blink should return a command");
}
