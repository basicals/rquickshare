# Offline maintenance work log

_Started: 2026-10-12. Baseline: `19ee501` on `dev`._

## Authority and recovered context

The user authorized offline maintenance after the repository audit and asked that the existing diary be reviewed first. There is no separately named diary in tracked files: [maintenance-audit.md](maintenance-audit.md) is the maintenance working log; the dated sections in [mode-b-inventory.md](mode-b-inventory.md) preserve the hardware investigation; [full-feature-roadmap.md](full-feature-roadmap.md) records the product goal; [hardware-test-automation.md](hardware-test-automation.md) records the prospective harness.

The important unresolved findings are cold discovery latency, native KDE request-notification lifetime/cancellation, Android versus Linux completion, and current-package feature smoke. The 30-second advertiser refresh and background 5s/25s fallback scan are confounded in past measurements. Successful direct TCP transfers do not validate the slot0 experiment. These conclusions must survive documentation cleanup.

This task uses Mode A (maintenance feature/CI changes with supporting regression tests). It does not claim repository-wide Mode B completion.

## Analysis

- `core_lib` owns discovery/protocol/transport; `app/main/src-tauri` owns desktop commands, notifications, logs and lifecycle; Vue owns presentation and plugin calls.
- The preflight and Mode B workflows execute core and Vue tests but omit eight existing desktop Rust tests. Clippy compiles test code but does not execute assertions.
- The Mode B inventory still marks implemented regressions as planned. The roadmap incorrectly describes BWU `last_endpoint_id` as missing.
- The existing A/B launcher captures raw traces and prints selected lines; it cannot produce a structured, privacy-safe evidence verdict.
- The host has Rust, pnpm, Python and Linux development libraries; cached frontend and desktop build dependencies are present. No package installation or physical-device operation is needed for this slice.

## Scope, decisions and rollback

1. **M1 — execute all existing automated suites.** Add desktop Rust execution to preflight, the ten-run workflow and coverage. Keep packaging manual. Validate existing suites locally without starting the GUI. Rollback: one CI commit.
2. **M2 — synchronize SSOT.** Reconcile completed deterministic cases against test source, retain explicit remaining cases and historical measurements, correct stale implementation claims, and fix README/BUILD instructions. Do not mark an entire category covered merely because one case exists. Rollback: one documentation commit.
3. **M3 — offline evidence analyzer.** Add a Python standard-library CLI and pure parser for one captured A/B run, plus deterministic table-driven tests and CI execution. Python avoids a new package/runtime dependency and does not require the app, BlueZ, ADB or network. Keep the existing launcher's behavior unchanged; document separate invocation. Rollback: one script/test commit.
4. **M4 — verification and handoff.** Run appropriate formatting/lint/type checks, all four suites, and a preliminary ten-run stability check where the environment permits. Record exact results and remaining work. A final hardware/post-fix Mode B gate remains separate. Rollback: earlier commits remain individually reversible.

No protocol, crypto, persistent-settings contract, notification lifetime or visibility policy changes are authorized by this plan. The corrupted numeric visibility panic is recorded below for a separate user decision: fallback visibility changes discoverability/privacy. Shared secure-session and broad UI/state-machine extractions remain deferred as the existing diary requires.

## Analyzer contract and test inventory (before implementation)

Input: one UTF-8 log path and required variant (`baseline`, `skip-deferred`, `periodic-10`); experiment (`periodic` or `slot0`) defaults to `periodic`. Output: JSON with schema version, requested variant/experiment, evidence verdict/reasons, sanitized event types and source line numbers, terminal-state counts, scan/slot0 observations. Never copy raw lines, paths, peer/session IDs, addresses, payload content or timestamps into the report. This allowlist makes synthetic PII tests meaningful without pretending arbitrary raw traces are safe to publish.

Implementation detail: accept only the current Fern envelope and expected Rust module targets after stripping ANSI color codes. Require one logger-start marker and one of each diagnostic marker to avoid validating concatenated runs. Parse the fixed leading `ChannelMessage` Debug fields rather than searching payload/metadata text for states. CLI input is bounded to 16 MiB (the application log cap is 5 MiB; launcher stdout captures may be larger); oversized input is rejected before decoding. Unknown diagnostic values and malformed channel records cannot create a pass. This is a conservative evidence-format contract, not a generic parser for arbitrary logs.

