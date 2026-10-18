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
2. [x] Implement all five planned test methods only.
3. [x] Verify rustfmt, Core Rust, Clippy and native GTK
   at the exact code SHA; record failures honestly.
4. [x] Update this ledger, `docs/mode-b-inventory.md`,
   `docs/current-state.md` and category D2.
5. [ ] Final full 10× Stage 11, all-production-branch Stage 12,
   actual BlueZ/Google Android/Windows/KDE acceptance **OPEN**.

Rollback only test and docs commits. Production behavior,
security policy, wire schema, installed package, credentials,
`master`, and release pipeline remain unchanged.

## Stage 9 implementation checkpoint

- Documentation-first scoped plan commit: `310d4ecd`.
- Two schedule lifecycle methods implemented (test-only)
  in `bc0ab2f7`.
- Three real FEF3 + mDNS encoding and BlueZ descriptor
  methods implemented (test-only) in `4a18f702`.
- First Preflight at `4a18f702` reported rustfmt
  formatting differences only. The seven exact source
  layout corrections were committed test-only as
  `38737fd7`; the test assumptions/assertions are unchanged.
- Exact-code [Preflight 39117187583](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117187583)
  and [Native GTK 39117187607](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117187607)
  are the run IDs to verify. Until the jobs complete,
  no new scoped PASS/coverage totals are claimed.
