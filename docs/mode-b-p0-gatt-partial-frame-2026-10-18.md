# MODE B P0 managed GATT worker: partial BLE frame, failure, and cancellation

Date: 2026-10-18. Primary mode: **MODE B (Testing)**. This is a
source-only test expansion on `dev`; runtime, wire format, cryptography,
BlueZ configuration, desktop installation, `master` and releases
are explicitly out of scope. The full repository-wide Completion Gate
continues to be **NO**.

## Stage 1 — repository scan and test facilities

Read user `agent.md` testing SSOT, `docs/mode-b-completion-audit-2026-10-18.md`,
`docs/mode-b-p0-gatt-worker-2026-10-18.md`,
`docs/mode-b-p0-gatt-slot-framing-2026-10-18.md`, and
`core_lib/src/hdl/gatt.rs`, `core_lib/src/hdl/gatt/managed_tests.rs`,
`core_lib/src/hdl/inbound.rs`, `core_lib/src/hdl/frame_header.rs`,
`core_lib/src/sessions.rs`, `core_lib/src/manager/session.rs`,
`.github/workflows/lint.yml`.
The production `run_inbound_worker` invokes the actual
`InboundRequest::handle` and `SessionHub` lifecycle. Existing
`worker_fixture` supplies an actual bounded Tokio duplex pair
instead of the *external BlueZ byte transport*; it does NOT
mock the worker, parser, cancellation selector, identity or
session ownership. The native Initial EOF, progressed EOF/oversized
frame, parent/request cancellation and real encrypted consent
already have tests. Missing intersections are *incomplete*
input in flight when cancellation/EOF happens, and malformed
native Initial frames without EOF.

GitHub quick Preflight stable Core runs 1×; release-tag full
Mode B 10× remains separately gated. Latest prior exact-source
quick Preflight and native GTK both SUCCESS
(run 39094606972 and run 39094606916).

## Stage 2 — P0 risk / invariants

- Unidentified/native `State::Initial` with malformed protobuf,
  invalid length or disallowed frame MUST not publish a spurious
  Disconnected/Finished UI transfer, allocate a result, retain a
  SessionHub lease or leave a worker alive.
- After progression into `State::ReceivedConnectionRequest`,
  truncation inside the four-byte frame prefix or inside an announced
  body MUST produce precisely one Disconnected event with a
  bounded transport error, terminal managed snapshot and no
  phantom received resources.
- Parent vs request cancellation while a **genuine partial frame
  has already been received** MUST preempt blocked socket I/O
  and report one Cancelled terminal event, with no later
  Disconnected or Finished and no leaked session.
- Tests must not treat raw duplex as a real
  BlueZ `CharacteristicNotifier`. Physical GATT notify/indicate,
  reassembly, packet-order, restart, and authenticated BLE→TCP
  continuity are **OPEN**. Alias admission concern is Issue #8.
- Do not introduce sleeps, arbitrary wait timeouts, fake business
  logic or feature flags to make a test pass.

## Stage 3 — exhaustive bounded artifact inventory and branch map

| Production code / executable branch | Level / risk | New methods |
|---|---|---:|
| `InboundRequest::handle` in native Initial: frame header complete, bad numeric length, invalid/missing protobuf or disallowed frame; `run_inbound_worker` suppresses `Some(Err)` event, drops managed owner | P0 integration | 1 table-driven method |
| `FrameHeader::read` partial header then EOF, or `stream_read_exact` partial announced body then EOF; progressed native worker maps `Some(Err)` to one Disconnected | P0 integration | 1 table-driven method |
| `manager::session::until_either_stops` with parent/request cancellation during incomplete header/body; `run_inbound_worker` emits Cancelled, preserves ownership invariants | P0 integration | 1 table-driven method |
| Existing genuine encrypted accept/reject, initial EOF, completed header processing, BlueZ transport | Already separate tests / hardware | 0 here, remaining physical gaps explicit |

Exactly three original Given_When_Then methods in
`core_lib/src/hdl/gatt/managed_tests.rs`. Test cases are explicit
table rows. Each asserts precise event ID, direction, type,
terminal state, no extra broadcast events, empty result fields,
closed control and empty SessionHub membership.

## Stage 4 — test method branch mapping

1. `given_unidentified_native_malformed_ble_preface_when_worker_exits_then_no_ui_or_session_is_published`:
   `State::Initial` with [0, corrupt 1-byte protobuf,
   `SANE_FRAME_LENGTH+1`, `u32::MAX`]. The peer stays
   connected, ensuring the error is from parsing/limits rather
   than an external EOF. No emitted events; lease closed.
