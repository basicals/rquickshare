# Native service coordinator — offline foundation

Status: implemented and verified offline, 2026-10-12. Primary mode: A (Feature / Refactor).

Current integration checkpoint: the single-owner coordinator/core runtime and ordered native session events are implemented and verified in [native-core-runtime.md](native-core-runtime.md) and [native-core-events.md](native-core-events.md). Earlier pending-adapter descriptions below are historical prerequisites; authenticated D-Bus, activated daemon, native UI and physical Google parity remain open.

## Scope and decisions

This slice implements the already accepted background-service ownership model from [native-desktop-design.md](native-desktop-design.md). It does not publish an IPC interface, activate a daemon or change the current Tauri app. A Linux-only `core_lib::service` module is the reusable, synchronous state owner; the future single daemon task serializes calls into it. Keeping it beside the owned-file types avoids another dependency graph before the executable and bus boundary exist. It has no GUI, BlueZ, clock, network, global state or asynchronous worker dependency. Epoch is injected by the future daemon (fresh nonzero value each startup) and is carried in every internal token, so a token from an older instance cannot alias a new one with the same sequence number.

The adapter must authenticate actual D-Bus senders, map unique connection names to coordinator-issued client tokens, acquire the service name before starting transports, and drop the client on bus disconnect. Coordinator tokens are internal capabilities, not authentication of arbitrary IPC arguments. No path-opening operation is added.

Internal budgets are configurable, validated and bounded: connected clients, live resource selections, reserved descriptors, selected bytes, pending sends, retained transfers, endpoints and replay records per connected client. Defaults are provisional implementation resource budgets, not Google protocol limits. These reservations cover outbound source handles, not every socket, inbound destination or bus-decoder descriptor in the process. Reserve **twice** each selection's file count to include the current sender's one duplicate per file. Keep a weak reference to the original selection until every original/core payload reference is gone, including after a terminal event. Current `OutboundRequest` retains its payload while duplicated readers exist. The adapter must submit each dispatch exactly once and retain the payload throughout any worker-owned duplicate lifetime; no repeated preparation or retry spawning parallel readers under the same reservation. Admission checks do not prevent descriptors received by a future bus decoder from exhausting its own budget: atomic CLOEXEC receipt and pre-decode/transport bounds remain required.

Pending selections belong to one connection. Send atomically moves ownership into a service transfer and bounded FIFO, with a per-client request-ID replay ledger. An identical accepted request returns the original transfer ID even after disconnecting its resource selection; conflicting reuse fails. Failed admission does not consume the selection or replay ID. Replay records are never silently evicted while the caller remains connected. Disconnect closes unsent resources and removes that caller's ledger; queued/active transfers survive. Discard is idempotent for absent IDs and cannot discard another client's pending selection.

The adapter supplies the bounded discovered-endpoint set; command callers cannot inject host addresses. Endpoint removal after queuing is handled as a failed connection by the adapter. Resource/accounting release follows actual handle ownership, not endpoint disappearance or a UI close.

Transfer snapshots are revisioned and contain only coordinator IDs, direction, coarse phase, consent revision and pending-control state; no filenames, source paths, contents or core error strings. They are **internal Rust structures**, not the future versioned JSON schema. Subscription/reconciliation and the eventual bounded wire schema remain bus-adapter work. A missed core event permanently marks this instance unsynchronized until restart or a future authoritative recovery seam. It pauses new sends, dispatch and consent; cancellation remains available. Later events cannot silently restore synchronization or invent missing outcomes. Terminal observations absorb later disconnects/duplicate states without resurrection.

Inbound consent uses the snapshot revision at entry into the current consent round. Repeated waiting events do not create new prompts. Same-token/same-answer replay is harmless within that round; conflicting answers or old rounds fail. Only inbound waiting transfers accept answers. Control intents occupy at most one slot per transfer, remain pending until adapter acknowledgement, and use a revision token so late acknowledgements cannot clear a newer cancellation. Cancellation supersedes a pending answer. A queued cancellation removes the queued work without transport activity; active cancellation remains a request until the core reports a terminal outcome. Never represent command acceptance as completed delivery or successful transfer.

