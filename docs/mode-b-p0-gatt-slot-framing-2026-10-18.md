# MODE B P0 negative matrix: GATT slot-0 and Weave framing

**Mode: MODE B (Testing). Status: planned before test changes.**
Date: 2026-10-18, `dev` only. This is a **bounded Stage 3–10
slice** of the incomplete repository-wide MODE B plan, not
a claim that all repository artifacts/negative categories
are already covered. The user requires critical scenarios
first, then secondary coverage measurements.

## Stage 1 — repository scan

Read `agent.md` universal test SSOT,
`docs/mode-b-completion-audit-2026-10-18.md`,
`docs/mode-b-inventory.md`,
`docs/mode-b-test-plan.md`,
`core_lib/src/hdl/gatt.rs`,
`core_lib/src/hdl/gatt/slot.rs`,
`core_lib/src/hdl/gatt/managed_tests.rs`,
`core_lib/src/hdl/ble_monitor.rs`,
and existing GitHub test workflows.
Test framework: in-crate Rust `#[test]` and
`cargo test --all-features`, ordinary
`Preflight` stable 1× with eight explicit
race-sensitive cases 10×, independent GTK test
job and separate strict full MODE B 10×. Existing
GATT tests check some valid/invalid handshakes and
frame bounds, plus two coherent slot-read cases,
but **not the full small-input/boundary cross product**.
No production-code changes planned.

## Stage 2 — critical invariants and risk

**P0 — discovery and resource integrity.** A remote device
can request partial GATT slot reads while the advertised
header/name changes. Managed/identity mode MUST never splice
bytes from different advertisement revisions or reveal
another peer's cached tail. The cache remains bounded to
32 peers, and a completed read cannot be replayed without
a new slot-zero request. The legacy static mode has no
per-peer cache. Every received Weave handshake must validate
control/command/version before negotiating packet size;
the length gate must reject zero/over-limit frames and
never accept a truncated payload as complete.

Hardware-only risks (MUST remain UNTESTED by unit tests):
actual BlueZ GATT characteristic read/notify, FE2C/FEF3
advertisement registration, reassembly/fragmentation and
BLE-to-TCP credential/key continuity. Existing managed
duplex tests cover some downstream paths, not a real
BlueZ notifier; these require a separate integration
testability plan/physical evidence. No DI seam, auth,
protocol or UI change is authorized by this Mode B slice.

## Stage 3 — exhaustive targeted artifact inventory

| Artifact (file) | Level / risk | Exact executable behavior & baseline | Planned new tests |
|---|---|---|---|
| `gatt/slot.rs::SlotValue::new` | Unit/P0 | coherent flag, initial empty/nonempty snapshot | included in four slot tests |
| `SlotValue::install` | Unit/P0 | new advertised revision; existing readers retain their pinned old revision | included in four slot tests |
| `SlotValue::read` legacy branch | Unit/P0 | arbitrary peer, 0/1/len/len+1/usize::MAX offsets, update | 1 |
| `SlotValue::read` coherent unprimed/empty | Unit/P0 | nonzero unknown offset rejected; zero starts snapshot even for empty; terminal at length evicts | 1 |
| `SlotValue::read` coherent peer/epoch | Unit/P0 | duplicate slot-zero replaces same peer, other peer retains old revision, consumed tail cannot be replayed | 1 |
| `SlotValue::read` cache capacity | Unit/P0 | 31/32/33 peers, evict oldest, repeated same peer does not evict another and cache remains bounded | 1 |
| `gatt.rs::slot0_deferred_advertising_enabled_for` | Unit/P1 | debug-only exact "0" override, otherwise true | existing source tests; no new cases |
| `gatt.rs::receiver_discovery_refresh` | Unit/P0 | offset 0 triggers mDNS and optional deferred ad, nonzero does not | existing 3 tests; no new cases |
| `gatt.rs::parse_connection_request` | Unit/P0 | prefix, control bit, command, version range, clamped packet bytes | 2 |
| `gatt.rs::connection_confirm` | Unit/P1 | exact 5-byte response encoding | existing one test |
| `gatt.rs::complete_framed_message_len` | Unit/P0 | short prefix, 0/max/max+1, truncation, exact, trailing frames | 2 |
| `gatt.rs::inbound_disconnected_message` | Unit/P1 | correct inbound Disconnect event, id & error | existing one test |
| `ReceiverGattServer::{with_header,with_identity,with_receive_policy,new,run}` | Integration/Smoke/P0 | BlueZ adapter, GATT D-Bus, identity revision, bounded write queue, send/notify callback, cancellation | no unit-only fake; needs BlueZ contract seam + hardware |
| `gatt.rs::weave_session` | Integration/Smoke/P0 | 24 mapped reassembly/fragment/interrupt branches in global inventory | existing full managed duplex tests 2; BLE notifier and error branches still OPEN |
| `gatt.rs::run_inbound_worker` | Integration/P0 | error/cancellation state, secure handoff, terminal event | existing managed duplex tests; untested combinations OPEN |
| `ble_monitor.rs::PassiveBleMonitor::{new,run}` | Integration/Smoke/P0 | monitor registration/fallback/DeviceFound/stop | only byte pattern unit test; BlueZ event lifecycle OPEN |

