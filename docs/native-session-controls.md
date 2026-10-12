# Common managed TCP/BLE session controls

2026-10-12, Mode A. Status: U0–U4 implemented and verified offline. The plan was recorded before code in `b774770`. The user selected option 1 in [native-session-control-review.md](native-session-control-review.md). Applicable guide: `/home/morozhenko/.codex/AGENTS.md`; one appropriate development verification run, not ten repetitions.

## Analysis and accepted implementation contract

Legacy handlers subscribe to a mixed event/control broadcast; TCP inbound IDs use socket addresses and GATT reuses `ble-weave`. `weave_session` detaches its handler intentionally so it survives BWU, but the GATT server has no shutdown barrier for that task. The native service needs owned live-session controls across both transports, independently of UI lifetime. Preserve legacy entrypoints/control and protocol bytes/crypto; the native route ignores legacy broadcast commands.

1. Add a platform-independent internal Rust `sessions` module, not an IPC/authentication interface. A per-run `SessionHub` has an injected nonzero 128-bit epoch, checked monotonic sequence and a provisional 32-live-session budget. No global registry. Native outbound jobs register before queue insertion; active TCP/BLE inbound handlers register before handling. Rejected submissions retain input ownership. Registration failure closes an unestablished inbound probe without a fabricated transfer event.
2. Opaque `SessionId` and `ConsentToken` cannot be constructed from peer addresses, caller strings or coordinator IDs. Inbound managed event IDs derive from the fresh session identity; outbound event correlation keeps SendInfo.id while its session capability remains independent. Keep the same lease and control through BLE-to-TCP stream migration. Retain no unbounded terminal history in the hub: completed handlers release their registry slot; retained client handles observe closure and their last actual state.
3. Each lease owns one bounded consent mailbox and one cancellation token. `SessionControl::respond` requires its current consent token, nonzero adapter command ID and answer; receipt is explicit. The handler checks the actual waiting state/token before accepting, records one replay decision per consent round and acknowledges acceptance before protocol side effects. Exact same token/command/answer replay returns its original receipt; conflicting reuse fails. New rounds invalidate old tokens/decisions. This acknowledgement never means that peer I/O succeeded or delivery finished. Mailbox Full/Closed/Cancelled/Stale/Conflict errors are explicit; dropping a receipt/control does not cancel service-owned work.
4. Session status is a watch of the last actual protocol state and current consent token, with no file names, contents, socket addresses or raw errors. It is an internal active-session view, not authoritative persistent recovery or a new public snapshot schema. Owner loss closes the mailbox/watch without inventing Finished/Cancelled. Event-stream loss still requires coordinator desynchronization. Managed outgoing release acknowledgement stays separate from session processing receipts.
5. Attach the lease to the existing generic inbound/outbound handlers. Publish status before existing UI events. Native handlers ignore the legacy command stream. Inbound consent is processed from its owned mailbox; owned cancellation encloses connect/initial writes/whole frame reads/BWU waits. Absorb terminal states on later cancellation/errors. Preserve legacy Accept/Reject/Cancel behavior and JSON/TS.
6. Pass the same hub from ManagedSender/RQS to TCP context and ReceiverGattServer. Add a GATT-owned bounded task group with an admission lock, tracked tasks and child lifetime. Reject new subscriptions/tasks after closure. On GATT stop, close admission, cancel and join the group. A migrated protocol worker remains tracked after its weave bridge closes; bridge closure alone never cancels its new TCP transport. The existing single weave bridge is bounded to one; the group budget is 33 (up to 32 session workers plus bridge), provisional resource limits rather than Google protocol limits. Server initialization/callback stop must not create tasks after the barrier.
7. Root-cause corrections within this scope: prune closed BWU oneshot waiters before register/route/has_pending so a cancelled upgrade cannot leave a stale route intercepting later sockets; retain existing routing/alias/crypto/timeouts. Replace the handlers' 10-microsecond broadcast-delivery sleeps with cooperative `yield_now`: scheduling progress is an event/executor concern, not evidence that a command was received. Existing BWU protocol pacing is not changed by that correction.
8. The bounded weave bridge must observe notification disconnect before its initial connection handshake as well as during data transfer; otherwise a disconnected probe can retain the sole bridge slot indefinitely. GATT initialization/registration and optional BLE constructors observe the core lifetime, and server failure closes/joins its group before returning. These are ownership cleanup, not a new discoverability policy; external SDK background internals still require physical lifecycle verification.