Verdicts describe **Linux log evidence only**, never physical interoperability or a winning A/B treatment:
- `FAIL`: any Rejected/Cancelled/Disconnected terminal transfer without a preceding Finished for that same transfer, or contradictory diagnostic treatment. A later successful retry must not erase the failure. Source reconciliation confirms `Rejected` is a distinct production `State`, so its terminal evidence must be preserved too.
- `INCONCLUSIVE`: missing/conflicting run boundaries, absent expected diagnostics, no Finished, incomplete consent session, unconfirmed graceful exit, or a slot0 experiment without an offset-zero read. Uncorrelated terminal records cannot establish success.
- `PASS`: matching debug diagnostics, a correlated Finished transfer, no failed/incomplete transfer, clean tray Quit followed by `MDnsServer: service unregistered: OK`, and required experiment-path evidence.
- CLI exits 0 after a valid report regardless of evidence verdict; invalid arguments/unreadable or invalid-UTF8 input exit nonzero. This prevents shell launch semantics from being confused with transfer success.

| Artifact / branches | Risk / level | Planned checks |
|---|---|---|
| Variant/experiment selection | P1 / Unit+CLI | All three expected knob pairs; slot0 versus periodic path requirement; invalid choices rejected |
| Diagnostics and boundaries | P1 / Unit | Matching/missing/wrong knobs; repeated startup markers mean multiple runs; marker-shaped payload text ignored |
| Transfer correlation | P0 / Unit | Finished; cancellation/disconnection before completion; Finished followed by normal disconnect; failed session plus successful retry; pending consent; missing IDs; duplicate Finished does not inflate transfer counts |
| Graceful exit | P1 / Unit | Tray then exact successful unregister; missing/reversed/failed unregister; terminal states after Quit cannot validate a run |
| Discovery observations | P1 / Unit | Offset-zero versus offset continuation; scan start/stop; passive/fallback; timestamps are not inferred as Android latency |
| Privacy | P0 / Unit | Only fixed labels/counts/line numbers emitted even with filenames, text, IP/MAC, PIN/key and peer ID in the input |
| File/CLI boundary | P1 / Integration | Temporary UTF-8 input, malformed bytes, missing file, invalid options, JSON-only stdout, deterministic repeated analysis |

Applicable negative categories: N1 malformed log/CLI; N2 empty/multiple/repeated events; N4 retries/duplicates; N6 partial traces; N7 spoof-like payload markers; N9 cancellation/abrupt shutdown; N10 report privacy; N12 file read failures. N3 and N5 are not applicable to the offline single-input parser (recorded disconnects are evidence, not live network/concurrency); N8 is not applicable because no wall-clock arithmetic is performed; N11 is not applicable (no billing). Interactions: variant × diagnostics; experiment × slot0 path; session outcome × retry; completion × disconnect order; exit × unregister order; marker-like payload × privacy.

Supporting tests for notification body generation are deferred until their complete scoped branch inventory is reconciled; the first correction is to actually execute the existing desktop assertions.

## Progress

- Analysis and diary review: COMPLETE.
- Plan and analyzer inventory: COMPLETE, before implementation.
- M1: COMPLETE. Preflight executes desktop tests after its existing desktop Clippy check; Mode B repeats desktop assertions alongside core/Vue; manual/final coverage includes the desktop crate. `--no-default-features` follows the existing desktop Clippy contract. Local stable desktop execution: **8 passed, 0 failed**. The command session exited normally (terminal closed). Three pre-existing `generic_array::as_slice` deprecation warnings appeared in core dependencies; no assertion failed.
- M2: COMPLETE. Added source-confirmed reconciliation with explicit remaining scope; corrected implemented BWU/text/model claims, stale workflow descriptions, historical coverage labels, BUILD instructions and literal README newline escapes. Historical hardware observations remain intact.
- M3: COMPLETE. `scripts/analyze_receiver_smoke.py` emits a conservative JSON evidence report; 25 deterministic unittest methods (table-driven variants included) passed locally. Preflight and all Mode B iterations execute them. Launcher invocation remains unchanged; usage/limitations are recorded in the hardware automation plan. Ten targeted mutations are checked on temporary copies: treatment checks, failed-retry retention, normal disconnect, missing-start marker, slot0-path requirement, exit ordering, exact input limit, inbound/outbound filtering and raw-event leakage.
- M4: COMPLETE. Exact four-suite loop extracted from `mode-b.yml` passed ten consecutive local iterations against final implementation commit `2051314`; an earlier loop passed on `29c3dc8` before the explicit-Rejected correction. Fresh core/desktop/Vue/analyzer coverage and targeted mutation results are recorded below. All task-owned command sessions exited (terminals closed); no watchers, desktop app or device processes were started or left running.

