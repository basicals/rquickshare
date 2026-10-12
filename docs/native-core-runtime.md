# Native coordinator/core runtime

Status: R0–R3 complete and verified offline, 2026-10-12. Mode A. Initial plan committed before code in `9a1bcfb`; event/admission foundation implemented in `fb316c2`; runtime implemented in `c7674e8`.

## Scope and rationale

Continue the accepted single-owner Rust service architecture after common TCP/BLE controls. Add an in-process Linux runtime owning the coordinator, RQS lifetime, discovery mapping, one pending dispatch and bounded live control mappings. This is not the D-Bus wire API, authentication, daemon executable, GTK UI, retention policy or Google parity acceptance. Legacy Tauri remains unchanged.

The live hub and coalescing per-session watch cannot recover sessions that open and close between polls or distinguish skipped consent rounds. Add a bounded ordered session lifecycle broadcast (512 events), subscribed before transport startup. Open/change/close carry opaque session identity and native state; open additionally carries its control. No payload, metadata, addresses or peer errors enter this feed. Loss is permanent coordinator desynchronization, as already specified; no invented recovery or terminal outcome. Preserve the existing live hub API and legacy broadcasts. Events are emitted synchronously by the existing owner, with no detached forwarding tasks.

Internal implementation choices: event-driven readiness waits on queue capacity and hub membership changes, never timers; reserve the mpsc slot before creating an outbound lease to avoid phantom sessions on queue rejection. Keep at most one already-dispatched request pending while capacity is unavailable. The original opened selection remains owned through pending/core lifetime. Discovery alone supplies numeric socket addresses; callers select an opaque discovery key. Missing endpoints fail before core dispatch. Unknown inbound sessions are registered from ordered open events; a control that closes without a terminal observation causes desynchronization. Pending consent is acknowledged only after its processing receipt resolves; exact intent revisions protect superseding cancellation. Core cancellation is a request, not a fabricated result.

Shutdown first closes hub admission and adopts already buffered opens, then begins coordinator drain, cancels all owned controls and pending work, awaits RQS.stop, then drains ordered events before checking coordinator completion. Lost outcomes remain unresolved; return an explicit error rather than claim successful shutdown. No live system registration is performed.

Preparation is synchronous (`NativeRuntime::new`); rejected preparation returns the unchanged boxed RQS owner. Async `start(&mut self)` borrows the retained runtime, so cancellation cannot discard the only owner capable of awaiting `shutdown`. Failure/abandonment requires shutdown and a fresh runtime; the same epoch is never silently restarted. `Drop` requests cancellation only and is not a joined shutdown. Internal completion/readiness work is bounded and progressed every step to prevent event traffic from starving receipts or resource release.

Discovery route budgets are internal admission limits: 128 event slots, configured endpoint count, 1024-byte fullname and 256-byte display label, numeric IP and nonzero u16 port. Unusable advertisements are ignored; live valid routes retain opaque IDs until removal, and a reappearance gets a new ID. A removed endpoint invalidates unsent pending work; an update is applied before admission. These are not Google identity/authentication guarantees. Runtime ownership retains at most 32 outstanding outbound release futures and one pending request; physical transport limits remain separate.

## Milestones / rollback

- R0: analyze contracts, record this plan and scoped verification inventory; documentation-only commit.
- R1: add ordered lifecycle feed and event-driven native admission readiness, preserving old APIs; regression tests.
- R2: implement the serial runtime, discovery/transfer mapping, bounded dispatch, receipt processing, event-loss handling and awaited shutdown; focused tests against real coordinator/managed queues, with external transport fixtures only.
- R3: run the core suite once, format and all-feature/all-target Clippy; repair actual failures with affected reruns only. Record actual observations, update diary and commit reversible implementation checkpoints.

Rollback R2 independently removes the unused native runtime; R1 is additive and can be reverted separately after R2. Risks: event order at close, startup subscription race, queue/hub backpressure, consent receipt racing state/cancel, and resource lifetime after terminal. No retention eviction or public security/UX decision is made here.

## Scoped inventory, branches and verification plan

| Artifact | Required scenarios | Level |
|---|---|---|
| Session lifecycle feed | ordered open/change/close even when owner finishes before polling; terminal absorbing; no synthetic terminal on owner loss; stale late subscription rejected; bounded lag explicit | Unit, real hub/lease |
| Admission/readiness | full queue creates no lease/event; full hub waits for drop; closure wakes; cancellation-safe waiting creates no external permit | Unit, real managed queue |
| Runtime discovery/dispatch | trusted numeric endpoint mapping; removed/invalid endpoint fails; FIFO and exactly-once submission; one pending payload survives backpressure; client disconnect does not cancel service work | Integration of real coordinator/queue |
| Observation/control | inbound registration; direction/state mapping; consent round mapping; receipt acknowledgement only after processing; cancellation supersedes answer; absorbing terminal; owner loss/lag marks desynchronized | Integration of real coordinator/hub |
| Lifetime/shutdown | startup failure rollback; pending cancellation releases files without dispatch; drain rejects work, joined stop precedes completion; incomplete outcomes fail closed | Integration |

