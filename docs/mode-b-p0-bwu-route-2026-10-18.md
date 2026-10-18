# MODE B P0 bandwidth-upgrade route ownership, replay and cancellation

Date 2026-10-18. **Primary MODE B — Testing.**
Source-only testing scope on `dev`, bounded to the
P0 BLE-to-TCP BWU owner handoff. This does not
authorize edits to product routing logic, API,
security boundaries, user devices, systemd services,
release infrastructure, `master`, or installed
native29.

## Stage 1: repository scan

Reviewed universal `agent.md` testing SSOT,
`docs/mode-b-completion-audit-2026-10-18.md`,
`docs/mode-b-inventory.md`,
`core_lib/src/hdl/bwu.rs` production and existing
11 tests, `core_lib/src/hdl/migratable.rs`,
`core_lib/src/hdl/gatt.rs` worker,
`core_lib/src/manager/session.rs`, and
current CI / recent P0 Weave worker reports.
Existing tests exercise basic routing, alias,
duplicate registration, stale receiver,
cancellation, exact-id preference and racing
double admission. They do **not** yet
assert end-to-end ownership and byte
isolation after an *untrusted alias*
or an *invalid/replayed/late socket*
in one shot, using the actual returned
TCP stream rather than accepting `is_err`
as adequate proof that data/owners are
preserved.

P0 threat: one peer's TCP socket is accidentally
consumed by another BLE session or a cancelled
old registration. Direct real BlueZ identity
and crypto continuity cannot be proved by
this router alone. Only router correctness
is being exercised with actual isolated
localhost TCP sockets. The caller must
separately authorize aliases, and this
separate auth contract remains OPEN.

## Stage 2: invariants

1. A route may consume **one** currently
   registered, live waiter only.
2. An unknown endpoint/alias **must return
   the same intact TCP socket** to the
   caller; it must not consume unrelated
   waiters or leak data to another owner.
3. If exact current ID and old alias
   are both live, exact wins and the
   old owner must remain intact and
   independently routable.
4. A successful route is one-shot;
   a second socket may not be replayed
   into the consumed owner or silently
   destroyed.
5. Cancelling the old waiter closes its
   receiver; a late TCP socket must be
   returned intact, and only a newly
   registered generation may accept it.
6. Real data on the two TCP paths must
   remain disjoint. No test credentials
   or actual private peer content.

## Stage 3: exhaustive scoped artifact inventory

| Production item | Risk / level | Planned |
|---|---|---|
| `BwuRouter::route_with_alias`, unknown id and unknown alias | P0 integration | reject and **return original usable socket**, preserve unrelated registrations |
| `route_with_alias` exact id and simultaneously live alias | P0 integration | exact owner receives its own bytes, alias remains and independently receives its own bytes |
| `route_with_alias` one-shot consumed route then extra inbound socket | P0 integration | extra socket returned intact; no duplicate notification/delivery to old owner |
| `BwuRouter::cancel` then late ingress and register fresh generation | P0 integration | old receiver closes, late bytes not delivered, new socket reaches only new lease |
| `BwuRouter::register`, `has_pending` | P0 unit/integration | existing tests cover invalid/duplicate/abandoned cleanup; reassert meaningful invariant in each variant |
| `MigratableStream::Tcp/Ble` forwarding | P0 integration | existing 4 tests for roundtrip/shutdown; this slice does NOT claim real crypto/BlueZ BWU |
| `do_bandwidth_upgrade` and actual GATT notify | P0 hardware/contract | OPEN, no fake positive outcome |

Planned **4 new independent test methods** using
real localhost `TcpStream` pairs, `oneshot`
receivers and actual router. No production
domain mocks, test sleeps, or bypass
of the live route ownership semantics.

## Stage 4: branches to exact test names

- Unknown identity and unknown alias branch
  returns `Err(original_socket)` before
  consuming any pending entry:
  `given_untrusted_bwu_alias...`
- Both exact+alias registered, exact priority
  despite old alias presence; alias can
  later be consumed normally:
  `given_two_live_bwu_owners...`
- Pending entry removed exactly once,
  second connection returns the original
  socket with bytes preserved; fresh
  register needed for subsequent route:
  `given_consumed_bwu_route...`
- Cancel removes old sender, closes old
  receiver; late socket returned; newly
  registered receiver is independent:
  `given_cancelled_bwu_generation...`

## Stage 5: Positive / N1–N12

