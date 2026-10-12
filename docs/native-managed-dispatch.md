# Managed core dispatch ownership

2026-10-12, Mode A. Status: Q0–Q3 complete. The initial plan was committed in `2f451e9` before code. Continues approved S3 in [native-transfer-supervision.md](native-transfer-supervision.md). This is an additive internal Rust integration seam for the already approved daemon/coordinator architecture, not a published IPC contract or UX choice.

## Analysis and decisions

The legacy `mpsc::Sender<SendInfo>` and broadcast CancelTransfer control cannot reliably cancel before a handler subscribes. Raw mpsc permits can also outlive receiver closure. Keep that legacy API and serialized SendInfo/TS unchanged for Tauri rollback. Add a managed bounded queue used by `RQS::run_managed()` for the Linux native service. The queue and entrypoint are platform-independent Rust so existing non-Linux core builds do not retain an unused private variant; descriptor input/coordinator remain Linux-only. It accepts the same internal SendInfo from a trusted core adapter, never directly from IPC callers; the future adapter must resolve coordinator-issued endpoint IDs itself rather than accepting caller addresses.

1. `ManagedSender::try_send` performs immediate bounded admission, exposing no reserve/send permit. Full/closed errors return ownership of the unsent SendInfo. Accepted dispatch returns a `TransferControl` containing a private cancellation token and a release notification created before queue insertion. Dropping the control does not cancel a service-owned send.
2. The TCP manager handles legacy and managed queues through a private receiver adapter, preserving its existing capacity and same-ID serialization. A cancelled held head is removed even while all session slots are full. A cancelled buffered item is removed when FIFO reaches it; there is no promise of immediate out-of-order resource release. It must never poll connect when already cancelled.
3. Outbound cancellation races both the manager lifetime and the owned request token over connect, initial writes and the entire real handler loop. Native cancellation closes the local transport and reports Cancelled unless a terminal outcome already exists. This is an interruption guarantee, not a promise that the remote peer sees Cancelled rather than disconnection. Existing legacy protocol control handling remains unchanged.
4. `TransferControl::wait_released` acknowledges only that normal manager ownership of the job has ended, after dropping its input payload/request. It never means delivered/Finished. Panic/lost supervisor notification produces an explicit control-loss error, never a fabricated release acknowledgement or transfer outcome. Original payload references retained by another owner can still keep coordinator reservations live.
5. Managed queue closure is an observable core-health loss signal (`is_closed` / `closed`). The daemon adapter must treat unexpected closure or lost control notification as desynchronization, close admission and await shutdown; do not fabricate terminal coordinator observations. RQS observes fatal TCP manager error/panic, logs a bounded static reason and cancels its transport lifetime. This does not certify the health or supervision of every optional BLE worker.
6. RQS startup gets a single internal helper shared by old/new entrypoints. Reject double start instead of abandoning old tracked transports. On startup error, close the tracker, cancel and await already started transports before returning the error. A drop guard cancels transports when the startup future is abandoned; a subsequent stop still closes/joins the retained tracker. Stop always closes the tracker before waiting, including partial initialization. No privacy/visibility fallback or settings migration.
7. Final source review found fixed-priority `biased` polling in the manager could favor a continuously ready outbound/completion source over accepts. Use Tokio's default fair select polling for this admission loop and check service cancellation before each iteration. Keep biased cancellation in the per-operation worker, where cancellation must win over starting more I/O. Existing simultaneous-direction/capacity/stop tests remain the applicable focused check after this internal scheduling correction; do not claim a measured physical latency bound.

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
| Queue / control | Covered | X | Covered | X | Covered | Covered | Covered | X | X | Covered | X | X | X |
| Manager / worker | Covered | X | Covered | Covered | Covered | Covered | Covered | Covered | X | Covered | X | X | X |
| Lifecycle | Covered | Covered | X | Covered | Covered | Covered | Covered | X | X | Covered | X | X | X |

X reasons: N1 queue capacity is internal nonzero constant, SendInfo validation remains core preparation; lifecycle invalid repeated start is applicable. N3 queue operations do not perform network I/O. N7 manager identity/terminal rules remain applicable; these Rust handles are not IPC authentication. N8 no clock/locale arithmetic, N10 no new payload diagnostics (bounded static lifecycle failures), N11 no billing, N12 no new storage operation (FD release is N9, storage faults remain file-reader tests).

Interactions: cancelled/not × queued/head/connect/handshake/terminal × service stop; full queue × rejected payload ownership × retry after release; worker panic × surviving active sibling × closed admission; partial start error/drop × active tracked worker × stop. Assertions validate event identity, no connector polling after pre-cancel, exact terminal count, actual weak payload lifetime, release ordering, no invented success, no leaked tracked task and absorbing terminal outcomes.

Mutation review targets: remove either cancellation branch, subscribe after insertion, acknowledge before payload drop, convert control loss to success, drop rejected input, cancel on UI/control drop, bypass duplicate start, omit tracker.close/stop on initialization error, discard manager errors. Instrumented coverage/mutation percentages are not claimed.

## Verification facts

