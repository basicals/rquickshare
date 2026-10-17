# Full Mode B CI execution: native GTK inclusion and coverage gate

Date: 2026-10-17. Primary MODE B (Testing). Direct `dev` source/CI
only. Installed Linux native29, source-only native30 packaging, Pixel,
Windows, machine services, secrets, signing, published releases and
`master` remain untouched. No changes to product behavior.

## Scan and authoritative documentation

Read the user-provided `agent.md` universal testing SSOT,
`docs/mode-b-test-plan.md`, `docs/mode-b-inventory.md`,
`docs/offline-maintenance.md`, `docs/current-state.md`, and
`.github/workflows/{mode-b,coverage,lint,native-gtk-ui}.yml`.
The in-repository full-scope inventory is historical (last substantive
reconciliation October 12); the intervening receive reliability,
managed runtime, IPC and source tests are documented in the current
2026-10-17 ledgers. This orchestration change does not magically
complete their global branch matrix.

Source baseline `64e84962` has confirmed previous Preflight 5/5,
Core library 704, modern-protocol/native integration 8, Python
contracts 54 and GTK 52, but those are **single** runs on the tested
source. Previous ten-run evidence in `offline-maintenance.md` is
not from this code and must not be relabeled current.

`mode-b.yml` is currently triggerable by manual `workflow_dispatch`
or an explicit `mode-b-*` Git tag. The GitHub connector available
to this session can edit repo files and read Actions but cannot
dispatch a workflow or create a Git tag. To actually validate the
headless candidate without asking the user to operate their machine,
add a *narrow* `push: branches [dev], paths:
[.github/workflows/mode-b.yml]` trigger. This runs only when the
explicit opt-in workflow source changes, not on general development
pushes; manual and tag triggers remain. No new backend permission is
needed (read-only). One workflow edit both improves the gate and
starts a pinned Actions source run. This is an internal, reversible
CI-only trigger, not a public API/architecture/security or UX change.

## Defect in current validation gate

The existing `full_suite` repeats **four** suites 10x: core Rust
(including integration), legacy Tauri desktop Rust, Vue/Vitest,
Python contract tests. Its `coverage` job needs only `full_suite`
and measures core, Vue and legacy desktop. The actual production
native GTK UI/daemon has a separate 52-case suite under
`native-gtk-ui.yml`, with an opt-in ten-repeat manual input, but that
suite is **absent** from the 'Complete automated suite 10x' gate.
A green Mode B workflow today would therefore omit the main Linux
UI from its ten-run stability claim.

## Scoped artifact inventory, branches and N1-N12 matrix

| Artifact/branch | Risk | Level | Planned assertion |
|---|---|---|---|
| mode-b trigger: tag/manual/dev workflow-file push vs other dev files | P1 uncontrolled workload | CI contract | preserve manual/tags; only workflow source update auto-runs |
| full_suite: 4 suites × attempts 1..10, fail-fast on first failure | P0 missing evidence | CI system | retain existing exact commands, sequential repetitions and shell `set -euo pipefail` |
| native_gtk: exact isolated 26.04 container dependency, build preview helper, Xvfb + D-Bus tests × attempts 1..10 | P0 missing GTK | CI system | reuse known-green commands from separate widget workflow; fail on any attempt |
| coverage job: dependency outcomes | P0 false success | CI graph | needs both ten-run full_suite and native GTK; skip coverage on either failure |
| coverage reporters: core/frontend/desktop | P1 metrics | diagnostic | retain existing measurement, never claim GTK coverage without instrumentation |
| secrets, repository access | P1 | CI policy | `contents: read`, no production deployment/token/external writes |
| product/Google peer | P0 | hardware | entirely out of scope, remains OPEN |

Branch transitions: workflow push event changed-file match true/false;
manual true; tag prefix true; each job zero/partial/ten passed;
GTK success/failure and full suite success/failure cross 2×2;
coverage only when 2× success; N9 job termination is a failed
run, not a fabricated pass. No mocks of the native application.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Trigger/gate conditions | P | X | P | P | P | P | P | P | X | P | X | X | X |
| Native GTK suite 10x | P | E | E | E | E | E | E | E | X | P | E | X | E |
| Coverage dependency | P | X | P | P | P | P | E | P | X | P | E | X | X |

P = this gate or intended execution, E = existing scenario suites
rerun, X = not applicable at orchestration layer: N1/N7 refer to
app validation already covered by existing suites; N8 has no
time/locale math; N11 no entitlement operations; N12
filesystem faults inside the native suite but not the CI trigger.
N10 path/privacy checks remain within source tests; job
logs do not intentionally expose user/device data.

**Coverage estimate:** four established suites × ten repetitions
(40 commands), plus the GTK suite × ten (10 executions),
one read-only coverage job after both branches. Existing
per-production-file maps live in individual module ledgers; the
global 100% branch/public API gate is **not** inferred from passing
the orchestration. No product files or test assertions to change.

## Planned implementation and rollback

1. [x] Verify current baseline, full coverage MODE B SSOT and CI gap;
   document the exact branch map, matrix, test count, decisions and
   unverified hardware/coverage scopes before workflow edits.
2. [ ] Keep the existing four-suites loop and coverage measurements
   unchanged; include a fifth independent GTK job with ten isolated
   X11/D-Bus test runs. Coverage must explicitly depend on BOTH
   ten-run jobs. Add narrowly scoped dev workflow-file push trigger.
   No new broad dev push, no token scope, no release artifacts.
3. [ ] Observe GitHub Actions on exact source. Verify 10 attempts
   in each branch by run logs; if red, classify root cause and
   repair CI/test fixture only, never weaken assertions. Do not
   claim ten green runs until observed.
4. [ ] Record source SHA, exact jobs, coverage summaries, GTK
   inclusion, limitations and YES/NO completion gate in this ledger
   and `docs/current-state.md`. Keep the final physical and
   all-branches/100%-inventory gate OPEN unless independently proven.

Rollback: one atomic workflow revert restores tag/manual-only
trigger and original four-suite gate; related docs are independent.
Preserve `master` and installed artifacts.

## Completion evidence

Pending workflow source update and exact GitHub Actions execution.