## Milestones, risk and rollback

- U0: commit approval, contracts, complete scoped inventory/matrix before production code.
- U1: implement hub, lease, bounded commands/receipts/replay and task-group ownership. Add deterministic domain/lifetime tests. Atomic commit with verification evidence after the integrated check.
- U2: connect native queue/TCP/generic handlers; same lease semantics for BLE stream and BWU. Add real protocol/stream tests preserving terminal/resource identity and proving broadcast cannot control native sessions.
- U3: connect GATT task ownership and RQS propagation; fix closed BWU routes and scheduler sleeps. Deterministic task/route regressions; no real BlueZ required for tests.
- U4: run changed core tests once and format/Clippy, repair actual failures and rerun only affected checks; update diary/index/coverage limits and commit. No service, package registration, system install or GUI launch.

Rollback: the common-session slice is independent of file descriptors/coordinator/TCP supervisor; legacy Tauri path remains usable. Main risks: consent replay versus phase advancement, stale session identity, BWU lifetime, group-close versus callback admission, and conflating receipts with outcome. Physical Google Android/Windows and actual BlueZ/KDE remain acceptance gates.

## Scoped artifact / branch inventory before tests

| Artifact | Branches / invariants | Level / expected methods |
|---|---|---|
| Hub / identity / reservations | zero epoch/budget, exact/over capacity, exhausted sequence, cross-instance isolation, fresh incarnation, slot release, managed ID versus outbound correlation | Unit, 7 |
| Consent control / replay | inbound/outbound, Initial/Waiting/Active/all terminal, wrong session/round/zero command, exact replay/conflict, full/closed queue, cancelled before processing, dropped receipt | Unit + real handler, 10 |
| Status / ownership | wait entry/repeat/new round, token exhaustion, no metadata exposure, cancellation vs terminal, owner drop without outcome, watch readiness/closure, control drop without cancel | Unit, 6 |
| Generic inbound/outbound handler | real consent Accept/Reject, legacy ignored only on native path, pending partial read cancellation, correct IDs/types, outbound early cancel/release, terminal completion | Loopback + duplex real protocol, 6 |
| BLE / BWU | generic BLE stream shares same lease/control, stream upgrade retains capability, cancelled BWU route pruned, old waiter cannot consume later route, alias precedence retained | Duplex + loopback/kernel, 4 |
| GATT task group | exact/over budget, close versus admission, cancellation/join, migrated worker survives bridge completion, panic observed, no caller cancellation, repeat stop | Controlled futures, 6 |

Estimate: 39 new methods plus existing protocol/manager/coordinator/file tests. Table cases cover independent state/limit variants. Only external sockets/streams/tasks are controlled; handlers, registry and replay rules are real. Transport/task tests use five-second outer watchdogs to detect deadlock; synchronization waits on frames, channels, watch/token changes or task completion, never sleeps.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Hub / status | Covered | Covered | Covered | X | Covered | Covered | Covered | Covered | X | Covered | Covered | X | X |
| Commands / receipt | Covered | Covered | Covered | X | Covered | Covered | Covered | Covered | X | Covered | Covered | X | X |
| Protocol / transport owners | Covered | Covered | Covered | Covered | Covered | Covered | Covered | Covered | X | Covered | X | X | X |

X reasons: hub/mailbox are in-memory with no network operations; N8 no locale/time arithmetic; N10 adds no new transport payload diagnostics, while public session snapshots/Debug must remain redacted; N11 no billing; N12 adds no file acquisition/storage operations (source/destination faults remain their existing contracts). Panic/closed channel is N6; cancellation/drop/handle lifetime is N9.

Interactions: session old/new × consent round old/current × command same/conflicting × phase/cancel; full queue × dropped receipt × replay; active/buffered/head × native/legacy commands; BLE bridge open/closed × migrated/non-migrated worker × service stop; group close × callback admission × capacity/panic; dropped BWU waiter × new registration × current/last endpoint alias. Assert exact state, capability/token identity, receipt result, resource/reference release and joined task completion.

