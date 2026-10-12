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

Verdicts describe **Linux log evidence only**, never physical interoperability or a winning A/B treatment:
- `FAIL`: any Cancelled/Disconnected terminal transfer without a preceding Finished for that same transfer, or contradictory diagnostic treatment. A later successful retry must not erase the failure.
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
- M2: IMPLEMENTED. Added source-confirmed reconciliation with explicit remaining scope; corrected implemented BWU/text/model claims, stale workflow descriptions, historical coverage labels and literal README newline escapes. Historical hardware observations remain intact.
- M3: PLANNED.
- M4: PLANNED.

## Remaining decisions and hardware gates

- Corrupt numeric visibility values currently reach `unreachable!()` through stored settings. A safe replacement needs an explicit visibility/privacy policy decision before implementation.
- Notification persistence/cleanup and protocol teardown ordering need synchronized hardware evidence and regression contracts before changing behavior.
- Hardware matrix and release promotion policy remain in the existing Mode B inventory. Do not promote `master` or claim hardware signoff from offline results.