2. `given_progressed_native_partial_ble_header_or_body_when_peer_eof_then_one_disconnected_and_owner_release`:
   remote sends [1,2,3 header bytes] or [4-byte declared
   length+1/2 actual body bytes], then EOF. Exact one
   Disconnected with bounded error and released owner.
3. `given_progressed_native_partial_ble_frame_when_parent_or_request_cancels_then_one_cancelled_and_no_replay`:
   [two-byte partial header or 4-byte header+one body byte] ×
   [parent lifetime vs managed request cancellation],
   pending first poll checked before triggering cancel.
   Terminal Cancelled and no false complete result.

## Stage 5 — Positive + N1–N12 matrix

C = existing relevant evidence, P = planned here,
NA = not applicable with reason, OPEN = physical/auth gap.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Initial malformed framing | C+P silent release | P | P max+1/max | P error/no EOF | P single exit | NA single worker | P bad protobuf | P no admission | NA no clock | P worker exit | P no premature UI | NA no billing | NA no filesystem |
| Progressed truncated EOF | C+P disconnected | P partial | P 1–3 header,1–2 body | P early EOF | P one terminal | NA no concurrent mutation | P incomplete wire | P no phantom consent | NA | P peer exit | P no phantom payload | NA | NA |
| Progressed cancellation in-flight | C+P closed lease | P partial prefix/body | P partial bytes | P blocked real duplex | P no replay | P parent/request × frame | P truncated stream | P cancellation authorization | NA | P explicit parent/request stop | P no post-cancel UI | NA | NA |
| True GATT notifier / BLE packet fragmentation / authenticated BWU | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | NA locale | OPEN | OPEN | NA | OPEN |

## Stage 6 — interaction matrix

Native state {Initial, ReceivedConnectionRequest} × payload
{corrupt full, invalid numeric, partial header, partial body} ×
termination {parse error, peer EOF, parent cancel, session cancel} ×
outcome {no UI before identity / one Disconnected after progress /
one Cancelled on intentional stop}. Exhaustiveness is limited
to all combinations meaningful for these three worker branches:
four initial malicious payloads; five progressed truncation
variants; four cancellation-trigger/partial-shape variants.
Each uses a fresh SessionHub epoch and separate socket, channel
and worker, no cross-test state.

## Stage 7 — estimated test counts

**Three methods**, 13 table variants.
Overlapping category assignments: Positive 3, N1 3, N2 3,
N3 3, N4 3, N5 1, N6 3, N7 3, N8 0 NA,
N9 3, N10 3, N11 0 NA, N12 0 NA.
These are method assignments, NOT unique per-category
test counts; the exact CI evidence is recorded after execution.

## Stage 8 — production-file map

- `core_lib/src/hdl/gatt.rs`: actual
  `run_inbound_worker` Some(Err), Some(None)/Cancelled,
  Initial-vs-progressed gates; hardware `ReceiverGattServer::run`
  and true `weave_session` notifier **OPEN**.
- `core_lib/src/hdl/inbound.rs`: actual
  `InboundRequest::handle/_handle` incorrect length/invalid
  protobuf/short-frame errors and cancellation preemption;
  all other file/consent/crypto/migration branches out of scope.
- `core_lib/src/hdl/frame_header.rs`: actual
  partial header progress + EOF; existing unit tests cover
  lower-level re-entry and read failures.
- `core_lib/src/manager/session.rs`,
  `core_lib/src/sessions.rs`: actual cancellation selector,
  terminal event projection, owned SessionHub lease closing.
- Other production files remain subject to global MODE B Stage 3–8
  inventory, NOT claimed covered.

## Stage 10 — planned mutation review

Removing the native Initial suppression, swallowing an
incomplete-frame error after progression, publishing a
second terminal event, treating half-body as completed,
ignoring parent/request cancellation, altering event ID,
or leaking an active session should fail explicit assertions.
Manual review only; no mutation engine execution is claimed.

## Milestones / rollback

1. [x] Stage 1–8 source inspection, inventory, branch/N matrices,
   interaction matrix and counts in repository before code.
2. [x] Stage 9 implement all three original test methods, no runtime changes.
3. [x] Stage 10 mutation review and CI on exact source, do not soften
   assertions if actual production behavior violates an invariant.
4. [x] Update this ledger, global MODE B inventory and Current State
   with actual verified job/commit receipts and D1–D5.
5. [ ] Global full 10× final suite, physical Linux/Google Android/
   Windows/BlueZ evidence and all-public-branch Stage 12 remain OPEN.

Rollback only the test-only commit and this ledger plus
corresponding documentation edits. No production behavior,
protocol, security, installation or release changes.