Shutdown is two-stage: drain rejects new work, drops pending selections, cancels queued work and requests cancellation of active transfers; completion requires all transfers terminal and all selection references released. The adapter must invoke and await `RQS::stop()` before declaring completion; this pure coordinator cannot perform or certify that await. Retained transfers are bounded and not evicted in this foundation; at capacity new transfers fail explicitly. A reviewed retention/recovery policy is required before shipping a continuously running daemon. No persisted recovery, settings migration, visibility change or Google identity implementation is introduced.

## Milestones, risk and rollback

1. C0: record contracts, complete scoped inventory and verification policy; commit documentation before code.
2. C1: implement bounded ownership, replay/FIFO, consent/control acknowledgement, snapshots/desynchronization and draining; add focused unit/kernel-FD integration tests in the same module.
3. C2: run the changed core suite once plus format/Clippy; repair actual failures and rerun only affected checks. Record measured results and remaining adapter work, commit implementation and handoff.

The user explicitly changed development verification on 2026-10-12: one appropriate run, no ten-run development loop. Historical measured repetitions in [native-file-input.md](native-file-input.md) remain facts. Ten repetitions are reserved for an applicable final stability/Mode B completion gate. This slice is not that gate.

Rollback: revert C1 without changing any current transport entrypoint or on-disk format. Main risk is overstating coordinator guarantees as daemon guarantees: authenticated IPC, authoritative core mapping, atomic FD receipt, supervised transport lifetime and native UI are not implemented here.

## Complete scoped inventory and branch map before tests

| Artifact | Branches / invariants | Level / expected methods |
|---|---|---|
| Constructor / counters | nonzero epoch and every budget; monotonically unique IDs/revisions; exhaustion fails without mutation | Unit, 3 |
| Connect / snapshot | client budget, unknown client, stable repeated snapshot, epoch/revision, no payload disclosure | Unit, 4 |
| Acquire / discard | running/client guards, file and byte totals exact/over limit/overflow, selection count, atomic failure, ownership, absent discard | Unit + kernel FD, 7 |
| Endpoint set | count/text bounds, duplicates, replacement, no revision on identical set, unknown endpoint | Unit, 3 |
| Send / replay | request bounds, identical/conflicting replay, per-client namespace/quota, selection owner, missing selection, queue/history capacity, atomic failure | Unit + kernel FD, 7 |
| Dispatch / resource lifetime | FIFO, empty queue, paused/draining, one dispatch per transfer, payload Arc/weak lifetime, queued vs active disconnect/cancel | Unit + kernel FD, 5 |
| Observe / consent | unknown/terminal, invalid adapter phase, inbound only, waiting duplicate, new round, stale/duplicate/conflicting answers, uncertain service | Unit, 6 |
| Controls / cancel | pending acknowledgement replay, stale acknowledgement, consent superseded, queued immediate terminal, active request vs completion, terminal idempotency | Unit, 4 |
| Desynchronization / shutdown | loss idempotency, no automatic resync, cancellation still available, running/draining/stopped guards, queued/active/selection cleanup, resource-held completion rejected | Unit + kernel FD, 4 |

Estimate: 43 methods with table subcases; final count may grow when an independent rule requires a separate scenario. Production file/function coverage will be described by this inventory and test index; no unsupported percentage/branch-coverage claim.

## Negative matrix

`C` = covered by the scoped tests below, `X` = not applicable with reason below. No cells omitted.

