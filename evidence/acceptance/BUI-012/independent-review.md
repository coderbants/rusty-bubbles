# BUI-012 Implementation Evidence

Status: implementation self-review complete; independent review, protected CI, and merge authorization remain lifecycle gates.

## Result

The rusty-bubbles table now materializes every declared column deterministically. A row with fewer values produces empty cells, surplus values cannot index past the column definition, and zero or maximum navigation inputs do not panic. Outer-height subtraction clamps at zero while preserving the existing option-application semantics.

## Scope

- `.github/workflows/ci.yml`: separates PR coverage validation from trusted `dev` badge publication and prevents the PR checkout from attempting to push `dev`.
- `Cargo.toml`: declares the standalone Cargo resolver and the compiler floor used by the candidate.
- `src/lib.rs`: documents the crate facade as a user-facing component collection.
- `src/table.rs`: hardens outer-height arithmetic, cursor/viewport movement, and ragged-row rendering.
- `tests/table_test.rs`: adds deterministic ragged-row, zero-height, and maximum-input coverage while extending the upstream overflow case.
- `docs/src/lib.rs`: documents the table shape contract and contains a compiling user-facing example.
- `UPSTREAM_MAPPING.md`: records the Rust-side table boundary adaptation.

## Contract checks

| Check | Result | Evidence |
| --- | --- | --- |
| Declared columns remain the rendered row shape | Pass | `tests/table_test.rs::test_ragged_rows_render_against_declared_columns` |
| Height underflow is closed at zero | Pass | `tests/table_test.rs::test_height_is_saturating_at_the_boundary` |
| Cursor movement handles maximum inputs | Pass | `tests/table_test.rs::test_navigation_saturates_at_cursor_bounds` |
| Zero-height navigation is safe | Pass | `tests/table_test.rs::test_zero_height_navigation_is_safe` |
| Sibling dependency direction remains unchanged | Pass | `Cargo.toml`; path dependencies remain rusty-bubbletea, rusty-lipgloss, and rusty-x-ansi |

## Focused validation

- `cargo test --test table_test --no-fail-fast`: 26 passed, 2 ignored.
- `cargo test --all-targets --no-fail-fast`: all unit and integration targets passed; 2 table tests and 1 viewport benchmark remain the repository's existing ignored tests.
- `cargo fmt --all --check`: passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo doc --no-deps --all-features`: passed without warnings.
- `rustdoc --test docs/src/lib.rs`: 1 passed.
- `scripts/verify_mapping.sh`: passed; the optional local `upstream-go/` checkout was absent as documented by the script.
- `yq eval '.' .github/workflows/ci.yml`: passed; the coverage badge publication is guarded to push events on `dev` and uses `HEAD:dev [skip ci]`.

This packet is implementation evidence, not an approval or merge authorization. The final independent review, exact-head protected CI, and post-merge absorb transition remain owned by the Mutate lifecycle.
