# Native coordinator/core runtime

Status: R0 planned and recorded before code, 2026-10-12. Mode A.

## Scope and rationale

Continue the accepted single-owner Rust service architecture after common TCP/BLE controls. Add an in-process Linux runtime owning the coordinator, RQS lifetime, discovery mapping, one pending dispatch and bounded live control mappings. This is not the D-Bus wire API, authentication, daemon executable, GTK UI, retention policy or Google parity acceptance. Legacy Tauri remains unchanged.

The live hub and coalescing per-session watch cannot recover sessions that open and close between polls or distinguish skipped consent rounds. Add a bounded ordered session lifecycle broadcast (512 events), subscribed before transport startup. Open/change/close carry opaque session identity and native state; open additionally carries its control. No payload, metadata, addresses or peer errors enter this feed. Loss is permanent coordinator desynchronization, as already specified; no invented recovery or terminal outcome. Preserve the existing live hub API and legacy broadcasts. Events are emitted synchronously by the existing owner, with no detached forwarding tasks.

Internal implementation choices: event-driven readiness waits on queue capacity and hub membership changes, never timers; reserve the mpsc slot before creating an outbound lease to avoid phantom sessions on queue rejection. Keep at most one already-dispatched request pending while capacity is unavailable. The original opened selection remains owned through pending/core lifetime. Discovery alone supplies numeric socket addresses; callers select an opaque discovery key. Missing endpoints fail before core dispatch. Unknown inbound sessions are registered from ordered open events; a control that closes without a terminal observation causes desynchronization. Pending consent is acknowledged only after its processing receipt resolves; exact intent revisions protect superseding cancellation. Core cancellation is a request, not a fabricated result.

Shutdown begins coordinator drain, cancels all owned controls and pending work, awaits RQS.stop, then drains ordered events before checking coordinator completion. Lost outcomes remain unresolved; return an explicit error rather than claim successful shutdown. No live system registration is performed.

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

No execution results for this slice yet. Prior measurements remain in their original SSOT sections.
