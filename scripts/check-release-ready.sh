#!/usr/bin/env bash
# Check that CI and the dependency audit have both completed successfully
# for a given commit before it gets tagged for release.
#
# publish.yml triggers directly on `push: tags: ["v*"]` and has no way to
# `needs:` a job defined in ci.yml or audit.yml (GitHub Actions cannot
# cross-reference jobs across workflow files). So this has to be checked
# by hand, before tagging - this script automates that check.
#
# Usage: scripts/check-release-ready.sh [ref]
#   ref defaults to HEAD.
#
# Exits 0 if both ci.yml and audit.yml have a completed, successful run
# for the resolved commit SHA. Exits 1 otherwise, with a message
# explaining what's missing.

set -euo pipefail

ref="${1:-HEAD}"

if ! command -v gh >/dev/null 2>&1; then
    echo "error: gh (GitHub CLI) is required but not found in PATH" >&2
    exit 1
fi


# `^{commit}` peels annotated tags to the commit they point at; git rev-parse
# on an annotated tag alone returns the tag object's own SHA, which never
# matches a workflow run's head SHA.
sha="$(git rev-parse "${ref}^{commit}" 2>/dev/null)" || {
    echo "error: could not resolve '$ref' to a commit" >&2
    exit 1
}

check_workflow() {
    local workflow="$1"
    local runs
    runs="$(gh run list -w "$workflow" -c "$sha" --json status,conclusion,url -L 1)"

    if [ "$(echo "$runs" | jq 'length')" -eq 0 ]; then
        echo "FAIL: no run of $workflow found for commit $sha"
        return 1
    fi

    local status conclusion url
    status="$(echo "$runs" | jq -r '.[0].status')"
    conclusion="$(echo "$runs" | jq -r '.[0].conclusion')"
    url="$(echo "$runs" | jq -r '.[0].url')"

    if [ "$status" != "completed" ]; then
        echo "FAIL: latest $workflow run for $sha is not completed (status: $status)"
        echo "  $url"
        return 1
    fi

    if [ "$conclusion" != "success" ]; then
        echo "FAIL: latest $workflow run for $sha did not succeed (conclusion: $conclusion)"
        echo "  $url"
        return 1
    fi

    echo "OK: $workflow succeeded for $sha ($url)"
    return 0
}

ci_ok=0
audit_ok=0

check_workflow ci.yml || ci_ok=1
check_workflow audit.yml || audit_ok=1

if [ "$ci_ok" -ne 0 ] || [ "$audit_ok" -ne 0 ]; then
    echo ""
    echo "Not ready to release $sha - see FAIL lines above." >&2
    exit 1
fi

echo ""
echo "Ready to release: CI and audit are both green for $sha."
exit 0
