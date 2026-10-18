# MODE B P0 receiver discovery advertisement and recovery invariants

Date: 2026-10-18. Primary mode: **MODE B (Testing)**. Target branch:
`dev`, test-only source scope `core_lib/src/hdl/blea.rs`. This is a
bounded Stage 1–10 slice; global Stage 12 stays **NO**. Never change the
installed package, production BLE behavior, public wire protocol,
`master`, releases, hardware settings, or credentials.

## Stage 1: repository scan and baseline

Authoritative SSOT: user 378-line `agent.md`; read
`docs/mode-b-completion-audit-2026-10-18.md`,
`docs/native-essential-priorities.md`,
`docs/mode-b-p0-gatt-slot-framing-2026-10-18.md`,
`core_lib/src/hdl/blea.rs`, `core_lib/src/hdl/receiver_header.rs`,
`core_lib/src/hdl/gatt.rs`, `core_lib/src/hdl/mdns.rs`,
`core_lib/src/utils.rs`, `core_lib/src/identity.rs`, and
`.github/workflows/lint.yml`.
Rust in-crate `#[test]` and `#[tokio::test(start_paused=true)]`
are the current test harness. Quick `Preflight` runs stable Core once
and excludes the eight separately grouped race probes. The strict
full 10-run suite is release-tag gated.

Already covered: basic FEF3 service-data encoding, ASCII 300-byte and
UTF-8 200-character truncation, packet UUID/layout/budget, visibility
checks, debug 5..120 second refresh override and one continuous-refresh
non-sliding deadline test. These are not duplicated merely for count.

## Stage 2: P0 risks and invariant analysis

Phone→desktop discovery depends on a consistent 4-byte endpoint ID
in the FEF3 GATT connection-advertisement **and** the mDNS service
name, correct type/name fields, a bounded one-byte endpoint-info
size and strict UTF-8 truncation. A corrupted byte length may
make Google Android reject the receiver. Each rebuild may use fresh
random identity and device token; tests MUST NOT assume fixed values
or assert random collisions never happen.

The receiver's periodic 30 s safety refresh and first deferred
2 s recovery must not drift on repeated GATT reads. Immediate
stop/restart, visibility and missing header must prevent stale
deadline reuse. Validate only the *actual* pure scheduler and
packet construction, not simulate the BlueZ event loop or pretend
an advertised packet was accepted by a physical peer.

Protocol-compatible service names come from
`crate::utils::gen_mdns_name`; type/name TXT records come from
`gen_mdns_endpoint_info`/`parse_mdns_endpoint_info`.
Public identity names are bounded by `MAX_NAME_BYTES=237`.
For a caller-supplied overlong string, BLE specifically truncates
at a valid codepoint; mDNS has a larger 255-byte budget, so only
**valid identity-domain names** are asserted equal across media.

## Stage 3: exact bounded artifact inventory

| Production artifact | Risk/level | Planned executable tests |
|---|---|---|
| `receiver_service_data` endpoint ID, FEF3 hash, 0x23 connection code, two length fields, reserved padding, token budget, bytewise name bounds | P0 unit/protocol | 1 table-driven test; empty/1/236/237/238/255/256+ ASCII, UTF-8 crossing cap; endpoints all zero/nonzero/all FF |
| `receiver_service_data` with `gen_mdns_name` and `gen_mdns_endpoint_info` | P0 cross-medium contract | 1 table-driven test; 4 endpoints × legal names/type; actual independent mDNS decode + BLE decode |
| `ReceiverSchedule::{started,defer,stopped}` | P0 fake-clock schedule | 2 tests: first-only deferred/restarts/boundary and stop/restart stale deadline |
| `ReceiverAdvertiser::packet` | P0 BlueZ outgoing payload contract | 1 table-driven test: exact 17-byte opaque header, discovery/legacy flags, interval and 31-byte budget |
| `ReceiverAdvertiser::run`, actual `AdvertisementHandle`, BlueZ notifier, D-Bus registration | P0 hardware | OPEN, not mocked here; needs explicit MODE A approval for minimal transport seam + physical tests |

Exactly **5 new methods**. No testing-only production hooks, dependency
injection, mocks or wall-clock sleeps. Existing tests retained.

## Stage 4: branch map

1. `receiver_service_data`: name length 0, 1, max−1 (236),
max (237), max+1 (238), 255, 256+; valid boundary or split UTF-8;
device type masked with `&7`; 4-byte endpoint preserved; calculated
connection length and endpoint-info length; random fields checked for
position/length only: `given_receiver_service_data_name_boundaries_when_encoded_then_all_lengths_and_codepoints_remain_valid`.
2. Cross-medium service identity for allowed identity inputs, distinct
endpoint IDs, exact mDNS binary `0x23 + id + 0xFC9F5E + 0x0000`,
decoded TXT type/name and FEF3 payload: `given_same_receiver_identity_when_encoded_for_ble_and_mdns_then_endpoint_and_name_agree`.
3. Deferred schedule first-wins at 2s, repeat at 1s/2s/31s does
not extend; periodic is independent and debug-allowed 5/30/120 s
deadlines: `given_repeated_gatt_refreshes_when_scheduled_then_first_deferred_and_periodic_deadlines_stay_fixed`.
4. Stop clears all pending timers, later start uses a fresh clock,
deferred is reset and no prior deadline leaks across multiple
active epochs: `given_advertising_stop_and_restart_when_scheduled_then_old_deadlines_never_reappear`.
5. `ReceiverAdvertiser::packet` exact supported 0xFEF3 UUID,
opaque payload unchanged, discoverable peripheral,
100..150 ms connection intervals and legacy 31-byte budget:
`given_legacy_receiver_headers_when_advertisement_is_built_then_blueZ_contract_and_budget_are_preserved`.

