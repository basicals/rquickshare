# Native inbound file final-chunk reliability

Mode A, 2026-10-17. Direct dev, headless-only continuation. Scope: existing
inbound file payload framing and Linux atomic receive. No user workstation,
installed package, Android peer, BLE/transport, signer or publication access.

## Repository analysis and root cause

`core_lib/src/hdl/inbound.rs` `process_offline_frame` checks exact offsets and
declared file size. A nonempty FILE Data chunk executes the write/progress branch,
whereas finalization is under `else if (chunk.flags() & 1) == 1`. Thus a frame
that carries both final file bytes and the lastChunk bit writes bytes but never
completes/publishes that file. The existing tests send body chunks followed by
a separate empty lastChunk; they do not test both properties on the same frame.

The selected per-file atomic receive contract in
[native-receive-reliability.md](native-receive-reliability.md) requires completed
members to be durably published once, with no partial final names, no overwrite,
explicit partial outcome and preserved finished-file capability. The bit's
existing meaning and frame type remain unchanged. Processing lastChunk after an
optional nonempty body is a correction to existing framing, not a new transport,
wire/API, trust or retry feature.

## Plan, invariants and rollback

1. [x] Inspect actual inbound FILE handler, existing EOF tests, Linux publication
   tests and approved essential reliability order. Record this plan first.
2. [x] Extend actual-handler regressions (no domain mocks) for:
   - lastChunk carrying the complete file in one data frame;
   - earlier data followed by final data+lastChunk; completed member remains
     available when another member is pending;
   - premature nonempty lastChunk below declared size and oversized final
     data (must fail without publishing);
   - existing separate empty lastChunk, empty nonfinal and unknown/missing
     opened file behavior unchanged.
3. [x] Allow a lastChunk flag to finalize after successful body write, validating
   exact byte count, existing owner and publication/sync/receipt order.
   Preserve the pre-accept and offset checks. Do not change request state on
   invalid/truncated frames or relax any assertion.
4. [x] Run real core regression coverage and strict formatting/lint in CI.
   Capture exact counts/failed jobs, remedy only proven failures. No physical
   compatibility claim, 10-run MODE B claim or source-to-installed inference.
5. [x] Update current-state and this ledger with observed results. Keep installed
   native29, native30 release packaging and peer/desktop acceptance gates open.

## Focused branch/risk inventory

| Artifact | Branch or interaction | Tests | Categories |
|---|---|---|---|
| Inbound FILE handler | nonempty final, separate final, prior byte offsets | actual handler | Positive, N2, N4 |
| Inbound FILE handler | premature end, oversize, missing destination | actual handler | N1, N2, N6, N12 |
| Linux staged receive | completed first/remaining second, full success, exact receipt and disk bytes | actual stage/publish/receipt | Positive, N5, N9, N12 |
| IPC/desktop interface | no signature/state representation changes | existing tests retained | N7, N10 |

Other N-categories are outside this narrow existing FILE framing fix (no
new clock, monetization, network retry, locale, permissions or schema contract).
Expected addition: at least one table-driven direct-handler test and one actual
Linux-stage multi-file regression, with existing EOF/receipt tests still green.

Risks: premature success after a short body, multiple publication, falsely
incremented progress, missed sync errors, or a new path overwrite. Verify exact
bytes and final state; weak end-state-only assertions are insufficient.
Rollback is a normal revert of this scoped handler fix and its tests while
preserving existing native29/30 artifacts and immutable receipts.

## Execution and source results

Source changes: `core_lib/src/hdl/inbound.rs` and its Linux staged-receive
regressions. One last data frame now validates its final offset before writes,
updates byte progress once, and follows the same existing publication, receipt,
directory sync and terminal transition as an empty terminal frame. The
publication-only reborrow is gated to Linux, preserving legacy build boundaries.
The original empty-EOF tests are not weakened.

Three new test functions cover both managed and legacy request modes, the two
shapes of combined data/EOF for a real multi-file staged transfer, exact file
bytes/receipt order, premature final and overflow cases. Mutation expectations:
reintroducing `else if` fails the positive cases; skipping the pre-write EOF
length check fails the zero-write/zero-ack negative cases; skipping publication,
receipt or ack fails final filesystem/state assertions.

The first complete core run at source `b7883e94` was 695 passed/1 failed:
the new legacy/managed regression expected partial bytes for a short last chunk,
whereas the new pre-write guard correctly left the file untouched (actual
empty bytes). Strengthened only the test expectation to require zero writes on
all invalid final frames, commit `1d286316`. In that first run the two actual
Linux publication regressions passed. Rust format first failed on two
new test-only layout differences, then passed on source `b7883e94` after
applying the exact nightly diff. Core Clippy and frontend passed at that
source. The post-expectation CI runs remain pending; do not claim final green,
a ten-run stability gate or actual Google peer compatibility.


## Final headless verification

After strengthening the failed assertion, source commit `1d286316` passed
the exact independent GitHub CI jobs:

- Core Rust: 696 library methods, 0 failed, plus 4 modern-protocol and 4
  native-daemon integration tests (all passed). The three added actual-handler
  tests include both published Linux final-data scenarios and invalid-final
  no-write tests.
- Native GTK: 52 isolated widget/controller tests, 0 failed, with native
  formatting and strict lint passed. This is a noninteractive X11 test display.
- Rust format, core strict Clippy, and frontend checks: passed in dev Preflight
  run 39051081414.
- Legacy Tauri Clippy: still in progress at this recorded checkpoint. It is
  not required to claim this scoped core fix green, but the overall Preflight
  must remain pending until it completes.

CI: https://github.com/morozhenko-apps/rquickshare/actions/runs/39051081414
and https://github.com/morozhenko-apps/rquickshare/actions/runs/39051081413.

The initial red test was not a production regression: the improved pre-write
rejection was stricter than the original test expectation. The corrected
assertion requires no write and no ack on invalid EOF; it was not weakened.
No full MODE B inventory refresh, ten-run stability gate, physical peer,
installed binary test, package build, release signing or promotion to master
is claimed. Installed native29 remains unchanged. Native30 remains source
only and retains the separate exact artifact/idle/upgrade/acceptance gates.
