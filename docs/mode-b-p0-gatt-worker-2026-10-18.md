# MODE B P0: BLE/Weave inbound worker terminal and cancellation paths

Date 2026-10-18. **MODE B (Testing)**, one bounded Stage 3–10
slice. This directly follows
[the slot/Weave negative ledger](mode-b-p0-gatt-slot-framing-2026-10-18.md)
and the [global Stage 12 NO audit](mode-b-completion-audit-2026-10-18.md).
Do NOT infer that green Stage 11 or eight extra unit tests
complete all applicable repository negative variants.
No production code, public contract, BlueZ services, physical
devices, user's installed native29 or `master` are in scope.

## Stage 1: scan and executable contract

Examined `core_lib/src/hdl/gatt.rs::run_inbound_worker`,
`gatt/managed_tests.rs`, `inbound.rs::new/attach_session/
cancellation/record_termination/handle`, session control,
`manager::session::until_either_stops`,
`manager::cancellation_message`, `channel::ChannelMessage`,
`SessionTasks`, the essential acceptance and testing SSOT.
Existing GATT managed integration runs real encrypted text transfer
over a duplex stream for accepted/rejected consent; pure
GATT parser/slot boundaries are covered by the previous slice.
But **error/cancellation terminal behavior for the actual worker**
is not directly asserted across the conditional `native`
and state gates.

The worker's contract: the native UI should **not** receive
an unsolicited Disconnected for a peer that fails during
`State::Initial` (no valid identification/consent yet);
legacy receiver sessions should report a transport error.
After native protocol has advanced, genuine I/O/protocol errors
must report Disconnected (not Finished, Rejected or Cancelled).
Explicit parent lifetime or session token cancellation after
progress must emit Cancelled; cancelling in Initial should
quietly release the unadmitted native session.
Every resulting managed terminal must be absorbing,
publish no received text/files, release SessionHub lease and
avoid stale UI events. The real worker, `InboundRequest`
and actual broadcast/session controller remain under test;
the duplex stream substitutes **only** the external BlueZ
byte I/O boundary, not the domain worker.

## Stage 2: P0 invariants

- Unidentified native peer failure MUST NOT produce a
  spurious user-visible Disconnected event or terminal artifact.
- Legacy transport failure emits a real LibToFront,
  Inbound, Disconnected message with an error.
- Native after protocol progress reports an actual
  transport/frame failure as Disconnected and updates
  the session snapshot; no false Finish/receipt.
- Parent cancellation, session cancellation and
  spontaneous socket EOF have distinct semantics,
  no detached worker, bounded cleanup and no restart.
- Managed lease closes, control is closed, active hub
  membership returns to zero; canceled worker cannot
  later publish success or move a terminal state.

## Stage 3/4: exhaustive scoped inventory and branch map

| Artifact / branch | Risk / level | Planned test |
|---|---|---|
| `InboundRequest::handle` at State::Initial, real EOF | P0 integration | native peer failure before identification must remain silent |
| `run_inbound_worker`: Some(Err), `native=true`, Initial | P0 integration | no erroneous UI event, session dropped, no progress |
| `run_inbound_worker`: Some(Err), `native=false` | P0 integration | explicit Disconnected notification and error, no terminal success |
| `run_inbound_worker`: Some(Err), `native=true`, state != Initial | P0 integration | unexpected EOF / oversized frame reports Disconnected, exact managed snapshot terminal |
| `run_inbound_worker`: None after parent lifetime cancelled, started progress | P0 integration | Cancelled event, managed session Cancelled and released, no stale Disconnected |
| `run_inbound_worker`: None after session token cancellation, started progress | P0 integration | Cancelled event and owner release, no automatic reconnect |
| `run_inbound_worker`: None when native Initial | P0 integration | cancellation is quiet and session releases |
| `InboundRequest::record_termination` at terminal | P0 integration | existing absorbing terminal tests; no new direct prod source |
| Some(Err) with AppError / already terminal, BWU pending | P0 integration | existing accept/reject/Cancel and direct TCP tests; additional combinations remain OPEN |

**Planned new methods: 5**: native initial EOF,
legacy initial EOF, native progressed error
(table for EOF and over-limit frame),
progressed worker cancellation (table for parent vs session),
native initial cancellation. These are
independent real worker invocations with bounded
in-memory duplex bytes and actual owned sessions.
No product code or mock worker.

## Stage 5: Positive/N1–N12 matrix

C = covered by relevant prior test;
P = planned with new assertion;
NA(x) = not applicable to the worker slice;
OPEN = hardware/integration gap, not a pass.

| Scoped artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Native Initial EOF + early cancellation | P | C | NA(no payload size) | P(real EOF) | P(no retry) | C | C | P(no unadmitted UI error) | NA(no clock) | P(shutdown) | P(no identifier/metadata) | NA(no billing) | NA(no FS) |
| Legacy Initial EOF | P | C | NA(no frame body) | P | P | C | C | NA(legacy not a security principal) | NA(no time) | P | P(no sensitive error) | NA(no entitlements) | NA(no FS) |
| Managed progressed frame error | P | P(overlimit frame) | P(SANE+1) | P(EOF) | P(no retry) | C | P(malformed frame) | P(no success authority) | NA(no clock) | P | P(no receipt) | NA(no billing) | NA(no FS) |
| Worker cancellation by parent/session | P | C | NA(no payload length) | NA(not error injection) | P(no retry) | P(two owners) | C | P(no new authority) | NA(no time) | P | P(no contents) | NA(no billing) | NA(no FS) |
| Actual BlueZ notifier failure and BWU | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | NA(no wall-clock logic in parser) | OPEN | OPEN | NA(no billing) | OPEN |

