#!/usr/bin/env bash
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

test -f docs/projection.yaml
test -f docs/src/lib.rs
test -f src/table.rs

grep -Fxq '  audience: user' docs/projection.yaml
grep -Fxq '  source: docs/src/lib.rs' docs/projection.yaml
grep -Fxq '  module: src/table.rs' docs/projection.yaml
grep -Fxq '  destination: rusty-bubbles' docs/projection.yaml
grep -Fxq '//! <user-docs>' src/table.rs
grep -Fxq '//! </user-docs>' src/table.rs
grep -Fxq '//! <user-docs>' docs/src/lib.rs
grep -Fxq '//! </user-docs>' docs/src/lib.rs
grep -Fq 'rusty_bubbles::table' docs/src/lib.rs

build_dir="${CARGO_BUILD_BUILD_DIR:-${CARGO_TARGET_DIR:-target}}"
BUI012_DOC_RLIB="$(
  cargo build --lib --message-format=json-render-diagnostics |
    node -e '
      const fs = require("node:fs");
      let rlib = "";
      for (const line of fs.readFileSync(0, "utf8").split(/\r?\n/u)) {
        if (line.trim() === "") continue;
        let message;
        try { message = JSON.parse(line); } catch { continue; }
        if (message.reason !== "compiler-artifact") continue;
        if (message.target?.name !== "rusty_bubbles") continue;
        if (!message.target?.kind?.includes("lib")) continue;
        rlib = message.filenames?.find((name) => name.endsWith(".rlib")) ?? rlib;
      }
      if (rlib === "") process.exit(1);
      process.stdout.write(rlib);
    '
)"
test -n "$BUI012_DOC_RLIB"
rustdoc --test docs/src/lib.rs \
  --edition=2021 \
  --extern "rusty_bubbles=$BUI012_DOC_RLIB" \
  -L "dependency=$build_dir/debug/deps"

echo "OK: docs projection is mapped and compiled"
