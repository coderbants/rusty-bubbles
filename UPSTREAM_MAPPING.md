# Upstream Mapping: `rusty-bubbles` (cleanroom Rust port of `charm.land/bubbles/v2`)

**Upstream target:** `charm.land/bubbles/v2` **v2.1.0** (tag `v2.1.0`)
**Upstream source:** checked out in `upstream-go/` (git-ignored).

## Core Libraries

| Upstream Go File | Rust Equivalent | Notes / Description |
| :--- | :--- | :--- |
| `bubbles.go` | `src/lib.rs` | Package root + module documentation |
| `cursor/cursor.go` | `src/cursor.rs` | Virtual cursor (blink, focus, modes) |
| `filepicker/filepicker.go` | `src/filepicker.rs` | File picker (directories, sizes, permissions) |
| `filepicker/hidden_unix.go` | `src/filepicker.rs` | Dot-file hiding (Unix) |
| `filepicker/hidden_windows.go` | `src/filepicker.rs` | `FILE_ATTRIBUTE_HIDDEN` check via `#[cfg(windows)]` FFI |
| `help/help.go` | `src/help.rs` | Short/full help views |
| `internal/memoization/memoization.go` | `src/internal/memoization.rs` | LRU memoization cache |
| `internal/runeutil/runeutil.go` | `src/internal/runeutil.rs` | Rune sanitizer |
| `internal/fuzzy` (inline port of `sahilm/fuzzy`) | `src/internal/fuzzy.rs` | Fuzzy matching |
| `internal/clipboard` (inline port of `atotto/clipboard`) | `src/internal/clipboard.rs` | `pbpaste`/`pbcopy` access |
| `internal/duration` (inline port of `time.Duration::String`) | `src/internal/duration.rs` | Duration formatting |
| `key/key.go` | `src/key.rs` | Keybindings + help metadata |
| `list/list.go` | `src/list.rs` | Feature-rich list (filtering, pagination, help, spinner) |
| `list/defaultitem.go` | `src/list.rs` | Default item delegate + styles |
| `list/keys.go` | `src/list.rs` | List keymap |
| `list/style.go` | `src/list.rs` | List styles |
| `paginator/paginator.go` | `src/paginator.rs` | Pagination (Arabic/Dots) |
| `progress/progress.go` | `src/progress.rs` | Progress bar with spring animation |
| `spinner/spinner.go` | `src/spinner.rs` | Spinner component + presets |
| `stopwatch/stopwatch.go` | `src/stopwatch.rs` | Stopwatch component |
| `table/table.go` | `src/table.rs` | Table component; Rust boundary handling keeps declared columns rectangular for ragged rows and clamps outer-height arithmetic safely |
| `textarea/textarea.go` | `src/textarea.rs` | Multi-line text area |
| `textinput/textinput.go` | `src/textinput.rs` | Single-line text input |
| `textinput/styles.go` | `src/textinput.rs` | Text input styles |
| `timer/timer.go` | `src/timer.rs` | Timer component |
| `viewport/viewport.go` | `src/viewport.rs` | Scrollable viewport |
| `viewport/keymap.go` | `src/viewport.rs` | Viewport keymap |
| `viewport/highlight.go` | `src/viewport.rs` | Highlight ranges |

## Tests

All 13 upstream Go test suites are ported to Rust integration tests under
`tests/`:

| Upstream Go Test | Rust Equivalent | Notes |
| :--- | :--- | :--- |
| `key/key_test.go` | `tests/key_test.rs` | Binding enabled-state test |
| `internal/runeutil/runeutil_test.go` | `tests/runeutil_test.rs` | Sanitizer table test |
| `paginator/paginator_test.go` | `tests/paginator_test.rs` | Pagination navigation |
| `cursor/cursor_test.go` | `tests/cursor_test.rs` | Blink-command concurrency smoke test (tag captured by value) |
| `help/help_test.go` | `tests/help_test.rs` | Full-help golden tests |
| `internal/memoization/memoization_test.go` | `tests/memoization_test.rs` | LRU cache semantics + fuzz-seed replay |
| `spinner/spinner_test.go` | `tests/spinner_test.rs` | Preset equality |
| `progress/progress_test.go` | `tests/progress_test.rs` | Blend golden tests |
| `list/list_test.go` | `tests/list_test.rs` | Status bar + filter tests (assert via full view) |
| `textinput/textinput_test.go` | `tests/textinput_test.rs` | Suggestions + view slicing |
| `table/table_test.go` | `tests/table_test.rs` | 21 tests incl. 10 golden view cases (2 upstream skips kept) |
| `viewport/viewport_test.go` | `tests/viewport_test.rs` | Navigation, highlights, 17 golden sizing cases |
| `textarea/textarea_test.go` | `tests/textarea_test.rs` | 30 tests incl. 67 TestView subtests |