Mutation review targets: compare only address/peer ID; reuse an epoch/sequence; accept stale/conflicting consent; acknowledge queue insertion instead of handler validation; cancel on client handle drop; read native commands from legacy broadcast; override Finished; detach GATT workers or cancel migrated worker at bridge close; spawn after group close; retain closed BWU sender. No unsupported coverage percentage or executed-mutation claim.

Final branch review adds two explicit queued-command interleavings: a duplicate/conflicting command queued after the first mailbox receive but before its acknowledgement, and an old-round command processed after a new waiting round begins. The estimate becomes 42 methods (40 implemented initial methods plus these two). BLE stream tests use the real TCP manager as the remote sender, preserving its existing absorbing terminal/error policy and asserting actual received text.

## Verification facts

- U0 completed in `b774770`, before production changes. Implementation checkpoint: `963ef1a`. U1–U3 implement `sessions.rs` and its private GATT task group; shared handler leases/mailboxes; managed outgoing registration; TCP/GATT/RQS propagation; closed BWU waiter pruning and cooperative handler yields. No dependency, lockfile, wire schema, crypto, settings, package registration or native GUI change.
- One full `cargo +stable test --all-features` assertion run executed 324 unit/export methods: **323 passed, 1 BLE fixture failed**. The initial preceding attempt had stopped at compilation on a test trying to move a State out of a Drop type; it was corrected to clone before assertions executed. Cargo did not run the integration binary after the failed BLE assertion.
- The failed fixture used a custom remote OutboundRequest loop that incorrectly required NotAnError after the handler had already reached Finished; EOF after that terminal state is normally absorbed by the actual manager. Replaced the custom loop with the real TCP manager. The updated BLE tests retain exact sender/receiver terminal assertions and additionally assert actual received text. One affected two-method BLE invocation passed, including Accept and Reject. No production defect was hidden by weakening a delivery assertion.
- Two added queued-command race methods passed once. Six lifecycle tests passed once after adding constructor/registration cancellation. The previously unexecuted modern_protocol integration binary passed all **4** methods once.
- Final boundary review strengthened last-valid/exhausted identity/round counters and parent-cancellation admission/drop ownership assertions. One affected `sessions::` invocation passed **31** methods. A subsequent one-method recheck passed after ensuring the exhausted-round snapshot preserves the actual waiting state without issuing an actionable token. Only affected checks were repeated; no ten-run stability loop or repeated unchanged full suite.
- Current inventory: **326 unit/export + 4 integration = 330**, including **42 new methods** in this slice. All methods have passing observations across the full/affected invocations above. This is not a claim of a single 330-test green run on final source, a final Mode B gate or full protocol coverage.
- Final all-feature/all-target Clippy with warnings denied, nightly format check and git diff check passed. Clippy's initial attempt found one redundant reference, then passed after correction. The preexisting bluer future-incompatibility notice remains. `cargo +stable check --no-default-features` passed with **9 feature-disabled unused-code/import/variable warnings**; that configuration is not claimed warning-clean. No cross-platform execution claim.
- No BlueZ, live mDNS, D-Bus service, GUI, physical peer or installer was started. Owned command sessions exited (terminal closed).

### Local/CI execution and scope

From `core_lib`: `cargo +stable test --all-features`; `cargo +stable clippy --all-features --all-targets -- -D warnings`; `cargo +nightly fmt --all --check`. Existing Linux preflight `.github/workflows/lint.yml` already executes the all-feature core suite and all-target Clippy, so no new CI job is needed. `.github/workflows/mode-b.yml` remains the separately invoked final stability workflow. Unchanged Vue/desktop/analyzer suites were not repeated. Core state ownership, protocol handlers, TCP/GATT tasks and BWU routing were analyzed; generated protobuf/TS DTOs are compiled, not chosen as coverage substitutes.

### File/function coverage and limitations

