#!/usr/bin/env bash

# Offline regression tests for the fail-closed ruleset predicates. The live
# admission script remains responsible for fetching the authoritative ruleset
# detail records and checking the tag, ancestry, and CI evidence.

set -euo pipefail

cd "$(dirname "$0")/.."
source scripts/verify_release_admission.sh

missing_bypass_dev="$(jq -n '{
  enforcement: "active",
  target: "branch",
  conditions: { ref_name: { include: ["refs/heads/dev"], exclude: [] } },
  rules: [{ type: "pull_request" }, { type: "required_status_checks" }]
}')"
if rulesets_admit_dev <<<"[${missing_bypass_dev}]"; then
  echo "ERROR: omitted bypass_actors must not admit a dev ruleset" >&2
  exit 1
fi

valid_dev="$(jq -n '{
  enforcement: "active",
  target: "branch",
  bypass_actors: [],
  conditions: { ref_name: { include: ["refs/heads/dev"], exclude: [] } },
  rules: [{ type: "pull_request" }, { type: "required_status_checks" }]
}')"
if ! rulesets_admit_dev <<<"[${valid_dev}]"; then
  echo "ERROR: a complete protected dev ruleset must admit" >&2
  exit 1
fi

tag_without_update="$(jq -n '{
  enforcement: "active",
  target: "tag",
  bypass_actors: [],
  conditions: { ref_name: { include: ["refs/tags/v*"], exclude: [] } },
  rules: [{ type: "deletion" }, { type: "non_fast_forward" }]
}')"
if rulesets_admit_tag <<<"[${tag_without_update}]"; then
  echo "ERROR: tag rulesets without update protection must not admit" >&2
  exit 1
fi

tag_with_exclude="$(jq -n '{
  enforcement: "active",
  target: "tag",
  bypass_actors: [],
  conditions: { ref_name: { include: ["refs/tags/v*"], exclude: ["refs/tags/v1*"] } },
  rules: [{ type: "update" }, { type: "deletion" }, { type: "non_fast_forward" }]
}')"
if rulesets_admit_tag <<<"[${tag_with_exclude}]"; then
  echo "ERROR: tag rulesets with an effective exclusion must not admit" >&2
  exit 1
fi

valid_tag="$(jq -n '{
  enforcement: "active",
  target: "tag",
  bypass_actors: [],
  conditions: { ref_name: { include: ["refs/tags/v*"], exclude: [] } },
  rules: [{ type: "update" }, { type: "deletion" }, { type: "non_fast_forward" }]
}')"
if ! rulesets_admit_tag <<<"[${valid_tag}]"; then
  echo "ERROR: a complete immutable v* tag ruleset must admit" >&2
  exit 1
fi

echo "OK: release-admission predicate regressions pass"
