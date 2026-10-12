# Common managed TCP/BLE session controls

2026-10-12, Mode A. Status: planned before code. The user selected option 1 in [native-session-control-review.md](native-session-control-review.md). Applicable guide: `/home/morozhenko/.codex/AGENTS.md`; one appropriate development verification run, not ten repetitions.

## Analysis and accepted implementation contract

Legacy handlers subscribe to a mixed event/control broadcast; TCP inbound IDs use socket addresses and GATT reuses `ble-weave`. `weave_session` detaches its handler intentionally so it survives BWU, but the GATT server has no shutdown barrier for that task. The native service needs owned live-session controls across both transports, independently of UI lifetime. Preserve legacy entrypoints/control and protocol bytes/crypto; the native route ignores legacy broadcast commands.

1. Add a platform-independent internal Rust `sessions` module, not an IPC/authentication interface. A per-run `SessionHub` has an injected nonzero 128-bit epoch, checked monotonic sequence and a provisional 32-live-session budget. No global registry. Native outbound jobs register before queue insertion; active TCP/BLE inbound handlers register before handling. Rejected submissions retain input ownership. Registration failure closes an unestablished inbound probe without a fabricated transfer event.
2. Opaque `SessionId` and `ConsentToken` cannot be constructed from peer addresses, caller strings or coordinator IDs. Inbound managed event IDs derive from the fresh session identity; outbound event correlation keeps SendInfo.id while its session capability remains independent. Keep the same lease and control through BLE-to-TCP stream migration. Retain no unbounded terminal history in the hub: completed handlers release their registry slot; retained client handles observe closure and their last actual state.
3. Each lease owns one bounded consent mailbox and one cancellation token. `SessionControl::respond` requires its current consent token, nonzero adapter command ID and answer; receipt is explicit. The handler checks the actual waiting state/token before accepting, records one replay decision per consent round and acknowledges acceptance before protocol side effects. Exact same token/command/answer replay returns its original receipt; conflicting reuse fails. New rounds invalidate old tokens/decisions. This acknowledgement never means that peer I/O succeeded or delivery finished. Mailbox Full/Closed/Cancelled/Stale/Conflict errors are explicit; dropping a receipt/control does not cancel service-owned work.
4. Session status is a watch of the last actual protocol state and current consent token, with no file names, contents, socket addresses or raw errors. It is an internal active-session view, not authoritative persistent recovery or a new public snapshot schema. Owner loss closes the mailbox/watch without inventing Finished/Cancelled. Event-stream loss still requires coordinator desynchronization. Managed outgoing release acknowledgement stays separate from session processing receipts.
5. Attach the lease to the existing generic inbound/outbound handlers. Publish status before existing UI events. Native handlers ignore the legacy command stream. Inbound consent is processed from its owned mailbox; owned cancellation encloses connect/initial writes/whole frame reads/BWU waits. Absorb terminal states on later cancellation/errors. Preserve legacy Accept/Reject/Cancel behavior and JSON/TS.
6. Pass the same hub from ManagedSender/RQS to TCP context and ReceiverGattServer. Add a GATT-owned bounded task group with an admission lock, tracked tasks and child lifetime. Reject new subscriptions/tasks after closure. On GATT stop, close admission, cancel and join the group. A migrated protocol worker remains tracked after its weave bridge closes; bridge closure alone never cancels its new TCP transport. The existing single weave bridge is bounded to one; the group budget is 33 (up to 32 session workers plus bridge), provisional resource limits rather than Google protocol limits. Server initialization/callback stop must not create tasks after the barrier.
7. Root-cause corrections within this scope: prune closed BWU oneshot waiters before register/route/has_pending so a cancelled upgrade cannot leave a stale route intercepting later sockets; retain existing routing/alias/crypto/timeouts. Replace the handlers' 10-microsecond broadcast-delivery sleeps with cooperative `yield_now`: scheduling progress is an event/executor concern, not evidence that a command was received. Existing BWU protocol pacing is not changed by that correction.

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

Estimate: 39 new methods plus existing protocol/manager/coordinator/file tests. Table cases cover independent state/limit variants. Only external sockets/streams/tasks are controlled; handlers, registry and replay rules are real. Five-second outer watchdogs detect deadlock; synchronization waits on frames, channels, watch/token changes or task completion, never sleeps.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Hub / status | Planned | Planned | Planned | X | Planned | Planned | Planned | Planned | X | Planned | Planned | X | X |
| Commands / receipt | Planned | Planned | Planned | X | Planned | Planned | Planned | Planned | X | Planned | Planned | X | X |
| Protocol / transport owners | Planned | Planned | Planned | Planned | Planned | Planned | Planned | Planned | X | Planned | X | X | X |

X reasons: hub/mailbox are in-memory with no network operations; N8 no locale/time arithmetic; N10 adds no new transport payload diagnostics, while public session snapshots/Debug must remain redacted; N11 no billing; N12 adds no file acquisition/storage operations (source/destination faults remain their existing contracts). Panic/closed channel is N6; cancellation/drop/handle lifetime is N9.

Interactions: session old/new × consent round old/current × command same/conflicting × phase/cancel; full queue × dropped receipt × replay; active/buffered/head × native/legacy commands; BLE bridge open/closed × migrated/non-migrated worker × service stop; group close × callback admission × capacity/panic; dropped BWU waiter × new registration × current/last endpoint alias. Assert exact state, capability/token identity, receipt result, resource/reference release and joined task completion.

Mutation review targets: compare only address/peer ID; reuse an epoch/sequence; accept stale/conflicting consent; acknowledge queue insertion instead of handler validation; cancel on client handle drop; read native commands from legacy broadcast; override Finished; detach GATT workers or cancel migrated worker at bridge close; spawn after group close; retain closed BWU sender. No unsupported coverage percentage or executed-mutation claim.

## Verification facts

Not run yet. Prior 288-test/44-manager observations remain in [native-managed-dispatch.md](native-managed-dispatch.md).
