//! Cleanroom Rust port of upstream Go source file:
//! `github.com/charmbracelet/x/exp/golden/golden.go`
//! Upstream Target Tag / Version: `v0.0.0-20250806222409-83e3a29d542f`
//!
//! Test support helpers shared by the widget integration tests:
//! golden-file assertions (port of `x/exp/golden.RequireEqual`) and a
//! here-doc un-indenter (port of `github.com/MakeNowJust/heredoc`).

// Helpers are only used by a subset of the test binaries; silence per-crate
// dead-code warnings.
#![allow(dead_code)]

use std::path::PathBuf;

/// Port of `golden.RequireEqual`: asserts that `out` matches the golden file
/// at `rel` (relative to `tests/testdata/`).
///
/// Set `UPDATE_GOLDEN=1` to rewrite the golden files, mirroring the
/// upstream `-update` flag. Control codes and escape sequences are escaped
/// before comparing, exactly like the upstream helper.
pub fn assert_golden(out: &str, rel: &str) {
    let path = golden_path(rel);
    if std::env::var("UPDATE_GOLDEN").is_ok_and(|v| v == "1") {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create testdata dir");
        }
        std::fs::write(&path, out).expect("write golden file");
        return;
    }
    let golden = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read golden file {}: {}", path.display(), e));
    let golden_esc = escape_seqs(&golden);
    let out_esc = escape_seqs(out);
    assert_eq!(
        golden_esc,
        out_esc,
        "output does not match golden file {}",
        path.display()
    );
}

/// Golden-path resolves a `tests/testdata`-relative path for golden files.
pub fn golden_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/testdata")
        .join(rel)
}

/// escape_seqs escapes control codes and escape sequences from the given
/// string. The only preserved exception is the newline character.
fn escape_seqs(in_: &str) -> String {
    in_.split('\n')
        .map(|l| format!("{l:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Port of `heredoc.Doc`: removes the common leading whitespace from all
/// lines and trims the leading newline, so multi-line string literals can
/// be written un-indented.
pub fn heredoc(s: &str) -> String {
    let s = s.strip_prefix('\n').unwrap_or(s);
    let lines: Vec<&str> = s.lines().collect();
    let indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut out = String::new();
    for (i, l) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&l[indent.min(l.len())..]);
    }
    out
}