C=prior evidence, P=new, NA=local
pure routing scope not applicable,
OPEN=requires OS / authenticated
caller boundary, not a pass.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Unknown alias route | C+P | P(unknown ID) | NA(no frame size) | P(return live socket) | P(no consumption) | P(other owners) | NA(no serialized data) | P(no route without owner) | NA(no clock) | NA(no UI) | P(no wrong-peer bytes) | NA(no billing) | NA(no disk) |
| Exact over alias | C+P | NA(valid IDs) | NA(no numeric limits) | P(real pair) | P(one-shot) | P(both live) | NA(no schema) | P(exact owner) | NA(no clock) | NA(no UI) | P(no cross-peer) | NA(no billing) | NA(no disk) |
| Replay after consumption | C+P | P(used ID) | NA | P(original TCP returned) | P(no implicit retry) | P(late socket) | NA | P(no unauthorized reuse) | NA | NA | P(no leaked bytes) | NA | NA |
| Cancel and new generation | C+P | P(cancelled ID) | NA | P(socket retained) | P(explicit re-register) | P(old/new waiters) | NA | P(no stale authority) | NA | P(cancel) | P(no leaked bytes) | NA | NA |
| Actual BLE→TCP peer auth/migration | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN where relevant | OPEN | OPEN | NA(no billing) | OPEN |

## Stage 6: interactions

Route {unknown/current/alias} × owners
{absent/one/two/consumed/cancelled/re-registered}
× actual TCP {first/late/fresh} × data
{peer A/peer B}; branch outcomes {return
unchanged socket/exact owner/previous owner}.
One router entry has one matching waiter,
and fresh explicit registration is required
before reuse. No real BLE key continuity
claim or simulated protocol success.

## Stage 7: scoped test counts

Four new P0 test methods (each with distinct
action/assertion group). Category assignment
counts overlap and are not global D2:
Positive 4, N1 3, N2 0 NA, N3 4,
N4 4, N5 4, N6 0 NA, N7 4,
N8 0 NA, N9 1, N10 4,
N11 0 NA, N12 0 NA.
Actual results to be recorded after CI.

## Stage 8: production-file coverage map

`core_lib/src/hdl/bwu.rs`:
`register/route_with_alias/cancel/has_pending`
route-id and stale-generation branches
covered by existing tests plus four new
cross-owner socket-content invariants.
`core_lib/src/hdl/migratable.rs`:
existing four real byte roundtrips and
half-shutdown tests, but **not** a full
migration handoff here. Upstream
identity validation and physical BlueZ
notifier remain OPEN. Other production
files remain subject to repository-wide
Stage 3–10 audit.

## Stage 10: planned mutation review

A mutant allowing route to succeed on
unknown alias, selecting alias when
exact owner exists, copying rather than
moving a route owner, retaining a consumed
registration, accepting late cancelled
socket, or dropping returned socket must
fail one or more assertions. Read actual
bytes both ways to ensure identity,
not merely `is_ok`. No mutation engine
run is claimed.

## Stages / rollback

1. [x] Analyze P0 BWU routing, document
   inventory, branch map, matrices first.
2. [x] Add 4 test-only methods to `bwu.rs`,
   checking real byte routes and owner closure.
3. [x] Verify on same-source Core CI.
   If generic Preflight is cancelled due
   unrelated parallel packaging, run
   separate dedicated exact-inventory P0
   BWU job with independent concurrency
   (not a replacement for full MODE B).
4. [x] Document actual outcomes, update
   global inventory/Current State,
   retain Stage 12 NO and hardware gaps.

Rollback reverts only tests / optional
separate CI workflow / documentation.

## Stage 9 source-only implementation checkpoint

Test-only commit `a086a142` adds exactly
four original P0 BWU owner/alias/late
route methods to `core_lib/src/hdl/bwu.rs`,
plus an isolated `TcpListener` connection
fixture. Uses real `BwuRouter` shared
pending map, actual localhost TCP streams,
real `oneshot` cancellation/delivery,
exact owner IDs and peer-originated
bytes; does not mock routing or caller
security policy. No runtime source code
changed.

- Invalid exact/alias identity returns
  **same original usable TCP socket**
  and preserves two unrelated waiters.
- Exact route with simultaneous live
  alias delivers two independent real
  sockets and exact content to each
  registered owner in sequence.
- Attempted same-ID replay after first
  route is rejected with usable late
  TCP socket; a new explicit registration
  admits only a separately connected
  fresh TCP stream.
- Cancellation closes the old receiver,
  refuses the late connection without
  losing its bytes, then a new owner
  accepts only a new generation's socket.

All four methods assert that pending
registration count is zero after explicit
cleanup or complete consumption. There
are no sleeps or external LAN discovery
requests. These contracts establish
the lower-level router owner boundary,
not authenticated device credentials
or end-to-end physical BLE/TCP handoff.

