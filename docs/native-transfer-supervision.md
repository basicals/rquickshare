# Structured TCP transfer supervision

2026-10-12, Mode A. Status: S0–S2 complete; S3 ownership seam is complete in [native-managed-dispatch.md](native-managed-dispatch.md); live state/consent mapping remains pending. The user approved option 1 from [native-core-adapter-review.md](native-core-adapter-review.md): fix bounded task supervision now, then continue the native service integration.

## Analysis and invariants

Before this change, `TcpServer::run` awaited an entire outbound session, blocking accepts; inbound tasks were detached with `tokio::spawn` and were not awaited by `RQS::stop()`. Cancellation was checked only inside the outbound handler loop, after TCP connect and initial writes. `RQS` tracks the manager future, so joining all manager-owned workers before returning extends its existing shutdown barrier to those TCP workers without changing `RQS::run()`/`SendInfo` JSON/TS contracts.

Changes are limited to TCP orchestration in `manager.rs` and a private worker module. Crypto, framing, payload preparation and protocol consent remain the existing real handlers. This does not supervise BLE/GATT workers inside their own modules, guarantee preempting synchronous filesystem operations, or provide an authoritative persistent core snapshot.

- A `JoinSet` owns all TCP session/classification tasks. No detached TCP workers. The manager collects completion/errors during operation; on stop or fatal accept/worker failure it closes admission, cancels its own child lifetime, drains queued commands, and joins all workers before returning. It never cancels the caller's token itself.
- One internal TCP budget has two allocations: 16 normal sessions shared by inbound/outbound, plus 4 classification slots. Defaults are provisional local resource limits, not Google limits. The bounded classification reserve is necessary to route BWU sockets into an already running BLE session when normal slots are full. Classification slots are released when a socket is routed/dropped or promoted into a session. No queued task per accepted socket beyond these bounds.
- At saturation, stop removing outbound commands until a session is available; at most one held queue head is kept in addition to the existing bounded mpsc buffer. Inbound sockets are classified using the reserve, then immediately dropped if no normal-session permit is available. Do not invent an inbound transfer UI event for an initial/unclassified probe. Existing BWU routing/timeout behavior is preserved; improving its peek polling is a separate protocol change.
- Legacy Tauri reuses endpoint IDs as transfer IDs. Serialize outbound jobs with an already active ID rather than run two same-ID handlers whose control messages would be ambiguous. The native coordinator issues unique IDs. Head-of-line blocking between duplicate legacy IDs may remain in the outbound queue; it must not block the accept loop.
- A worker's shutdown token covers TCP connect, initial writes, frame reads/handler work and BWU classification. Cancelled active outbound requests emit one Cancelled event with their stable ID unless a terminal outcome already exists. Inbound initial probes stay silent; established inbound cancellation is typed Inbound. Existing user Accept/Reject/Cancel handling remains in the protocol handlers. This shutdown guarantee must not be mislabeled as reliable replay of legacy frontend commands issued before a worker subscribes; the upcoming managed-service adapter requires an owned cancellation handle across queue admission.
- Already Finished/Rejected/Cancelled/Disconnected states are absorbing for later manager errors/cancellation. Socket, payload and permit lifetimes end before the joined worker completes. No new cancellation timeout or abort-without-await shortcut.
- Worker panics are observed by the manager and cause a supervised error shutdown; they are not silently ignored or represented as successful transfers. Remaining workers are cancelled/joined. A future service core-health seam must surface a fatal manager exit; current RQS background logging is not an authoritative recovery channel.

## Milestones and rollback

1. S0: record approval, contracts, branch/risk inventory and test matrix before code; atomic docs commit.
2. S1: extract cloneable protocol context; implement bounded manager-owned workers, cancellation and drain; preserve existing constructors/legacy interfaces. Commit with focused tests.
3. S2: run the affected core suite once, format/Clippy, update measured evidence and handoff. No ten-run loop. Do not repeat unchanged desktop/frontend suites.
4. S3: continue the native adapter: typed ownership/cancellation across dispatch, state mapping and supervised core-health reporting. Document the concrete adapter slice before implementing it.

Rollback S1 independently of descriptor input/coordinator. Risks: scheduling/race changes, peer behavior on shutdown, slot saturation and legacy reused IDs; real Android/Windows and BLE-to-Wi-Fi physical acceptance remain gates. No activation or package registration yet.

## Scoped artifact / branch / interaction inventory before tests