## 2026-10-12 validation evidence

Final implementation head: `2051314e91294b29e1c3364d1b9753acd6f9d2bb`. Later work-log-only commits do not change this tested implementation. The final log was independently checked for exactly 40 suite invocations and ten successful results for every suite. Tests ran sequentially; no desktop app, Bluetooth session, ADB device or package installer was started. Runtime versions: Rust stable 1.99.0, Node 24.21.0, pnpm 9.7.0, Python 3.14.4. Both Cargo lockfiles and the pnpm lockfile are tracked and unchanged by this task.

| Suite | Per iteration | Consecutive successful iterations |
|---|---:|---:|
| Core Rust | 118 unit/export tests + 4 protocol contract tests | 10 |
| Desktop Rust | 8 unit/integration tests | 10 |
| Vue/Vitest | 41 tests in 7 files | 10 |
| Offline analyzer | 25 unittest methods, with table-driven subcases | 10 |
| Total | 196 test executions | 10 (1,960 executions) |

Rust format, core Clippy, desktop Clippy, frontend ESLint and Vue typecheck passed. Seven workflow YAML files parsed successfully; the launcher passes `bash -n`; the analyzer's actual CLI help entrypoint exits normally. Core tests include generated TypeScript export assertions and must not be represented as 122 independent business rules. These are preliminary offline stability results, not final post-hardware release signoff.

Analyzer methods by primary purpose (categories do not imply repository-wide N coverage): Positive 6; N1 3; N2 5; N4 3; N6 3; N7 2; N9 1; N10 1; N12 1 = 25. N3/N5/N8/N11 are not applicable for the reasons stated in the preregistered inventory. Table subcases exercise multiple variants within those methods.

### Targeted analyzer mutation review

Each change below was applied independently to a temporary copy of the real analyzer, with the unmodified test suite copied beside it. All 25 methods executed for every mutant; every mutant failed assertions (no compilation/import-error mutants counted). No source mutation was applied to the working tree.

| Mutation | Assertion that catches it | Result |
|---|---|---|
| Emit raw event text in JSON | Private-content absence and exact event-key allowlist | Caught |
| Ignore slot0 treatment mismatch | Wrong/unknown knob verdict and reason | Caught |
| Ignore periodic treatment mismatch | Wrong/unknown knob verdict and reason | Caught |
| Forget failed attempt after successful retry | Failed-session count and FAIL verdict | Caught |
| Treat post-Finished disconnect as failure | Normal teardown counts and PASS verdict | Caught |
| Accept missing logger-start marker | Missing-marker INCONCLUSIVE verdict | Caught |
| Accept slot0 experiment without offset-zero read | Missing/continuation-offset path verdict | Caught |
| Reverse Quit/unregister ordering | Valid baseline and reversed-exit cases | Caught |
| Reject exact 16 MiB boundary (`>` to `>=`) | Exact-limit CLI acceptance | Caught |
| Count outbound state as inbound | Exact inbound count and transfer-event count | Caught |

This is a targeted mutation check of the new analyzer, not a replacement for the historical/full protocol mutation review.

### Fresh coverage snapshot

Measured with `cargo-llvm-cov 0.9.1` / stable Rust 1.99.0, Vitest V8 and Python's standard-library `trace`. Rust/Vue sources are unchanged between `29c3dc8` and the rejection-evidence correction `2051314`; analyzer trace is refreshed after that correction.

| Scope | Lines | Functions | Other measured data |
|---|---:|---:|---|
| Core Rust | 48.05% | 45.78% | Regions 50.15% |
| Desktop Rust, excluding core dependency | 21.31% | 16.18% | Regions 21.91% |
| Vue frontend | 54.69% | 64.81% | Branches 84.79% |
| Offline analyzer | 99.3% | Not measured | Standard-library executable-line tracing, not branch coverage |

