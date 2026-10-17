# Headless transport interruption and partial-receipt audit

Mode A, 2026-10-17. Essential priority 2: reliable failure and explicit
retry. Direct dev only. The user's Ubuntu/Pixel are unavailable, so this
slice uses isolated Rust duplex sockets, real Linux anonymous file staging,
existing session snapshots and CI. No installed service, peer, personal
filesystem, BLE, signing, publication, master merge or automatic retry.

## Inspection and failure hypotheses

Read `docs/native-essential-priorities.md`,
`docs/native-receive-reliability.md`, `docs/native-transfer-supervision.md`,
`docs/native-managed-dispatch.md`, `core_lib/src/hdl/inbound.rs`,
`core_lib/src/hdl/inbound/receive_reliability.rs`,
`core_lib/src/manager/session.rs` and `core_lib/src/receive.rs`.

Seven relevant failure sources:
1. Transport EOF before the frame header begins.
2. EOF after consuming 1-3 bytes of the four-byte length header.
3. EOF after the complete header but before the announced body.
4. Local storage write failure / no space.
5. Destination collision or access denial during no-overwrite publication.
6. User cancellation during partially completed multi-file receive.
7. Service shutdown or request generation loss while work is active.

The last four already have specific filesystem/session/manager regressions
and their own existing contracts. The highest-value unverified interaction
for this scoped pass is 1-3 multiplied by an accepted two-file transfer:
one completed and published member, another incomplete. Existing partial
tests directly call `record_termination` for several cases, rather than
forcing a real socket EOF on `handle`. This is a test-evidence gap, not yet
a demonstrated production defect.

The manager owns mapping non-AppError established inbound failures to
Disconnected and calls `record_termination`. The inbound request's
`finish_received` discards unnamed incomplete inodes, preserves linked
complete members and exposes `finish_partial` only at terminal state.
No retry/send API is called by this pathway. Validate each boundary without
changing any source contract, and fix code only if the real regressions fail.

## Plan and rollback

1. [x] Inspect the error flow and existing handler/manager/receipt tests;
   inventory concrete hypotheses and select the missing transport boundary.
2. [x] Add deterministic accepted-file tests using the actual `DuplexStream`
   from the existing `admitted()` fixture: finish file A, write incomplete
   bytes of file B, close the peer before/within length/body of its next frame.
   Require actual `handle()` error (UnexpectedEof) before terminal mapping.
3. [ ] Apply the same terminal mapping as the manager and assert exact original
   manifest, Disconnected state, one retained A receipt with a verified
   read-only descriptor, no B final filename, no staging FD/publication owner,
   no change to accepted A bytes, and no second transfer/source replay.
   Compare pre-terminal versus post-terminal capability visibility.
4. [ ] Run affected core CI, format, strict lint and GUI check once, investigate
   any concrete failures and only then fix their cause. Record source SHA,
   jobs, assertions and unresolved physical limits.
5. [ ] Update current state and handoff. Keep native29 installed and native30
   packaging/desktop/peer acceptance pending. No new branch or merge.

Rollback is the isolated test commit and any separately justified production
fix. No API, framing, crypto, consent, storage policy or UX decision is being
made. No speculative timeouts or mocks of `InboundRequest`/receipt domain
logic; only the external peer's socket is controlled.

## Scoped test inventory and branch map

| Artifact / branch | Test level | Expected table cases | Risk |
|---|---|---:|---|
| `InboundRequest::handle` / `FrameHeader::read`: EOF with 0/1/2/3 prefix bytes | real duplex integration | 4 | high |
| `InboundRequest::_handle`: complete length header then truncated body | real duplex integration | 1 | high |
| `InboundRequest::record_termination` / `finish_received`: pre/post partial receipt and one preserved published member | real Linux FS + session | paired with five cases | high |
| `ReceivedFiles::open`: sorted identity, read-only FD, no unaccepted member | real Linux FD contract | paired with five cases | high |
| Manager's failure classification: established non-AppError versus initial probe | existing manager tests | unchanged | high |

Branch interactions: accepted consent x published A x partially written B
x [EOF at prefix 0/1/2/3 | EOF during frame body] x terminal mapping.
For each case, assert the exact received header error, absence of Finished,
post-termination receipt subset and resource cleanup. Existing regressions
cover successful EOF, explicit cancel, short final chunks, collision, write
failure and service shutdown independently.

## Positive/N1-N12 applicability matrix

| Category | Scoped disposition |
|---|---|
| Positive | Existing complete-first and completed-multi-file assertions remain green |
| N1 | Planned: invalid/truncated framing is rejected |
| N2 | Planned: prefix offsets 0-3 and declared-body short read |
| N3 | Planned: actual external peer transport closure and EOF |
| N4 | Planned: no duplicate result/source replay after terminal call |
| N5 | Existing owned generation and manager cancellation tests; no new parallel request |
| N6 | Planned: malformed/incomplete frame is not interpreted as another payload |
| N7 | Existing authenticated control/consent; no new privileged API |
| N8 | N/A: no wall clock, date or locale rule |
| N9 | Existing explicit cancel/quit; peer interruption is this scope |
| N10 | Planned: no peer/private paths or bytes in user-visible error assertions |
| N11 | N/A: no purchases/entitlements |
| N12 | Existing `/dev/full`, collision and fsync failure tests; incomplete staging cleanup is rechecked |

Estimated new methods: one table-driven integration method, five independent
socket/fragment cases, plus direct cross-check of existing tests. Mutation
review should catch dropping the prefix, treating EOF as successful completion,
retiring the first published member, publishing the second partial member,
forgetting FD cleanup, manufacturing Finished or replaying Send.
Full repository-wide MODE B inventory/coverage, 10 full runs and real device
acceptance remain separate gates.

## Results and failure triage

First exact-source run at `a3d08591` passed Rust format, frontend
and core strict Clippy. The full core library run was 696 passed / 1 failed
in the new EOF regression; the new test stopped on `unwrap_err()` because
one `handle()` iteration legitimately returned `Ok(())` before processing
the EOF. Native GTK CI completed successfully. This was not evidence of
lost received bytes.

The shared `admitted()` fixture manually adds a managed `SessionLease` to
a legacy `test_request()`, leaving the old broadcast subscriber enabled.
`process_introduction` and `accept_transfer` also emit legacy UI events;
one of those ready events wins the first `handle()` select and is ignored,
returning `Ok(())`. The real managed constructor calls `attach_session`,
which disables the legacy receiver. Corrected only the new test setup with
`request.receiver = None` at `7820b155`. The EOF and filesystem
assertions remain unchanged and no production code was modified.

The resulting headless Preflight and native GTK checks are in progress:
https://github.com/morozhenko-apps/rquickshare/actions/runs/39052547406
https://github.com/morozhenko-apps/rquickshare/actions/runs/39052547479

No completed pass is claimed for the corrected test at this checkpoint.

