# Agent Instructions for `rusty-bubbles`

> [!IMPORTANT]
> **Subsequent Cycle Requirement**: On every development cycle, before doing any work, the agent MUST inspect [`UPSTREAM_MAPPING.md`](UPSTREAM_MAPPING.md) to verify that all upstream Go files and examples are accounted for. When adding, modifying, or refactoring files, the agent MUST update [`UPSTREAM_MAPPING.md`](UPSTREAM_MAPPING.md) to reflect the current state.

## Core Rules & Workflow
1. Refer to the workspace-level rules in [`../AGENTS.md`](../AGENTS.md).
2. Maintain 100% rustdoc documentation.
3. Every ported file MUST include the guiding comment header:
   ```rust
   //! Cleanroom Rust port of upstream Go source file: `<upstream-go-filepath>`
   //! Upstream Target Tag / Version: `v2.1.0`
   ```
4. Verify all tests pass with `cargo test --all-targets` before committing.

## Releases
- **Version policy: the crate version and every release tag MUST equal the tracked upstream
  version exactly** (the `Upstream Target Tag / Version` header in `src/lib.rs`) — never
  ahead, never behind. `scripts/verify_upstream_version.sh` enforces this in CI and in the
  publish workflow. If upstream has not yet overtaken the published crate version, DO NOT
  bump or release — wait for upstream, then set the crate version to the upstream version.
- GitHub Releases MUST match upstream: every upstream release tag tracked by this port must
  exist as a `v*` tag and a GitHub release on this repo (the publish workflow creates the
  release automatically from the tag). If upstream has no tagged releases (pseudo-version
  pins), no GitHub release is required.
- To release: push the `v*` tag (e.g. `git tag v2.1.0 && git push origin v2.1.0`); the
  workflow runs tests, example parity, creates the GitHub Release, and attempts the
  crates.io publish (non-fatal without a registry token).
- The crates.io publish step is tag-gated; dev pushes only run tests and parity.
