#!/usr/bin/env bash

# Fail-closed admission checks for the tag-triggered publication workflow.
# This script runs in the secretless verification job with a read-only
# GitHub token. Publication is not admitted from an arbitrary tag, an
# unverified commit, or an unprotected ref.

set -euo pipefail

: "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"
: "${GITHUB_SHA:?GITHUB_SHA is required}"
: "${GITHUB_REF_NAME:?GITHUB_REF_NAME is required}"
: "${GH_TOKEN:?GH_TOKEN is required}"

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

if ! rulesets="$(gh api "repos/${GITHUB_REPOSITORY}/rulesets?includes_parents=true&per_page=100")"; then
  echo "ERROR: could not read repository ref-protection rulesets; refusing publication" >&2
  exit 1
fi

if ! jq -e '
  any(.[];
    .enforcement == "active"
    and ((.bypass_actors // []) | length == 0)
    and ((.conditions.ref_name.include // [])
      | any(. == "refs/heads/dev" or . == "~DEFAULT_BRANCH"))
    and ((.rules // [])
      | any(.type == "pull_request"))
    and ((.rules // [])
      | any(.type == "required_status_checks"))
  )
' <<<"${rulesets}" >/dev/null; then
  echo "ERROR: no active no-bypass dev protection ruleset with pull-request and status-check rules" >&2
  exit 1
fi

if ! jq -e '
  any(.[];
    .enforcement == "active"
    and ((.bypass_actors // []) | length == 0)
    and ((.conditions.ref_name.include // [])
      | any(. == "refs/tags/v*" or . == "refs/tags/*"))
    and ((.rules // [])
      | any(.type == "deletion"))
    and ((.rules // [])
      | any(.type == "non_fast_forward"))
  )
' <<<"${rulesets}" >/dev/null; then
  echo "ERROR: no active no-bypass immutable v* tag protection ruleset" >&2
  exit 1
fi

if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
  echo "ci_run_id=${ci_run_id}" >>"${GITHUB_OUTPUT}"
fi

echo "OK: release tag ${GITHUB_REF_NAME} at ${GITHUB_SHA} admitted by dev CI run ${ci_run_id} and protected refs"
