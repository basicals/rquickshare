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
| `route_bandwidth_upgrade_if_pending`: no bytes or prefix/body incomplete through the outer timeout, return original socket without byte consumption or BWU admission | P0 real TCP integration with fake Tokio clock | 1 table-driven method, 6 incomplete prefix/body boundaries |
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
2. [x] Add all 3 planned test-only methods, with no new
   production seam and no sleep.
3. [x] Confirm rustfmt, real compiled tests, Core CI and
   actual per-case names/results on exact source SHA.
4. [x] Record D2 category outcomes, mutation review,
   evidence, global inventory and current-state updates.
5. [ ] Do not mark final repo-wide MODE B Stage 11/12
   or hardware acceptance complete without the full
   final-source verification and physical acceptance.

Rollback: revert only test source and documentation.

## Stage 9 code checkpoint (awaiting CI)

Test-only source commit `bccbab68d0d02026ffa79b964a359f8728e09123`
adds three new test methods in `core_lib/src/manager.rs`.
Exactly **six** still-incomplete prefix/body byte counts,
**three** peer EOF positions and **one** in-flight classifier
cancellation followed by genuine admission of a fresh new TCP
connection are asserted. The timeout tests use deterministic
`#[tokio::test(start_paused = true)]`, `futures::poll!`, and
`tokio::time::advance` with no external clocks or sleeps.
Production implementation is unchanged.

Automatic exact-code checks:
[Preflight 39094220209](https://github.com/morozhenko-apps/rquickshare/actions/runs/39094220209)
and [Native GTK 39094220199](https://github.com/morozhenko-apps/rquickshare/actions/runs/39094220199).
**Neither can be counted green until its completed status and actual
per-method logs are checked**. If a new test uncovers a product defect,
keep invariant/assertion intact, record the failing outcome and STOP
for a MODE A decision rather than weakening the test.

## Red CI triage before changing the test

Initial formatted test source `9cbb4291` failed to compile due to
one test fixture passing `Result<SocketAddr>` where the actual
`SocketAddr` was required. The test-only `.unwrap()` correction was
committed as `c20af686`; production code and assertions were intact.

On [Preflight 39094391990](https://github.com/morozhenko-apps/rquickshare/actions/runs/39094391990),
the Core job compiled and executed **728** stable methods:
**727 passed, 1 failed**. The two fake-clock timeout/EOF methods
passed. The cancelled-socket test failed at
`abandoned_peer.read(...).await.unwrap()` with Linux
`Os { code: 104, kind: ConnectionReset }`. The actual
error comes from the peer observing a closed TCP descriptor with
two unread bytes on the opposite side, which may be reported
as an abortive RST rather than a graceful FIN. Other candidate
causes considered: wrong peer address, unconsumed registered
route, invalid introduction frame, early normal closure,
lost classifier cancellation, or unexpected socket write errors.
The observed `ECONNRESET` on the *abandoned* TCP read is
consistent with valid cancellation, not evidence of a wrong
BWU owner or a production regression.

**Reconciled test contract (before edit):** the peer of an
aborted classifier must observe a closed TCP connection:
either `Ok(0)` for graceful EOF or precisely
`ConnectionReset` for a socket closed with unread data.
Any successful nonempty read or other I/O error still fails.
The later fresh legitimate connection MUST route to the original
live owner and preserve the full protobuf bytes, with no
pending registration left. Record the actual read result in the
test's diagnostic assertion log. This corrects an incorrect
test-level assumption about TCP teardown; it is not an approved
production logic change or generalized weakening of the security
invariant. Run exact-source CI again after this correction.

## Confirmed scoped execution — three P0 methods

Exact test SHA `c8753010c863049aee5a26ac39482ffb80f5605d`;
[GitHub Preflight run 39094606972](https://github.com/morozhenko-apps/rquickshare/actions/runs/39094606972),
Core Rust job `117578366939`: **SUCCESS**.
Read the complete job log, not merely the Actions badge:

- Stable Core **728 passed / 0 failed / 0 ignored**, with
  the standard **8 race-sensitive methods filtered** into
  their separately scheduled execution profile.
- Existing integration bins **4/4 + 4/4** pass.
- Each of the **three new methods** appears
  exactly **once** in the real Core output as `... ok`.
- Source has **six table variants** for live incomplete
  prefaces, **three** early-EOF variants and one
  abort-then-new-connection path with peer-address
  and full-wire content assertions.
- **All five Preflight jobs SUCCESS** on the exact
  source: Core tests, Core Clippy, Rust format, frontend
  lint/typecheck/unit tests and desktop Tauri
  Clippy/unit tests (10/10). The ordinary quick
  Preflight is fully green, not the final 10x gate.

### D2 — actual method counts by Positive/N categories

Exactly **3 new method implementations**, with overlapping
category tags, each executed successfully once on final SHA:

| Category | New methods assigned | Verified scoped result |
|---|---:|---|
| Positive | 3 | PASS: preserved stream/owner, proper EOF, later valid connection |
| N1 input validation | 2 | PASS: incomplete 0–4 prefix/body and early EOF |
| N2 boundaries | 2 | PASS: 6 split counts, 3 EOF positions |
| N3 network failures | 3 | PASS: deadline, EOF, cancelled ingress TCP |
| N4 idempotency/retry | 3 | PASS: no false registration consumption or automatic replay |
| N5 concurrency | 2 | PASS: in-flight partial classifier and cancellation vs owner |
| N6 corrupt data | 0 | N/A: only incomplete frames, no decoded corrupted payload |
| N7 security/ownership | 3 | PASS: other BLE owner remains, no wrong stream |
| N8 locale/time | 0 | N/A: monotonic transport deadline belongs to N3, not locale/DST |
| N9 interruption | 3 | PASS: slow peer, peer half-close and local classifier abandon |
| N10 privacy/cross-peer | 3 | PASS: original TCP bytes and peer identity are isolated |
| N11 monetization | 0 | N/A: transfer classifier contains no entitlements |
| N12 storage failure | 0 | N/A: no file system in this scope |

The purpose is the named bounded TCP ingress branches, not
an aggregate repository category count. No mutations were
executed with a mutation engine. Manual mutation review:
consuming a byte in peek, treating a partial header as
routable, removing the timeout, incorrectly consuming the
owner, leaking the old socket, or losing the subsequent
correct fresh connection would fail explicit assertions.
Changing the OS RST to graceful FIN after cancellation
**does not** violate the intended property, hence the
closure assertion accurately accepts those two exact
outcomes and rejects all other outcomes.

**Remaining:** complete Stage 11 ten-pass on the exact
final source, full Stage 12 all-method/all-branch proof,
actual BlueZ notifier/GATT, Google Android/Windows/KDE
physical acceptance and security [Issue #8](https://github.com/morozhenko-apps/rquickshare/issues/8).
No production code or installed packages changed.

## Final same-source quick CI result

At exact SHA `c8753010c863049aee5a26ac39482ffb80f5605d`,
[Preflight 39094606972](https://github.com/morozhenko-apps/rquickshare/actions/runs/39094606972)
**completed SUCCESS (5/5 jobs)**; desktop Tauri Rust **10/10**
passed, verified from desktop job logs. Independent
[Native GTK UI verification 39094606916](https://github.com/morozhenko-apps/rquickshare/actions/runs/39094606916)
**completed SUCCESS** with **52/52 GTK unit tests** from
its actual job log. This is real exact-code automated
CI evidence, **not** 10 consecutive complete Mode B suite
runs and **not** actual installed KDE/Wayland or Google peer
acceptance. All seven repository-wide Stage 12 answers
remain NO until independently reconciled.
