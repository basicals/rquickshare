# MODE B P0: partial TCP introduction timeout, EOF and classifier cancellation

Date: 2026-10-18. Primary **MODE B / Testing**. Direct `dev` only.
Production logic and authorization boundaries are unchanged. Never
change `master`, installed/native packages, peer hardware, signing,
release tags, or user services under this test-only scope.

## Stage 1 — repository scan

Read the 378-line user universal `agent.md`, current
`docs/mode-b-completion-audit-2026-10-18.md`,
`docs/native-essential-priorities.md`,
`docs/mode-b-p0-manager-ingress-2026-10-18.md`,
`docs/mode-b-p0-bwu-route-2026-10-18.md`,
`core_lib/src/manager.rs`, `core_lib/src/manager/session.rs`
and `core_lib/src/hdl/bwu.rs`. Runtime is Tokio Rust with
`#[tokio::test(start_paused = true)]` already in repository.
Current `dev` Preflight runs real stable Core Rust once with
separately scheduled race probes; final full 10x MODE B requires
the release-gated suite and is **not** satisfied by quick CI.

## Stage 2 — P0 invariants, observed branches and risks

The real ingress classifier reads with `TcpStream::peek` and
uses a 750 ms outer timeout plus a 5 ms internal retry delay
when a prefix/body is incomplete. The source must not consume
TCP bytes while classifying, route a partial introduction or
steal another session's pending `BwuRouter` reservation.

An uncompleted TCP preface, peer EOF, and dropping the
classifier future are independent negative paths.
A pending registered owner remains able to receive a
**new** legitimately classified connection after either path.
The old/returned socket's data remains intact until its owner
receives or explicitly disposes of the connection.