| Artifact | Branches / rules | Test level / expected methods |
|---|---|---|
| Limits / permits | zero limits; exact/over session/classifier capacity; permit released on route/failure/stop; shared directions | Unit + loopback, 4 |
| Main manager loop | outbound/inbound readiness, empty/closed queue, held queue head, duplicate active ID, worker completion, stop before/after admission, accept/worker fatal error | Loopback + controlled futures, 7 |
| Outbound context | connect/constructor/init/handler failure, active vs terminal cancellation, stable identity, every stage under lifetime token | Loopback + contract future, 5 |
| Inbound context | initial/established/terminal, normal completion/error/cancellation, classification route/drop/error/pending, full sessions | Loopback + existing real protocol fixtures, 5 |
| Shutdown / task results | drain buffered/head jobs; joining blocked reads/classification; released payload refs; panic causes error and cancels siblings | Loopback + controlled task, 4 |

Estimate: 25 new methods plus all existing manager/handler/BWU/resource/coordinator tests. External waits use event/oneshot/channel readiness, never test sleeps. Generic pending-future tests may verify cancellation wiring; actual TCP tests must independently reproduce simultaneous sessions and stalled peers. No domain handler is mocked.

One additional integration method will drive two real TCP managers through repeated text transfers and actual consent, without external devices or clipboard integration. This covers successful protocol completion/terminal absorption under the new supervisor, beyond stalled-handshake scenarios. The estimate becomes 26 methods; it does not certify Google peer compatibility.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Limits / admission | P | P | P | X | P | P | X | P | X | P | X | X | X |
| Manager / context | P | P | P | P | P | P | P | P | X | P | X | X | X |
| Shutdown / resources | P | X | P | P | P | P | P | P | X | P | X | X | X |

P planned, X not applicable: N8 no clocks/locales/currencies; N10 no new payload logging/diagnostic schema (existing formatter tests retained); N11 no billing; N12 adds no storage operation here; owned-handle release is N9, while actual read/write faults remain file-reader tests; N3 absent from permit-only logic; N6 means worker/channel loss, not a new persisted-data format. N7 means identity/control direction and capacity containment, not new authentication.

Testability detail: private `AcceptSource`/`Connector` contracts inject only external network operations. Production uses Tokio TCP unchanged; controlled futures/failures/panics permit deterministic admission, connect-cancellation and fatal-worker tests without mocking the protocol handlers or manager. Integration waits observe frames, accepted sockets, channels or joined completion. A five-second outer scenario watchdog detects deadlocks; it is not a sleep or an expected timing threshold, and no scenario waits for a fixed duration to pass.

Interactions: stalled outgoing handshake × new incoming accept × another outbound; shared capacity full × normal inbound/BWU route; reused/distinct IDs × queue head × cancellation/completion; shutdown × queued/head/connecting/initial-writing/reading/classifying/terminal; worker error/panic × siblings/payload refs. Assert event IDs/types/outcomes, byte/frame readiness, socket EOF, joined completion and capacity reuse. Mutation targets: await outgoing inline; detach inbound; drop join barrier; remove/shift `>=` guards; release permits too early; cancel caller instead of child; override a terminal outcome; start duplicate legacy IDs concurrently.

## Verification facts

