# Native core adapter readiness review

Recorded: 2026-10-12. Mode A. Coordinator foundation is implemented in `0b0d9fd`; this review records the next concrete integration blockers before changing transport orchestration.

## Immediate bounded fix plan

Source inspection of `core_lib/src/manager.rs` found inconsistent outbound failure correlation: queue/startup errors use `SendInfo.id`, but errors inside the handshake/transfer loop use `SendInfo.addr`. The coordinator must map every core observation to its transfer; an address is not that identity. A separate `State::Initial` error arm silently suppresses a terminal failure. That arm is a source-level finding, not a claim that a normal post-init test can reach it. Already terminal transfers must retain their outcomes on subsequent I/O failure.

1. Record this contract and test inventory before code (this document).
2. Centralize outbound failure-message construction: use the stable request ID, `LibToFront`, `Outbound`, `Disconnected`, bounded error text and current metadata; preserve already Finished/Cancelled/Rejected/Disconnected states. Queue/startup and post-handshake failures use the same helper. No crypto/framing, concurrency, ingress or settings changes.
3. Verify three new methods once: a table over all current protocol states (report failure for active/initial states, absorb every terminal state), an actual loopback connection with constructor rejection, and an actual loopback peer sending malformed handshake input. Both integration cases assert exactly one terminal event, correct ID/type/state and bounded error text; the handshake case preserves metadata. Existing manager tests run in the same targeted invocation; format/Clippy follow. No sleeps or physical network/BlueZ.
4. Commit independently and record results. Revert this fix without reverting the coordinator or file boundary.

Scope inventory/branches: failure builder (initial/active/terminal, bounded Unicode error, optional metadata, stable identity); queue execution (connect/new error versus successful core handling); handshake failure (decode/I/O error, preserved metadata, no duplicate terminal result). N1 malformed peer input; N2 bounded error text (existing test); N3 controlled local peer failure; N4 terminal-event idempotency; N5 late failure after terminal; N7 caller/core identity correlation; N9 worker exits after failure. N6/N8/N10/N11/N12 add no independent behavior here (no persistent data, clock, new logging/privacy formatter, billing or storage writes). The existing `error_for_ui` test remains applicable. No claim of full protocol coverage or completed Mode B.

## Significant orchestration decision required before a live adapter

`TcpServer::run` calls `self.connect(...).await` directly inside its main `select!` loop. `connect` includes TCP connection establishment, handshake, remote consent and the entire outbound transfer. While it is pending, that task does not poll `tcp_listener.accept()` or another queued outbound. Inbound requests are spawned into independent tasks only after accept, so an outgoing transfer waiting for peer consent can block accepting a new incoming transfer. Cancellation during TCP connect/initial writes is also outside the inner transfer-loop cancellation select. This is source-level evidence; it has not been benchmarked on a physical peer. Do not present the current manager as concurrently servicing outbound and inbound work.

This cannot be fixed by a panel spinner or by inventing coordinator events. Structured supervision changes core task ownership, concurrency and shutdown. The user approved option 1 on 2026-10-12: "1. fix it and continue". Implementation/verification is tracked in [native-transfer-supervision.md](native-transfer-supervision.md).

| Option | Concrete change | Benefits | Costs / risks |
|---|---|---|---|
| 1 — structured bounded transfer supervision (recommended) | Keep accept/control responsive, spawn each outbound into a service-owned tracked task, enforce one documented concurrency budget, supervise completion and cancellation across connect/handshake/data, await all workers on stop | Removes outbound/inbound head-of-line blocking; gives the native service truthful completion/resource lifetime; fits required Google parity direction | Substantial core refactor; races, queue admission and shutdown require a separate inventory and loopback tests; physical compatibility remains a gate |
| 2 — retain sequential outbound manager for the first adapter | Connect the native daemon to the current serial core and explicitly retain the limitation; schedule supervision later | Smaller initial migration and easier rollback | Incoming acceptance still blocks during outbound work; incomplete concurrency parity and cancellation semantics; must not advertise finished service lifecycle guarantees |

Recommendation: option 1 before the live core adapter. The coordinator's ownership/quota model remains useful, but wrapping an unsupervised serial worker would preserve the root cause and require another lifecycle rewrite. No new transport, cloud credential boundary or GUI decision is bundled into this choice.

## Verification facts

The bounded correlation fix is implemented in `90067ee`. `outbound_failure` is shared by queue/startup and in-loop failures; `execute_outbound` is the same owner entrypoint used by `run` and the two loopback tests. The subsequently approved orchestration change is implemented and verified separately in [native-transfer-supervision.md](native-transfer-supervision.md); the observations below are historical evidence for the correlation fix.

- One `cargo +stable test --all-features manager::tests` invocation passed **9 tests**, including the **3 new methods**. The earlier full coordinator/core run (237 tests) is recorded in [native-service-coordinator.md](native-service-coordinator.md); it was not repeated after this localized manager change. The current compiled unit-test inventory is 236 plus 4 integration tests; this is an inventory count, not a claim of a new 240-test execution.
- `cargo +stable clippy --all-features --all-targets -- -D warnings` passed after this change. Format/diff checks passed. Only the existing bluer future-incompatibility warning remains. No test retry loop, sleep, external peer or live desktop process was used. Command sessions exited (terminal closed).
- New primary-category counts: N1=1 (malformed peer handshake), N7=1 (constructor failure retains request identity), N4=1 (all 18 states and absorbing terminal outcomes). Positive/N2/N3/N5/N9 overlap these methods; they are not counted again. Existing Unicode/error-bound tests also ran. N6/N8/N10/N11/N12 add no independent coordinator/manager contract in this slice, as explained in the inventory.
- Assertions reject address-as-ID, missing Outbound type, missing/duplicate failure event, losing metadata, unbounded Unicode error text and overriding a terminal outcome. This is assertion/source mutation reasoning, not executed mutation tooling.

The supervision decision is approved and implemented in `ee2139a`; native outgoing ownership/startup is implemented in `2354c1d`. The user subsequently selected common TCP/BLE session controls; implementation and results are recorded in [native-session-controls.md](native-session-controls.md). The current Tauri app, file capability input and coordinator remain independently reversible; no native daemon or UI is installed/activated by these reviews.