| Production file | Exercised rules | Remaining / reason |
|---|---|---|
| `sessions.rs` | Hub limits/epochs/last-valid counters, registration/release, all protocol states/directions, exact/stale/conflicting token/command handling, queued races, receipt ownership, watch updates/closure, redacted Debug and absorbing outcomes | Private mutex-poison paths require corrupting otherwise encapsulated state; not manufactured by mocking the owner. No persistent history, IPC authentication or authoritative recovery. |
| `sessions/tasks.rs` | Budget validation/exact/over/reuse, close/parent cancellation, joined children, failed child/panic, late admission and bridge-versus-worker ownership | Tasks cooperate with the owned lifetime. Arbitrary synchronous work/external SDK internals are not preempted or certified. |
| `managed.rs`, `manager.rs`, `manager/session.rs` | Managed registration, old queue compatibility, pre-dispatch/connect/read cancellation, truthful release/correlated failure, saturation, fresh inbound IDs, real native consent and ignoring legacy controls | Native adapter still must map coordinator IDs, retain exact receipts and treat loss as desynchronization. Active hub snapshots are not completed-transfer history. |
| `hdl/inbound.rs`, `hdl/outbound.rs` | Real TCP and BLE-stream UKEY2/encrypted-text Accept/Reject, shared control/status attachment, native/legacy separation, terminal/cancel/error rules, original file regressions in the full attempt | Kernel send-buffer saturation and every possible scheduler interleaving are not forced. Native accept receipt means handler acceptance before protocol/storage side effects, not their success. |
| `hdl/gatt.rs`, `lib.rs` | GATT protocol worker tested with real TCP peer and duplex adapter; owned group and lifecycle helpers exercised; optional constructor/registration cancellation and callback wiring compiled by Clippy | Actual BlueZ notifier callbacks/registration and SDK cleanup are not run. Task-group ownership plus real stream replacement are tested; full live Weave/BWU negotiation, advertising and hardware shutdown remain gates. |
| `hdl/bwu.rs`, `protocol.rs` | Closed/current/alias route handling and re-registration; existing routing tests retained; removed delivery-delay constant | Existing BWU timeouts/200ms protocol pacing remain unchanged. No discovery-latency claim. |

Source/assertion mutation review covers reused capabilities, early/late counter guards, removed phase/round/replay checks, acknowledgements at queue insertion, owner-drop as success, control-drop cancellation, legacy control reaching native handlers, terminal resurrection, omitted task join/admission barrier and closed BWU routes. No mutation tool or instrumented branch/line percentage was run for this slice.

### Integration contract and next work

`RQS::run_managed()` still returns the managed outgoing sender and nearby indication receiver. `ManagedSender::sessions()` supplies the common active hub; an outgoing TransferControl also supplies its own `session()`. For inbound handlers obtain the matching active SessionControl using its generated event ID. Read snapshot/changed, use the current opaque consent token plus stable nonzero adapter command ID in respond, and await its receipt before acknowledging the matching coordinator control revision. Owner/control loss is explicit; never infer Finished from a receipt, release or hub removal. Dropping a UI handle does not cancel a transfer; explicit cancel targets its owned live session.

The session-control foundation is complete for both transports. The single-owner coordinator/core runtime is now implemented in [native-core-runtime.md](native-core-runtime.md). Its ordered lifecycle feed supersedes active-hub lookup for reliable native registration; `active` remains a live view only. Authenticated/versioned D-Bus descriptor protocol, retained-history policy, native GTK tray/notifications and Dolphin ingress remain pending. Acquiring the unique bus service name before transports, keeping original-file capabilities, and preserving settings remain required. Physical Android/Windows/KDE acceptance and full Google account/contact/QR/other parity are not completed by local core tests.


### Implemented test index

Primary categories, without double-counting overlaps/table rows: Positive=4, N1=2, N2=6, N3=1, N4=3, N5=3, N6=3, N7=6, N8=0, N9=13, N10=1, N11=0, N12=0. N8/N11/N12 are inapplicable as explained above; network/identity/concurrency interactions overlap several primary categories.

