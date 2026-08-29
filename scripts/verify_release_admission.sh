#!/usr/bin/env bash

# Fail-closed admission checks for the tag-triggered publication workflow.
# This script runs in the secretless verification job with a read-only
# GitHub token and consumes a hash-bound trusted ruleset attestation. The
# privileged admission job is the only stage that fetches ruleset details.
# Publication is not admitted from an arbitrary tag, an unverified commit, or
# an unprotected ref.

set -euo pipefail

rulesets_admit_dev() {
  jq -e '
    any(.[];
      .enforcement == "active"
      and .target == "branch"
      and (has("bypass_actors") and (.bypass_actors | type == "array" and length == 0))
      and (has("conditions") and (.conditions | type == "object" and has("ref_name")))
      and (.conditions.ref_name | type == "object" and has("include") and has("exclude"))
      and (.conditions.ref_name.include | type == "array" and any(.[]; . == "refs/heads/dev" or . == "~DEFAULT_BRANCH"))
      and (.conditions.ref_name.exclude | type == "array" and length == 0)
      and (.rules | type == "array" and any(.[]; .type == "pull_request"))
      and (.rules | type == "array" and any(.[]; .type == "required_status_checks"))
    )
  ' >/dev/null
}

rulesets_admit_tag() {
  jq -e '
    any(.[];
      .enforcement == "active"
      and .target == "tag"
      and (has("bypass_actors") and (.bypass_actors | type == "array" and length == 0))
      and (has("conditions") and (.conditions | type == "object" and has("ref_name")))
      and (.conditions.ref_name | type == "object" and has("include") and has("exclude"))
      and (.conditions.ref_name.include | type == "array" and any(.[]; . == "refs/tags/v*"))
      and (.conditions.ref_name.exclude | type == "array" and length == 0)
      and (.rules | type == "array" and any(.[]; .type == "update"))
      and (.rules | type == "array" and any(.[]; .type == "deletion"))
      and (.rules | type == "array" and any(.[]; .type == "non_fast_forward"))
    )
  ' >/dev/null
}

rulesets_from_attestation() {
  : "${RULESET_ATTESTATION_FILE:?RULESET_ATTESTATION_FILE is required}"
  : "${RULESET_ATTESTATION_SHA256:?RULESET_ATTESTATION_SHA256 is required}"
  if [[ ! -f "${RULESET_ATTESTATION_FILE}" ]]; then
    echo "ERROR: trusted ruleset attestation is missing; refusing publication" >&2
    return 1
  fi

  local actual_sha256
  actual_sha256="$(sha256sum "${RULESET_ATTESTATION_FILE}" | awk '{print $1}')"
  if [[ "${actual_sha256}" != "${RULESET_ATTESTATION_SHA256}" ]]; then
    echo "ERROR: trusted ruleset attestation digest mismatch; refusing publication" >&2
    return 1
  fi

  if ! jq -e \
    --arg repository "${GITHUB_REPOSITORY}" \
    --arg commit "${GITHUB_SHA}" \
    --arg workflow_run_id "${GITHUB_RUN_ID}" '
      .schema_version == 1
      and .source == "github-ruleset-detail-attestation"
      and .repository == $repository
      and .commit == $commit
      and .workflow_run_id == $workflow_run_id
      and (.rulesets | type == "array")
    ' "${RULESET_ATTESTATION_FILE}" >/dev/null; then
    echo "ERROR: trusted ruleset attestation is not bound to this repository/run/commit; refusing publication" >&2
    return 1
  fi

  jq -c '.rulesets' "${RULESET_ATTESTATION_FILE}"
}

main() {
  : "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"
  : "${GITHUB_SHA:?GITHUB_SHA is required}"
  : "${GITHUB_REF_NAME:?GITHUB_REF_NAME is required}"
  : "${GH_TOKEN:?GH_TOKEN is required}"
  : "${GITHUB_RUN_ID:?GITHUB_RUN_ID is required}"

  if [[ ! "${GITHUB_REF_NAME}" =~ ^v[0-9] ]]; then
    echo "ERROR: release admission requires a semantic v* tag, got ${GITHUB_REF_NAME}" >&2
    exit 1
  fi

  git fetch --quiet origin dev --no-tags
  if ! git merge-base --is-ancestor "${GITHUB_SHA}" FETCH_HEAD; then
    echo "ERROR: tag commit ${GITHUB_SHA} is not an ancestor of origin/dev" >&2
    exit 1
  fi

  ci_runs="$(gh run list --repo "${GITHUB_REPOSITORY}" --workflow .github/workflows/ci.yml --commit "${GITHUB_SHA}" --limit 100 --json databaseId,headSha,status,conclusion,event,headBranch)"

  ci_run_id="$(jq -r --arg sha "${GITHUB_SHA}" '
    map(select(
      .headSha == $sha
      and .status == "completed"
      and .conclusion == "success"
      and .event == "push"
      and .headBranch == "dev"
    ))
    | sort_by(.databaseId)
    | last
    | .databaseId // empty
  ' <<<"${ci_runs}")"

  if [[ -z "${ci_run_id}" ]]; then
    echo "ERROR: no successful exact-SHA CI push run on dev admits ${GITHUB_SHA}" >&2
    exit 1
  fi

  if ! rulesets="$(rulesets_from_attestation)"; then
    echo "ERROR: could not validate trusted repository ref-protection attestation; refusing publication" >&2
    exit 1
  fi

  if ! rulesets_admit_dev <<<"${rulesets}"; then
    echo "ERROR: no active no-bypass dev protection ruleset with pull-request, status-check, target, and exclude rules" >&2
    exit 1
  fi

  if ! rulesets_admit_tag <<<"${rulesets}"; then
    echo "ERROR: no active no-bypass immutable v* tag ruleset with target, update, deletion, force-push, and exclude protections" >&2
    exit 1
  fi

  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    echo "ci_run_id=${ci_run_id}" >>"${GITHUB_OUTPUT}"
  fi

  echo "OK: release tag ${GITHUB_REF_NAME} at ${GITHUB_SHA} admitted by dev CI run ${ci_run_id}, trusted ruleset attestation, and protected refs"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  main "$@"
fi