| Artifact | Cositive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Identity / limits / snapshot | C | C | C | X | C | C | C | C | X | C | C | X | X |
| Resource ownership / admission | C | C | C | X | C | C | X | C | X | C | C | X | X |
| Endpoint / send / replay | C | C | C | X | C | C | C | C | X | C | C | X | X |
| Consent / controls / observations | C | C | C | X | C | C | C | C | X | C | C | X | X |
| Desync / shutdown | C | C | C | X | C | C | C | C | X | C | C | X | X |

N3: no network operations; core event loss is an explicit adapter input tested under N6. N8: no locale/date/currency conversion; identifiers are opaque bounded ASCII, epoch injected. N11: no monetization. N12 is not applicable to coordinator methods: they perform no storage reads/writes, enumeration or fcntl calls. Disposable-file integration fixtures prove descriptor lifetime/readability; actual storage failures remain in the already tested file-source/reader contracts. Cleanup/races are counted under N5/N9, not mislabeled as simulated storage failures.

Interactions: caller A/B × unsent/queued/dispatched/terminal × disconnect/discard/cancel; quota exact/over × pending/externally held/released resources; replay same/different arguments × full queue/terminal/endpoint removed; consent round old/current × duplicate/conflicting answer × cancel/late acknowledgement; synchronized/lost × send/respond/dispatch/cancel; shutdown × pending/queued/active/external payload references. Assertions must validate final snapshots, intent identity and resource admission, not merely `Ok`. Important mutations: remove owner/replay/token checks; change `>=` limits to `>`; release budgets on terminal instead of last Arc; resurrect terminal state; clear a newer intent on an old acknowledgement; auto-resync on a later event; consume a selection on failed admission.

## Verification facts

- C0 complete in `5c8407d`: contracts and inventory committed before implementation; global AGENTS section 12 matches the latest user instruction.
- C1 complete: `core_lib/src/service.rs`, `core_lib/src/service/tests.rs`, and the Linux-gated export in `core_lib/src/lib.rs`. No dependency, lockfile, wire protocol, current UI or settings changes.
- Implementation checkpoint: `0b0d9fd`. Subsequent bounded core-error correlation and the separate substantial-supervision decision are recorded in [native-core-adapter-review.md](native-core-adapter-review.md).
- C2 complete: **one** `cargo +stable test --all-features` run passed: **233 unit/export tests + 4 integration tests = 237**, including **53 new coordinator methods** with table subcases. No failed/ignored tests. Unit suite reported 0.38 seconds after compilation; this is an observation on this machine, not a runtime guarantee.
- `cargo +stable clippy --all-features --all-targets -- -D warnings`, `cargo +nightly fmt --manifest-path core_lib/Cargo.toml --all --check`, and `git diff --check` passed. The existing bluer future-incompatibility warning remains. The first format-check invocation used the repository root without a manifest and was corrected to the explicit manifest above; no test loop was repeated.
- Existing core CI already runs this suite. Unchanged desktop/Vue/Python suites were not rerun. Historical file-boundary observations remain in their own verification section. This is neither a ten-run stability gate nor full Mode B, physical Google compatibility or live daemon validation.
- No spawned service, watcher or bus registration remains. Owned command sessions finished (terminal closed).

The final method count exceeds the preliminary estimate by ten: independent quota interactions, privacy, client isolation and counter-exhaustion cases deserved separate assertions. All planned coordinator domain scenarios are implemented. Real credential validation, FD decoding, actual core command delivery and awaited shutdown are future adapter contracts, not mocked as passed here.

### File/function coverage map and mutation review

| Production file | Functions / branches exercised | Remaining / reason |
|---|---|---|
| `core_lib/src/service.rs` | All 17 public coordinator operations, constructor, phase predicate, limit/identifier validation, checked reservation sums, revision advance, lifecycle/client/sync guards, resource refresh/usage and redacted Debug/Display. Exact/over quotas, stale/conflicting replay, epoch mismatch, queued/active/terminal and drain paths are explicitly asserted. | Private map-consistency fallbacks in `next_send` require corrupting an otherwise unreachable internal invariant; not manufactured by mocking the state owner. No instrumented branch/line percentage is claimed. Operating-system allocation aborts are not recoverable application errors. |
| `core_lib/src/lib.rs` | Linux module export compiled by the full core test/Clippy invocation. | Other platforms do not expose this Linux native module; no cross-platform execution claim. |

