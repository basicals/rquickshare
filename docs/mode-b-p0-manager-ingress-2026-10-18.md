# MODE B P0 TCP ingress classifier: fragmented prefaces and competing owners

Date: 2026-10-18. Mode: **MODE B (Testing)**. Scope: test-only
`core_lib/src/manager.rs` / `route_bandwidth_upgrade_if_pending` and
actual `BwuRouter` from `core_lib/src/hdl/bwu.rs`. Branch: `dev`.
No production routing/authentication change, OS services, installation,
`master`, release tags, cloud storage or peer hardware action.

## Stage 1 — repository scan

Binding guides: project universal `agent.md` plus
`docs/mode-b-test-plan.md`, `docs/mode-b-completion-audit-2026-10-18.md`,
`docs/mode-b-p0-client-introduction-2026-10-18.md`,
`docs/mode-b-p0-bwu-route-2026-10-18.md`,
and `docs/native-essential-priorities.md`.
Read `core_lib/src/manager.rs` TCP ingress classifier and its original
five BWU tests, `core_lib/src/hdl/inbound.rs` peek parser and validator,
`core_lib/src/hdl/bwu.rs` real-socket router.
Stable Core Rust test suites run via quick `Preflight` in GitHub.
Full repeated MODE B is governed by current release-tag CI; do not
re-enable parallel ten-pass workflows on routine `dev` pushes.

## Stage 2 — risks / invariants

P0: a concurrent pending BWU route must not capture unrelated ordinary
Quick Share ingress. Partial TCP prefixes/bodies must be non-consuming;
a fully delivered legitimate introduction within the classifier window
must be routed with exactly the same bytes available to its owner.
Routing a current endpoint must not steal the previous endpoint's
pending socket when both exist. Pending owners must remain after
non-BWU traffic, including zero/oversized or malformed frames.
Network delays beyond the current product timeout and authenticated
peer alias proof are separate known gaps, NOT assumed solved.

Production `route_bandwidth_upgrade_if_pending` currently uses
an internal 750 ms deadline with 5 ms polling for incomplete
prefaces. Tests will synchronize with direct `futures::poll!` and
real local sockets, not add sleeps or infer timing-sensitive exact
counts. Test scope must not silently change this product behavior.

## Stage 3 — scoped exhaustive testable inventory

| Artifact / path | Priority, test level | New methods planned |
|---|---|---:|
| `route_bandwidth_upgrade_if_pending`: partial prefix 1..3 and partial body at multiple split offsets -> eventual correct route | P0 integration / real loopback | 1 table-driven |
| Same classifier: ordinary frame, malformed protobuf body, length zero, > 8KiB classifier budget -> original stream returned and unrelated router registration untouched | P0 integration / real loopback | 1 table-driven |
| Classifier + live `BwuRouter`: simultaneous exact current/previous alias subscriptions and connections -> exact ID wins, no cross-peer bytes, independent one-shot consumption | P0 integration / real loopback | 1 |
| Valid/invalid protobuf parser itself | Previous 3 focused unit methods | Existing |
| `BwuRouter` register/alias/cancel/replay byte ownership | Previous 15 test methods | Existing |
| Crypto authenticated alias, live BlueZ/GATT, notifier and installed Android/Windows | OS/protocol security | OPEN, cannot claim from loopback |

Three new methods, independent inputs exercised via explicit in-method
table loops. Test real TCP sockets, protobuf frames, oneshot receivers;
no mock domain, real external LAN or time sleeps.

## Stage 4 — executable branch map

- `socket.peek` returns incomplete length/header/body, later valid
  complete bytes -> `peek_client_introduction` identifies owner and
  `route_with_alias` transfers exact stream without consuming header:
  `given_segmented_bwu_tcp_introduction_when_full_frame_arrives_then_original_stream_routes_without_byte_loss`.