## Golden-File Seam (`charmbracelet/x/exp/golden`)

- Ported as a small test-support module `tests/common/mod.rs` (`assert_golden`,
  `UPDATE_GOLDEN=1` mirrors the `-update` flag). The `go-udiff` dependency is
  dropped (assertion prints escaped expected vs actual instead of a unified
  diff). All 39 upstream `.golden` files are copied into `tests/testdata/`
  (segments: `TestFullHelp*`, `TestBlend*`, `TestModel_View*`,
  `TestModel_View_CenteredInABox`, `TestTableAlignment*`, `TestSizing*`).
- `tests/common/mod.rs` also ports `github.com/MakeNowJust/heredoc` (`heredoc`).

## Inline Ports of Third-Party Dependencies

Upstream bubbles depends on these libraries; they are ported inline (kept out of the
dependency tree as upstream keeps them out of the bubbletea library module):

| Upstream Library | Where | Notes |
| :--- | :--- | :--- |
| `charmbracelet/harmonica` (spring physics) | `src/progress.rs` | Exact `NewSpring` coefficient port |
| `sahilm/fuzzy` | `src/internal/fuzzy.rs` | `Find`/`FindNoSort` scoring port |
| `dustin/go-humanize` (`Bytes`) | `src/filepicker.rs` | Byte-count formatting |
| `atotto/clipboard` | `src/internal/clipboard.rs` | `pbpaste`/`pbcopy` access |
| Go `time.Duration::String()` | `src/internal/duration.rs` | Exact `fmtFrac`/`fmtInt` port |
| `charmbracelet/x/ansi` `Wordwrap`/`Hardwrap` | `src/textarea.rs` | Char-based wrap ports |

## Rust-Side Adaptations (documented in code)

- `Item` trait requires `box_clone` + `as_any` (trait objects cannot structurally clone).
- `ItemDelegate::update` receives `&Model` (shared) instead of a mutable model pointer.
- `viewport::Model` `Clone` drops gutter/style-line closures (kept `None` in clones).
- `textinput::view`/`textarea::view` operate on a cloned virtual cursor (upstream copies
  the model in `View`).
- Test-support accessors added so white-box upstream tests port verbatim:
  `viewport::Model::highlights()` + public `HighlightInfo`,
  `textarea::Model::cursor_position()`/`set_cursor_position()`,
  `textarea::Model::set_scroll_y_offset()`/`total_visual_lines()`.
- Tests that touch private internals (`textarea.row/col`, `viewport.lines`,
  `viewport.longestLineWidth`, `m.setInitialValues()`, `statusView()`) assert through
  the public API (documented at each call site).
- `viewport::scroll_left` uses `saturating_sub` (upstream int semantics clamp to 0).
- `table::Model` uses saturating outer-height arithmetic and reapplies the
  declared table shape during rendering: missing cells are empty and surplus
  cells are ignored, while cursor and viewport movement remains safe for zero
  and maximum inputs.

## Dependency Manifest

| Upstream Go Module | Rust Crate | Notes |
| :--- | :--- | :--- |
| `charm.land/bubbletea/v2` | `rusty-bubbletea` (path) | v2.0.8 port |
| `charm.land/lipgloss/v2` | `rusty-lipgloss` (path) | v2.0.5 port |
| `github.com/charmbracelet/x/ansi` | `rusty-x-ansi` (path) | width/cut/truncate |
| `github.com/mattn/go-runewidth` + `rivo/uniseg` | `unicode-width` | Rune widths |
| `golang.org/x/crypto/sha256` | `sha2` | Memoization hashing |

## Support / Non-Ported Upstream Files

| Upstream File | Rust Equivalent | Notes |
| :--- | :--- | :--- |
| `LICENSE` | `LICENSE` | MIT License (matching upstream copyright) |
| `README.md` | `README.md` | Documented Rust port header |
| `UPGRADE_GUIDE_V2.md` | `README.md` (notes) | v1 -> v2 migration guidance |
| `bubbles.go` | `src/lib.rs` | Package root (also listed above) |
| `Taskfile.yaml` / `.goreleaser.yml` / `.golangci.yml` | `.github/workflows/publish.yml` | Build/lint/release config -> CI workflow |
| `.github/workflows/*` (upstream) | `.github/workflows/publish.yml` | CI/CD -> Rust publish workflow |
| `.github/ISSUE_TEMPLATE/*`, `.github/dependabot.yml`, `.github/CODEOWNERS`, `.gitignore` | — | Process files; not applicable to the Rust crate |
| `go.mod` / `go.sum` | `Cargo.toml` | Dependency manifest (Go modules -> Cargo crates) |