Assertions target owner/replay/token checks, every resource boundary, weak-reference lifetime after terminal, absorbing terminal states, late acknowledgements and permanent desynchronization. Counter tests additionally guard against a mutation committing state before discovering revision exhaustion. This stage used a source/assertion mutation review, **not executed mutation tooling**; historical 18 assertion-killed mutants concern the file-input slice only.

### Continuation / deployment gates

1. Implement the authenticated private-bus adapter and freeze its versioned descriptor/manifest and snapshot fixtures. Atomic CLOEXEC receipt and decoder/pre-admission FD bounds must precede activation. Never implement the historical path-based `CreateSelection` alternative.
2. Add the single-owner core adapter with endpoint/transfer-ID mapping, event-loss input, one dispatch per reservation, truthful command acknowledgement and awaited `RQS::stop()`. Missing authoritative core state currently requires remaining unsynchronized; a new recovery seam needs its own documented invariant.
3. Review bounded terminal/replay retention before shipping a continuously running service. This foundation explicitly rejects new work at retained-transfer capacity; it must not silently invent an eviction policy. Also bound inbound/core resources and transport-owned sockets separately from the outbound reservations here.
4. Continue GTK4/KDE tray-panel feasibility, native notifications/settings, capability ingress CLI and Dolphin packaging. Do not install a context action until its real recipient chooser exists. Physical Android/Windows peers and clock-area positioning remain separate acceptance gates.


### Implemented test index

Primary-category counts (overlaps and table rows are not double-counted): Positive=7, N1=5, N2=14, N3=0, N4=8, N5=3, N6=4, N7=6, N8=0, N9=5, N10=1, N11=0, N12=0. N3/N8/N11/N12 are inapplicable for the reasons above.

