# BUI-012 Implementation Evidence

Status: current exact-head implementation evidence for material repair
`284590fae2a56951e8350e495d375498173851ca`; the R-01 replay and merge
authorization remain lifecycle gates. This packet is not merge authorization.

## Historical boundary

The previous packet covered `b1a42d2a8f6cf48597dc8eeb997aba627dcbde5a` over
base `2a88f46...`; it is historical only. The synchronized candidate includes
current `dev` at `bea9af850c7891d2006ce7c581f80846df99ece7`, Rust `1.98.0`,
immutable action references, release-boundary guards, and the API-based trusted
`dev` coverage-badge publication. The previous statement that the workflow
publishes with `git push HEAD:dev [skip ci]` is historical; the current workflow
uses the guarded GitHub API publication path and does not push from pull-request
coverage.

## Current result

The rusty-bubbles table now materializes every declared column deterministically. A row with fewer values produces empty cells, surplus values cannot index past the column definition, and zero or maximum navigation inputs do not panic. Outer-height subtraction clamps at zero while preserving the existing option-application semantics.

## Current change set

The current pull request change set is:

- `.github/workflows/ci.yml`: runs the exact-head documentation projection check.
- `UPSTREAM_MAPPING.md`: records the Rust-side table boundary adaptation.
- `docs/projection.yaml`: maps the user documentation source to the table module.
- `docs/src/lib.rs`: documents the table shape contract and contains a compiling user-facing example.
- `evidence/acceptance/BUI-012/independent-review.md`: this exact-head evidence packet.
- `scripts/verify_docs_projection.sh`: compiles the documentation projection against the built library.
- `src/lib.rs`: documents the crate facade as a user-facing component collection.
- `src/table.rs`: hardens outer-height arithmetic, cursor/viewport movement, ragged-row rendering, and the public user-documentation contract.
- `tests/table_test.rs`: adds deterministic ragged-row, zero-height, and maximum-input coverage while extending the upstream overflow case.

The synchronized `dev` changes to `Cargo.toml` and the CI security/release
configuration are imported base changes, not additional BUI-012-owned edits.

## Contract checks

| Check | Result | Evidence |
| --- | --- | --- |
| Declared columns remain the rendered row shape | Pass | `tests/table_test.rs::test_ragged_rows_render_against_declared_columns` |
| Height underflow is closed at zero | Pass | `tests/table_test.rs::test_height_is_saturating_at_the_boundary` |
| Cursor movement handles maximum inputs | Pass | `tests/table_test.rs::test_navigation_saturates_at_cursor_bounds` |
| Zero-height navigation is safe | Pass | `tests/table_test.rs::test_zero_height_navigation_is_safe` |
| Sibling dependency direction remains unchanged | Pass | `Cargo.toml`; path dependencies remain rusty-bubbletea, rusty-lipgloss, and rusty-x-ansi |
| User-facing table documentation is projected and compilable | Pass | `docs/projection.yaml`; `scripts/verify_docs_projection.sh`; `src/table.rs`; `docs/src/lib.rs` |

## Focused validation

- `cargo test --test table_test --no-fail-fast`: 26 passed, 2 ignored.
- `cargo test --all-targets --no-fail-fast`: all unit and integration targets passed; 2 table tests and 1 viewport benchmark remain the repository's existing ignored tests.
- `cargo fmt --all --check`: passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo doc --no-deps --all-features`: passed without warnings.
- `cargo test --doc table:: --no-fail-fast`: 6 table doctests passed, including the changed public operations.
- `bash ./scripts/verify_docs_projection.sh`: passed; the projection manifest and the `docs/src/lib.rs` example compile against the current library. The same check passed under the CI split `CARGO_TARGET_DIR`/`CARGO_BUILD_BUILD_DIR` layout.
- `scripts/verify_mapping.sh`: passed in protected CI; the optional local `upstream-go/` checkout is absent as documented by the script.
- `./scripts/test-release-guards.sh`: passed.
- `yq '.' .github/workflows/ci.yml`: passed.
- Protected CI run [`33235506720`](https://github.com/coderbants/rusty-bubbles/actions/runs/33235506720) passed on exact head `284590fae2a56951e8350e495d375498173851ca`: version gate, lint/build/docs/tests, documentation projection, mapping, release guards, and coverage all passed. The PR-only badge-update job was skipped as designed.

The repository-wide `cargo test --doc --no-fail-fast` run remains a known
out-of-scope baseline failure in the unchanged `src/key.rs` example/API
signature; all BUI-012 table doctests and the dedicated documentation
projection test pass.

## Review boundary

R-01 requested two repairs: refresh this packet after base synchronization and
add a complete `<user-docs>` contract plus exact-head projection proof. The
material repairs are present at `284590fae2a56951e8350e495d375498173851ca`;
the repaired head must be sent back to the same reviewer for confirmation.

The final independent review, exact-head protected CI, and post-merge absorb
transition remain owned by the Mutate lifecycle.