These P/C judgments are scoped, not a repo-wide
D2 category total. Hardware remains STOP-FACTOR
for significant transport/DI redesign; tests do not
change its architecture.

## Stage 6: interaction matrix

Transport state {Initial, ReceivedConnectionRequest} ×
receiver {native, legacy} ×
terminal trigger {socket EOF, oversized 4-byte frame,
parent cancel, session cancel} × session ownership
{managed, legacy}. Only meaningful distinct routes
are tested: legacy sender receives event, unadmitted
native stays silent, progressed managed receives
correct terminal. Cross-effects: cancellation must
not leave a SessionHub member or incorrect result.

## Stage 7: estimates / Stage 8 per-production map

**5 test methods**, case counts: 1 native initial
EOF, 1 legacy EOF, 2 native progressed errors
(EOF/oversize), 2 progressed cancellation triggers
(parent/session), 1 initial cancellation: **7
specific executions**. New-category overlap
(planned): Positive 5, N1 1, N2 1, N3 3,
N4 5, N5 1, N6 1, N7 4, N8 NA, N9 5,
N10 5, N11 NA, N12 NA.

| Production file | Target functions | Explicit covered/planned branches | Remaining |
|---|---|---|---|
| `hdl/gatt.rs` | `run_inbound_worker`, `inbound_disconnected_message` | native Initial error suppression, non-native/advanced failure, parent/session cancellation, no false terminal | AppError already-terminal and real BWU/client notifier remain OPEN |
| `hdl/inbound.rs` | `handle`, `record_termination` | frame-header EOF/invalid frame and terminal delivery via actual worker | More full UKEY2/BWU permutations and hardware remain OPEN |
| `sessions.rs` | session lease/drop and control snapshot | failure and cancellation state, automatic release | Global session API branch map OPEN |
| `manager/session.rs` | `until_either_stops` | cancellation-vs-IO scope through real function | Other concurrent combinations OPEN |
| `channel.rs` | message state/direction/type | actual event assertions | DTO-only properties not an independent P0 priority |

## Stage 10: planned mutation review

Removing native Initial suppression must fail
the early EOF test. Treating legacy as native
or swallowing progressed errors must fail
the type/status/error/snapshot assertions.
Returning Finished on error/cancellation,
silencing a real Cancelled event, releasing
only channel but not SessionHub lease,
or incorrectly restarting must fail a
case. Replace equality or boolean gates
with a mutant -> at least one assertion must
change. **No instrumented mutation run is
claimed**; mutation survival not yet measured.

## Milestones

1. [x] Scan actual worker, callbacks and session controls;
   document exhaustive bounded inventory/matrix first.
2. [x] Add all five integration test methods to
   `gatt/managed_tests.rs`, preserving the real
   production worker, sender, session and protocol.
3. [ ] Verify Core stable 1x / race-sensitive 10x
   via Preflight, and existing GTK; diagnose red
   results, never weaken domain assertions.
4. [ ] Mark scoped branch/category outcomes,
   report actual counts and source/CI results.
5. [ ] Reconcile `mode-b-inventory.md` and
   `current-state.md`; global Stage 12 remains NO.

Rollback: revert only the new test additions
and scoped docs. No production logic changes
are permitted under this Mode B slice.

## Implemented exact real-worker cases — awaiting CI

Test-only `core_lib/src/hdl/gatt/managed_tests.rs`
commit `e7f88543` added five P0 integration methods
plus a small actual managed-session + in-memory duplex
fixture. No product API, GATT notifier dependency,
auth rule or transport code changed. The methods
exercise: native silent initial EOF, legacy explicit
error, native post-initial EOF or 0xffffffff
frame-length attack, parent/session cancellation
of active native transfer, and silent initial
cancellation. Each assertion checks real
session-lease release, actual broadcast event
state/type/id/error, no false success/result,
no duplicate events. Existing real encrypted
consent tests remain untouched.

The native after-initial precondition is set in
`InboundRequest.state.state`, deliberately
scoping the worker's conditional error path,
not claiming to re-run full UKEY2/BlueZ setup.
Hardware BlueZ characteristic lifecycle and
actual BLE-to-TCP upgrade remain separate OPEN
P0 obligations. New tests will not be labeled
green until exact-source CI verifies them.


## Verification checkpoint / independent package-change CI blocker

The test implementation `e7f88543` and test-only rustfmt
repair `9030e6d9` are committed. Their Preflight
[39081896620](https://github.com/morozhenko-apps/rquickshare/actions/runs/39081896620)
and GTK
[39081896702](https://github.com/morozhenko-apps/rquickshare/actions/runs/39081896702)
were **cancelled** due concurrent pushes, not
counted as five successful scenarios.

Parallel `dev` commits `c918ebb0` and
`78d00da1` changed native Debian/APT
release identity and scripts, not GATT
tests. On [Preflight 39081994995](https://github.com/morozhenko-apps/rquickshare/actions/runs/39081994995)
formatting, frontend and Core strict Clippy are green;
Python contract tests fail during APT fixture setup
with `ValueError: Unexpected package path`.
The earlier parallel packaging build had
7 APT assertion failures / 2 missing old-package
path errors (`0.11.5+native1` vs `1.0.0-1`).
Those are genuine CI failures in a separate
changing packaging contract, **not** GATT
worker findings. No assertions are being
weakened. We avoid modifying packaging
owned by a concurrent workstream.

Five added worker test methods must be
verified individually `ok` and Core
stable 1× / risk-sensitive 10× checked
at the same HEAD before marking Stage 9
execution. Entire Preflight and full MODE B
remain NOT SUCCESS until Python APT
contract parity is restored and rerun.
Strict previously successful 10× was
before these new worker tests. Global
MODE B Stage 12 remains **NO**.