| Method | Primary category |
|---|---|
| `given_zero_epoch_or_budget_when_constructed_then_input_is_rejected` | N1 |
| `given_new_epoch_when_old_tokens_are_used_then_no_identity_aliases` | N7 |
| `given_full_client_budget_when_disconnected_then_slot_returns_with_fresh_token` | N2 |
| `given_two_clients_when_snapshotted_then_only_owned_selections_are_visible` | Positive |
| `given_unknown_client_when_commands_are_called_then_no_state_changes` | N7 |
| `given_private_resources_when_debugged_then_diagnostics_exclude_input_strings` | N10 |
| `given_owned_selection_when_admitted_then_reader_duplicates_are_reserved` | Positive |
| `given_descriptor_budget_when_exact_or_exceeded_then_admission_is_atomic` | N2 |
| `given_byte_budget_when_boundary_sizes_arrive_then_only_bounded_totals_are_admitted` | N2 |
| `given_existing_selected_bytes_when_combined_limit_is_exceeded_then_old_selection_survives` | N2 |
| `given_selection_budget_when_payload_is_queued_or_dispatched_then_reservation_remains` | N2 |
| `given_accounting_overflow_when_reserved_then_every_sum_fails_closed` | N2 |
| `given_foreign_selection_when_discarded_then_owner_resources_are_untouched` | N7 |
| `given_owned_selection_when_discarded_twice_then_resources_are_released_once` | N4 |
| `given_client_with_unsent_files_when_disconnected_then_other_client_is_preserved` | N9 |
| `given_endpoint_set_when_replaced_or_repeated_then_revision_changes_only_on_change` | Positive |
| `given_bad_endpoint_input_when_updated_then_existing_discovery_is_unchanged` | N1 |
| `given_unknown_endpoint_when_sent_then_selection_and_request_remain_retryable` | N1 |
| `given_request_boundaries_when_sent_then_invalid_ids_do_not_consume_selection` | N1 |
| `given_foreign_or_missing_selection_when_sent_then_transfer_is_not_created` | N7 |
| `given_accepted_send_when_replayed_after_terminal_and_endpoint_loss_then_original_receipt_returns` | N4 |
| `given_used_request_id_when_arguments_conflict_then_replay_is_rejected_atomically` | N4 |
| `given_distinct_clients_when_same_request_id_is_used_then_ledgers_are_independent` | N4 |
| `given_full_replay_ledger_when_transfer_finishes_then_records_are_not_silently_evicted` | N2 |
| `given_full_queue_when_dispatch_releases_slot_then_failed_send_can_retry` | N2 |
| `given_full_transfer_history_when_new_work_arrives_then_inbound_and_outbound_fail_closed` | N2 |
| `given_multiple_queued_sends_when_dispatched_then_order_and_payload_are_preserved_once` | Positive |
| `given_sent_transfer_when_client_disconnects_then_queued_and_active_payloads_survive` | N9 |
| `given_terminal_event_with_live_payload_when_admission_retries_then_budget_waits_for_last_reference` | N5 |
| `given_empty_queue_when_dispatch_requested_then_revision_is_stable` | Positive |
| `given_adapter_phase_incompatible_with_direction_when_observed_then_snapshot_is_unchanged` | N6 |
| `given_terminal_transfer_when_late_events_arrive_then_no_state_resurrects` | N6 |
| `given_inbound_registration_when_observed_then_no_outbound_resource_is_reserved` | Positive |
| `given_same_waiting_event_when_repeated_then_consent_token_and_answer_remain_stable` | N4 |
| `given_current_consent_when_answer_is_replayed_then_same_answer_is_idempotent_and_conflict_fails` | N4 |
| `given_new_consent_round_when_old_token_arrives_then_only_current_round_accepts_answer` | N7 |
| `given_transfer_outside_inbound_wait_when_answered_then_no_control_is_emitted` | N7 |
| `given_unknown_transfer_when_control_or_observation_arrives_then_state_is_unchanged` | N1 |
| `given_pending_control_when_peeked_and_acknowledged_then_delivery_does_not_invent_transfer_progress` | Positive |
| `given_answer_superseded_by_cancel_when_old_acknowledgement_arrives_then_cancel_remains_pending` | N5 |
| `given_queued_transfer_when_cancelled_then_no_transport_command_is_dispatched` | N9 |
| `given_active_transfer_when_cancelled_then_only_terminal_core_observation_completes_it` | N5 |
| `given_terminal_transfer_when_cancelled_then_every_terminal_outcome_is_preserved` | N4 |
| `given_event_loss_when_commands_arrive_then_new_send_dispatch_and_consent_pause_but_cancel_works` | N6 |
| `given_lost_events_when_later_events_and_reconnect_arrive_then_synchronization_is_not_fabricated` | N6 |
| `given_pending_and_active_work_when_shutdown_begins_then_resources_and_completion_are_drained` | N9 |
| `given_shutdown_replay_when_already_draining_or_stopped_then_revision_is_stable` | N4 |
| `given_draining_or_stopped_service_when_new_work_arrives_then_every_admission_path_is_closed` | N9 |
| `given_exhausted_revision_when_mutation_is_requested_then_no_partial_change_occurs` | N2 |
| `given_last_revision_when_discard_releases_resources_then_mutation_finishes_without_post_commit_error` | N2 |
| `given_exhausted_revision_when_dispatch_or_ack_is_requested_then_pending_work_is_preserved` | N2 |
| `given_exhausted_revision_when_shutdown_completion_is_requested_then_draining_is_preserved` | N2 |
| `given_exhausted_revision_when_worker_drops_last_payload_then_resource_refresh_fails_closed` | N2 |
