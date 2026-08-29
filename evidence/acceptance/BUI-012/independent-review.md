# BUI-012 Implementation Evidence

Status: implementation evidence for material parent
`276be3a99a90c9187c1371fbef682386012ddd0d` is the current security-remediation
material head. The attestation-only descendant preserves the prior Principal
and Testing approvals; the repaired exact head must receive targeted Security
confirmation. Exact-head CI and review binding are recorded in the GitHub
Issue ledger. Merge authorization remains a lifecycle gate. This packet is not
merge authorization.

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

The current pull request change set contains the original table/documentation
implementation and the release-boundary remediation required by the
independent Security review:

- `.github/workflows/ci.yml`: runs the exact-head documentation projection check and the complete seven-test table doctest selection.
- `UPSTREAM_MAPPING.md`: records the Rust-side table boundary adaptation.
- `docs/projection.yaml`: maps the user documentation source to the table module.
- `docs/src/lib.rs`: documents the table shape contract and contains a compiling user-facing example.
- `evidence/acceptance/BUI-012/independent-review.md`: this exact-head evidence packet.
- `.github/workflows/publish.yml`: adds a protected `release-admission` job with a dedicated `RULESET_ADMISSION_TOKEN`, uploads a hash-bound ruleset attestation, and keeps verification and publication credential-separated.
- `scripts/verify_docs_projection.sh`: compiles the documentation projection against the built library.
- `scripts/verify_release_admission.sh`: validates the hash-bound attestation's repository, commit, workflow-run binding, and no-bypass ruleset predicates without direct ruleset API access.
- `scripts/test-release-admission.sh`: covers valid and invalid ruleset predicates plus attestation digest and workflow-run binding.
- `scripts/test-release-guards.sh`: enforces immutable workflow/sibling references, the protected admission boundary, and read-only verifier artifact wiring.
- `src/lib.rs`: documents the crate facade as a user-facing component collection.
- `src/table.rs`: hardens outer-height arithmetic, cursor/viewport movement, ragged-row rendering, and the public user-documentation contract.
- `tests/table_test.rs`: adds deterministic ragged-row, zero-height, and maximum-input coverage while extending the upstream overflow case.

The synchronized `dev` changes to `Cargo.toml` remain imported base changes.
The publication workflow and release guards are ticket-owned remediation for
the Security review because they govern whether this crate can be released.

## Contract checks

| Check | Result | Evidence |
| --- | --- | --- |
| Declared columns remain the rendered row shape | Pass | `tests/table_test.rs::test_ragged_rows_render_against_declared_columns` |
| Height underflow is closed at zero | Pass | `tests/table_test.rs::test_height_is_saturating_at_the_boundary` |
| Cursor movement handles maximum inputs | Pass | `tests/table_test.rs::test_navigation_saturates_at_cursor_bounds` |
| Zero-height navigation is safe | Pass | `tests/table_test.rs::test_zero_height_navigation_is_safe` |
| Sibling dependency direction remains unchanged | Pass | `Cargo.toml`; path dependencies remain rusty-bubbletea, rusty-lipgloss, and rusty-x-ansi |
| User-facing table documentation is projected and compilable | Pass | `docs/projection.yaml`; `scripts/verify_docs_projection.sh`; `src/table.rs`; `docs/src/lib.rs` |
| Release admission evidence is hash-bound and run-bound | Pass | `.github/workflows/publish.yml`; `scripts/verify_release_admission.sh`; `scripts/test-release-admission.sh` |
| Privileged admission does not execute candidate repository code | Pass | `.github/workflows/publish.yml`; `scripts/test-release-guards.sh` |

## Focused validation

- `cargo test --test table_test --no-fail-fast`: 26 passed, 2 ignored.
- `cargo test --all-targets --no-fail-fast`: all unit and integration targets passed; 2 table tests and 1 viewport benchmark remain the repository's existing ignored tests.
- `cargo fmt --all --check`: passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo doc --no-deps --all-features`: passed without warnings.
- `cargo test --doc table:: --no-fail-fast`: 6 table doctests passed, including the changed public operations.
- `cargo test --doc table --no-fail-fast`: 7 table doctests passed, including the module-level user-facing example.
- `bash ./scripts/verify_docs_projection.sh`: passed; the projection manifest and the `docs/src/lib.rs` example compile against the current library. The same check passed under the CI split `CARGO_TARGET_DIR`/`CARGO_BUILD_BUILD_DIR` layout.
- `scripts/verify_mapping.sh`: passed in protected CI; the optional local `upstream-go/` checkout is absent as documented by the script.
- `./scripts/test-release-guards.sh`: passed.
- `scripts/test-release-admission.sh`: passed, including valid binding, digest-mismatch rejection, and wrong-workflow-run rejection.
- `bash -n scripts/verify_release_admission.sh scripts/test-release-admission.sh scripts/test-release-guards.sh`: passed.
- `yq '.' .github/workflows/publish.yml`: passed.
- Historical protected CI run [`33236603654`](https://github.com/coderbants/rusty-bubbles/actions/runs/33236603654) passed on exact head `41cc48b161060842b99a6bd2048de59cbcb011ed`: version gate, lint/build/docs/tests, documentation projection, seven table doctests, mapping, release guards, and coverage all passed. The PR-only badge-update job was skipped as designed. It is not proof for the later security-remediation descendant; the current exact-head run is recorded in the GitHub Issue ledger.

The repository-wide `cargo test --doc --no-fail-fast` run remains a known
out-of-scope baseline failure in the unchanged `src/key.rs` example/API
signature; all BUI-012 table doctests and the dedicated documentation
projection test pass.

## Review boundary

R-01 requested two repairs: refresh this packet after base synchronization and
add a complete `<user-docs>` contract plus exact-head projection proof. R-01
replay passed with no findings on `8f21bb1`. R-02 then identified
`TST-R02-001`: the module-level table doctest was not protected by CI. The
protected `cargo test --doc table --no-fail-fast` step now covers all seven
table doctests on `2e85a43`, and the exact current head `41cc48b` has a passing
protected rerun. The R-02 replay at `d8938132b5dc179c12a8ab83d764e0c316d374a2`
passed and remains valid for this non-build attestation descendant.

The Security review identified two blocking release-boundary findings and two
non-blocking guard/evidence findings. SEC-R03-001 was the remaining blocking
finding: the read-only verifier could not authoritatively inspect
`bypass_actors`. The remediation moves that read to the protected
`release-admission` job, binds the complete ruleset response to the repository,
commit, and workflow run, and supplies only the digest-bound artifact to the
read-only verifier. This does not alter the candidate build or admission
predicates, so prior Principal and Testing approvals are retained; only
targeted Security confirmation of the repaired exact head remains. The final
protected CI and post-merge absorb transition remain owned by the Mutate
lifecycle.
