#!/usr/bin/env bash

# Regression checks for release and CI trust boundaries. These checks are
# intentionally static and fast so every gate can prove that workflow changes
# did not reintroduce mutable dependencies or accidental write access.

set -euo pipefail

cd "$(dirname "$0")/.."

fail=0
workflow_files=(.github/workflows/*.yml)

report() {
  echo "ERROR: $1" >&2
  fail=1
}

if [ ! -e "${workflow_files[0]}" ]; then
  report "no GitHub workflow files were found"
fi

check_all_pinned_actions() {
  local line
  local ref
  local action_lines=()
  mapfile -t action_lines < <(grep -nHE '^[[:space:]]*(-[[:space:]]*)?uses:[[:space:]]*[^@[:space:]]+@[^[:space:]]+' "${workflow_files[@]}" || true)
  if [ "${#action_lines[@]}" -eq 0 ]; then
    report "no external workflow actions were found to validate"
    return
  fi
  for line in "${action_lines[@]}"; do
    ref="${line##*@}"
    ref="${ref%%[[:space:]]*}"
    if [[ ! "${ref}" =~ ^[0-9a-f]{40}$ ]]; then
      report "every external workflow action must use a full immutable commit SHA: ${line}"
    fi
  done
}

check_sibling_refs() {
  local sibling=0
  local line
  local ref
  while IFS= read -r line; do
    if [[ "${line}" == *"repository: coderbants/rusty-"* ]]; then
      sibling=1
      continue
    fi
    if (( sibling )) && [[ "${line}" =~ ^[[:space:]]*ref:[[:space:]]*(.+)$ ]]; then
      ref="${BASH_REMATCH[1]}"
      ref="${ref%%[[:space:]]*}"
      if [[ ! "${ref}" =~ ^[0-9a-f]{40}$ ]]; then
        report "every sibling checkout must use a full immutable commit SHA: ${line}"
      fi
      sibling=0
    elif (( sibling )) && [[ "${line}" =~ ^[[:space:]]*-[[:space:]]name: ]]; then
      report "sibling checkout is missing an immutable ref"
      sibling=0
    fi
  done < <(cat "${workflow_files[@]}")
  if (( sibling )); then
    report "sibling checkout is missing an immutable ref"
  fi
}

check_all_pinned_actions
check_sibling_refs

if grep -n 'workflow_dispatch' .github/workflows/publish.yml >/dev/null; then
  report "publish workflow must not expose a manual dispatch path"
fi

if ! sed -n '1,18p' .github/workflows/publish.yml | grep -nE '^permissions:|^  contents: read$' >/dev/null; then
  report "publish workflow must default to read-only repository permissions"
fi

if ! grep -n 'verify_release_admission.sh' .github/workflows/publish.yml >/dev/null; then
  report "publish workflow must run the fail-closed release-admission gate"
fi

if ! grep -n 'git merge-base --is-ancestor' scripts/verify_release_admission.sh >/dev/null; then
  report "release admission must require the tag commit to be on dev"
fi

if ! grep -n -- '--workflow .github/workflows/ci.yml' scripts/verify_release_admission.sh >/dev/null; then
  report "release admission must query the exact CI workflow"
fi

if ! grep -n 'refs/tags/v\*' scripts/verify_release_admission.sh >/dev/null; then
  report "release admission must require immutable v* tag protection"
fi

if ! grep -n 'environment:' .github/workflows/publish.yml >/dev/null || ! grep -n 'name: release' .github/workflows/publish.yml >/dev/null; then
  report "publication must be gated by the protected release environment"
fi

publish_job="$(awk '
  /^  publish:/ { in_job=1 }
  in_job && /^  [A-Za-z0-9_-]+:/ && $0 !~ /^  publish:/ { exit }
  in_job { print }
' .github/workflows/publish.yml)"
verify_job="$(awk '
  /^  verify:/ { in_job=1 }
  in_job && /^  [A-Za-z0-9_-]+:/ && $0 !~ /^  verify:/ { exit }
  in_job { print }
' .github/workflows/publish.yml)"

if [[ "${verify_job}" == *"contents: write"* || "${verify_job}" == *"CARGO_REGISTRY_TOKEN"* ]]; then
  report "secretless verification job must not receive write permission or registry credentials"
fi

if [[ "${publish_job}" != *"contents: write"* || "${publish_job}" != *"needs: verify"* ]]; then
  report "only the artifact publication job may receive contents: write and it must require verification"
fi

if [[ "${publish_job}" == *"actions/checkout@"* || "${publish_job}" == *"cargo test"* || "${publish_job}" == *"cargo build"* || "${publish_job}" == *"cargo clippy"* ]]; then
  report "credential-bearing publication job must not checkout or execute repository verification code"
fi

if [[ "${publish_job}" != *"actions/download-artifact@"* || "${verify_job}" != *"actions/upload-artifact@"* ]]; then
  report "release must exchange a verified artifact between isolated jobs"
fi

if [[ "${publish_job}" != *"cargo publish --no-verify"* ]]; then
  report "publication must use the verified package without running build scripts/tests on the credential-bearing runner"
fi

if grep -n -- '--clobber' .github/workflows/publish.yml >/dev/null; then
  report "release assets must never be silently overwritten"
fi

if awk '
  /repository: coderbants\/rusty-/ { sibling=1; next }
  sibling && /ref: dev/ { bad=1 }
  sibling && /^      - name:/ { sibling=0 }
  END { exit bad ? 0 : 1 }
' .github/workflows/ci.yml .github/workflows/publish.yml; then
  report "sibling dependency checkouts must not use a branch ref"
fi

if ! grep -n 'git clone --quiet --no-tags' .github/workflows/ci.yml >/dev/null; then
  report "the upstream checkout must suppress mutable tag discovery"
fi

if ! grep -nE 'git checkout --quiet [0-9a-f]{40}' .github/workflows/ci.yml >/dev/null; then
  report "the upstream checkout must use an immutable commit"
fi

coverage_job="$(awk '
  /^  coverage:/ { in_coverage=1 }
  in_coverage && /^  [A-Za-z0-9_-]+:/ && $0 !~ /^  coverage:/ { exit }
  in_coverage { print }
' .github/workflows/ci.yml)"

case "${coverage_job}" in
  *"contents: write"*)
    report "coverage must not have repository write permission while running tests"
    ;;
esac

if ! grep -n 'needs: coverage' .github/workflows/ci.yml >/dev/null; then
  report "coverage badge publication must depend on the read-only coverage job"
fi

if ! grep -nE 'uses: actions/(upload|download)-artifact@[0-9a-f]{40}' .github/workflows/ci.yml >/dev/null; then
  report "coverage must exchange its report through immutable artifact actions"
fi

if grep -nE 'x-access-token:|git (remote set-url|push)|cargo publish.*--token' .github/workflows/ci.yml .github/workflows/publish.yml >/dev/null; then
  report "workflow credentials must not be embedded in URLs or command-line arguments"
fi

if ! grep -n 'gh api --method PUT' .github/workflows/ci.yml >/dev/null; then
  report "coverage badge updates must use the GitHub API credential channel"
fi

if ! bash -n scripts/verify_release_admission.sh; then
  report "release admission script must pass bash syntax validation"
fi

if ! scripts/verify_upstream_version.sh >/dev/null; then
  report "the tracked upstream version must pass the release-version guard"
fi

if scripts/verify_upstream_version.sh not-a-release-tag >/dev/null 2>&1; then
  report "the release-version guard must reject non-v tags"
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi

echo "OK: release and CI trust-boundary guards pass"
