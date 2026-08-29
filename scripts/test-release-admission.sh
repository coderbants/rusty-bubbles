#!/usr/bin/env bash

# Offline regression tests for the fail-closed ruleset predicates. The live
# admission job fetches authoritative ruleset detail records; the verification
# script only validates the hash-bound attestation and checks tag, ancestry,
# and CI evidence.

set -euo pipefail

cd "$(dirname "$0")/.."
source scripts/verify_release_admission.sh

assert_rejected() {
  local label="$1"
  local predicate="$2"
  local fixture="$3"
  if "$predicate" <<<"[${fixture}]"; then
    echo "ERROR: ${label} must be rejected" >&2
    exit 1
  fi
}

assert_admitted() {
  local label="$1"
  local predicate="$2"
  local fixture="$3"
  if ! "$predicate" <<<"[${fixture}]"; then
    echo "ERROR: ${label} must be admitted" >&2
    exit 1
  fi
}

attestation_dir="$(mktemp -d)"
trap 'rm -rf "${attestation_dir}"' EXIT
attestation_file="${attestation_dir}/ruleset-attestation.json"
export GITHUB_REPOSITORY="coderbants/rusty-bubbles"
export GITHUB_SHA="0123456789abcdef0123456789abcdef01234567"
export GITHUB_RUN_ID="12345"
jq -n \
  --arg repository "${GITHUB_REPOSITORY}" \
  --arg commit "${GITHUB_SHA}" \
  --arg workflow_run_id "${GITHUB_RUN_ID}" \
  '{schema_version: 1, source: "github-ruleset-detail-attestation", repository: $repository, commit: $commit, workflow_run_id: $workflow_run_id, rulesets: []}' \
  >"${attestation_file}"
export RULESET_ATTESTATION_FILE="${attestation_file}"
export RULESET_ATTESTATION_SHA256="$(sha256sum "${attestation_file}" | awk '{print $1}')"
if ! rulesets_from_attestation >/dev/null; then
  echo "ERROR: a correctly bound ruleset attestation must be admitted" >&2
  exit 1
fi

export RULESET_ATTESTATION_SHA256="not-the-file-digest"
if rulesets_from_attestation >/dev/null; then
  echo "ERROR: an attestation with a digest mismatch must be rejected" >&2
  exit 1
fi

jq '.workflow_run_id = "different-run"' "${attestation_file}" >"${attestation_file}.wrong-run"
export RULESET_ATTESTATION_FILE="${attestation_file}.wrong-run"
export RULESET_ATTESTATION_SHA256="$(sha256sum "${RULESET_ATTESTATION_FILE}" | awk '{print $1}')"
if rulesets_from_attestation >/dev/null; then
  echo "ERROR: an attestation for a different workflow run must be rejected" >&2
  exit 1
fi

missing_bypass_dev="$(jq -n '{
  enforcement: "active",
  target: "branch",
  conditions: { ref_name: { include: ["refs/heads/dev"], exclude: [] } },
  rules: [{ type: "pull_request" }, { type: "required_status_checks" }]
}')"
assert_rejected "dev ruleset with omitted bypass_actors" rulesets_admit_dev "${missing_bypass_dev}"

valid_dev="$(jq -n '{
  enforcement: "active",
  target: "branch",
  bypass_actors: [],
  conditions: { ref_name: { include: ["refs/heads/dev"], exclude: [] } },
  rules: [{ type: "pull_request" }, { type: "required_status_checks" }]
}')"
assert_admitted "complete protected dev ruleset" rulesets_admit_dev "${valid_dev}"
dev_with_bypass="$(jq '.bypass_actors = [{ actor_id: 123, actor_type: "User", bypass_mode: "always" }]' <<<"${valid_dev}")"
assert_rejected "dev ruleset with a bypass actor" rulesets_admit_dev "${dev_with_bypass}"

valid_tag="$(jq -n '{
  enforcement: "active",
  target: "tag",
  bypass_actors: [],
  conditions: { ref_name: { include: ["refs/tags/v*"], exclude: [] } },
  rules: [{ type: "update" }, { type: "deletion" }, { type: "non_fast_forward" }]
}')"
assert_admitted "complete immutable v* tag ruleset" rulesets_admit_tag "${valid_tag}"

tag_without_bypass="$(jq 'del(.bypass_actors)' <<<"${valid_tag}")"
assert_rejected "tag ruleset with omitted bypass_actors" rulesets_admit_tag "${tag_without_bypass}"
tag_with_bypass="$(jq '.bypass_actors = [{ actor_id: 123, actor_type: "User", bypass_mode: "always" }]' <<<"${valid_tag}")"
assert_rejected "tag ruleset with a bypass actor" rulesets_admit_tag "${tag_with_bypass}"
tag_with_wrong_target="$(jq '.target = "branch"' <<<"${valid_tag}")"
assert_rejected "tag ruleset with the wrong target" rulesets_admit_tag "${tag_with_wrong_target}"
tag_with_wrong_include="$(jq '.conditions.ref_name.include = ["refs/tags/release*"]' <<<"${valid_tag}")"
assert_rejected "tag ruleset without exact v* coverage" rulesets_admit_tag "${tag_with_wrong_include}"
tag_without_update="$(jq '.rules |= map(select(.type != "update"))' <<<"${valid_tag}")"
assert_rejected "tag ruleset without update protection" rulesets_admit_tag "${tag_without_update}"
tag_without_deletion="$(jq '.rules |= map(select(.type != "deletion"))' <<<"${valid_tag}")"
assert_rejected "tag ruleset without deletion protection" rulesets_admit_tag "${tag_without_deletion}"
tag_without_force_push_protection="$(jq '.rules |= map(select(.type != "non_fast_forward"))' <<<"${valid_tag}")"
assert_rejected "tag ruleset without force-push protection" rulesets_admit_tag "${tag_without_force_push_protection}"
tag_with_exclude="$(jq '.conditions.ref_name.exclude = ["refs/tags/v1*"]' <<<"${valid_tag}")"
assert_rejected "tag ruleset with an effective exclusion" rulesets_admit_tag "${tag_with_exclude}"

echo "OK: release-admission predicate regressions pass"