- `frame_len == 0`, `frame_len > BWU_PEEK_LIMIT - 4`,
  complete invalid protobuf, and complete ordinary Quick Share frame
  -> `Ok(Some(original_socket))` and pending registration unchanged:
  `given_non_bwu_tcp_ingress_when_classifier_checks_it_then_socket_and_live_bwu_owner_remain_intact`.
- simultaneous current and previous pending owner, introduction
  current with previous alias, plus separate introduction for previous:
  two valid `route_with_alias` results, no stolen bytes or live entries:
  `given_two_concurrent_bwu_tcp_peers_when_exact_and_previous_routes_compete_then_no_owner_or_bytes_mix`.

Zero-byte EOF, socket read errors, unknown alias with no owner,
timeout with no new data and cancellation during classification
already have lower-level branches or remain separate full-scope gaps.
Do not claim an end-to-end auth guarantee.

## Stage 5 — Positive / N1–N12

C = existing scoped tests; P = planned; X = inapplicable with reason;
OPEN = different real implementation/hardware boundary still pending.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Fragmented TCP ingress | C+P | P(partial prefix) | P(1..3 and body splits) | P(segmentation) | X(no retry) | P(in-flight read) | X(valid frame) | P(right owner only) | X(no clock math) | P(fragmented arrival) | P(no byte spill) | X(no billing) | X(no disk) |
| Ordinary or invalid traffic | C+P | P(bad protobuf) | P(0/>8192 header) | P(real socket preserved) | P(no consume) | P(pending another) | P(corrupt bytes) | P(no false route) | X | X(no UI) | P(no foreign admission) | X | X |
| Simultaneous exact + previous | C+P | X(valid IDs) | X(no numeric limit) | P(two actual sockets) | P(one-shot) | P(two peers) | X | P(exact priority) | X | X | P(no cross-peer) | X | X |
| Authenticated cross-transport handoff | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | X | OPEN | OPEN | X | OPEN |

## Stage 6 — interaction matrix

TCP framing {partial header / partial body / whole / corrupt /
ordinary / invalid numeric length} × router {one live / two live /
explicit competing alias} × result {return same stream / consume
correct socket / keep unrelated pending}. Every planned cell above
has deterministic socket content assertions. Real BLE credentials,
physical discoverability, genuine tcp-peek timeout and platform
shutdown are independent OPEN acceptance groups.

## Stage 7 — expected counts

3 new unique methods. Overlapping category method assignments
(not unique per-category totals): Positive 3; N1 2; N2 2;
N3 3; N4 2; N5 3; N6 1; N7 3; N8 0 X;
N9 1; N10 3; N11 0 X; N12 0 X.
Record actual CI method execution results separately.

## Stage 8 — production coverage map

`core_lib/src/manager.rs`: `route_bandwidth_upgrade_if_pending`
partial peek/read classifier; reject/return original; current-vs-alias
route during competing reservations and socket byte preservation.
Still OPEN: true network timeout/read failure, live multi-peer BlueZ,
uncaught other code branches, physical handoff and all repository-wide
executable method/branch inventory. `core_lib/src/hdl/bwu.rs` and
`core_lib/src/hdl/inbound.rs` already have focused unit/real-socket
coverage but no authentication guarantee. Global Stage 12 = NO.

## Stage 9–12 milestone plan and rollback

1. [x] Document source scan, scoped inventory, per-branch map,
   complete applicability matrix, interactions, expected counts and
   risks **before editing tests**.
2. [x] Implement exactly three new deterministic integration test
   methods against actual classifier and router in `manager.rs`.
3. [x] Confirm formatter, Core test execution and CI exact SHA.
4. [x] Review assertion-sensitive mutations, report each real pass
   and category mapping, update current-state and global inventory.
5. [ ] Global complete suite 10x/physical devices remains OPEN
   unless independently verified on the final source.

Mutation expectations: dropping incomplete bytes, early returning
to ordinary ingress, consuming other pending owner, selecting alias
before exact ID, consuming peek header, routing malformed protobuf or
losing unused socket should all fail an assertion. No automated
mutation engine is claimed.