## Stage 9 implementation checkpoint, execution pending

- Documentation-first baseline: `5685b41b`.
- Three original test-only methods committed in `2afd1112`; no
  production source or protocol branch modified.
- First automatic Rust format job in
  [Preflight 39116008314](https://github.com/morozhenko-apps/rquickshare/actions/runs/39116008314)
  reported only wrapping differences in new assertion/table syntax.
- Formatting-only test correction: `86ebe678` with assertions
  unchanged. Its exact-source
  [Preflight 39116064673](https://github.com/morozhenko-apps/rquickshare/actions/runs/39116064673)
  and [Native GTK 39116064680](https://github.com/morozhenko-apps/rquickshare/actions/runs/39116064680)
  must complete before any final PASS claim.
- Existing quick Preflight may cancel older runs when newer
  `dev` commits appear; cancelled cases never count as green.
- Stage 12 remains NO; observed BlueZ notifier and crypto gaps
  are not resolved by these duplex tests.

## Stage 9–11 verified execution

Exact test+format source SHA
`86ebe6780693bcac08eb3db363cc919a73cc95dc`.
[GitHub Preflight 39116064673](https://github.com/morozhenko-apps/rquickshare/actions/runs/39116064673),
Core Rust job `117642032398`: **SUCCESS**. Inspected the
actual complete GitHub job log:

- **731 stable Core unit tests passed / 0 failed / 0 ignored**.
  Eight separately scheduled race-sensitive methods were
  intentionally filtered from this quick stable group.
- Both existing Core integration binaries **4/4 + 4/4** passed.
- All **three newly added methods** appeared **exactly once**
  with explicit `... ok` result.
- Each method iterates every planned table variant:
  four Initial malformed payloads; five advanced EOF
  lengths; four partial-header/body × parent/session cancel.
  Thus **13 variant assertions groups** are executed inside
  the 3 successful methods. The log reports three test
  methods, not thirteen independent Rust test processes.
- Rust format, Core Clippy and frontend Preflight jobs
  **SUCCESS** on that source; desktop Tauri Clippy/unit
  status must be verified separately for the global
  five-job quick Preflight conclusion.
- [Native GTK job 39116064680](https://github.com/morozhenko-apps/rquickshare/actions/runs/39116064680)
  completed **SUCCESS**. This is a distinct native GTK
  smoke subgate, not a BlueZ device test.

### D2: overlapping scoped actual category counts

Three new methods, one successful execution of each
(13 internal table variants). Counts describe unique
*methods assigned* to each applicable category, not
independent category-specific tests.

| Category | New methods | Outcome |
|---|---:|---|
| Positive | 3 | PASS scoped expected terminal/quiet release |
| N1 bad input | 3 | PASS malformed/truncated frames |
| N2 boundary | 3 | PASS 0/max+1/u32 max/prefix 1–3/body 1–2 |
| N3 external I/O loss | 3 | PASS malicious live peer, actual duplex EOF, interrupt pending I/O |
| N4 idempotency | 3 | PASS one terminal or silent release, no replay |
| N5 concurrency | 1 | PASS parent-vs-request cancellation while pending |
| N6 corrupted data | 3 | PASS malformed protobuf / incomplete frame |
| N7 ownership/security | 3 | PASS no premature native event, exact owned event |
| N8 locale/time | 0 | N/A: no clock/locale in this managed worker slice |
| N9 interruption | 3 | PASS EOF/early parse failure/local stop |
| N10 privacy | 3 | PASS no phantom payload/consent or duplicate UI |
| N11 monetization | 0 | N/A: sharing worker has no entitlements |
| N12 storage | 0 | N/A: no file staging in these test preconditions |

### Stage 10 mutation sensitivity and limitations

Removing the native Initial-state UI suppression would
fail the first method, swallowing EOF errors in the
progressed state would fail the second, and suppressing
either parent or request cancellation / generating a
late Disconnected instead of Cancelled would fail the
third. Changing owner ID, terminal event duplication,
retaining lease, or inventing received resources fails
direct assertions. **No mutation engine was executed.**
Hardware GATT notification lifecycle remains unverified;
its production API currently consumes a concrete BlueZ
`CharacteristicNotifier`, and a reliable contract-level
notifier test would require a separately approved
transport seam (MODE A; compatibility and security
review), plus physical Google/BlueZ acceptance.
This test-only work makes no change to that API.

Global Stage 11 full ten-consecutive-suite execution on
the final SHA is **NOT** claimed. Global Stage 12 still
**NO** due unmapped branches and P0 hardware/alias auth.