## Stage 5: complete Positive/N1–N12 applicability matrix

C = already covered; P = planned; X = not applicable **with reason**;
OPEN = another required production boundary.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| FEF3 wire encoding | C+P | P(name UTF-8) | P(0/1/236/237/238/255+) | X(pure) | P(rebuild contract) | X(pure) | P(overlong normalized) | P(length and ID isolation) | X(no locale/time input) | X(no interactive UI) | P(no extra name bytes) | X(no entitlements) | X(no FS) |
| BLE + mDNS identity | C+P | P(legal names) | P(0/237 names) | X(pure) | P(same endpoint across rebuild) | X(pure) | X(no stale cache) | P(no cross-peer ID confusion) | P(Unicode, locale independent) | X | P(same published name only) | X | X |
| First deferred schedule | C+P | X(typed inputs) | P(2 s and 5/30/120 s) | X(no network) | P(repeated triggers) | P(deadlines) | X(no bytes) | X(no permissions) | P(monotonic time; no DST) | P(refresh bursts) | X(no PII) | X | X |
| Stop/restart deadlines | C+P | X(typed inputs) | P(multiple epochs) | X | P(stale deadline not replayed) | P(stop/restart transitions) | X | X | P(monotonic clock) | P(stop) | X | X | X |
| Legacy packet contract | C+P | P(opaque byte variations) | P(31 byte budget) | X(no BlueZ) | X(pure) | X(pure) | P(unknown header bits copied) | P(exact bytes) | X | X | P(no secret expansion) | X | X |
| Live BlueZ registration/notify | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | X | OPEN |

## Stage 6: interaction matrix

Endpoint `{0x00000000, ASCII, mixed byte, 0xFFFFFFFF}`
× identity length `{0,1,236,237,238,255,256+, multi-byte}`
× device type `{Unknown, Phone, Tablet, Laptop}`
× medium `{BLE, mDNS}`, with the cross-medium equality
constraint only for accepted identity-domain names.
Schedule `{deferred early/repeated/overdue, periodic,
stopped, restarted}` × `{5,30,120 seconds}`; combine
only meaningful independent stages, no combinatorial
hardware simulation. Packet headers `{zeros, patterned,
maximal bytes}` preserve their exact 17-byte content.

## Stage 7: coverage estimate

Exactly 5 unique methods; table rows evaluated inside the methods.
Overlapping method attribution (not sums of independent test methods):
Positive 5, N1 3, N2 5, N3 0 X, N4 4, N5 2,
N6 2, N7 3, N8 3, N9 2, N10 3, N11 0 X, N12 0 X.
Report actual exact-source CI in Stage 11.

## Stage 8: per-production-file coverage map

`core_lib/src/hdl/blea.rs`: add branch/contract assertions for
`receiver_service_data`, `ReceiverSchedule::started/defer/stopped`,
`ReceiverAdvertiser::packet`. Existing
`ReceiverAdvertiser::should_refresh`,
`receiver_periodic_refresh_secs_for`, and visibility tests
remain. **OPEN**: actual `ReceiverAdvertiser::new/run`
BlueZ/D-Bus allocation, stop/start ownership and dropped
advertisement handle; `BleAdvertiser::new/run` hardware.
`core_lib/src/utils.rs`: call real mDNS encoder/parser,
but do not claim all their branches are exercised.
`core_lib/src/hdl/gatt.rs`: GATT slot0 read notification
triggers this scheduler; actual D-Bus read path remains OPEN.
All other source modules require the repository-wide Stage 8 map.

## Stage 10: mutation review (planned)

Bit-flipping endpoint IDs, losing bytes from UTF-8 boundary,
changing `min` to `max` or `<` to `<=`, truncating
random byte fields, changing connection length/mDNS
service hash, deferring on every refresh, retaining a
stale deadline after stop or modifying the 17-byte
header must fail an exact assertion. Manual mutant
review is not an executed mutation engine.

## Stages, rollback, and acceptance gate

1. [x] Stage 1–8 scanned and documented BEFORE test changes.
2. [ ] Implement all five planned test methods only.
3. [ ] Verify rustfmt, Core Rust, Clippy and native GTK
   at the exact code SHA; record failures honestly.
4. [ ] Update this ledger, `docs/mode-b-inventory.md`,
   `docs/current-state.md` and category D2.
5. [ ] Final full 10× Stage 11, all-production-branch Stage 12,
   actual BlueZ/Google Android/Windows/KDE acceptance **OPEN**.

Rollback only test and docs commits. Production behavior,
security policy, wire schema, installed package, credentials,
`master`, and release pipeline remain unchanged.
