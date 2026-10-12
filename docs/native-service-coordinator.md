# Native service coordinator — offline foundation

Status: planned, 2026-10-12. Primary mode: A (Feature / Refactor).

## Scope and decisions

This slice implements the already accepted background-service ownership model from [native-desktop-design.md](native-desktop-design.md). It does not publish an IPC interface, activate a daemon or change the current Tauri app. A Linux-only `core_lib::service` module is the reusable, synchronous state owner; the future single daemon task serializes calls into it. Keeping it beside the owned-file types avoids another dependency graph before the executable and bus boundary exist. It has no GUI, BlueZ, clock, network, global state or asynchronous worker dependency. Epoch is injected by the future daemon (fresh nonzero value each startup).

The adapter must authenticate actual D-Bus senders, map unique connection names to coordinator-issued client tokens, acquire the service name before starting transports, and drop the client on bus disconnect. Coordinator tokens are internal capabilities, not authentication of arbitrary IPC arguments. No path-opening operation is added.

Internal budgets are configurable, validated and bounded: connected clients, live resource selections, reserved descriptors, selected bytes, pending sends, retained transfers, endpoints and replay records per connected client. Defaults are provisional implementation resource budgets, not Google protocol limits. Reserve **twice** each selection's file count to include the current sender's one duplicate per file. Keep a weak reference to the original selection until every original/core payload reference is gone, including after a terminal event. Current `OutboundRequest` retains its payload while duplicated readers exist. The adapter must submit each dispatch exactly once and retain the payload throughout any worker-owned duplicate lifetime; no repeated preparation or retry spawning parallel readers under the same reservation. Admission checks do not prevent descriptors received by a future bus decoder from exhausting its own budget: atomic CLOEXEC receipt and pre-decode/transport bounds remain required.

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

`P` = planned, `X` = not applicable with reason below. No cells omitted.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Identity / limits / snapshot | P | P | P | X | P | P | P | P | X | P | P | X | X |
| Resource ownership / admission | P | P | P | X | P | P | X | P | X | P | P | X | P |
| Endpoint / send / replay | P | P | P | X | P | P | P | P | X | P | P | X | X |
| Consent / controls / observations | P | P | P | X | P | P | P | P | X | P | P | X | X |
| Desync / shutdown | P | P | P | X | P | P | P | P | X | P | P | X | P |

N3: no network operations; core event loss is an explicit adapter input tested under N6. N8: no locale/date/currency conversion; identifiers are opaque bounded ASCII, epoch injected. N11: no monetization. N12 applies only to held-resource cleanup/readability; enumeration/storage-write failures belong to the already tested file-source layer. No new acquisition or storage-write logic is added here. Fcntl/kernel errors remain that layer's contract.

Interactions: caller A/B × unsent/queued/dispatched/terminal × disconnect/discard/cancel; quota exact/over × pending/externally held/released resources; replay same/different arguments × full queue/terminal/endpoint removed; consent round old/current × duplicate/conflicting answer × cancel/late acknowledgement; synchronized/lost × send/respond/dispatch/cancel; shutdown × pending/queued/active/external payload references. Assertions must validate final snapshots, intent identity and resource admission, not merely `Ok`. Important mutations: remove owner/replay/token checks; change `>=` limits to `>`; release budgets on terminal instead of last Arc; resurrect terminal state; clear a newer intent on an old acknowledgement; auto-resync on a later event; consume a selection on failed admission.

## Verification facts

Not run yet. Historical file-boundary results remain in their own SSOT verification section. The existing core CI executes this module's tests through `cargo test --all-features`; no new job or hardware claim is needed.