**Planned new methods: 8** (4 slot lifecycle, 2
handshake, 2 frame length). Existing tests are retained,
never skipped or weakly patched. These cover **the planned
deterministic P0 slice** only; the BlueZ and worker cases
remain explicitly unclosed in global MODE B.

## Stage 4 — branch map to exact new test names

- Slot `!coherent` returns current.get(offset..)
  or empty; unknown peer allowed, never populates
  reads: `given_legacy_gatt_slot...`.
- Coherent `offset==0` stores snapshot; `offset!=0`
  with no peer returns None; `offset==len` returns
  empty and removes owner; `offset>len` returns
  None: `given_coherent_unprimed...`.
- Repeated offset zero for same peer removes
  old mapping/replaces it with newest; independent
  peer retains previous version, and terminal
  consumes the peer mapping: `given_coherent_peer_restart...`.
- `reads.len()==READ_LIMIT` pop oldest, exactly
  READ_LIMIT safe and READ_LIMIT+1 evicts, no cache
  growth with duplicate peer slot-zero:
  `given_coherent_slot_capacity...`.
- Handshake length 0..6, missing CONTROL, wrong CMD
  1..15, unsupported non-inclusive and inverted
  version ranges:
  `given_bad_weave_connection_requests...`.
- Valid min/max requested size below/equal/above
  negotiated endpoints and range including version 1:
  `given_valid_weave_version_ranges...`.
- Frame prefix length 0..3 -> incomplete; encoded
  zero/max+1/u32::MAX -> error; incomplete payload
  at 1/min/max:
  `given_bad_or_partial_weave_frame_lengths...`.
- Exactly framed 1-byte and 5MiB max -> first
  frame total; trailing second frame must not be
  swallowed/mis-sized; max+1 rejected at prefix:
  `given_complete_weave_frame_boundaries...`.

## Stage 5 — Positive/N1–N12 matrix (no empty cells)

Legend: `P` = new planned, `C` = existing,
`NA(reason)` = not applicable **to this particular
pure function**, not to whole product;
`S` = separate hardware/contract test, OPEN.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| GATT read-slot pure policy | C+P | P(bad/unknown offset) | P(0/len/len+1/MAX,31/32/33) | NA(no socket) | P(restart/replay) | P(multiple peers) | P(changed ad revision) | P(peer isolation) | NA(no clock/locale) | P(complete read) | P(no cross-peer bytes) | NA(no billing) | NA(no storage) |
| Weave request pure parser | C+P | P(short/invalid command/range) | P(0..6, min−1/max+1) | NA(no network I/O) | NA(no retries) | NA(pure) | P(invalid version bytes) | P(untrusted frame) | NA(no time) | NA(no UI) | NA(no payload logging) | NA(no billing) | NA(no storage) |
| Framed length pure gate | C+P | P(malformed/zero prefix) | P(0,1,MAX,MAX+1,MAX-u1) | NA(no I/O) | NA(no retries) | NA(pure) | P(truncated frame) | P(malicious length) | NA(no time) | NA(no UI) | NA(no logging) | NA(no billing) | NA(no storage) |
| Real BlueZ notifier & worker | S | S | S | S | S | S | S | S | NA(no currency/time math) | S | S | NA(no billing) | S(received FS after protocol) |

## Stage 6 — meaningful interactions