- S0 completed in `e8506b8`, before production changes. S1 implemented in `core_lib/src/manager.rs` and private `manager/session.rs`; Linux loopback/contract tests live in `manager/supervision_tests.rs`. No dependency, JSON/TS, crypto or framing changes.
- One `cargo +stable test --all-features` attempt executed 261 unit/export methods: 260 passed, one test failed because it incorrectly required FIN/EOF when closing a socket with an unread partial frame. Linux may send RST for unread data ([upstream TCP close implementation](https://raw.githubusercontent.com/torvalds/linux/master/net/ipv4/tcp.c), `__tcp_close`). The corrected test accepts only zero-byte EOF or `ConnectionReset`, still requires no bytes/UI event and awaits the manager barrier. Its affected targeted rerun passed (1 method). No production change was needed for that failure.
- Cargo stopped before integration tests on the failed unit assertion; a separate `cargo +stable test --all-features --test modern_protocol` run passed all 4 integration methods. No unchanged unit suite was rerun.
- One subsequently added targeted method passed: two actual managers execute two sequential full UKEY2/encrypted-text transfers with real user-consent messages. Both directions report Finished and correct payload/identity; shutdown emits no overriding Cancelled state. This added method ran once after compilation in 0.37 seconds. Current inventory is 262 unit/export + 4 integration = 266; this is not a claim that all 266 ran in one invocation.
- Format, Clippy with all features/all targets and warnings denied, and diff checks passed after the final test change. The preexisting bluer future-incompatibility notice remains. No ten-run development loop, physical devices, live Bluetooth service, GUI, installation or system registration. Owned command sessions exited (terminal closed).

### Coverage and continuation limits

26 new methods cover the scoped manager/context contracts. Primary categories (each method counted once): Positive=4, N1=3, N2=3, N3=3, N4=1, N5=0, N6=2, N7=1, N8=0, N9=9, N10=0, N11=0, N12=0. Concurrent admission, shutdown and lifetime scenarios also exercise N5; handle release overlaps N9 rather than claiming storage-failure injection. N8/N10/N11/N12 introduce no independent behavior, as explained above. The matrix P cells describe the preregistered plan and are exercised by these tests; there is no instrumented branch-percentage claim.

`manager.rs`: limits, shared admission, closed queue, duplicate IDs, concurrent directions, worker collection, fatal accept/panic, queue drain and join barrier. `session.rs`: cancellation before polling/connect, real connection/constructor/handshake failures, initial versus established inbound errors, partial reads, BWU route under saturation, terminal suppression and successful repeated protocol completion. Existing manager tests retain error Unicode bounds and all-state failure mapping. Initial write interruption is covered by the enclosing cancellation operation contract, not a separately forced kernel send-buffer saturation case. BLE/GATT internals and synchronous filesystem preemption are outside this slice.

Source/assertion mutation review targets detached workers, inline outbound awaits, removing shared permits, running reused IDs together, skipping drains/joins, cancelling the parent token and overriding terminal outcomes. No mutation-tool execution was performed for this slice. No repository-wide Mode B completion or Google interoperability claim.

The legacy raw mpsc API permits external callers to hold reserved permits across receiver closure. This barrier drains commands available at shutdown; it cannot wait indefinitely for an externally owned permit. The managed adapter must expose bounded immediate submission and owned cancellation, without exposing mpsc permits. Its next plan must also address reporting fatal manager exits and rollback of partially started RQS transports.

### Implemented test index

| Method | Primary category |
|---|---|
| `given_invalid_worker_limits_when_validated_then_zero_and_overflow_are_rejected` | N2 |
| `given_cancellation_when_message_is_built_then_initial_probes_and_terminal_outcomes_are_preserved` | N4 |
| `given_live_lifetime_when_operation_completes_then_original_result_returns` | Positive |
| `given_cancelled_lifetime_when_operation_is_ready_then_operation_is_not_polled` | N9 |
| `given_blocked_operation_when_lifetime_stops_then_future_is_dropped_before_join_returns` | N9 |
| `given_stalled_outgoing_when_new_inbound_and_outgoing_arrive_then_all_handshakes_progress` | Positive |
| `given_shared_session_budget_when_inbound_arrives_at_capacity_then_probe_is_closed_without_ui_transfer` | N2 |
| `given_full_session_budget_when_worker_fails_then_next_queued_worker_starts` | N3 |
| `given_distinct_outgoing_ids_when_two_slots_are_full_then_third_waits_until_capacity_returns` | N2 |
| `given_reused_legacy_id_when_another_job_is_queued_then_same_id_handlers_never_overlap` | N7 |
| `given_buffered_head_and_active_owned_payloads_when_stopped_then_all_resources_drop_before_return` | N9 |
| `given_pre_cancelled_manager_when_queue_contains_work_then_no_connector_is_polled` | N9 |
| `given_pre_cancelled_outbound_context_when_started_then_payload_closes_without_connecting` | N9 |
| `given_initial_inbound_probe_when_service_stops_then_socket_closes_without_ui_transfer` | N9 |
| `given_established_inbound_when_service_stops_then_one_typed_cancellation_is_published` | N9 |
| `given_malformed_initial_probe_when_worker_exits_then_no_transfer_notification_is_created` | N1 |
| `given_malformed_established_peer_when_worker_exits_then_failure_keeps_inbound_identity` | N1 |
| `given_partial_inbound_body_when_service_stops_then_blocked_frame_read_is_joined` | N9 |
| `given_full_normal_budget_when_bwu_socket_arrives_then_reserved_classifier_routes_without_consuming_bytes` | Positive |
| `given_pending_classification_workers_when_service_stops_then_every_socket_closes_before_return` | N9 |
| `given_fatal_accept_failure_when_outbound_is_blocked_then_child_workers_stop_without_cancelling_caller` | N3 |
| `given_panicking_network_worker_when_sibling_is_active_then_fatal_result_joins_and_releases_all_resources` | N6 |
| `given_closed_outbound_queue_when_inbound_arrives_then_accept_loop_remains_responsive` | N6 |
| `given_invalid_budgets_when_manager_runs_then_network_and_queue_are_untouched` | N1 |
| `given_constructor_failure_when_another_send_is_queued_then_session_capacity_is_reused` | N3 |
| `given_two_real_managers_when_text_is_accepted_then_repeated_transfers_finish_without_shutdown_overrides` | Positive |
