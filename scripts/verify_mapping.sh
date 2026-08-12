#!/usr/bin/env bash
# Verifies that UPSTREAM_MAPPING.md accounts for every file in upstream-go/.
set -u

cd "$(dirname "$0")/.."
MAPPING=UPSTREAM_MAPPING.md
fail=0

# Glob prefixes mentioned in the golden-testdata section, e.g. "TestTable*"
# covers "TestTableANSI.golden". Also collect exact testdata words.
read -r -a GLOB_PREFIXES <<< "$(grep -oE 'Test[A-Za-z0-9_]*\*' "$MAPPING" | tr -d '*' | tr '\n' ' ')"
read -r -a TESTDATA_WORDS <<< "$(grep -oE 'Test[A-Za-z0-9_]+' "$MAPPING" | sort -u | tr '\n' ' ')"

covered_by_glob() {
  local seg="$1"
  for p in ${GLOB_PREFIXES[@]+"${GLOB_PREFIXES[@]}"}; do
    if [[ "$seg" == "$p"* ]]; then
      return 0
    fi
  done
  return 1
}

covered_by_word() {
  local seg="$1"
  for w in "${TESTDATA_WORDS[@]}"; do
    if [[ "$seg" == "$w" ]]; then
      return 0
    fi
  done
  return 1
}

while IFS= read -r f; do
  case "$f" in
    *.go)
      if ! grep -qF "$f" "$MAPPING"; then
        echo "MISSING (.go): $f"
        fail=1
      fi
      ;;
    */testdata/*)
      seg="${f#*testdata/}"
      seg="${seg%%/*}"
      seg="${seg%.golden}"
      if ! covered_by_glob "$seg" && ! covered_by_word "$seg" && ! grep -qF "$seg" "$MAPPING"; then
        echo "MISSING (testdata segment): $seg  ($f)"
        fail=1
      fi
      ;;
    *)
      base="$(basename "$f")"
      dir="${f%/*}"
      if ! grep -qF "$base" "$MAPPING" && ! grep -qF "$dir/" "$MAPPING"; then
        echo "MISSING (support): $f"
        fail=1
      fi
      ;;
  esac
done < <(cd upstream-go && git ls-files)

if [ "$fail" -eq 0 ]; then
  echo "OK: every upstream file is accounted for in $MAPPING"
fi
exit "$fail"