Interactions: full queue/hub x cancel/disconnect/shutdown; session open/change/close x poll delay; consent answer x state transition/cancel/receipt loss; endpoint present/removed x queued/pending; terminal state x release acknowledgement. Assertions target identity, snapshots, exact intent revisions, queue counts and actual Arc/FD ownership. No mocked coordinator or protocol logic. N1 invalid discovery data, N2 bounded admission/feed, N3 core closure, N4 dispatch/replay, N5 races, N6 lost events/owner, N7 identity/consent, N9 interruption and N10 redacted observations apply. N8/N11/N12 add no new clock/billing/storage operation here; existing file-source/storage tests remain separate. This is development verification, not exhaustive Mode B or physical protocol certification.

## Verification facts

- R0: analysis, plan and inventory recorded before code (`9a1bcfb`). R1: ordered events/admission readiness implemented (`fb316c2`), with [separate contracts](native-core-events.md). R2: serial Linux runtime and 23 focused methods implemented; R3: measured checks below completed. No dependency, lockfile, legacy UI, protocol wire format, service registration or system setting changed.
- First full-suite invocation stopped before assertions on two new runtime `Result<bool>`/`Result<()>` mismatches. Corrected return mapping; the next completed `cargo +stable test --manifest-path core_lib/Cargo.toml --all-features` passed **350 unit/export + 4 integration = 354**, 0 failed/ignored, unit time 0.89s after compilation. This included all five new event/admission methods and the then-current 19 runtime methods.
- Subsequent source review corrected the shutdown admission cutoff and changed consuming async construction into synchronous preparation plus borrowed startup. Affected runtime checks passed after each production correction. The final `cargo +stable test --manifest-path core_lib/Cargo.toml --all-features service::runtime` passed **23 methods**, 331 unit/export and 4 integration methods filtered, 0 failed/ignored, 0.01s. No assertion failure or weakened assertion occurred. Current final inventory: **354 unit/export + 4 integration = 358**, including **28 new methods**. No single final-source 358-method full run or ten-run stability claim is made.
- Final `cargo +stable clippy --manifest-path core_lib/Cargo.toml --all-features --all-targets -- -D warnings`, nightly format check and `git diff --check` passed. An earlier Clippy pass preceded the ownership correction; it was rerun only because production changed. Existing bluer future-incompatibility notice remains.
- `cargo +stable check --manifest-path core_lib/Cargo.toml --no-default-features` passed with the same **9 inactive-feature unused-item warnings** documented previously; no claim of a warning-clean disabled-feature build.
- The real TCP fixture uses the actual manager and protocol handlers with controlled loopback socket closure or service cancellation. It proves correlated Disconnected/Cancelled outcomes, joined workers, closed mappings and zero retained file handles. Domain/queue/consent tests use the real coordinator, hub, owned leases, receipts and managed queue; only external I/O/lifetime fixtures are controlled.
- Successful production mDNS/BlueZ bootstrap is not executed here. A real occupied-port bind failure exercises startup cleanup before optional Bluetooth initialization. Borrowed startup retention is exercised with the real RQS startup guard and tracked task, not physical SDK initialization. Earlier real encrypted TCP/duplex BLE tests ran in the full suite; no new full encrypted native-runtime happy-path or live Weave/BWU/Google peer claim.
- CI already runs core all-feature tests and all-target Clippy in `.github/workflows/lint.yml`. Unchanged frontend/Tauri/analyzer suites were not rerun. Commands completed; no owned long-running process remains.

### Scoped source coverage and mutation reasoning

| Production file | Exercised contracts | Remaining / limits |
|---|---|---|
| `sessions.rs` | ordered lifecycle, late subscription, real owner loss, exact event overflow, membership wake and close, existing session/token/terminal invariants | Mutex poisoning and allocation abort not manufactured; no line/branch instrumentation |
| `managed.rs` | full/closed queue, no phantom lease, rejected payload ownership, ready wake/closure, existing release/loss/cancel contracts | Readiness is explicitly not atomic admission for independent concurrent senders |
| `service/runtime.rs` | discovery filtering/removal/update, opaque IDs, FIFO/one pending request, client disconnect, exact receipt revision, stale round/state/cancel, event/owner/queue loss, all protocol phase mappings, real TCP failure/cancel and awaited shutdown | Successful complete SDK/bootstrap and D-Bus scheduling/authentication not run; defensive corrupt-binding/poison/allocation fallbacks not manufactured. Thin command delegations reuse the coordinator's 53-method coverage; not every wrapper is independently asserted |
| `service.rs` | Linux runtime module compiled by full and affected suites and Clippy | Existing pure coordinator measurements remain in their SSOT |

Assertions reject duplicate dispatch, early FD release, false processing acknowledgement, stale consent, late-ack cancellation loss, terminal resurrection, silent event loss and Stopped before the core joins. They also reject losing an already started core on constructor rejection and ignoring buffered inbound opens during shutdown. Mutation reasoning is source/assertion review, not an executed mutation tool or 100% Mode B claim.

