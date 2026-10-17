# Isolated Native GTK instrumented coverage plan

2026-10-17 — MODE B coverage Stage 8/11 follow-up. This plan
follows the exact verified strict four-suite 10x + Native GTK 10x
run `39066477128` at source `f8e5d2d0`. The full results and
per-file core/frontend/legacy Tauri measured baseline are in
[mode-b-coverage-baseline-2026-10-17.md](mode-b-coverage-baseline-2026-10-17.md).
No product source changes. The `native_desktop` (GTK4)
crate has **52 confirmed successful isolated tests × 10**, but
currently no instrumented per-file/function/line measurement.
The similarly named `app/main/src-tauri` report (23.86%
lines) is **legacy Tauri**, not the current native GTK UI.

## Repository scan, risk, invariants and source map

Read `.github/workflows/native-gtk-ui.yml`,
`.github/workflows/mode-b.yml`,
`.github/workflows/coverage.yml`, Rust crate paths, and current
MODE B SSOT. The GTK test harness uses Ubuntu 26.04
container, GTK4, layer-shell, X11/Xvfb, isolated D-Bus
`docs/prototypes/preview-bus.conf`, native preview helper,
`GTK_A11Y=none GDK_BACKEND=x11 GSK_RENDERER=cairo`, and
a workspace-owned `TMPDIR` because host /tmp may differ.

Adding coverage to the complete Mode B workflow now would
trigger another expensive full 10x of **unchanged source** and
conflate two distinct goals. Instead, create one *read-only*
focused `.github/workflows/native-gtk-coverage.yml`.
The workflow has only `workflow_dispatch` plus an opt-in
`dev` push limited to that workflow path so the first
implementation can be verified immediately. It runs one
instrumented Native GTK isolated suite using
`cargo llvm-cov` with `llvm-tools-preview`. It
reuses exact GTK dependencies and isolated environment
from the already-successful widget CI, builds the preview
helper, and preserves test assertions.

Job failure, zero executed tests, missing coverage totals or
unhealthy container must fail the workflow. It must not install
anything on the user workstation, run a real BlueZ adapter
or physical peer, connect to production accounts,
publish releases, alter native29 or update `master`.
Permissions: only `contents: read`, no artifact
publication/secrets, and no general branch-trigger workload.
Rollback: delete this new workflow and its docs.

## Scoped exhaustive branch inventory, test matrix

| Artifact/branch | Level/risk | Evidence plan |
|---|---|---|
| Workflow trigger: exact dev YAML path vs other push/manual | CI/P1 | only explicit file-update or manual run |
| Toolchain+llvm coverage install: successful/error | CI/P1 | fail-fast; no report if instrumentation unavailable |
| GTK4 container dependency setup and preview helper | CI/P1 | real runtime helper, no stub |
| Isolated X11/D-Bus test suite: success/failed/zero selected | CI/P0 | preserve exact `--bin rquickshare` test target and all 52 assertions |
| Cargo llvm-cov measured per-file regions/functions/lines | Instrumented/P0 | full reporter, no invented aggregate or Rust branch claims |
| Native GTK tests invoking child helpers with workspace TMPDIR | Integration/P0 | isolated preview helper exists, no destructive system effects |
| Existing final four-suite+GTK 10x Mode B | Independent/P0 | unchanged and retains successful result |

Test-count estimate: exactly **one** instrumented run of the
same 52-case GTK isolated executable in one new job. The
existing strict 10x job remains the actual Stage 11 proof;
do not mislabel the instrumented one-shot as another 10x.

Positive/N1–N12 applicability by artifact:

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Instrumented GTK test execution | E | E | E | E | E | E | E | E | X | E | E | X | E |
| Coverage collector/parser | P | P | P | X | X | X | P | P | X | P | P | X | P |
| Opt-in read-only Actions trigger | P | P | E | X | E | X | E | E | X | E | E | X | X |

E = existing GTK test cases executed with real services,
P = new instrumentation/CI validation, X = inapplicable to
the scope (no billing or date math). These do **not** claim
every member of each category or complete product branch
coverage. Interaction matrix: GTK test target × instrumented
build × isolated D-Bus/X11; helper build × llvm-cov clean
behavior; error before measurement × job false-negative;
instrumentation stdout × report count. Mutation review:
removing Xvfb/D-Bus context, selecting a wrong binary,
swallowing llvm-cov failures, or running no tests must
fail or become visibly unsupported. No mutation tool run.

## Implementation milestones

1. [x] Read exact existing workflow and full strict job results;
   establish isolation and test-count/risk/matrix before code.
2. [x] Create a separate opt-in GitHub workflow for GTK
   llvm-cov with 26.04 runner, no product changes and
   no duplicate complete 10x workload.
3. [ ] Observe the exact-source job and record per-file
   summary plus limitations; fix only CI compatibility
   errors without changing product assertions.
4. [ ] Update overall MODE B coverage map and current-state
   SSOT, leaving real hardware/branch and mutation gates OPEN.

## Execution

Workflow created `668917d8`, then corrected the exact
Rust output-matching regex `3bb41cb6` (the script initially
double-escaped its literal dot). Both are workflow-only
GitHub commits. The first file-trigger run was superseded,
not counted as coverage evidence. Exact corrected-source
[Native GTK instrumented coverage](https://github.com/morozhenko-apps/rquickshare/actions/runs/39071770811)
is running at the initial checkpoint. Job success,
reported percentages and the 52-case instrumented test summary
must be read from the complete result, not guessed from its
start. All product code, prior 10x validation, installed
host and Google peer acceptance remain unchanged.
