# Managed core dispatch ownership

2026-10-12, Mode A. Status: planned before code. Continues approved S3 in [native-transfer-supervision.md](native-transfer-supervision.md). This is an additive internal Rust integration seam for the already approved daemon/coordinator architecture, not a published IPC contract or UX choice.

## Analysis and decisions

The legacy `mpsc::Sender<SendInfo>` and broadcast CancelTransfer control cannot reliably cancel before a handler subscribes. Raw mpsc permits can also outlive receiver closure. Keep that legacy API and serialized SendInfo/TS unchanged for Tauri rollback. Add a managed bounded queue used by `RQS::run_managed()` on Linux. It accepts the same internal SendInfo from a trusted core adapter, never directly from IPC callers; the future adapter must resolve coordinator-issued endpoint IDs itself rather than accepting caller addresses.

1. `ManagedSender::try_send` performs immediate bounded admission, exposing no reserve/send permit. Full/closed errors return ownership of the unsent SendInfo. Accepted dispatch returns a `TransferControl` containing a private cancellation token and a release notification created before queue insertion. Dropping the control does not cancel a service-owned send.
2. The TCP manager handles legacy and managed queues through a private receiver adapter, preserving its existing capacity and same-ID serialization. A cancelled held head is removed even while all session slots are full. A cancelled buffered item is removed when FIFO reaches it; there is no promise of immediate out-of-order resource release. It must never poll connect when already cancelled.
3. Outbound cancellation races both the manager lifetime and the owned request token over connect, initial writes and the entire real handler loop. Native cancellation closes the local transport and reports Cancelled unless a terminal outcome already exists. This is an interruption guarantee, not a promise that the remote peer sees Cancelled rather than disconnection. Existing legacy protocol control handling remains unchanged.
4. `TransferControl::wait_released` acknowledges only that normal manager ownership of the job has ended, after dropping its input payload/request. It never means delivered/Finished. Panic/lost supervisor notification produces an explicit control-loss error, never a fabricated release acknowledgement or transfer outcome. Original payload references retained by another owner can still keep coordinator reservations live.
5. Managed queue closure is an observable core-health loss signal (`is_closed` / `closed`). The daemon adapter must treat unexpected closure or lost control notification as desynchronization, close admission and await shutdown; do not fabricate terminal coordinator observations. RQS observes fatal TCP manager error/panic, logs a bounded static reason and cancels its transport lifetime. This does not certify the health or supervision of every optional BLE worker.
6. RQS startup gets a single internal helper shared by old/new entrypoints. Reject double start instead of abandoning old tracked transports. On startup error, close the tracker, cancel and await already started transports before returning the error. A drop guard cancels transports when the startup future is abandoned; a subsequent stop still closes/joins the retained tracker. Stop always closes the tracker before waiting, including partial initialization. No privacy/visibility fallback or settings migration.

## Milestones and rollback

- Q0: commit contracts, complete scoped branch inventory and matrix before production changes.
- Q1: add queue/control ownership, receiver adapter, paired cancellation and managed RQS entrypoint; preserve legacy protocol/serialization.
- Q2: add RQS fatal-manager observation and startup rollback/stop guard; focused contract/loopback tests, no BlueZ or mDNS process in fixtures.
- Q3: run changed tests once, format/Clippy, record actual evidence, update diary and commit. Existing unaffected suites are not repeated. Then continue actual coordinator-to-core mapping before D-Bus/UI activation.

Rollback: revert this additive slice independently of the supervisor/coordinator. Risks are cancellation versus terminal races, hidden payload ownership, closed queue versus successful delivery, and partially initialized transport cleanup. Real Google peer behavior and optional BLE lifetime remain physical/independent gates.

## Scoped artifact and branch inventory

| Artifact | Branches / invariants | Level / expected methods |
|---|---|---|
| Managed admission | live/full/closed, rejected ownership retained, accepted control independent of handle lifetime, no externally reserved permits | Unit/kernel FD, 4 |
| Release/control | before/after normal completion, repeat cancel/wait, loss before acknowledgement, external payload retained; no delivery implication | Unit/kernel FD, 4 |
| Manager receiver | legacy/managed receive, close/drain, cancelled buffered/head/active, duplicate IDs under cancellation, closed queue retains accepts | Loopback + external-network DI, 5 |
| Outbound paired cancellation | pre-cancelled request/service, pending connect, stalled handshake/partial frame, successful Finished then cancel | Real protocol + controlled connector, 4 |
| RQS lifecycle | duplicate start, start error cancels and joins, abandoned initialization cancels, stop closes unclosed tracker; TCP success/error/panic closure and parent cancellation | Controlled task/tracker contract, 5 |

Estimate: 22 new methods with table subcases. Reuse the supervisor's real protocol fixtures; do not mock manager/coordinator/handlers. Tests wait on channels, frames, token cancellation or task completion. A five-second outer deadlock watchdog is not a synchronization sleep.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Queue / control | Planned | X | Planned | X | Planned | Planned | Planned | X | X | Planned | X | X | X |
| Manager / worker | Planned | X | Planned | Planned | Planned | Planned | Planned | Planned | X | Planned | X | X | X |
| Lifecycle | Planned | Planned | X | Planned | Planned | Planned | Planned | X | X | Planned | X | X | X |

X reasons: N1 queue capacity is internal nonzero constant, SendInfo validation remains core preparation; lifecycle invalid repeated start is applicable. N3 queue operations do not perform network I/O. N7 manager identity/terminal rules remain applicable; these Rust handles are not IPC authentication. N8 no clock/locale arithmetic, N10 no new payload diagnostics (bounded static lifecycle failures), N11 no billing, N12 no new storage operation (FD release is N9, storage faults remain file-reader tests).

Interactions: cancelled/not × queued/head/connect/handshake/terminal × service stop; full queue × rejected payload ownership × retry after release; worker panic × surviving active sibling × closed admission; partial start error/drop × active tracked worker × stop. Assertions validate event identity, no connector polling after pre-cancel, exact terminal count, actual weak payload lifetime, release ordering, no invented success, no leaked tracked task and absorbing terminal outcomes.

Mutation review targets: remove either cancellation branch, subscribe after insertion, acknowledge before payload drop, convert control loss to success, drop rejected input, cancel on UI/control drop, bypass duplicate start, omit tracker.close/stop on initialization error, discard manager errors. Instrumented coverage/mutation percentages are not claimed.

## Verification facts

Not run yet. Historical supervisor results remain in their own SSOT section.
