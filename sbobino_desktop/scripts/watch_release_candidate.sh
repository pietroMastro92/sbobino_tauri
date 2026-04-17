#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage: watch_release_candidate.sh [repo-slug] [run-id]

If run-id is omitted, watches the most recent run of the Release Candidate
workflow.
EOF
}

if [[ $# -gt 2 ]]; then
  usage
  exit 1
fi

REPO_SLUG=${1:-pietroMastro92/Sbobino}
RUN_ID=${2:-}

if ! command -v gh >/dev/null 2>&1; then
  echo "Missing required command: gh" >&2
  exit 1
fi

if [[ -z "${RUN_ID// }" ]]; then
  RUN_ID=$(gh run list \
    --repo "$REPO_SLUG" \
    --workflow "Release Candidate" \
    --limit 1 \
    --json databaseId \
    --jq '.[0].databaseId')
fi

if [[ -z "${RUN_ID// }" || "$RUN_ID" == "null" ]]; then
  echo "Could not resolve a Release Candidate run to watch." >&2
  exit 1
fi

gh run watch "$RUN_ID" --repo "$REPO_SLUG" --exit-status

gh run view "$RUN_ID" \
  --repo "$REPO_SLUG" \
  --json jobs,url,workflowName,headSha,displayTitle,status,conclusion \
  --jq '{
    workflow: .workflowName,
    title: .displayTitle,
    status: .status,
    conclusion: .conclusion,
    url: .url,
    headSha: .headSha,
    jobs: [.jobs[] | {name, status, conclusion}]
  }'