- Q1/Q2 are implemented in `managed.rs`, `manager.rs`, `manager/session.rs` and the shared RQS startup/lifecycle helper in `lib.rs`. The legacy run return type, SendInfo serialization, protocol framing and crypto remain unchanged. No dependency, lockfile, live service, system setting or package registration change.
- One completed `cargo +stable test --all-features` execution passed **284 unit/export + 4 integration = 288 tests**, including **22 new methods**. The first attempt stopped at compilation on three test-fixture type adaptations; those were corrected before any assertions ran. The successful unit execution reported 0.41 seconds after compilation, not a runtime guarantee. Historical supervisor results remain in their own section.
- Final review then corrected manager admission fairness and made the pure-Rust managed entrypoint available independent of platform cfg. One affected `cargo +stable test --all-features manager::` invocation passed **44 methods** on that final source. No unchanged full suite was repeated. Linux lifecycle execution is unchanged by removal of the cfg; no non-Linux execution/cross-compile claim.
- Final all-feature/all-target Clippy with warnings denied, nightly format check and git diff check passed. Only the existing bluer future-incompatibility notice remains. No ten-run loop. Owned command sessions exited (terminal closed).
- Primary categories, without double-counting table rows or overlaps: Positive=2, N1=1, N2=1, N3=2, N4=2, N5=0, N6=5, N7=0, N8=0, N9=9, N10=0, N11=0, N12=0. N5/N7 overlap concurrent cancellation, identity and terminal-preservation checks; other X categories have the reasons recorded above. No full Mode B, live D-Bus/GUI or Google interoperability acceptance.

### File/function coverage and assertion review

| Production file | Exercised rules | Remaining / limits |
|---|---|---|
| `managed.rs` | try_send success/full/closed with owned rejected input; receiver live/close/drain; early/repeated cancel, drop-without-cancel; normal/lost release, external payload refs; queue closure | Release is only local ownership acknowledgement. Other platform runtimes are not executed. No storage writes, IPC decoder or authentication implemented. |
| `manager.rs` | Legacy and managed receipt, cancelled buffered/held/active jobs, capacity reuse, stop drain, panic loss, accepts after sender disconnect | Fair select removes fixed priority; no measured physical latency or exhaustive scheduler interleaving claim. Legacy reserved-permit limitation remains outside managed API. |
| `manager/session.rs` | Paired service/request cancellation over real connect/handshake/read path, full protocol completion and absorbing terminal outcomes | Explicit kernel send-buffer saturation during initial writes is not forced; cancellation encloses the operation. Native interruption need not emit a graceful remote Cancelled frame. |
| `lib.rs` | Both duplicate-start entrypoints; initial TCP bind failure; actual lifecycle error cleanup/join, guard drop, unclosed tracker stop; normal/error/panic TCP exit classification | Successful full RQS initialization would involve real mDNS/BlueZ and is not performed. Optional BLE constructor/child lifetime and global download ownership remain separate adapter/release gates. |

Assertions target dropping rejected resources, acknowledging before request release, treating lost notification as success, cancelling on UI-handle drop, missing either lifetime signal, overriding Finished, ignoring fatal manager exit and failing to close an initialized tracker. This is a source/assertion review, not executed mutation tooling or instrumented branch coverage.

### Continuation boundary

The managed outgoing queue/control seam is ready for the trusted native adapter. Inbound Accept/Reject still travels through the lossy legacy broadcast with no per-session processing acknowledgement; the adapter must not equate broadcast send success with coordinator `acknowledge_control`. A concrete session-control review is required before that larger core control-plane refactor. No provisional live adapter is activated around this limitation.


### Implemented test index

| Method | Primary category |
|---|---|
| `given_full_queue_when_admission_retries_then_rejected_input_is_retained_until_capacity_returns` | N2 |
| `given_closed_receiver_when_submitted_then_closure_is_observable_and_input_returns` | N6 |
| `given_accepted_control_when_cancelled_before_receive_then_job_already_observes_cancellation` | N9 |
| `given_service_owned_send_when_control_is_dropped_then_job_is_not_cancelled` | Positive |
| `given_owned_payload_when_release_is_acknowledged_then_job_drops_its_reference_first` | N9 |
| `given_lost_dispatch_owner_when_waiting_then_no_release_or_success_is_invented` | N6 |
| `given_completed_dispatch_when_cancel_and_wait_repeat_then_release_remains_idempotent` | N4 |
| `given_legacy_receiver_when_closed_then_buffered_inputs_are_drained_with_identity_preserved` | Positive |
| `given_full_sessions_when_managed_head_and_buffer_are_cancelled_then_no_cancelled_job_connects` | N9 |
| `given_connecting_managed_job_when_cancelled_then_slot_is_reused_without_stopping_service` | N9 |
| `given_stalled_managed_handshake_when_cancelled_then_socket_and_payload_release_before_acknowledgement` | N9 |
| `given_partial_outbound_frame_when_owned_control_cancels_then_blocked_read_is_joined` | N9 |
| `given_managed_worker_panic_when_joined_then_queue_closes_and_release_loss_is_explicit` | N6 |
| `given_managed_active_head_and_buffered_jobs_when_service_stops_then_all_inputs_release_before_return` | N9 |
| `given_managed_transfer_finished_when_owned_control_is_cancelled_then_success_is_not_overridden` | N4 |
| `given_managed_senders_disconnected_when_inbound_arrives_then_manager_still_accepts` | N6 |
| `given_started_instance_when_both_entrypoints_restart_then_existing_lifetime_is_preserved` | N1 |
| `given_partial_startup_when_initialization_fails_then_workers_are_joined_before_error_returns` | N3 |
| `given_abandoned_startup_guard_when_dropped_then_started_workers_cancel_and_stop_can_join` | N9 |
| `given_unclosed_tracker_when_stopped_twice_then_stop_closes_and_joins_once` | N9 |
| `given_tcp_manager_exit_when_observed_then_error_panic_and_unexpected_success_cancel_transports` | N6 |
| `given_occupied_port_when_startup_fails_then_state_is_reset_without_starting_other_transports` | N3 |