- Rollback is limited to `core_lib/src/hdl/blea.rs`
  test modules and this documentation. Live BlueZ
  advertising, JNI/Windows/KDE receivers and cryptographic
  BWU [Issue #8](https://github.com/morozhenko-apps/rquickshare/issues/8)
  remain unverified, and global Stage 12 is NO.

## Red CI triage before test correction

Exact-source [Preflight 39117187583](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117187583)
at `38737fd7` reported success for rustfmt/frontend,
but Core and Core Clippy both failed to **compile the
new test module**. The complete job logs show five
`E0433: cannot find type DeviceType in this scope`
at the new wire-contract cases in
`core_lib/src/hdl/blea.rs`. Candidate causes checked:
(1) missing test-only import; (2) wrong duplicate enum from
securegcm; (3) incorrect utility module reference;
(4) disabled feature gate; (5) changed production
type/trait API; (6) stale generation/toolchain mismatch.
The compiler's exact locations and suggested
`use crate::DeviceType` plus repository's public
re-export support **(1)** as root cause. Add exactly
that import **inside the test module**, not production.

Additionally, remove the vacuous assertion that a
`u8`-decoded length is <= 255; precise two-sided
frame-length equality and UTF-8/name checks already
enforce the real invariant. No assertion on the real
on-wire length/content may be relaxed. No production
refactor or wire-format change is authorized.
Re-run format, Core, Clippy, GTK on the correction SHA;
until then do not count any new method as PASS.

## Import fix and exact-source CI retry

Test-only correction `6401c98a452fb0283ccfe3c4949e2c6524c010b4`
imports the canonical `crate::DeviceType` **only inside the
existing Rust test module**, and drops the vacuous assertion on
a value already decoded as `u8`. Independent asserted
connection/endpoint-info lengths and exact name bytes remain.
No product source/runtime differences.

[Retry Preflight 39117363576](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117363576)
and [Native GTK 39117363572](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117363572)
must be checked after completion; previous failed
compilation is explicitly retained as evidence and is
not counted as a passing test iteration.

## Verified first green source and mutation-tightening

At test source `6401c98a`,
[Preflight 39117363576](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117363576)
**completed SUCCESS**. Inspected actual Core job `117645757985`:
**736 stable Core tests passed, 0 failed, 0 ignored**;
two integration bins **4/4 + 4/4**; each of the five new
method names occurs exactly once as `... ok`.
The same Preflight also completed its Core Clippy,
Rust format, frontend and desktop Tauri jobs successfully.
Independent [Native GTK 39117363572](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117363572)
completed **SUCCESS**. This was one complete *quick*
source validation, not the strict 10-pass release gate.

Stage 10 review identified one missing mutation killer:
the original direct service-data matrix used only the legal
Laptop type, so replacing the three-bit `& 7` mask
might not fail any new assertion. Added seven explicit
raw-device-type boundary cases `{0,1,2,3,7,8,255}`
to the **existing first method** (not a sixth method);
test-only commits `9b746c51`, `385674f6`.
The strengthened source must be verified by
[Preflight 39117611694](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117611694)
or a newer exact-SHA run if superseded,
rather than reusing the previous green receipt.
The added mask assertions check the actual single-byte
BLE endpoint-information field. No runtime code change.

## Verified final strengthened test source and scoped D1–D5

At exact code SHA `385674f6b1267af3f0b117af773c4c8348f37826`,
[Preflight 39117637254](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117637254),
Core Rust job `117646604296`, full job log inspected:

- **736 stable Core methods passed / 0 failed / 0 ignored**;
  eight separately scheduled race-sensitive methods
  were filtered by the existing quick CI policy.
- The other Core integration binaries each report
  **4/4 passed**.
- Exactly **five** new test method names each appear
  once with `... ok`, including the strengthened
  device-mask matrix (7 extra raw type inputs).
- Format, frontend and Core Clippy jobs **SUCCESS**.
  Desktop Tauri Clippy/unit job was independently
  verified **SUCCESS**; its actual Rust log reports
  **10/10 passed, 0 failed/ignored**. The complete
  quick Preflight finished **SUCCESS, 5/5 jobs**.
- Independent [Native GTK 39117637258](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117637258)
  **SUCCESS**, and the actual GTK job `117646561562`
  reports **52/52 passed, 0 failed/ignored**.

### D2 category assignment (method counts, overlapping)

Five new original methods, executed once each under the
same-source quick Core job. Counts below describe **methods
mapped to categories**, not unique mutually exclusive
tests or automatic Rust branch coverage.

| Category | New methods | Actual scoped evidence |
|---|---:|---|
| Positive | 5 | PASS: correct FEF3, mDNS, schedule, BlueZ packet descriptor |
| N1 validation | 3 | PASS: overlong and UTF-8 input, allowed names, arbitrary headers |
| N2 boundaries | 5 | PASS: names 0/1/236/237/238/255/256+, raw types 0/1/2/3/7/8/255, 2/5/30/120 s, 31-byte budget |
| N3 network failure | 0 | X pure helpers; live BlueZ and mDNS discovery remain OPEN |
| N4 repeated events | 4 | PASS: first deferred deadline, explicit restart, deterministic wire rebuild |
| N5 races | 2 | PASS: overdue/repeated scheduler triggers and stop/restart epochs (not real BlueZ races) |
| N6 corrupt/outdated data | 2 | PASS: oversized UTF-8 normalized; opaque header preserved |
| N7 security/ownership | 3 | PASS: exact endpoint ID, masked device type and bounded wire |
| N8 locale/time | 3 | PASS: multi-byte UTF-8 and monotonic fake-clock deadlines; DST not applicable |
| N9 interruption | 2 | PASS: stopped/restarted schedule, repeated refresh burst |
| N10 data disclosure | 3 | PASS: bounded endpoint-name fields, exact name and opaque bytes |
| N11 entitlements | 0 | X no billing/subscription |
| N12 storage | 0 | X no filesystem |

### D3 implementation and mutation sensitivity

Files changed: only `core_lib/src/hdl/blea.rs` **test
modules**, this scoped ledger and the two global
documentation indexes. No runtime code, build flags,
mock production domain, or additional SDK dependencies.
The 17 direct wire-boundary subcases, 80 cross-media
identity/type/name combinations, three interval/epoch
cases each and three opaque headers form the planned
5 distinct method groups. The assertions are
independent of the values of secure random bytes.

Manual mutation review: truncating a name at a non-UTF-8
boundary, dropping reserved bytes, modifying the 4-byte
endpoint, changing the advertised/announced length,
unmasking/reducing the raw type field, extending the
first deferred deadline after a repeat, replaying
deadlines after stop, or mutating the packet
service/intervals/headers would fail specific
assertions. No automated mutation engine run is
claimed.

### D4/D5 boundaries and acceptance

No refactor was necessary for the above pure production
functions. A BlueZ `AdvertisementHandle`/
`CharacteristicNotifier` contract seam is still
needed to automate the actual D-Bus registration,
subscribe/stop/error/reassembly path, but it changes
injection architecture; approval under MODE A is
required. Google Android, Windows Quick Share and
KDE/Wayland hardware interoperability are unverified.

The five new methods are proven passing in one
exact-source quick Core run. **Do not claim
0 flakes from ten consecutive runs**: full strict
10x is currently release-tag-gated, and no new
version tag or publication is authorized.
Repository-wide Stage 11 for this final source and
Stage 12 (all public methods/branches/N1–N12)
both remain **NO**. Issue #8 authenticated BWU
alias ownership is independent and OPEN.

## Final exact-source quick CI closure

[Preflight 39117637254](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117637254)
and [Native GTK 39117637258](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117637258)
**both completed SUCCESS** at the same source SHA
`385674f6b1267af3f0b117af773c4c8348f37826`.
Preflight jobs: **5/5 SUCCESS**; stable Core
**736/736**, integration bins **4+4**, desktop
Tauri **10/10**; independent GTK **52/52**.
Each of five new Rust methods was verified by exact
`... ok` entries in the Core job log. The full
10× Mode B and global Stage 12 remain **NO**.
