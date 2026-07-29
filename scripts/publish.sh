#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

REPO="rwjblue/rigtether"
DESCRIPTION="Open hardware and software for a portable phone-to-radio audio, CAT, and PTT interface."

command -v git >/dev/null || { echo "git is required" >&2; exit 1; }
command -v gh >/dev/null || { echo "GitHub CLI (gh) is required" >&2; exit 1; }
gh auth status >/dev/null

python3 tools/check_repo.py

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git init -b main
fi

if ! git rev-parse --verify HEAD >/dev/null 2>&1; then
  git add --all
  git commit -m "Bootstrap RigTether project"
elif ! git diff --quiet || ! git diff --cached --quiet; then
  echo "The repository has uncommitted changes. Review and commit them before publishing." >&2
  exit 1
fi

if gh repo view "$REPO" >/dev/null 2>&1; then
  if ! git remote get-url origin >/dev/null 2>&1; then
    git remote add origin "git@github.com:${REPO}.git"
  fi
  git push --set-upstream origin HEAD:main
else
  gh repo create "$REPO" \
    --public \
    --description "$DESCRIPTION" \
    --source . \
    --remote origin \
    --push
fi

python3 scripts/bootstrap_github.py --repo "$REPO"

echo "Published https://github.com/$REPO"