- Legacy/coherent × peer identity × (0, 1, length,
  length+1, MAX) × install old/new revision.
- Coherent × 31/32/33 simultaneous peers × same-peer
  refresh × pinned old bytes vs current.
- Handshake CONTROL flag × 16 CMDs × version range
  (valid/minimum/excluded/inverted) × packet size
  (0, 1, min−1, min, min+1, max−1, max, max+1, u16MAX).
- Length prefix bytes × incomplete/exact/extra-body
  × 1-byte and max-size payload.
- No fake BlueZ success or replacement of real peer
  with arbitrary stub. Hardware-dependent cross-products
  remain OPEN with documented limitation.

## Stage 7 — expected new counts by category

**Eight new test methods**; scenarios may be table-driven
within a method. Mapping is *overlapping* by applicable
category: Positive 4, N1 4, N2 6, N3 0 NA to pure
cases, N4 2, N5 2, N6 3, N7 6, N8 0
NA to pure cases, N9 2, N10 2, N11 0 NA to this
sharing scope, N12 0 NA to pure cases.
These are planned test-method category counts,
NOT separate unique method totals and NOT the
repository-wide D2 category counts.

## Stage 8 — per-production-file/function/branch map

| Production file | Function-level status | Covered branches after Stage 9? | Reason / remaining |
|---|---|---|---|
| `core_lib/src/hdl/gatt/slot.rs` | `new/install/read` deterministic decision inventory above | planned targeted complete branch exercising | Full BlueZ characteristic integration not proved by source unit tests |
| `core_lib/src/hdl/gatt.rs` | handshake, confirm, framed length, refresh, disconnect event | planned expanded negative and limit boundaries | `new/run/weave_session/run_inbound_worker` require further GATT/worker work, cannot mark fully covered |
| `core_lib/src/hdl/ble_monitor.rs` | static FE2C pattern covered | only pattern policy covered | `run` adapter API and monitor lifecycle depend on real BlueZ |
| Rest of repository | **Not in this bounded implementation slice** | **global coverage NO** | Formal current full inventory still needs Stage 3–10 mapping |

## Stage 10 — mutation sensitivity review

An exact mutant that changes offset==0, removes
peer lookup, serves `self.current` instead of pinned,
changes `READ_LIMIT ==` to another threshold,
stops eviction, omits terminal removal,
accepts a command without CONTROL, ignores
invalid protocol range, clamps to wrong min/max,
swaps `>=` to `>` for frame-completion,
accepts zero/max+1, or consumes trailing frame
must fail an explicit case in the suite.
Mutation operators are reviewed, but **a mutation
engine has not been run**; do not label that
a measured mutation score.

## Milestones and rollback

1. [x] Scan SSOT and P0 GATT/BlueZ framing and
   read-slot production files; register all target branches
   and every N-category before code.
2. [x] Implement eight new table-driven, deterministic
   tests only; never edit runtime/authorization
   and never relax existing assertions.
3. [x] Verify exact-source Preflight Core, separate
   GTK only if triggered, analyze each failure
   (test fixture error vs product defect).
4. [x] Update per-category actual counts and
   runtime evidence. Do not launch a new expensive
   strict full 10× just for unchanged production
   unless Stage 11 revalidation after tests is needed.
5. [ ] Reconcile global Completion Gate NO and
   next P0 deterministic/physical gaps.
Rollback is the scoped test-only diff and this
ledger; existing tests, prod, installed KDE,
`master`, secrets and packages unchanged.

## Stage 9 implementation checkpoint — before CI verdict

Committed two atomic, production-source-free test changes:

- `core_lib/src/hdl/gatt/slot.rs` at `0b17a2f8`:
  four original unit methods cover legacy no-cache
  reads, coherent unprimed/empty reads, per-peer
  advertising epochs, bounded 32-peer eviction
  with same-peer refresh. The original two
  positive tests remain.
- `core_lib/src/hdl/gatt.rs` at `6222c70d`:
  four original unit methods with exhaustive
  small-input tables: 0..6 handshake prefixes,
  all 15 nonzero control commands, missing
  CONTROL bit, unsupported/inverted version
  ranges, accepted version-range cross products,
  negotiation min/max -1/+1, 0/u16::MAX,
  framed prefixes 0..3, zero/overmax/u32::MAX,
  truncated header/body and exact 1-byte/5MiB
  framed payloads including adjacent second
  frames.