### Primary-category counts and method index

Positive=3, N1=2, N2=3, N3=3, N4=1, N5=3, N6=5, N7=1, N8=0, N9=7, N10=0, N11=0, N12=0 (28 methods; overlapping assertions/table rows not double-counted). N10 privacy assertions overlap opaque discovery IDs and rejected-runtime Debug plus existing session/snapshot redaction tests in the full run; there is no separate new primary N10 method. N8/N11/N12 are inapplicable for the reasons in the inventory.


| Method | Primary category |
|---|---|
| `given_short_lived_session_when_observed_later_then_lifecycle_order_survives_owner_drop` | Positive |
| `given_late_subscription_or_owner_loss_when_read_then_no_recovery_or_terminal_is_invented` | N6 |
| `given_more_than_bounded_events_when_unpolled_then_loss_is_explicit` | N2 |
| `given_full_hub_when_lease_drops_or_hub_closes_then_readiness_wakes_without_timer` | N2 |
| `given_full_queue_when_rejected_then_no_phantom_session_is_published` | N2 |
| `given_unusable_discovery_when_updated_then_only_bounded_numeric_routes_are_exposed` | N1 |
| `given_queued_files_when_caller_disconnects_then_dispatch_is_fifo_once_and_resources_follow_core` | N9 |
| `given_full_queue_when_pending_send_is_cancelled_then_no_duplicate_dispatch_or_file_leak_occurs` | N9 |
| `given_pending_backpressure_when_capacity_returns_then_same_payload_is_submitted_once` | N4 |
| `given_removed_endpoint_when_queued_or_pending_then_unsent_work_fails_and_releases_resources` | N6 |
| `given_inbound_answer_when_queued_then_only_processed_receipt_acknowledges_exact_intent` | Positive |
| `given_cancel_superseding_answer_when_late_receipt_arrives_then_cancel_remains_truthful` | N5 |
| `given_second_consent_round_when_old_revision_is_replayed_then_only_new_round_can_be_answered` | N7 |
| `given_state_racing_answer_when_core_rejects_stale_consent_then_no_false_acknowledgement_occurs` | N5 |
| `given_owner_loss_or_event_lag_when_polled_then_new_work_is_permanently_paused` | N6 |
| `given_lost_processing_receipt_when_owner_disappears_then_adapter_does_not_acknowledge` | N6 |
| `given_closed_core_queue_when_polled_then_health_loss_is_not_delivery` | N3 |
| `given_empty_runtime_when_step_is_cancelled_then_no_work_or_resource_is_created` | N9 |
| `given_tracked_core_when_shutdown_returns_then_workers_are_joined_before_stopped_snapshot` | N9 |
| `given_lost_observation_when_shutdown_fails_then_core_barrier_is_still_awaited` | N9 |
| `given_every_protocol_state_when_mapped_then_coarse_direction_and_terminal_outcomes_are_preserved` | Positive |
| `given_real_tcp_worker_when_peer_closes_or_service_stops_then_runtime_preserves_outcome_and_joins` | N3 |
| `given_started_core_when_runtime_preparation_rejects_then_unchanged_ownership_is_returned` | N9 |
| `given_discovery_loss_when_polled_then_endpoint_history_is_not_treated_as_current` | N6 |
| `given_buffered_inbound_open_when_shutdown_begins_then_admission_closes_before_coordinator_drain` | N5 |
| `given_invalid_runtime_configuration_when_prepared_then_core_is_returned_without_starting` | N1 |
| `given_bind_failure_when_start_finishes_then_owned_runtime_can_shutdown_and_cannot_restart` | N3 |
| `given_abandoned_startup_guard_when_runtime_is_retained_then_shutdown_can_join_it` | N9 |

### Continuation / deployment gates

Use `NativeRuntime::new`, retain it, await `start`, then serialize authenticated commands with `step` in the future service owner. On failure/abandonment await `shutdown` and create a fresh instance; Drop alone cannot certify cleanup. A snapshot/endpoint update must be delivered after step progress in the future bus adapter. Client tokens remain internal capabilities, not caller authentication.

Next: concrete authenticated/versioned D-Bus descriptor transactions and fixtures, daemon/exclusive service-name startup, bounded terminal/replay retention review, settings migration, GTK tray anchoring/notifications and Dolphin ingress. The core runtime is implemented; it is not a registered daemon or a native UI. Event loss permanently pauses new work; restart/recovery and terminal-history eviction remain unreviewed. Acquire the unique bus name before starting transports and exclude the legacy GUI as a competing transport owner. Full Google identity/contact/own-device/QR/transport parity and physical Android/Windows/BlueZ/KDE remain open.

The next public selection API decision is prepared in [native-dbus-selection-review.md](native-dbus-selection-review.md). Implementation is paused at this new contract choice; no earlier session/runtime approval is pending.