No test may claim that the current 750 ms limit is optimal for
Google peers, that physical BlueZ handoff succeeds, or that the
plaintext endpoint ID is cryptographically authenticated.
The latter remains P0 [Issue #8](https://github.com/morozhenko-apps/rquickshare/issues/8)
and requires a separately approved MODE A design.

## Stage 3 — exhaustive scoped artifact inventory

| Artifact | Risk / level | Planned new test methods |
|---|---|---|
| `route_bandwidth_upgrade_if_pending`: no bytes or prefix/body incomplete through the outer timeout, return original socket without byte consumption or BWU admission | P0 real TCP integration with fake Tokio clock | 1 table-driven method, 5 incomplete prefix/body boundaries |
| `route_bandwidth_upgrade_if_pending`: empty/short/partial inbound preface then peer half-close before completing BWU | P0 network EOF integration | 1 table-driven method, 3 early-EOF branches |
| `route_bandwidth_upgrade_if_pending`: cancelled/dropped in-flight classifier, peer disconnect, unrelated route survives, fresh socket accepted | P0 cancellation and ownership integration | 1 method |
| `BwuRouter::register/has_pending/cancel/route_with_alias` | P0 already scoped in previous BWU tests | Reuse actual router, no production code edits |
| Real GATT/BlueZ, authenticated session handoff and installed KDE/Android/Windows | Hardware and protocol acceptance | OPEN, not represented by these tests |

Expected: **3 original `Given_When_Then` methods**, internally
table-driven where multiple numeric boundaries share the same rule.
Real localhost `TcpStream`/`TcpListener` and the actual router
are used. Timer advancement uses `tokio::time::advance` with
`start_paused=true`, not a real wall-clock delay, and a
`futures::poll!` establishes the in-flight state first.
No mock of the classifier or router.

## Stage 4 — branch-to-test map

1. `socket.peek` initially pending, shorter than four bytes,
   or full numeric header with incomplete body; outer timeout
   `Err(_) => None`; `Ok(Some(original_socket))`, pending route
   not consumed, original buffered prefix delivered unchanged:
   `given_incomplete_bwu_tcp_prefaces_when_classifier_deadline_passes_then_socket_and_owner_remain_intact`.
2. `socket.peek` returns zero bytes immediately after peer
   closes write side, or returns truncated data indefinitely;
   no BWU admission, pending owner untouched and EOF preserved:
   `given_tcp_peer_eof_before_complete_bwu_introduction_when_classified_then_no_owner_is_consumed`.
3. Future dropped while incomplete preface pending; incoming
   socket closes, existing router reservation remains live,
   fresh valid TCP connection routes and retains complete
   original bytes:
   `given_bwu_classifier_is_cancelled_mid_header_when_new_connection_arrives_then_existing_owner_still_routes`.

The new tests deliberately do not claim exhaustive true
concurrent GATT/BlueZ or kernel-read-error coverage.

## Stage 5 — Positive / N1–N12 matrix

C = existing, P = planned, X = not applicable with reason,
OPEN = independent security or physical gate.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Incomplete TCP prefix/body | C+P (next connection intact) | P (incomplete bytes) | P (0,1,2,3,header,body-1) | P (timeout) | P (reservation not consumed) | P (in-flight read) | X (bytes not decoded) | P (no false owner) | X (no user locale; monotonic timer covered by N3) | P (late delivery) | P (no wrong-session bytes) | X (no billing) | X (no disk) |
| Peer EOF during preface | C+P | P | P | P (EOF before header/body) | P (no implicit retry) | X (no shared race in EOF branch) | X (not protobuf corruption) | P | X | P (peer exit) | P | X | X |
| Classifier cancellation | C+P | X (valid partial prefix) | X | P (socket close) | P (new explicit route) | P (classifier vs cancellation) | X | P (no stale reservation loss) | X | P (local cancellation) | P | X | X |
| Authenticated hardware BLE→TCP | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | X (no locale requirement) | OPEN | OPEN | X | OPEN |

## Stage 6 — interaction matrix

Prefix state {empty, 1, 2, 3, complete header,
incomplete body} × peer state {live slow, write EOF,
aborted classifier} × owner {registered and waiting,
same owner on new fresh TCP connection}.
Outcomes {returned same socket unchanged / EOF after
buffer / fresh stream routes without replay or byte loss}.
No real hardware assumptions or sleeps.

## Stage 7 — estimated category counts

Three unique new methods (subcases not separately reported
as unique methods). Overlapping method attribution:
Positive 3, N1 2, N2 2, N3 3, N4 3, N5 2,
N6 0 X, N7 3, N8 0 X, N9 3, N10 3,
N11 0 X, N12 0 X. Actual verification and subcase
counts must be recorded from real CI.

## Stage 8 — per-production-file coverage map

`core_lib/src/manager.rs`: `route_bandwidth_upgrade_if_pending`
outer timeout, zero-byte read, short preface, partial
body polling, ownership handoff and cancellation,
plus existing valid/malformed/dynamic/concurrent routes.
Remaining: unexpected OS `peek` errors, scheduling
under real network jitter, authentication of alias and
real GATT/BlueZ handoff. `core_lib/src/hdl/bwu.rs`
exercises actual owner registration/consumption, but the
new tests add no changes to its production functions.
`core_lib/src/manager/session.rs` has supervisor
cancellation beyond this scope. Every other production
file and complete global Stage 8 map remain OPEN.

## Stage 10 — planned mutation review

A classifier that consumes bytes while peeking, routes
incomplete prefixes, drops the unrelated waiter on EOF
or timeout, ignores cancellation, leaks an old socket, or
fails a subsequent correct register/route is expected to
fail an exact byte/state assertion. Test timer durations
are driven by the actual source 750 ms value, with
fake clock advancement sufficient to trigger it.
No executable mutant engine run is claimed.

## Stages / completion and rollback

1. [x] Source scan, risks, full scoped inventory, branch
   map, N1–N12 matrix, interactions and estimates **before code**.
2. [ ] Add all 3 planned test-only methods, with no new
   production seam and no sleep.
3. [ ] Confirm rustfmt, real compiled tests, Core CI and
   actual per-case names/results on exact source SHA.
4. [ ] Record D2 category outcomes, mutation review,
   evidence, global inventory and current-state updates.
5. [ ] Do not mark final repo-wide MODE B Stage 11/12
   or hardware acceptance complete without the full
   final-source verification and physical acceptance.

Rollback: revert only test source and documentation.
