//! Cleanroom Rust port of upstream Go source file: `key/key_test.go`
//! Upstream Target Tag / Version: `v2.1.0`

use rusty_bubbles::key::{self, Binding};

#[test]
fn test_binding_enabled() {
    let mut binding: Binding = key::new_binding(vec![
        key::with_keys(&["k", "up"]),
        key::with_help("↑/k", "move up"),
    ]);
    assert!(binding.enabled(), "expected key to be Enabled");

    binding.set_enabled(false);
    assert!(!binding.enabled(), "expected key not to be Enabled");

    binding.set_enabled(true);
    binding.unbind();
    assert!(!binding.enabled(), "expected key not to be Enabled");
}
