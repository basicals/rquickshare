# Structured TCP transfer supervision

2026-10-12, Mode A. Status: planned. The user approved option 1 from [native-core-adapter-review.md](native-core-adapter-review.md): fix bounded task supervision now, then continue the native service integration.

## Analysis and invariants

`TcpServer::run` currently awaits an entire outbound session, blocking accepts; inbound tasks are detached with `tokio::spawn` and are not awaited by `RQS::stop()`. Cancellation is checked only inside the outbound handler loop, after TCP connect and initial writes. `RQS` tracks the manager future, so joining all manager-owned workers before returning extends its existing shutdown barrier to those TCP workers without changing `RQS::run()`/`SendInfo` JSON/TS contracts.

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

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Limits / admission | P | P | P | X | P | P | X | P | X | P | X | X | X |
| Manager / context | P | P | P | P | P | P | P | P | X | P | X | X | X |
| Shutdown / resources | P | X | P | P | P | P | P | P | X | P | X | X | P |

P planned, X not applicable: N8 no clocks/locales/currencies; N10 no new payload logging/diagnostic schema (existing formatter tests retained); N11 no billing; N12 applies only to closing owned input handles, actual read/write faults remain file-reader tests; N3 absent from permit-only logic; N6 means worker/channel loss, not a new persisted-data format. N7 means identity/control direction and capacity containment, not new authentication.

Interactions: stalled outgoing handshake × new incoming accept × another outbound; shared capacity full × normal inbound/BWU route; reused/distinct IDs × queue head × cancellation/completion; shutdown × queued/head/connecting/initial-writing/reading/classifying/terminal; worker error/panic × siblings/payload refs. Assert event IDs/types/outcomes, byte/frame readiness, socket EOF, joined completion and capacity reuse. Mutation targets: await outgoing inline; detach inbound; drop join barrier; remove/shift `>=` guards; release permits too early; cancel caller instead of child; override a terminal outcome; start duplicate legacy IDs concurrently.

## Verification facts

Not run yet. Pending and historical results are deliberately separate. No physical acceptance claim.