Rollback is isolated `test:` commit plus this documentary ledger.
No production behavior, ABI, public security contract, release or
installed package change.

## Stage 9 implementation checkpoint (CI pending)

Test-only source `core_lib/src/manager.rs` updated at `5d48f850`
with three original scoped methods. Genuine localhost `TcpListener`/
`TcpStream`, real `BwuRouter` and generated protobuf are exercised.
Segmented payload method uses `futures::poll!` and condition-based
completion, no new test sleeps; competing two owners are classified
concurrently with `tokio::join!` and compared using actual peer
addresses and full original byte streams. All cases check final
pending state and byte ownership.

Original Preflight format job reported two rustfmt-only diffs;
test source formatting repaired by `811fd6ef` (assertions unchanged).
**Do not report success until exact-code Core logs pass.** No
production code changes. Mode B global Stage 12 remains NO.

## Verified source and execution

The repaired exact test SHA `811fd6ef5eaf49ee8a654c4842783a3526939d07`
triggered [Preflight 39092368844](https://github.com/morozhenko-apps/rquickshare/actions/runs/39092368844),
Core job `117571667714`: **SUCCESS**. Read the actual full
Core job log; each of the three new method names independently
appeared as `... ok` exactly once. Compiled stable suite:
**725 passed / 0 failed / 0 ignored**, plus two integration
binaries **4/4 + 4/4**. The separate 8 race-sensitive probes
are excluded from the stable count by current CI scheduling.
Rust format, Core Clippy and frontend jobs also completed
SUCCESS; legacy desktop Clippy was still in progress at the
first verified checkpoint, so do not claim the overall
5/5 workflow yet.

### D2 — actual scoped category assignment

**3 new unique methods**, not 3× category totals. Each has
one successful complete-method execution (with table-driven
input subcases); category assignments overlap:

| Category | New methods | Verified scope |
|---|---:|---|
| Positive | 3 | Valid full fragmented frame, preserved ordinary ingress, two accepted owners |
| N1 malformed input | 2 | Prefix/body partials, corrupt protobuf |
| N2 bounds | 2 | Splits + zero/over 8KiB classifier limit |
| N3 network | 3 | Real separate loopback TCP sockets |
| N4 replay/idempotency | 2 | Non-BWU does not consume live route, two independent one-shot routes |
| N5 concurrency | 3 | In-flight classification and two simultaneous owners |
| N6 corrupt protocol data | 1 | Malformed protobuf does not claim owner |
| N7 ownership/security | 3 | No route for ordinary/bad data; exact owner priority |
| N8 time/locale | 0 | N/A for scoped path: no locale/clock conversion |
| N9 interruption | 1 | Incomplete packet before completion |
| N10 privacy/cross-peer | 3 | Original bytes and socket addresses not mixed |
| N11 monetization | 0 | N/A: no entitlements |
| N12 storage | 0 | N/A: no filesystem |

### Mutation review

Observed full-wire and two-way ownership assertions would kill
mutants that consume the prefix while inspecting it, return an
incomplete introduction as routable, select prior alias over exact
current owner, route invalid protobuf, drop ordinary frames, or
consume unrelated pending registrations. This is a manually reviewed
mutation sensitivity map; **no executed mutation engine** is claimed.

### Open P0 and global gates

[GitHub Issue #8](https://github.com/morozhenko-apps/rquickshare/issues/8)
tracks the distinct **unauthenticated TCP route-consumption /
availability** risk (spoofed current/last endpoint strings before
a cryptographic channel binding). This observation **does not
establish encrypted payload decryption or forgery**. It needs
explicit MODE A security design approval; these tests do not
claim to fix it.

The product currently polls incomplete TCP peeks in a bounded
750 ms interval. Hostile network stalls, real BlueZ/GATT notifier,
authenticated BLE→TCP continuity, KDE/Android/Windows physical
acceptance and complete 10× full repository Mode B all remain
OPEN. **Stage 12 = NO**. No production code or installed package
changed in this test-only batch.