| Method | Primary category |
|---|---|
| `given_zero_epoch_or_budget_when_hub_is_created_then_invalid_configuration_is_rejected` | N1 |
| `given_different_run_epochs_when_sessions_are_opened_then_old_capabilities_cannot_alias` | N7 |
| `given_completed_incarnation_when_new_session_opens_then_identity_is_never_reused` | N7 |
| `given_exact_session_budget_when_owner_drops_then_retained_controls_do_not_hold_the_slot` | N2 |
| `given_exhausted_session_sequence_when_opened_then_no_partial_registration_or_alias_occurs` | N2 |
| `given_live_hub_when_closed_twice_then_all_owners_cancel_and_admission_stays_closed` | N9 |
| `given_outbound_correlation_when_session_is_registered_then_capability_is_independent_and_debug_is_redacted` | N10 |
| `given_protocol_phase_or_direction_when_responded_then_only_inbound_current_wait_accepts_commands` | N7 |
| `given_invalid_command_or_foreign_token_when_responded_then_mailbox_remains_empty` | N1 |
| `given_queued_consent_when_waiting_for_receipt_then_only_handler_validation_acknowledges` | Positive |
| `given_accepted_consent_when_exact_command_replays_after_progress_then_receipt_replays_without_side_effect` | N4 |
| `given_accepted_decision_when_command_or_answer_conflicts_then_replay_is_rejected` | N4 |
| `given_full_consent_mailbox_when_processing_frees_capacity_then_exact_retry_obtains_receipt` | N2 |
| `given_cancelled_session_when_pending_consent_is_processed_then_no_consent_is_applied` | N9 |
| `given_actual_handler_phase_changed_when_queued_consent_is_checked_then_stale_command_has_no_effect` | N5 |
| `given_unprocessed_command_when_session_owner_disappears_then_receipt_reports_loss` | N6 |
| `given_dropped_receipt_when_handler_processes_then_accepted_decision_is_not_rolled_back` | N9 |
| `given_repeated_wait_and_new_round_when_state_updates_then_only_new_round_invalidates_old_tokens` | N7 |
| `given_exhausted_consent_round_when_waiting_begins_then_owner_cancels_without_token_alias` | N2 |
| `given_live_status_when_observed_then_watch_publishes_actual_state_and_token` | Positive |
| `given_terminal_session_when_cancel_and_late_states_arrive_then_every_terminal_outcome_is_absorbing` | N4 |
| `given_retained_control_when_owner_closes_then_watch_and_mailbox_close_without_fake_terminal` | N6 |
| `given_client_control_when_dropped_then_service_owned_session_stays_live_and_uncancelled` | N9 |
| `given_consent_queued_before_first_ack_when_first_command_applies_then_duplicate_replays_and_conflict_fails` | N5 |
| `given_consent_queued_in_old_round_when_new_wait_begins_then_old_answer_cannot_accept_new_round` | N7 |
| `given_task_budget_when_full_or_invalid_then_admission_is_bounded_and_completion_returns_capacity` | N2 |
| `given_owned_worker_when_group_stops_twice_then_cancellation_is_joined_without_cancelling_caller` | N9 |
| `given_migrated_worker_when_bridge_finishes_then_group_keeps_worker_owned_until_service_stop` | N9 |
| `given_panicking_worker_when_supervised_then_sibling_cancels_and_stop_reports_loss_after_joining` | N6 |
| `given_closed_admission_when_late_callback_arrives_then_it_cannot_spawn_after_join_barrier` | N5 |
| `given_parent_shutdown_when_group_is_active_then_children_stop_and_late_admission_fails` | N9 |
| `given_two_managed_tcp_sessions_when_native_consent_accepts_then_transfer_finishes_and_legacy_cancel_is_ignored` | Positive |
| `given_two_managed_tcp_sessions_when_native_consent_rejects_then_exact_receipt_and_terminal_rejection_are_preserved` | N9 |
| `given_established_native_inbound_when_owned_control_cancels_then_partial_read_closes_with_fresh_session_identity` | N9 |
| `given_native_outbound_cancelled_before_dispatch_when_released_then_session_closes_cancelled_without_connecting` | N9 |
| `given_native_connect_failure_when_owner_releases_then_snapshot_records_disconnection_without_success` | N3 |
| `given_full_native_session_budget_when_inbound_probe_arrives_then_it_closes_without_fabricated_notification` | N2 |
| `given_managed_ble_stream_when_owned_consent_accepts_then_real_encrypted_transfer_finishes` | Positive |
| `given_managed_ble_stream_when_owned_consent_rejects_then_rejection_survives_group_shutdown` | N9 |
| `given_managed_ble_request_when_stream_moves_to_tcp_then_session_identity_and_cancellation_survive` | N9 |
| `given_abandoned_bwu_waiter_when_checked_or_registered_then_closed_route_is_pruned` | N9 |
| `given_closed_current_route_when_alias_is_live_then_socket_routes_only_to_live_waiter` | N7 |
