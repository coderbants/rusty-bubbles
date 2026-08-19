//! Cleanroom Rust port of upstream Go source file: `key/key.go`
//! Upstream Target Tag / Version: `v2.1.0`
//!
//! <public-docs>
//! # Keymappings
//!
//! Types and functions for generating user-definable keymappings useful in
//! Bubble Tea components. There are a few different ways you can define a
//! keymapping with this package. Here's one example:
//!
//! ```rust
//! use rusty_bubbles::key;
//!
//! struct KeyMap {
//!     up: key::Binding,
//!     down: key::Binding,
//! }
//!
//! fn default_key_map() -> KeyMap {
//!     KeyMap {
//!         // actual keybindings
//!         up: key::new_binding(
//!             key::with_keys(&["k", "up"]),
//!             // corresponding help text
//!             key::with_help("↑/k", "move up"),
//!         ),
//!         down: key::new_binding(
//!             key::with_keys(&["j", "down"]),
//!             key::with_help("↓/j", "move down"),
//!         ),
//!     }
//! }
//! ```
//!
//! The help information, which is not used in the example above, can be used
//! to render help text for keystrokes in your views.
//! </public-docs>

use std::fmt;

/// Binding describes a set of keybindings and, optionally, their associated
/// help text.
#[derive(Debug, Clone)]
pub struct Binding {
    keys: Vec<String>,
    help: Help,
    disabled: bool,
}

/// BindingOpt is an initialization option for a keybinding. It's used as an
/// argument to [`new_binding`].
pub type BindingOpt = Box<dyn FnOnce(&mut Binding)>;

/// NewBinding returns a new keybinding from a set of BindingOpt options.
pub fn new_binding(opts: Vec<BindingOpt>) -> Binding {
    let mut b = Binding {
        keys: Vec::new(),
        help: Help {
            key: String::new(),
            desc: String::new(),
        },
        disabled: false,
    };
    for opt in opts {
        opt(&mut b);
    }
    b
}

/// WithKeys initializes a keybinding with the given keystrokes.
pub fn with_keys(keys: &[&str]) -> BindingOpt {
    let keys: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
    Box::new(move |b| {
        b.keys = keys;
    })
}

/// WithHelp initializes a keybinding with the given help text.
pub fn with_help(key: &str, desc: &str) -> BindingOpt {
    let key = key.to_string();
    let desc = desc.to_string();
    Box::new(move |b| {
        b.help = Help { key, desc };
    })
}

/// WithDisabled initializes a disabled keybinding.
pub fn with_disabled() -> BindingOpt {
    Box::new(|b| {
        b.disabled = true;
    })
}

impl Binding {
    /// SetKeys sets the keys for the keybinding.
    pub fn set_keys(&mut self, keys: &[&str]) {
        self.keys = keys.iter().map(|k| k.to_string()).collect();
    }

    /// Keys returns the keys for the keybinding.
    pub fn keys(&self) -> Vec<String> {
        self.keys.clone()
    }

    /// SetHelp sets the help text for the keybinding.
    pub fn set_help(&mut self, key: &str, desc: &str) {
        self.help = Help {
            key: key.to_string(),
            desc: desc.to_string(),
        };
    }

    /// Help returns the Help information for the keybinding.
    pub fn help(&self) -> Help {
        self.help.clone()
    }

    /// Enabled returns whether or not the keybinding is enabled. Disabled
    /// keybindings won't be activated and won't show up in help. Keybindings
    /// are enabled by default.
    pub fn enabled(&self) -> bool {
        !self.disabled && !self.keys.is_empty()
    }

    /// SetEnabled enables or disables the keybinding.
    pub fn set_enabled(&mut self, v: bool) {
        self.disabled = !v;
    }

    /// Unbind removes the keys and help from this binding, effectively
    /// nullifying it. This is a step beyond disabling it, since applications
    /// can enable or disable key bindings based on application state.
    pub fn unbind(&mut self) {
        self.keys.clear();
        self.help = Help {
            key: String::new(),
            desc: String::new(),
        };
    }
}

/// Help is help information for a given keybinding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Help {
    /// The key(s) used for the binding, e.g. "↑/k".
    pub key: String,
    /// A short description of the action, e.g. "move up".
    pub desc: String,
}

/// Matches checks if the given key matches the given bindings.
pub fn matches<K: fmt::Display>(k: K, bindings: &[Binding]) -> bool {
    let keys = k.to_string();
    for binding in bindings {
        for v in &binding.keys {
            if keys == *v && binding.enabled() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_binding_methods() {
        let mut b = new_binding(vec![with_disabled(), with_help("q", "quit")]);
        assert!(!b.enabled());
        b.set_enabled(true);
        assert!(!b.enabled()); // No keys set yet

        b.set_keys(&["q", "ctrl+c"]);
        assert!(b.enabled());
        assert_eq!(b.keys(), vec!["q".to_string(), "ctrl+c".to_string()]);
        assert_eq!(
            b.help(),
            Help {
                key: "q".to_string(),
                desc: "quit".to_string()
            }
        );

        b.set_help("esc", "exit");
        assert_eq!(
            b.help(),
            Help {
                key: "esc".to_string(),
                desc: "exit".to_string()
            }
        );

        assert!(matches("q", &[b.clone()]));
        assert!(matches("ctrl+c", &[b.clone()]));
        assert!(!matches("x", &[b.clone()]));

        b.set_enabled(false);
        assert!(!matches("q", &[b]));
    }
}
