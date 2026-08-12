//! Cleanroom Rust port of upstream Go source file: `internal/runeutil/runeutil.go`
//! Upstream Target Tag / Version: `v2.1.0`

/// Sanitizer is a helper for bubble widgets that want to process
/// Runes from input key messages.
pub trait Sanitizer {
    /// Sanitize removes control characters from runes in a KeyRunes
    /// message, and optionally replaces newline/carriage return/tabs by a
    /// specified character.
    ///
    /// The rune array is modified in-place if possible. In that case, the
    /// returned slice is the original slice shortened after the control
    /// characters have been removed/translated.
    fn sanitize(&self, runes: &[char]) -> Vec<char>;
}

/// NewSanitizer constructs a rune sanitizer.
pub fn new_sanitizer(opts: Vec<Option>) -> Sanitizer_ {
    let mut s = Sanitizer_ {
        replace_new_line: "\n".chars().collect(),
        replace_tab: "    ".chars().collect(),
    };
    for o in opts {
        s = o(s);
    }
    s
}

/// Option is the type of option that can be passed to Sanitize().
pub type Option = Box<dyn FnOnce(Sanitizer_) -> Sanitizer_>;

/// ReplaceTabs replaces tabs by the specified string.
pub fn replace_tabs(tab_repl: &str) -> Option {
    let tab_repl = tab_repl.chars().collect();
    Box::new(move |s: Sanitizer_| Sanitizer_ {
        replace_tab: tab_repl,
        ..s
    })
}

/// ReplaceNewlines replaces newline characters by the specified string.
pub fn replace_newlines(nl_repl: &str) -> Option {
    let nl_repl = nl_repl.chars().collect();
    Box::new(move |s: Sanitizer_| Sanitizer_ {
        replace_new_line: nl_repl,
        ..s
    })
}

#[derive(Clone)]
pub struct Sanitizer_ {
    replace_new_line: Vec<char>,
    replace_tab: Vec<char>,
}

impl Sanitizer for Sanitizer_ {
    fn sanitize(&self, runes: &[char]) -> Vec<char> {
        // dstrunes are where we are storing the result.
        let mut dstrunes: Vec<char> = Vec::with_capacity(runes.len());

        for r in runes {
            match r {
                // invalid utf8 replacement char: skip
                &'\u{FFFD}' => {}

                &'\r' | &'\n' => {
                    dstrunes.extend_from_slice(&self.replace_new_line);
                }

                &'\t' => {
                    dstrunes.extend_from_slice(&self.replace_tab);
                }

                c if c.is_control() => {
                    // Other control characters: skip.
                }

                _ => {
                    // Keep the character.
                    dstrunes.push(*r);
                }
            }
        }
        dstrunes
    }
}