No production functions, API contracts,
BlueZ setup/Wayland host or application
behavior changed. The tests assert exact
old/new revision bytes, cache ownership,
read termination, message sizes and invalid
input errors. The 5MiB case allocates only
local process memory and has no real
network side effect.

GitHub automatically started exact-source
[Preflight 39080885453](https://github.com/morozhenko-apps/rquickshare/actions/runs/39080885453)
and
[GTK verification 39080885432](https://github.com/morozhenko-apps/rquickshare/actions/runs/39080885432).
**At this checkpoint both jobs are running.**
Do not count the new eight methods as
verified until the same-source core test
logs are read and the full 5-job Preflight
is green. The independent strict full
10× run 39078162262 was from an earlier
source and is not evidence for these
eight newly added tests.

### Actual newly introduced test-category assignments

8 new methods, overlapping category assignments:
Positive **4** (both slot policies, handshake
accepted range, valid frames);
N1 **4** (unknown offsets, header/command,
malformed lengths, unprimed slot),
N2 **6**, N3 **0/NA** (pure computation),
N4 **2** (restart and consumed-owner replay),
N5 **2** (peer isolation and bounded concurrent
slots), N6 **3** (advertising revision,
unsupported version, corrupt/truncated
length), N7 **6** (all slot peer boundary
and frame inputs), N8 **0/NA** (no clock/locale),
N9 **2** (completed read/fragment interruption),
N10 **2** (no cross-peer data; no byte content
in production logs), N11 **0/NA** (no entitlements),
N12 **0/NA** (pure functions, no persistence).
These are audit *assignments*, not executed
mutation kills or global D2 counts.

### Testability gap exposed by Stage 8

The GATT runtime's `weave_session` takes a
concrete `bluer::gatt::local::CharacteristicNotifier`,
and `ReceiverGattServer::run` establishes a
live `bluer::Session`. Negative lifecycle,
read/notify fragmentation, channel closure,
reconnection and subscription conflicts
cannot be exhaustively reproduced through
the pure helper unit tests. This is a P0
**testability defect**. A future MODE A
refactor proposal should expose a minimal
transport/notification adapter for contracted
fake peer testing while preserving all
production behavior and public wire format.
Risk: high (BlueZ lifecycle, cancellation,
bounded write queue); compatibility: must
remain unchanged. Defer actual refactor
until approved architecture/security choice
and maintain real hardware smoke gate.

**Global Stage 12 remains NO.**

## Verified P0 execution result (same-source CI)

Final test-source `23b71de30eefddba46cc6ce4615923203ac65fba`
ran ordinary
[Preflight 39080989423](https://github.com/morozhenko-apps/rquickshare/actions/runs/39080989423)
and
[Native GTK 39080989573](https://github.com/morozhenko-apps/rquickshare/actions/runs/39080989573).

GitHub Core Rust job `117537738242`: **SUCCESS**. Its
full log explicitly shows **all eight new exact-named
GATT methods `ok`**, 710/0 stable library cases,
4+4/0 integration tests, and 10 numbered
race-sensitive attempts totalling 80/0
single-case passes. The format and Python
contract job, frontend and strict Core Clippy
are also SUCCESS, and native GTK 52-case
verification is SUCCESS. Legacy Tauri Clippy
was still executing when the first evidence
was recorded; full 5/5 Preflight verdict
must be checked independently.

First test-source `6222c70d` failed
`cargo fmt --check` due only to new
test line wrapping. Commits `f613db4e`
(slot) and `23b71de3` (Weave)
conformed to the exact rustfmt diff.
This is formatting-only test code,
not a protocol assertion being weakened.
The superseded initial CI was
cancelled after the new source arrived
and is not counted as successful.

Per-file scoped Stage 8 outcome:
`gatt/slot.rs::new/install/read`: all
planned deterministic branch cases
now have exercised tests. `gatt.rs`
handshake/length guard: all new
targeted negative/limit scenarios
are exercised. Real notifier
`weave_session`, `run_inbound_worker`,
BlueZ DeviceFound/register paths
remain globally UNTESTED for their
OS/branch combinations. No full
mutation runner was executed, and
Stage 12 global remains **NO**.