**Limits:** LLVM reports include inline `#[cfg(test)]` code in the source files. They are not production-only coverage or evidence of exhaustive business-rule testing. Rust branch counters were not enabled (the report shows `-`), so regions must not be called branch coverage. Python trace's uncovered CLI exit statement is exercised separately by actual CLI help; no 100% branch claim is made.

| Important production file | Current line coverage | Remaining work |
|---|---:|---|
| `hdl/inbound.rs` | 48.93% | Secure state-machine/error/storage/migration branches |
| `hdl/outbound.rs` | 50.37% | Text consent/send/cancel integration and filesystem errors |
| `hdl/gatt.rs` | 32.67% | Weave reassembly/fragmentation and BlueZ lifecycle |
| `hdl/ble.rs` | 5.59% | Active-fallback lifecycle and hardware backend selection |
| `hdl/ble_monitor.rs` | 21.31% | Real D-Bus monitor registration/events/cancellation |
| `hdl/bwu.rs`, `hdl/migratable.rs` | 100.00% each | Line coverage does not replace migration/order/identity invariants or the full branch review |
| `lib.rs`, `hdl/mdns_discovery.rs` | 0.55%, 0.00% | OS/task/discovery lifecycle |
| Desktop `main.rs`, `store.rs` | 0.00% each | Settings/lifecycle seams and packaged hardware smoke |
| Desktop `notification.rs` | 10.82% | Body formatting fallback, actions, expiry/cancellation and synchronization |
| Desktop `logger.rs` | 29.66% | Rotation/setup/failure paths beyond capped-writer tests |
| Desktop `cmds/open_download_destination.rs` | 65.19% | Native file-manager integration and platform branches |
| Desktop `cmds/clipboard_image.rs` | 49.49% | Native clipboard/image-save/error lifecycle |

The tool was installed under the task's scratch directory, not added as a project dependency. No system package installation was necessary. Existing non-blocking warnings remain: older core-lock BlueZ future incompatibility, newer desktop-lock GenericArray deprecations in the core dependency, and the frontend's legacy `eslint-env` comment warning. These were not hidden or changed as part of the analyzer/CI scope.

## Handoff and continuation

- Work is committed locally on `dev`; no push, remote workflow dispatch, PR, package publication or `master` promotion was performed. Local offline checks validate commands, not execution of a newly published Actions run.
- Reproduce suite commands from `BUILD.md`; the complete ten-run sequence is the `Run complete suite ten consecutive times` step in `.github/workflows/mode-b.yml`. `CARGO_TARGET_DIR` may point both Rust commands at one scratch cache without changing their separate tracked dependency locks.
- Reproduce coverage using the commands in `.github/workflows/coverage.yml` and the final Mode B coverage job. Install `cargo-llvm-cov --locked` and the stable `llvm-tools-preview` component when absent. Python line coverage can use standard-library `trace`; its percentage is not branch coverage.
- The new analyzer's planned cases are implemented; its targeted mutations are caught. No broad refactor or application protocol change was required for testability: the analyzer is a pure function with a separate file/CLI adapter, and desktop assertions use the existing crate.
- Next deterministic work is **not completed by this slice**: text state-machine integration, Weave reassembly/fragmentation, desktop notification body/action handling, settings/error/event interactions and full protocol mutation review. Preregister the selected artifact's full contract/branch/matrix before adding tests.
- Next physical work remains the dated Pixel/KDE matrix, controlled discovery A/B, notification/Android synchronization and a fresh installed debug package. Do not turn analyzer PASS into payload-integrity or Android-UI signoff.
- Ask the user before choosing corrupt-settings visibility recovery or changing notification/product/security policy. Do not silently apply the known visibility-panic fix with a chosen privacy fallback.

Rollback checkpoints: `065858a` records the plan; `c4ad7b7` adds desktop CI/coverage; `4a706ae` reconciles documentation; `29c3dc8` adds analyzer/tests/CI; `2051314` preserves explicit rejection. Revert dependent commits in reverse order for full rollback, or revert a single independent stage when appropriate. No persisted app data or external state requires migration.

## Remaining decisions and hardware gates

- Corrupt numeric visibility values currently reach `unreachable!()` through stored settings. A safe replacement needs an explicit visibility/privacy policy decision before implementation.
- Notification persistence/cleanup and protocol teardown ordering need synchronized hardware evidence and regression contracts before changing behavior.
- Hardware matrix and release promotion policy remain in the existing Mode B inventory. Do not promote `master` or claim hardware signoff from offline results.
