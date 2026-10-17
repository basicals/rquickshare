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
2. [ ] Extend actual-handler regressions (no domain mocks) for:
   - lastChunk carrying the complete file in one data frame;
   - earlier data followed by final data+lastChunk; completed member remains
     available when another member is pending;
   - premature nonempty lastChunk below declared size and oversized final
     data (must fail without publishing);
   - existing separate empty lastChunk, empty nonfinal and unknown/missing
     opened file behavior unchanged.
3. [ ] Allow a lastChunk flag to finalize after successful body write, validating
   exact byte count, existing owner and publication/sync/receipt order.
   Preserve the pre-accept and offset checks. Do not change request state on
   invalid/truncated frames or relax any assertion.
4. [ ] Run real core regression coverage and strict formatting/lint in CI.
   Capture exact counts/failed jobs, remedy only proven failures. No physical
   compatibility claim, 10-run MODE B claim or source-to-installed inference.
5. [ ] Update current-state and this ledger with observed results. Keep installed
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

Pending.
