#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

fail() {
  printf 'CI policy violation: %s\n' "$*" >&2
  exit 1
}

# Expensive one-off regression, coverage and Debian packaging workflows
# must never be activated by editing their YAML during a dev push.
for name in \
  p0-bwu-router.yml p0-client-introduction.yml p0-weave-worker.yml \
  native-gtk-coverage.yml native-package-smoke.yml native-release-check.yml
do
  file=".github/workflows/$name"
  [[ -f "$file" ]] || fail "missing $file"
  if grep -Eq '^  push:' "$file"; then
    fail "$name still triggers on push"
  fi
  grep -Eq '^  workflow_dispatch:' "$file" ||
    fail "$name is no longer manually accessible"
done

mode_b=".github/workflows/mode-b.yml"
grep -Eq '^  workflow_call:' "$mode_b" || fail 'Mode B reusable entry is missing'
if grep -Eq '^  (push|workflow_dispatch):' "$mode_b"; then
  fail 'Full 10x Mode B must not run directly from development'
fi
grep -Fq 'needs: admission' .github/workflows/native-tag-publish.yml ||
  fail 'Release Mode B lacks tag-admission dependency'
grep -Fq 'uses: ./.github/workflows/mode-b.yml' .github/workflows/native-tag-publish.yml ||
  fail 'Release no longer calls full Mode B'
grep -Fq "cancel-in-progress: false" .github/workflows/native-tag-publish.yml ||
  fail 'Release publication may cancel a previous tag'

preflight=".github/workflows/lint.yml"
grep -Fq 'scripts/ci-core-test-groups.sh stable' "$preflight" ||
  fail 'Dev core stable tests have disappeared'
if grep -Fq 'scripts/ci-core-test-groups.sh stress' "$preflight"; then
  fail 'Dev core tests still run stress 10x'
fi

for file in .github/workflows/p0-*.yml; do
  grep -Fq "if: github.ref_type == 'tag'" "$file" ||
    fail "$file may run the ten-pass job without a tag"
  grep -Fq 'git rev-parse origin/master^{commit}' "$file" ||
    fail "$file lacks mainline HEAD verification"
done

printf 'PASS: dev fast-test and protected tag-only stress workflow contracts\n'