Exact-source CI invoked automatically:
[Preflight 39084068799](https://github.com/morozhenko-apps/rquickshare/actions/runs/39084068799)
and
[Native GTK 39084068790](https://github.com/morozhenko-apps/rquickshare/actions/runs/39084068790).
**Both were running at the checkpoint**.
Do not count test methods as passing until
their logs verify the assertions; red
Python APT packaging tests in earlier
workflows were a separate concurrent
source issue, not bypassed here.

## Focused exact-source CI provision

Formatting-only test diff `4f35fc35` corrects
rustfmt wrapping in four original test
methods without touching assertions.
A dedicated read-only,
non-cancelling path-scoped
[P0 BWU routing workflow](https://github.com/morozhenko-apps/rquickshare/actions/workflows/p0-bwu-router.yml)
at commit `7a08e57c` isolates
the critical router tests from the
parallel APT/signing release work.
It compiles the genuine Core crate,
checks all **four new names exactly
once**, verifies exactly **15 BWU
test methods** (11 existing + four
new) and runs **all 15 ×10**
consecutively with a strict
`15 passed / 0 failed / 0 ignored`
output gate. Different from global
Stage 11: no other Core, GTK,
frontend/Python suites are implied.

Initial workflow
[39084212567](https://github.com/morozhenko-apps/rquickshare/actions/runs/39084212567)
was running at this checkpoint.
This dedicated job must complete
successfully before marking the
new four methods as verified.


## Stage 9 exact-source verification — confirmed

Workflow SHA `7a08e57cb95d3f4854df69c84cf3a2d85019ed22`
includes all four original P0 BWU regression
methods, rustfmt-only source fix `4f35fc35`,
and independent
[GitHub run 39084212567](https://github.com/morozhenko-apps/rquickshare/actions/runs/39084212567),
job `117547278250`: **SUCCESS**.

The verified job log contains all ten
numbered attempts `1/10` through `10/10`,
ten complete `test result: ok.
15 passed; 0 failed; 0 ignored` reports,
and **exactly ten `... ok` lines for EACH of
the four new methods**. The other 11
pre-existing BWU tests also executed 10×.
Thus: **15 methods ×10 = 150/150 pass,
0 failed, 0 ignored**, no missing
method, skipped attempt, implicit retry,
or softened expectation. Tests were
run with real `TcpStream` pairs and
real `BwuRouter`, on the exact
source SHA.

This is a **focused Stage 9/11
BWU subgate**, NOT complete Mode B
full-repository Stage 11; other
product/UI/APT suites were not part
of this targeted job.

### D2 scoped actual category mapping

Exactly **4 new test methods**; overlap
across applicable negative categories,
not a unique-per-category tally or
global D2 count:

| Category | New methods assigned | Scope outcome |
|---|---:|---|
| Positive | 4 | PASS: legitimate subsequent routing after correct owner |
| N1 invalid identity | 3 | PASS: forged/used/cancelled identifier |
| N2 size/limits | 0 | N/A: router has no numeric payload limit |
| N3 transport | 4 | PASS: real returned/delivered connected socket |
| N4 replay/idempotency | 4 | PASS: one-shot/current/late/new registration |
| N5 concurrency | 4 | PASS: independent live owners and epochs |
| N6 corrupt data | 0 | N/A: no parser/deserializer |
| N7 ownership/permissions | 4 | PASS: exact owner and stale owner restrictions |
| N8 locale/clock | 0 | N/A: no locale or clock |
| N9 user interruption | 1 | PASS: cancellation and new explicit registration |
| N10 privacy | 4 | PASS: no cross-peer or stolen socket bytes |
| N11 entitlements | 0 | N/A: no billing |
| N12 storage failure | 0 | N/A: no persisted files |

Mutation *review*, not mutation-engine
measurement: removing one-shot owner
consumption would fail the second-socket
assertions; falling back to alias before
exact would fail both owner identities
and byte checks; ignoring `cancel`
would leave old receiver open/allow
stale delivery; returning an unrelated
socket or dropping the original would
fail actual two-way TCP byte assertions.

### Remaining P0 and Stage 12 risks

`BwuRouter` enforces registered
one-shot ownership but **does not
authenticate a peer or derive
alias authorization**. The untrusted
`last_endpoint_id` must be validated
by its upstream protocol boundary.
Real BLE/BlueZ (GATT notifier),
actual encrypted BWU role negotiation,
TCP discovery through mDNS and actual
Pixel/Windows dual-device handoff
remain **OPEN**, plus every applicable
global N1–N12 outside this bounded
router slice. No production code changed.
Full MODE B Stage 12: **NO**.
