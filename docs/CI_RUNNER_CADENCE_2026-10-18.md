# CI cadence, runner policy and failure triage (2026-10-18)

Status: investigation complete for current dev; CI changes pending validation.

## Observed failures (October 18 UTC, actual GitHub job logs)

- Native GTK job 117553112991: rustup-init fails because `HOME=/github/home` differs from the root user's actual home `/root` inside Ubuntu 26.04 container.
- Native package job 117552410527: `cargo-clippy` missing from stable Rust toolchain despite passing core tests.
- The most recent inspected Preflight job set in run 39086175208 passed all five jobs. Billing display `Gross $24.47 / Billed $0` is not proof that public standard GitHub-hosted usage consumed billed quota.
- Current dev Preflight explicitly runs race-sensitive Rust tests 10 times per push. Current `mode-b.yml` can run on a dev push modifying its own YAML, or through manual dispatch; that violates final-only stability intent.

## Required invariants

- Public repository: standard GitHub-hosted Ubuntu/macOS/Windows runs remain the default. Never silently enable untrusted PR workflows on local organization self-hosted runners.
- Routine dev/master pushes: fast deterministic tests, lint and type checks exactly once; no 10x repetitions and no automatic packaging/publishing. Preserve existing positive, negative and security coverage.
- Final full Mode B: exactly ten repetitions, invoked only by an authorized release-tag workflow after verifying the tag points to current master HEAD.
- Per-branch/PR concurrency cancels superseded checks, release tags are not canceled by branch push. Keep separate workflow groups where necessary.
- Existing `dev` contains substantially more commits than `master`; do not fast-forward unverified product code as part of CI maintenance.
- Rustup inside the Ubuntu 26.04 container needs a consistent `HOME` for the container user and explicit Rust components; never weaken lint/test checks.

## Implementation plan

1. [complete] Read source, workflows, job logs and current branch ancestry; differentiate platform quota rules from actual failing steps.
2. [pending] Limit automatic Preflight to one stable test pass; reserve race-sensitive 10x for the reusable release Mode B suite.
3. [pending] Remove dev/manual triggers from full Mode B so only the existing validated `v*` master-tag publishing workflow calls it.
4. [pending] Set container-user HOME consistently for containerized Rust jobs and explicitly install Clippy where required.
5. [pending] Add/check concurrency on any missing dev-triggered checks; preserve release-tag noncancellation.
6. [pending] Inspect dev CI outcome, fix infrastructure defects without masking test failures, and record measured evidence.
7. [pending] Fast-forward master only if all relevant gates are green and all intervening dev commits are verified, without force-pushing.

## RunnerRouter integration

The organization uses an external Cloudflare Worker for compatible private repositories, but public rquickshare is not in its current allowlist. Routing this public repo to self-hosted would change the trust boundary; leave it GitHub-hosted pending explicit security approval. This is not the cause of the confirmed Rust container failures.

## Rollback

Revert CI commits on dev, retain the protected master SHA and immutable release tags, do not change packaging semantics or disable deterministic assertions.

## 2026-10-18 execution evidence

- Completed on `dev`: normal Preflight now runs stable Core once; full Mode B is callable only by the master-HEAD tagged release workflow. Three P0 10x jobs reject dev refs and have no push triggers. Heavy native package, coverage and release syntax checks are manual. Native GTK scoped widget tests remain single-pass when native sources change. Per-ref cancellation preserves new pushes, while protected publication is non-cancelable.
- Added `scripts/ci-check-workflow-policy.sh`, invoked automatically by Preflight. The exact development Preflight SHA `aec1f1f7bb7579b88ab5980eab32ba0ec1ce78cc` passed in GitHub Actions run `39091401200`; native GTK gate also passed at preceding SHA.
- **Not yet complete:** no fresh valid release-tag 10x run, no bundled Windows/macOS acceptance, no master promotion. The very large outstanding development branch contains product changes beyond this CI scope. Public hosted runners remain deliberate for untrusted repository safety.
