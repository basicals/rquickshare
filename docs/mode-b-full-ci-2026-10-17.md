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
2. [x] Keep the existing four-suites loop and coverage measurements
   unchanged; include a fifth independent GTK job with ten isolated
   X11/D-Bus test runs. Coverage must explicitly depend on BOTH
   ten-run jobs. Add narrowly scoped dev workflow-file push trigger.
   No new broad dev push, no token scope, no release artifacts.
3. [x] Observe GitHub Actions on exact source. Verify 10 attempts
   in each branch by run logs; if red, classify root cause and
   repair CI/test fixture only, never weaken assertions. Do not
   claim ten green runs until observed.
4. [x] Record source SHA, exact jobs, coverage summaries, GTK
   inclusion, limitations and YES/NO completion gate in this ledger
   and `docs/current-state.md`. Keep the final physical and
   all-branches/100%-inventory gate OPEN unless independently proven.

Rollback: one atomic workflow revert restores tag/manual-only
trigger and original four-suite gate; related docs are independent.
Preserve `master` and installed artifacts.

## Completion evidence — execution started

Workflow-only change `7107339e49b577be9f292004513d80d09d706188`
adds the opt-in path-limited dev push trigger, a sequential
`native_gtk` job with the exact existing Ubuntu 26.04/Xvfb/D-Bus
runtime and **ten** full test runs, and changes
`coverage.needs` to `[full_suite,native_gtk]`. The four-suite
loop and existing three coverage instruments remain byte-for-byte
unchanged. The workflow consumes no product data, performs no
deployment and has only `contents: read` permission.

GitHub Actions accepted the push trigger and started:
https://github.com/morozhenko-apps/rquickshare/actions/runs/39063291246
with `full_suite` in progress at this checkpoint; the native GTK
job is dependency-gated and has not yet begun. Ordinary
Preflight for the same source also started:
https://github.com/morozhenko-apps/rquickshare/actions/runs/39063291248.
No pass count or coverage value is asserted until the resulting
CI logs are explicitly inspected. This is the exact tested workflow
source; later documentation-only commits should not alter its code.

The full current artifact/method/N1-N12 and OS-hardware inventory
remains open as noted in `docs/mode-b-test-plan.md`; adding a fifth
CI suite does not complete missing domain tests, run a mutation
framework, establish zero flakiness across future environments, or
substitute for real Pixel/Windows/Ubuntu transfer validation.

## 2026-10-17 first strict run failure and correction

First Mode B workflow run
https://github.com/morozhenko-apps/rquickshare/actions/runs/39063291246
**failed on attempt 1**, not on a stochastic core or GTK race:
Core 704/0 plus 8 integration, legacy desktop 10/0 and Vue
passed before Python failed 1 of 54 cases. The error came from
the GitHub Ubuntu system `botocore.config.Config` rejecting
the already-used `request_checksum_calculation` SDK parameter.
The pinned Python dependencies from `scripts/requirements-r2.txt`
had **not been installed** in Mode B, whereas the exact-source
ordinary Preflight
https://github.com/morozhenko-apps/rquickshare/actions/runs/39063291248
was 5/5 green and did install them. As designed, this failed
the full suite and **skipped both GTK and coverage**; no 10x
proof or percentage is available from the first run.

The new `dev` source commit `f8e5d2d0` adds a pinned venv
identical in principle to the passing Preflight Python isolation.
The strict Mode B job still repeats **all** four complete
suites ten times, followed by native GTK ten times, then
conditional coverage. GitHub run
https://github.com/morozhenko-apps/rquickshare/actions/runs/39066477128
was started from that correction and was still active at the
initial entry; no success should be inferred from its launch.

An independent **ordinary CI** profile now runs the stable
Core subset 1x and a documented 8-case race-sensitive subset
10x, with a live selector-integrity check. This is the
user-requested speed optimization for development checks,
**not** a substitute for mandatory full Mode B Stage 11;
see [stable/stress partition and CI evidence](ci-stable-race-sensitive-2026-10-17.md).
A future product defect must never be hidden by calling a
failing test 'flaky' or silently skipping it.


## Exact completed ten-run execution and measured coverage

**Strict automated Stage 11 stability subgate PASSED**, but Stage
12 and hardware acceptance remain OPEN. Exact tested source:
`f8e5d2d01d62200fcfd570c7bdf1f9051874845d`.
Workflow:
https://github.com/morozhenko-apps/rquickshare/actions/runs/39066477128.
Read and verified all three job statuses and exact attempt/test
summaries from GitHub Actions:

- Full-suites job 117495109038: **SUCCESS**. Exactly ten
  sequential iterations with all four attempt markers each.
  Each iteration: 704/0 core library tests, 4+4 integration,
  10/0 legacy Tauri Rust, successful Vue/Vitest, 54/0 Python
  publication/contract tests in an isolated pinned venv.
- Native GTK job 117499392551: **SUCCESS**. Exactly ten
  separate isolated X11/D-Bus attempts, each 52/0 native GTK
  widget/controller tests; formatter and strict GTK Clippy
  also succeeded.
- Coverage job 117499829172: **SUCCESS**, started after both
  ten-pass gates and measured exactly this source:
  core Rust 86.20% regions / 85.30% functions /
  **89.35% lines**;
  Vue/Vitest 56.00% statements and lines /
  75.86% functions / 88.04% branches;
  legacy Tauri 24.98% regions / 19.29% functions /
  **23.86% lines**.

Source-by-source/region/function/line detail, limits of those
denominators, coverage holes, priorities and a formal unclosed
Stage 12 matrix are in
[the exact-source coverage baseline](mode-b-coverage-baseline-2026-10-17.md).
**Native GTK was tested ten times but not instrumented in
this workflow**; an independent real GTK4 llvm-cov job is
being established to close that measurement gap, not rerun
unchanged full suites needlessly.

The corrected Mode B workflow was validated after the first
environment-parity failure (old system `botocore` did not
support a required configuration option); the final successful
run uses pinned `scripts/requirements-r2.txt`.
Normal dev Preflight is separately 5/5 at the exact code SHA
with 696 stable Rust library tests once, eight risk-sensitive
tests ×10, and 4+4 integration. No production source or
installed artifacts were modified for this CI investigation.

**Remaining explicitly NO:** every current public business
method/branch, all applicable N1–N12 and interaction combinations,
mutation coverage, real BlueZ/GATT/FEF3/FE2C discovery and
installed KDE/Wayland/Pixel/Windows acceptance. Correctness
for an isolated CI environment is not proof of Google Quick Share
interoperability. The 10 consecutive clean runs prove that
specific subgate only.
