# Mode B inventory, branch map, and test matrix


## 2026-10-18 P0 mDNS TXT parser and service identity contracts

Four new original tests in `core_lib/src/utils.rs`
and `core_lib/src/hdl/mdns.rs` cover malformed,
truncated and exact-length remote `n=` TXT;
compact/normal record formats, unknown device
normalization, valid UTF-8 and invalid encodings,
SDK ServiceInfo endpoint ID/fullname/port/name/type
cross-product and rename immutability.

The [scoped MODE B plan, Positive/N1–N12 matrix,
per-production-method branch map, mutation review
and D1–D5](mode-b-p0-mdns-service-txt-2026-10-18.md)
was recorded before code. Exact source `bd8c6b61`,
[Preflight 39118831405](https://github.com/morozhenko-apps/rquickshare/actions/runs/39118831405):
**740/740** stable Core passed, other
integrations **4/4 + 4/4**, all 4 new methods
separately `... ok`; formatter, frontend and Core
Clippy SUCCESS. [Native GTK 39118831372](https://github.com/morozhenko-apps/rquickshare/actions/runs/39118831372)
**SUCCESS 52/52**. Desktop Tauri Clippy/unit
**SUCCESS 10/10**, verified in its job log;
the complete same-source quick Preflight
**5/5 SUCCESS**. An earlier test fixture failed to
compile due to Rust cast/shift precedence; corrected
without changing test intentions or production code.

Actual `MDnsDiscovery::run` event handling,
cache, TCP reachability and physical interoperability
remain **UNTESTED** without a significant SDK seam.
[Issue #9](https://github.com/morozhenko-apps/rquickshare/issues/9)
holds the MODE A options. [Issue #8](https://github.com/morozhenko-apps/rquickshare/issues/8)
holds authenticated TCP BWU route ownership. Strict
final-source ten-run release gate and complete
all-branch Stage 12 stay **NO**.


## 2026-10-18 P0 receiver BLE FEF3 advertising and recovery contract

Five new original `core_lib/src/hdl/blea.rs` test methods
cover 17 direct service-data boundary/mask subcases, 80
cross-medium endpoint/device/identity combinations,
three periodic/deferred scheduling intervals, three
stop/restart epochs and three raw legacy-header
packet variants. Exact code SHA `385674f6` verified
[Preflight 39117637254](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117637254)
Core **736/736**, separate integrations **4/4+4/4**
and every new test independently `... ok`.
[Native GTK 39117637258](https://github.com/morozhenko-apps/rquickshare/actions/runs/39117637258)
**SUCCESS**, actual GTK log **52/52**. Format, Core
Clippy and frontend SUCCESS. Desktop Tauri Clippy
and unit tests **SUCCESS, 10/10**, completing the
quick Preflight **5/5 SUCCESS**. This is **one**
fast CI pass, not the mandatory 10-run complete
repository validation.

The binding
[Stage 1–10 inventory, Positive/N1–N12 matrix, interaction
and production function coverage map](mode-b-p0-receiver-discovery-advertisement-2026-10-18.md)
was committed before tests. The note includes the
red rustfmt and missing test-only import diagnosis.
This slice explicitly does NOT prove actual
`ReceiverAdvertiser::run` BlueZ/D-Bus lifetime,
notification stop/retry, Pixel/Windows compatibility,
authenticated BWU [Issue #8](https://github.com/morozhenko-apps/rquickshare/issues/8),
or global Stage 12. All remain **OPEN**.


## 2026-10-18 P0 managed GATT partial-frame failure and cancellation

Three new original test-only `run_inbound_worker` methods in
`core_lib/src/hdl/gatt/managed_tests.rs` cover 13 table-driven
variants: Initial native malformed packets, progressed EOF
inside the frame prefix/body, and parent/session cancellation
during an in-flight incomplete read. The scoped
[Stage 1–10 inventory, positive/N1–N12 matrix, interaction
and per-production-function branch map](mode-b-p0-gatt-partial-frame-2026-10-18.md)
was recorded before implementation.
Actual exact-SHA `86ebe678`
[Preflight 39116064673](https://github.com/morozhenko-apps/rquickshare/actions/runs/39116064673)
Core log verified all 3 new methods independently `ok`,
**731/731** stable Core tests, two **4/4** integrations,
zero failed/ignored. Separate [Native GTK 39116064680](https://github.com/morozhenko-apps/rquickshare/actions/runs/39116064680)
completed **SUCCESS, 52/52 GTK methods**.
The same-source Preflight completed **5/5 SUCCESS**,
including desktop Tauri unit tests **10/10**.
This evidence does not exercise
actual BlueZ GATT notifier, authenticated BWU, release-only
full 10× or the still-incomplete global Stage 12.


## 2026-10-18 P0 partial TCP preface timeout, early EOF and cancellation

Three new original `core_lib/src/manager.rs` real TCP methods
cover six prefix/body splits against the 750 ms fake-clock
deadline, three peer half-close EOF variants, and cancelled
classifier followed by a fresh accepted socket to the same
pending owner. Their source-defined
[Stages 1–10, positive/N1–N12, per-function branch and D2 map](mode-b-p0-classifier-timeout-cancellation-2026-10-18.md)
is included in the scoped ledger. Verified
[Preflight 39094606972](https://github.com/morozhenko-apps/rquickshare/actions/runs/39094606972)
Core **728 passed / 0 failed / 0 ignored**, integration **4+4**,
and each new method separately reported `ok` at
exact test SHA `c8753010`. The full ordinary Preflight
finished **5/5 SUCCESS** (including Tauri 10/10), and
[Native GTK UI 39094606916](https://github.com/morozhenko-apps/rquickshare/actions/runs/39094606916)
finished **SUCCESS with 52/52 tests**. Earlier test-only
red states and Linux RST behavior are documented,
not silently excluded from evidence.
Full repo Stage 11 ten-pass after all tests,
true BlueZ and authenticated BWU [Issue #8](https://github.com/morozhenko-apps/rquickshare/issues/8)
and global Stage 12 **remain OPEN**.


## 2026-10-18 P0 segmented TCP ingress and simultaneous BWU owner routing

`core_lib/src/manager.rs::route_bandwidth_upgrade_if_pending`
now has three additional test-only, real-localhost methods with
seven fragmentation positions, invalid/ordinary wire input cases,
and simultaneous exact-vs-previous endpoint owners. The exact
[scoped Stage 1–10 plan, branch/interaction/N1–N12 map and D2 counts](mode-b-p0-manager-ingress-2026-10-18.md)
was documented before implementation. Exact-source
[Preflight 39092368844](https://github.com/morozhenko-apps/rquickshare/actions/runs/39092368844)
Core job reports each new test method `ok` and
stable Core **725 passed / 0 failed / 0 ignored**
plus existing 4+4 integration. The complete ten-run and full
per-production-file Stage 12 remain unclosed. [Issue #8](https://github.com/morozhenko-apps/rquickshare/issues/8)
separately tracks the authenticated BWU TCP admission/ownership
boundary; parser correctness does **not** prove that boundary.


## 2026-10-18 P0 TCP CLIENT_INTRODUCTION parser negative cases

The pure `core_lib/src/hdl/inbound.rs` BWU introduction scope now
has three new exact-source tested methods: malformed/missing protobuf
fields, the four-byte TCP prefix and complete-first-frame contract,
and exact current/previous endpoint identity string comparisons.
See [scoped Stage 1–10 plan, Positive/N1–N12 and file/function branch map](mode-b-p0-client-introduction-2026-10-18.md).
Actual independent [CI 39085557977](https://github.com/morozhenko-apps/rquickshare/actions/runs/39085557977)
completed **SUCCESS** at `216d0fac`: 3 test methods × 10 attempts =
**30/30 passed**, no ignored or failed tests; test inventory and each
method's ten successful results verified from complete job logs.
Only the declared pure-parser branches are within this closure.
Plaintext `last_endpoint_id` is NOT authenticated ownership,
real encrypted BLE→TCP/mDNS/BlueZ handoff and all global Stage 12
requirements remain **OPEN**. No total-coverage claim is made.



## 2026-10-18 P0 BWU actual TCP socket ownership test closure

See the binding scoped
[BWU owner, alias, replay and cancellation
negative matrix](mode-b-p0-bwu-route-2026-10-18.md).
Test code `core_lib/src/hdl/bwu.rs`
was expanded by four original source-only
methods; exact CI
[P0 BWU 39084212567](https://github.com/morozhenko-apps/rquickshare/actions/runs/39084212567)
completed **SUCCESS**. Compiled method
registry has exactly 15 router tests
(11 preexisting plus four new), and
logs show ten numbered consecutive
`15 passed / 0 failed / 0 ignored`
runs, totaling **150/150** real tests.
Each new method independently appears
`ok` ten times.

| Production branch/interaction | Evidence | Status |
|---|---|---|
| Unknown/untrusted `endpoint_id` and unregistered alias | Real returned usable TCP socket; two other owners not consumed | **C — scoped 10×** |
| Both exact + alias live and conflicting | Exact wins; old alias remains routable; real peer bytes never cross | **C — scoped 10×** |
| Replayed/late second route for consumed registration | Original TCP returned and still readable; only first accepted | **C — scoped 10×** |
| Cancel old session then reuse same logical endpoint | Old receiver closes; late socket rejected; newly registered owner gets only new socket | **C — scoped 10×** |
| Upstream `last_endpoint_id` authenticity | Router does NOT validate credential/identity; contract must be established at caller | **OPEN** |
| GATT characteristic subscribe/notify error, multi-packet corruption | Requires actual BlueZ adapter or approved contract seam | **OPEN** |
| Full encrypted BLE→TCP migration, Google Android/Windows interoperability | Requires final protocol+hardware evidence | **OPEN** |

Coverage category assignments:
Positive + N1/N3/N4/N5/N7/N9/N10 are
verified **within this source-scoped
set**; N2/N6/N8/N11/N12 were correctly
classified N/A *for the router-only
pure ownership boundary*, not for
other product components. Complete
per-production-public-method
Stage 4 branch maps, mutation engine,
all global applicable N1–N12 counts,
hardware gates, and full newest-source
Mode B Stage 11 remain **OPEN**;
overall Stage 12 **NO**.



## 2026-10-18 P0 inbound Weave worker terminal error test inventory

Source-only
[worker lifecycle Stage 3–10 plan and P0 test matrix](mode-b-p0-gatt-worker-2026-10-18.md)
registers **five additional real inbound-worker
integration methods** (with two table-driven
two-variant methods; seven scenario executions)
in `core_lib/src/hdl/gatt/managed_tests.rs`.
Tests use real `run_inbound_worker` and
`SessionHub` ownership, with only the
outside BlueZ byte boundary replaced by
a real in-memory duplex socket. They
explicitly assert:

| P0 executed branch | New method intent | MODE B state |
|---|---|---|
| Native Initial EOF | Silence false UI error; drop unadmitted lease | **C — scoped 10× PASS** |
| Legacy Initial EOF | Emit bounded inbound Disconnected event with correct id/type | **C — scoped 10× PASS** |
| Native after-initial EOF / hostile 32-bit length | Emit Disconnected, update session, no receipt | **C — scoped 10× PASS** |
| Native parent vs session-token cancel after protocol progress | Emit Cancelled exactly once, close session owner | **C — scoped 10× PASS** |
| Native Initial cancellation | Quiet release, no fabricated UI state, no lease | **C — scoped 10× PASS** |

Prior exact commits: `e7f88543`
(test), `9030e6d9` (format only).
CI on source was repeatedly cancelled
due **concurrent release/packaging
feature commits**. Latest
[Preflight 39082223808](https://github.com/morozhenko-apps/rquickshare/actions/runs/39082223808)
had a separate red Python APT contract step
(`Unexpected package path`) after
parallel version/package-path changes.
**Do not conflate CI failure with
worker test correctness**; Core Rust
job and five exact methods require
completed logs to move from P to C.

GATT `CharacteristicNotifier` error,
send/notify queue saturation,
service-hash rejection, real multi-packet
reassembly, counter wrap, BLE↔TCP
upgrade and adapter disappearance
remain **OPEN / hardware or contract
seam tests**. A substantial
injectable-adapter redesign would be
MODE A and cannot be silently
introduced as part of Mode B tests.
Full Stage 12 and all-negative-category
closure remain **NO**.




**Verification update:** all five new methods and two
existing managed BLE positive cases were proved in
[P0 CI run 39082536345](https://github.com/morozhenko-apps/rquickshare/actions/runs/39082536345),
**10 numbered attempts, all 7/0 tests pass per
attempt, 0 ignored**, on source `a9f56aad`.
The older launch status paragraphs above
refer to provisional checks before this
independent workflow; superseded by this
scoped verified result. Other `Preflight`
APT contract failures remain unclosed and
must not be called a pass. Full Stage 11
suite 10× and global Stage 12 remain NO
for the current final candidate.

## 2026-10-18 current P0 GATT/Weave negative-scenario reconciliation

The original October 12 tables and C/P cells below are historical
scope records. They must not be treated as a complete current
`core_lib` + `native_desktop` + frontend inventory.
Current global Stage 12 remains **NO**; see
[formal completion audit](mode-b-completion-audit-2026-10-18.md)
and scoped
[GATT source branch and N1–N12 ledger](mode-b-p0-gatt-slot-framing-2026-10-18.md).

The latest source-only
[Preflight 39080989423](https://github.com/morozhenko-apps/rquickshare/actions/runs/39080989423)
is **5/5 SUCCESS**. Core unit inventory now reports
**718 methods**, split as 710 stable 1× + 8
risk-sensitive 10× (80 exact passes). Eight separate
integration cases remain 4+4. All eight new
critical negative/boundary test methods in
`core_lib/src/hdl/gatt.rs` and
`core_lib/src/hdl/gatt/slot.rs` independently passed.
[Native GTK 39080989573](https://github.com/morozhenko-apps/rquickshare/actions/runs/39080989573)
also completed successfully. The CI formatter originally
identified whitespace/line-wrap drift, fixed without
changing assertions. No product source changed.

| P0 scoped production artifact | Status | New direct coverage | Remaining obligation |
|---|---|---|---|
| GATT `SlotValue::read` legacy and coherent states | **C for planned pure branches** | unknown peer, offset 0/1/len/len+1/MAX, empty revision, peer 0-slot refresh, other-peer isolation, 31/32/33 slots and bounded eviction | Live BlueZ read/notify lifecycle and races across app restarts |
| `parse_connection_request` | **C for planned pure branches** | truncated lengths 0–6, 15 unknown control commands, missing control, unsupported/inverted versions, version-1 range cross products, all size min/max ±1 bounds | malformed/control interactions after subscription and notify worker admission |
| `complete_framed_message_len` | **C for planned pure branches** | incomplete length prefixes, zero, 1, max, max+1, u32MAX; payload short/exact/trailing and adjacent message boundaries | Real Weave multi-packet reassembly, invalid service hash, notification failures and payload forwarding |
| `ReceiverGattServer::{new,run}` | **S/OPEN** | pure slot policy is tested but **not** real adapter registration or GATT handler concurrency | BlueZ setup/errors, bounded writes, callback cancellation and header publication |
| `weave_session` / `run_inbound_worker` | **P/S OPEN** | 2 existing managed duplex acceptance cases and pure framing helpers | Full 24-branch Weave session matrix, disconnection/counter-wrap, BLE→TCP secure migration and system failure variants |

At source `23b71de3` the ordinary Preflight and GTK are
both green; the previously successful strict full 10×
`mode-b.yml` run `39078162262` covered an **earlier**
source and cannot certify the newly added eight methods.
Rerun full Stage 11 on the final accepted candidate
after filling the remaining P0 scenarios.

N1 input validation, N2 limit boundary, N4 replay,
N5 peer concurrency, N6 stale/corrupted revision,
N7 untrusted peer/control, N9 partial interruption
and N10 no peer-data leakage are now **better covered
within these scoped pure helpers**, but this does
not turn those global categories into YES. The actual
GATT notifier type `bluer::gatt::local::CharacteristicNotifier`
remains concrete and the GATT runner establishes real
`bluer::Session`. An architectural transport seam,
if chosen, must be separately approved under MODE A;
a fake adapter cannot be treated as evidence of
real BLE interoperability. The physical Pixel/Windows
tests and broader per-production-method Stage 3–10
completion work remain OPEN.


_Last updated: 2026-10-12_

This document is subordinate to `docs/mode-b-test-plan.md` and records Stages 3-8 for the current maintenance/Pixel receiver scope.

The original tables below retain their historical planning estimates. For cases implemented after those tables were written, the **2026-10-12 source reconciliation** below is authoritative; historical `P` cells are not evidence that an implemented regression is missing. Hardware-only `S` cells remain open unless dated physical evidence explicitly satisfies them. Offline continuation and exact validation results are tracked in [offline-maintenance.md](offline-maintenance.md).

Legend:
- `C` = covered by an existing automated test.
- `P` = planned automated coverage.
- `S` = package/live smoke only because the behavior depends on BlueZ, WebKit, desktop lifecycle, or a real peer.
- `NA(reason)` = not applicable to this artifact.

## Stage 3 - Test inventory

| Artifact | Risk | Level | Current evidence | Planned delta |
|---|---|---|---|---|
| `parseListeningPort` | P2 | Unit | Empty, min/max and common invalid values covered | Add NaN/Infinity and whitespace/format abuse cases |
| `BleAdvertiser::should_advertise` | P1 | Unit | Visible/Temporary/Invisible covered | None |
| `BleAdvertiser::{new,run,get_advertisement}` | P1 | Integration/Smoke | No deterministic BlueZ adapter seam | Keep smoke-only before hardware validation |
| `PassiveBleMonitor::{new,run,quick_share_pattern}` | P0 | Unit + Integration/Smoke | FE2C service-data filter is unit-covered; BlueZ monitor lifecycle is OS-bound | Smoke monitor activation, repeated DeviceFound cycles, cancellation, and monitor-failure fallback |
| `receiver_service_data` | P0 | Unit | Layout and long ASCII truncation covered | Add multibyte UTF-8 truncation/length-bound coverage |
| `ReceiverAdvertiser::{new,run}` | P0 | Integration/Smoke | No deterministic BlueZ adapter seam | Smoke visibility, retry, periodic re-register, FE2C/FEF3 coexistence |
| `BwuRouter::new` | P1 | Unit | Exercised indirectly | No dedicated test needed beyond router behavior |
| `BwuRouter::register` | P0 | Unit/Integration | Unique registration/cancel covered | Add empty ID, multiple endpoints, concurrent duplicate registration |
| `BwuRouter::route` | P0 | Integration | Successful consume covered | Add unknown endpoint and dropped receiver paths |
| `BwuRouter::cancel` | P1 | Unit | Existing endpoint covered | Add unknown endpoint/idempotent cancel |
| `BwuRouter::has_pending` | P1 | Unit | Empty/non-empty covered | Add multi-endpoint lifecycle |
| `parse_connection_request` | P0 | Unit | Short/malformed/version mismatch and clamp covered | Add exact min/max, wrong command, inverted range, trailing bytes |
| `connection_confirm` | P1 | Unit | Indirect only | Add exact encoding test |
| `complete_framed_message_len` | P0 | Unit | Short/complete/zero covered | Add incomplete declared frame, max, max+1, trailing next-frame bytes |
| `ReceiverGattServer::{new,run}` | P0 | Integration/Smoke | BlueZ-bound | Smoke GATT registration, slot-0 offset reads, queue overflow behavior |
| `weave_session` | P0 | Integration | Main branch logic not directly unit-testable through `CharacteristicNotifier` | Record testability defect; cover pure framing/reassembly seams before broad refactor |
| `validate_received_file_name` | P0 | Unit | Safe, traversal/absolute/control/oversize covered | Add exact 255-byte boundary and multibyte byte-length boundary |
| `prepare_inbound_files` / folder materialization | P0 | Unit/FS integration | Duplicate names/IDs, negative size, transactional failure, nested hierarchy, root collision remap, slash/backslash normalization, Windows absolute/traversal rejection, symlink-parent rejection and consent-time directory creation covered | Hardware-smoke real Pixel folder metadata and occupied-root behavior |
| `parse_wifi_password_payload` | P1 | Unit | Valid 16-byte and malformed variants covered | Confirm invalid UTF-8, exact/truncated trailer variants |
| `read_plain_frame_from` / `send_plain_frame_on` | P0 | Unit/Integration | Round-trip and zero-length read covered | Add max/max+1, truncated stream, zero-length send |
| `validate_client_introduction` / `peek_client_introduction` | P0 | Unit | Valid intro, incomplete prefix and invalid event/payload variants covered | Add explicit wrong endpoint semantics at BWU handoff layer |
| `InboundRequest::{new,enable_bandwidth_upgrade,take_bwu_pending}` | P1 | Unit | Exercised indirectly | Add one-shot pending flag behavior |
| Inbound secure-session/state-machine methods | P0 | Integration | Regression coverage exists around introductions, payload guards, P-256 validation and consent | Keep branch-by-branch map below; fill uncovered deterministic branches only |
| `InboundRequest::do_bandwidth_upgrade` | P0 | Integration/Smoke | Indirect BWU routing evidence, no complete deterministic handoff test | Add pure frame/order assertions where possible; real migration remains hardware smoke |
| `MigratableStream::{poll_read,poll_write,poll_flush,poll_shutdown}` | P0 | Integration | BLE read/write/flush/shutdown covered | Add TCP variant round-trip/shutdown; flush survivor remains equivalent for concrete transports |
| `is_cancel_request` | P1 | Unit | Direction/id/action covered | None |
| `prepare_outbound_files` | P0 | Unit/FS integration | image/video/audio/apk/unknown classification, missing input, zero-file rejection, recursive folders, selected-root preservation, duplicate root disambiguation, symlink skipping and unique payload IDs covered | Hardware-smoke Linux folder -> Pixel; add unreadable/non-UTF8 cases only where deterministic |
| Outbound secure-session/state-machine methods | P0 | Integration | Consent accept/reject/malformed plus previous hardening regressions | Fill only deterministic branch gaps discovered below |
| `route_bandwidth_upgrade_if_pending` | P0 | Integration | Successful route without consuming bytes covered | Add no-pending, malformed/ordinary frame with pending route, unknown endpoint |
| `error_for_ui` | P2 | Unit | No dedicated test | Add 512-character boundary/truncation |
| `TcpServer::{new,run,connect}` | P1 | Integration/Smoke | Main listener path exercised indirectly | Keep accept-loop/cancellation live; test pure route classifier separately |
| `checked_payload_buffer_size` | P0 | Unit | 0, max, -1, max+1 covered | Add 1 and max-1 table entries for mutation strength |
| `parse_mdns_endpoint_info` | P1 | Unit | standard/compact/empty/truncated/too-short covered | Add invalid base64 and invalid UTF-8 |
| `normalize_p256_coordinate` | P0 | Unit | short pad, signed 33-byte, invalid lengths covered | Add exact 32 and bad 33-byte prefix explicitly if not already parameterized |
| `get_download_dir` | P1 | Unit/Integration | Poison recovery code untested | Test only if global-lock isolation can be deterministic |
| `is_virtual_interface` | P0 | Unit | known tunnel prefixes and common LAN names covered | Add case-insensitive variants |
| `local_lan_ipv4` | P0 | Unit/Integration | Environment-dependent only | Extract pure selector seam; cover private/public/fallback/IPv6/loopback/tunnel interactions |
| `is_not_self_ip` | P2 | Integration | Environment-dependent | Leave integration-only unless selector seam makes it cheap |
| `RQS::{new,run,discovery,stop_discovery,change_visibility,set_foreground,stop,set_download_path}` | P1 | Integration/Smoke | Orchestration not unit-covered | Treat BlueZ/mDNS/task lifecycle as integration/smoke; pure state setters may be unit-tested if isolated |
| Frontend `ContentStatus.vue` | P2 | Component | Ready/drop, file select, folder select/cancel, clipboard text/image and discovery idempotence covered | Hardware-smoke native directory picker and directory drag/drop |
| Frontend `Heading.vue` | P2 | Component | Host/version/settings/update link | No pre-smoke gap identified |
| Frontend `SettingsModal.vue` | P2 | Component | load/save/clear port, invalid port, startup toggles, download folder | Extend only if `parseListeningPort` contract changes |
| Frontend `SideMenu.vue` | P2 | Component | visibility states and cancel event | No pre-smoke gap identified |

## Stage 4 - Branch map

### Listening port parser
1. Trimmed input empty -> automatic port.
2. Non-integer -> reject.
3. Integer < 1024 -> reject.
4. Integer > 65535 -> reject.
5. Integer inside inclusive range -> accept.

### Linux passive BLE monitor

1. BlueZ monitor/session/adapter setup succeeds -> register an OrPatterns monitor for 16-bit Service Data UUID 0xFE2C.
2. Monitor setup or registration fails -> return the contextual error so `BleListener` activates the compatibility active-scan fallback.
3. `DeviceFound` -> emit the existing BLE activity signal used by `MDnsServer` to resend the receiver announcement.
4. `DeviceLost` or another monitor event -> do not emit a discovery activity signal.
5. Monitor event stream ends unexpectedly -> return an error and activate the compatibility fallback.
6. Cancellation -> stop the monitor cleanly and do not activate fallback.
7. The monitor path must not create or hold a `btleplug` discovery session.

### Receiver service data
1. Device type is masked to three bits.
2. Name length <= 237 bytes -> preserve full byte sequence.
3. Name length > 237 bytes -> truncate to protocol limit.
4. Endpoint-info length must fit one byte.
5. Service-data framing length must match the encoded connection advertisement.

**Historical defect, fixed:** byte truncation was not UTF-8-boundary-aware. Current receiver advertisement and mDNS encoding preserve UTF-8 boundaries with regression tests.

### BWU router
1. Empty endpoint -> reject registration.
2. New endpoint -> register.
3. Duplicate pending endpoint -> reject.
4. Route known endpoint -> remove pending entry and deliver socket.
5. Route unknown endpoint -> return socket.
6. Route with dropped receiver -> return socket.
7. Cancel known endpoint -> remove.
8. Cancel unknown endpoint -> no-op.
9. Multiple independent endpoint IDs -> independent lifecycle.

### Weave connection request
1. Length < 7 -> reject.
2. Control bit absent -> reject.
3. Command != connection request -> reject.
4. Protocol version 1 outside requested range -> reject.
5. Requested packet size < 20 -> clamp to 20.
6. Requested packet size 20..509 -> preserve.
7. Requested packet size > 509 -> clamp to 509.

### Framed-message length
1. Prefix < 4 bytes -> incomplete.
2. Declared length 0 -> reject.
3. Declared length > 5 MiB -> reject.
4. Prefix + body incomplete -> incomplete.
5. Prefix + body complete -> return first-frame total.
6. Trailing bytes after a complete frame -> still return first-frame total.

### Weave session
1. Receiver unavailable -> error.
2. Ignore pre-handshake junk until a valid connection request.
3. Handshake channel closes -> error.
4. GATT notification stops -> clean exit.
5. Packet channel closes -> clean exit.
6. Empty packet -> ignore.
7. Control ERROR -> error.
8. Other control packet -> ignore.
9. FIRST resets reassembly.
10. Reassembly overflow or > safety limit -> error.
11. Non-LAST -> continue buffering.
12. Reassembly < 3 bytes -> clear/ignore.
13. Socket introduction -> ignore as control.
14. Socket disconnection -> clean exit.
15. Unknown service hash -> clear/ignore.
16. Missing framed length -> error.
17. Declared 0 or > max -> error.
18. Declared/body mismatch -> error.
19. Valid inbound framed message -> forward to duplex.
20. Duplex EOF -> clean exit.
21. Outbound buffer overflow -> error.
22. Complete outbound frame -> prepend service hash and fragment.
23. FIRST/LAST bits set correctly across fragments.
24. Send counter wraps without corrupting payload.

### Inbound filename/file/folder preparation
1. Empty/oversized/unsafe leaf name -> reject.
2. Parent folder empty -> top-level file.
3. Forward- or backslash-separated relative parent -> normalize to safe components.
4. Absolute Unix/Windows path, empty component, `.`, `..`, NUL/control, oversized component or excessive depth -> reject.
5. Negative file size -> reject.
6. Duplicate payload ID against existing/current batch -> reject.
7. Top-level file destination occupied/reserved -> increment numeric prefix until unique.
8. Folder root free -> preserve announced root.
9. Folder root occupied/reserved -> choose one numeric-prefixed root alias and reuse it for every descendant.
10. Introduction preparation must not create directories or files.
11. Consent-time materialization creates missing parents one component at a time.
12. Existing symlink/non-directory parent or canonical escape from download root -> reject before file creation.
13. Duplicate selected filenames in one destination -> reserve independent unique leaf destinations.
14. Total transfer size/suffix arithmetic overflow -> reject.
15. Full batch succeeds -> return complete prepared set; any failure leaves request state and filesystem unmodified.

### Wi-Fi password parser
1. Payload < 4 -> reject.
2. Prefix != 0x0A -> reject.
3. Declared password length arithmetic overflow -> reject.
4. Declared payload/trailer beyond available bytes -> reject.
5. Trailer marker != 0x10 -> reject.
6. Invalid UTF-8 password -> reject.
7. Valid payload -> return password.

### Plain BWU frame
1. Read: zero or > 5 MiB -> reject.
2. Read: valid length + full body -> return.
3. Read: truncated prefix/body -> I/O error.
4. Send: empty -> reject.
5. Send: > 5 MiB -> reject.
6. Send: length not representable as u32 -> reject.
7. Send: valid -> write prefix, body, flush.

### CLIENT_INTRODUCTION classifier
1. Incomplete prefix -> None.
2. Zero/oversized length -> None.
3. Incomplete body -> None.
4. Complete valid BWU CLIENT_INTRODUCTION with non-empty endpoint -> Some(endpoint).
5. Complete malformed/non-BWU/empty-endpoint frame -> None at peek layer.
6. Strict validator returns an error for malformed/non-BWU/empty endpoint.

### Migratable stream
1. BLE variant delegates read/write/flush/shutdown.
2. TCP variant delegates read/write/flush/shutdown.
3. Transport swap happens by replacing enum variant while request-owned crypto state remains untouched.

### Outbound file/folder preparation
1. Missing input -> skip; selected symlink/non-regular input -> skip.
2. Regular file -> top-level attachment with empty `parent_folder`.
3. Directory -> deterministic recursive walk without following symlinks.
4. Direct child of selected directory -> `parent_folder = selected_root`.
5. Nested child -> forward-slash parent such as `selected_root/photos`.
6. Two selected directories with the same root name -> disambiguate the later root once and keep its descendants together.
7. Empty directory / selection with zero regular files -> reject before protocol setup.
8. Folder depth or unsafe/non-UTF8 parent component beyond contract -> reject.
9. File metadata and every FILE payload header carry the same parent-folder value.
10. Total byte-size/payload-id invariants remain identical to ordinary multi-file transfer.
11. image/video/audio/apk/other -> correct attachment type.
12. Missing filename -> error.
13. Non-UTF8 filename -> error.
14. Payload ID collision -> regenerate.
15. File size not representable in i64 -> error.
16. Total transfer size overflow -> error.
17. Empty input -> reject with no transfer plan.

### BWU listener classifier
1. No pending route -> return socket immediately.
2. Pending route + EOF -> normal inbound socket.
3. Pending route + invalid/oversized length -> normal inbound socket.
4. Pending route + incomplete frame until deadline -> normal inbound socket.
5. Pending route + ordinary Quick Share frame -> normal inbound socket.
6. Pending route + valid CLIENT_INTRODUCTION for registered endpoint -> consume route, do not consume bytes.
7. Valid CLIENT_INTRODUCTION for unregistered endpoint -> consume/classify socket without creating a normal inbound session.

### Payload-size guard
1. < 0 -> reject.
2. 0 -> accept.
3. 1..max -> accept.
4. max -> accept.
5. max+1 -> reject.

### LAN address selection
1. Interface enumeration fails -> None.
2. Virtual/tunnel interface -> ignore.
3. IPv6 -> ignore.
4. loopback/link-local IPv4 -> ignore.
5. private non-virtual IPv4 -> return immediately.
6. public non-virtual IPv4 -> remember as fallback.
7. later private address beats earlier public fallback.
8. only public candidates -> first fallback.
9. no eligible candidates -> None.

## Stage 5 - Positive/N1-N12 matrix

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Port parser | C | P | C | NA(no network) | NA(no retry) | NA(pure) | NA(no storage) | NA(no auth) | NA(no time) | NA(no UI interruption in parser) | NA(no PII) | NA(no billing) | NA(no storage) |
| Receiver service data | C | P | C | NA(no network call) | NA(no retry) | NA(pure) | NA(no persisted data) | NA(no auth) | NA(no time) | NA(no UI) | NA(no PII log) | NA(no billing) | NA(no storage) |
| Receiver advertiser lifecycle | S | NA(no external input parser) | S | S | S | S | NA(no persisted data) | NA(no auth) | NA(no time semantics) | S | NA(no PII) | NA(no billing) | NA(no storage) |
| Linux passive BLE monitor | S | C(FE2C filter bytes) | C(pattern offset/type) | S(monitor unavailable/stream ends) | S(fallback activation) | S(repeated found/lost cycles) | NA(no persisted data) | NA(no auth) | NA(no wall-clock semantics) | S(cancel/shutdown) | C(no payload content logged) | NA(no billing) | NA(no storage) |
| BWU router | C | P | P | NA(no network semantics beyond socket ownership) | P | P | NA(no persisted data) | P(endpoint identity) | NA(no time) | P(cancel/drop) | NA(no PII) | NA(no billing) | NA(no storage) |
| Weave request/framing helpers | C | P | P | NA(pure helpers) | NA(no retry) | NA(pure helpers) | P(malformed frames) | P(protocol misuse) | NA(no time) | NA(no UI) | NA(no PII) | NA(no billing) | NA(no storage) |
| Weave session | S | P | P | S | P | P | P | P | NA(no locale/time) | S(disconnect) | NA(no PII by contract) | NA(no billing) | NA(no storage) |
| Inbound filename/files | C | C | P | NA(no network in helper) | P(duplicate) | NA(single-thread helper) | C | C(path abuse) | NA(no time) | P(partial failure) | P(filename/log review) | NA(no billing) | P(write/path failures) |
| Wi-Fi payload parser | C | P | P | NA(pure parser) | NA(no retry) | NA(pure) | P | P(malformed peer input) | NA(no time) | NA(no UI) | P(secret must not be logged) | NA(no billing) | NA(no storage) |
| Plain BWU frame | C | P | P | P(truncated/offline I/O) | NA(no retry policy) | NA(single stream) | P | P(peer framing abuse) | NA(no time) | P(peer closes) | NA(no PII) | NA(no billing) | NA(no storage) |
| CLIENT_INTRODUCTION | C | P | P | P(incomplete socket data) | P(repeat intro) | P(route race) | P | P(endpoint mismatch/malformed) | NA(no time) | P(peer closes) | NA(no PII) | NA(no billing) | NA(no storage) |
| Inbound secure state machine | C | P | P | P | P | P | P | P | NA(protocol has no locale/time rule) | P(cancel/disconnect) | P(no secrets in logs) | NA(no billing) | P(file writes) |
| Migratable stream | C | NA(byte stream input) | P(buffer/EOF) | P(I/O errors) | NA(no retry) | P(swap timing) | NA(no persisted data) | NA(no auth) | NA(no time) | P(shutdown) | NA(no PII) | NA(no billing) | NA(no storage) |
| Outbound file preparation | C | P | P | NA(no network in helper) | P(payload collision) | NA(single-thread helper) | P | P(path/name abuse) | NA(no time) | P(cancel handled elsewhere) | P(file path log review) | NA(no billing) | P(open/read failures) |
| Outbound secure state machine | C | P | P | P | P | P | P | P | NA(no locale/time) | C(cancel/deny) | P(no secrets in logs) | NA(no billing) | P(file read failures) |
| BWU listener classifier | C | P | P | P | P | P | P | P(endpoint identity) | P(deadline behavior) | P(peer closes) | NA(no PII) | NA(no billing) | NA(no storage) |
| Payload-size guard | C | C | C | NA(pure) | NA(no retry) | NA(pure) | NA(no persisted data) | P(peer abuse) | NA(no time) | NA(no UI) | NA(no PII) | NA(no billing) | NA(no storage) |
| mDNS parser | C | P | P | NA(pure parser) | NA(no retry) | NA(pure) | P | P(malformed peer record) | NA(no time) | NA(no UI) | P(device-name handling) | NA(no billing) | NA(no storage) |
| P-256 normalization | C | C | C | NA(pure) | NA(no retry) | NA(pure) | P | C(peer key abuse) | NA(no time) | NA(no UI) | NA(no PII) | NA(no billing) | NA(no storage) |
| LAN selector | P | P | P | P(interface enumeration failure) | NA(no retry) | P(interface-order interaction) | P | P(VPN/tunnel exclusion) | NA(no time) | NA(no UI) | P(no tunnel leakage) | NA(no billing) | NA(no storage) |
| RQS orchestration | S | P(config inputs) | P(port/buffer boundaries) | S | S | S | P | P | NA(no locale/time) | S | P | NA(no billing) | S(download path) |
| Frontend components | C | C | C | P(clipboard/plugin failures where applicable) | P(double action) | P(rapid events) | P(stale settings) | NA(no auth boundary here) | NA(no time-sensitive UI in scope) | P(close/back/drop) | P(no secret rendering/logging) | NA(no billing) | P(settings/download failures) |

## Stage 6 - Interaction matrix

Mandatory interaction sets:

1. Visibility x advertiser type x repeated transfer:
   - Visible/Temporary/Invisible x FE2C/FEF3 x first/repeated receive.
   - Automation: pure visibility predicate only.
   - Smoke: BlueZ coexistence and re-registration.

2. Linux receive discovery backend x lifecycle:
   - Advertisement Monitor available/unavailable/terminates x foreground/background x first/repeated Pixel receive.
   - Preferred path must not start active discovery; unavailable/terminated monitor must select bounded active fallback.
   - Cancellation must never be interpreted as monitor failure and must not start fallback.

3. BWU route state x incoming frame:
   - no pending / pending other endpoint / pending same endpoint
   - ordinary frame / incomplete intro / malformed intro / valid intro.
   - Highest risk: valid intro for a stale or dropped registration.

4. Transport x session state:
   - BLE/TCP x pre-UKEY2/post-UKEY2/post-upgrade x read/write/shutdown.
   - Keys and sequence counters must remain request-owned rather than transport-owned.

5. Filename x destination state x metadata validity:
   - safe/unsafe name x free/existing/reserved destination x positive/negative size x unique/duplicate payload ID.
   - Any invalid member of a batch must leave request state unchanged.

6. Frame length x actual bytes x transport closure:
   - 0/1/max/max+1 x short/exact/trailing x open/EOF.
   - No allocation before validation.

7. Outbound source x filesystem state:
   - image/video/audio/apk/other x exists/missing/unreadable/non-UTF8 x one/many files.

8. LAN interfaces:
   - private/public x physical/virtual x IPv4/IPv6 x order permutations.
   - Private eligible address must win over public fallback regardless of order.

9. User action x outbound state:
   - cancel/accept/reject x Initial/WaitingForConsent/transferring/Finished.
   - Foreign transfer IDs and LibToFront messages must not cancel the active transfer.

## Stage 7 - Coverage estimation

Expected pre-smoke additions from the current gaps:

- Port parser: 1 table-driven test.
- Receiver advertisement data: 1 UTF-8 boundary test, potentially accompanied by a small production fix.
- BWU router: 3-4 tests.
- Weave pure helpers: 4-6 tests.
- Inbound filename/file preparation and framing: 4-6 tests.
- Migratable TCP variant: 2 tests.
- Outbound preparation: 2-4 tests.
- BWU listener classifier: 3-5 tests, preferably after extracting a deterministic classification seam.
- Payload-size guard: 1 table-driven expansion.
- mDNS/P-256 utilities: 2-3 tests.
- LAN selection: 5-7 table-driven cases after a pure selector seam.
- Error truncation: 1 table-driven test.

Estimated additional automated scenarios before hardware smoke: **29-40**. This is an estimate, not a quota. Tests may be consolidated with parameterized cases only when they validate the same branch/rule.

## Stage 8 - Coverage map

| Production file | Important functions/branches | Current state | Remaining reason |
|---|---|---|---|
| `hdl/ble.rs` | monitor-first orchestration, active fallback lifecycle, foreground/background duty policy | Partial | Backend selection and BlueZ active-scan lifecycle require package/live smoke |
| `hdl/ble_monitor.rs` | FE2C pattern, BlueZ monitor registration/events/cancellation | Partial | Pattern is unit-covered; D-Bus Advertisement Monitor lifecycle is hardware-bound |
| `hdl/blea.rs` | visibility predicate, receiver data, BlueZ lifecycle | Partial | BlueZ lifecycle is hardware-bound; UTF-8 boundary gap is deterministic |
| `hdl/bwu.rs` | register/route/cancel lifecycle | High line coverage | Negative/concurrency lifecycle branches still need explicit tests |
| `hdl/gatt.rs` | request parsing, frame length, weave reassembly/fragmentation | Partial | Core session is coupled to BlueZ notifier; pure branches can be extracted/tested |
| `hdl/inbound.rs` | filesystem guards, frame parsing, client intro, secure state machine, BWU | Partial | Large state machine includes hardware/crypto integration branches and deterministic gaps |
| `hdl/migratable.rs` | BLE/TCP delegation | High | TCP variant lacks explicit tests |
| `hdl/mod.rs` | state/data definitions | Mostly declarative | No standalone behavior beyond state containers |
| `hdl/outbound.rs` | file plan, cancel/consent, secure send state machine | Partial | Filesystem failures and some state branches remain |
| `lib.rs` | task orchestration, visibility, discovery, shutdown | Low | mDNS/BlueZ/task orchestration; package/live integration is more representative |
| `manager.rs` | BWU classification, accept/send loops | Partial | Classifier negative branches and accept-loop lifecycle remain |
| `protocol.rs` | payload size guard | Strong branch coverage | Add inner boundary values for mutation confidence |
| `utils.rs` | mDNS, P-256, download dir, LAN selection | Partial | LAN selection is environment-coupled; some malformed input cases remain |
| `app/main/src/vue_lib/network.ts` | listening port parser | Strong | NaN/Infinity explicit cases remain |
| Frontend components | settings/status/header/menu flows | High component coverage | OS/plugin/window lifecycle remains smoke/integration territory |

Stages 3-8 are now documented. Test implementation may start from the planned deterministic gaps without waiting for hardware smoke.


## Consolidated dev scope extension - Full Quick Share

After PR #3 was integrated into `dev`, the Mode B scope expanded. The following artifacts are mandatory before the final completion gate.

### Stage 3 extension - Test inventory

| Artifact | Risk | Level | Current evidence | Planned delta |
|---|---|---|---|---|
| Modern `sharing.nearby` protobuf fields | P1 | Contract | Round-trip tests cover modern introduction fields, QR handshake material, and resume attachment details | Add explicit backward-compatibility decode of legacy/minimal frames and enum/default-field boundaries where generated-code behavior can regress |
| `classify_outbound_text` | P1 | Unit | URL, phone number, plain text covered | Add case/whitespace boundaries, false-positive phone inputs, short digit strings, address-like text |
| `prepare_outbound_text` | P0 | Unit | Empty rejection and UTF-8 byte-size preservation covered | Add max/max+1 payload-size boundary, URL payload type, 64-character title truncation, whitespace-only policy |
| `byte_payload_frames` | P0 | Unit/Contract | Multi-chunk framing, offsets, final marker, reconstruction covered | Add empty body, exact chunk, chunk+1, max/max+1 payload boundaries and sensitive/header invariants |
| `ManagedEphemeralFile` / `OutboundPayload::EphemeralFiles` | P0 | Unit/FS integration | Security defect identified: raw strings previously acquired deletion capability | Add constructor path-boundary abuse cases, missing-file idempotence, verified cleanup, and ensure normal `Files` payload never deletes source files |
| Tauri `managed_temp_path` | P0 | Unit/Security | Prefix restriction test exists | Add path traversal / sibling path / extension / non-temp-root abuse cases |
| Tauri `save_clipboard_image` | P1 | Integration/Smoke | No deterministic clipboard backend seam | Package/live smoke with image clipboard; keep backend integration smoke-only unless DI is introduced |
| Tauri `remove_ephemeral_file` | P0 | Unit/Integration | Prefix guard is shared but command path is not directly covered | Add allowed managed path removal, missing-file idempotence, forbidden path rejection |
| `ContentStatus.shareClipboard` | P1 | Component | Text success, image fallback, no-content error covered | Add whitespace text + valid image fallback and discovery-already-running for clipboard path |
| Frontend `clearSending` ephemeral cleanup | P0 | Unit | No direct test found | Add EphemeralFiles cleanup, cleanup failure resilience, normal Files no-delete, state reset invariants |
| Outbound text consent/send state machine | P0 | Integration | Existing consent tests cover file/empty flows; text helper tests do not prove state-machine integration | Add accepted text sends BYTE payload then Finished+disconnect; cancellation before text send; rejected text follows normal rejection path |
| Wi-Fi SAE inbound handling | P1 | Unit/Integration | SAE branch added to production parsing | Add SAE password payload regression matching WPA/WEP parsing behavior |

### Stage 4 extension - Branch map

#### Outbound text classification
1. Trim surrounding whitespace for classification only.
2. `http://` and `https://`, case-insensitive -> URL.
3. Phone candidate strips whitespace, `-`, `(`, `)`.
4. At least seven digits and only digits/optional `+` -> phone number.
5. Short/invalid phone-like input -> plain text.
6. Everything else -> plain text.

#### Outbound text preparation
1. Empty string -> reject.
2. Non-empty UTF-8 -> preserve original payload bytes.
3. Payload byte length > protocol maximum -> reject.
4. URL classification -> `TextPayloadType::Url`; other classifications -> `Text`.
5. Metadata title uses at most 64 Unicode scalar values.
6. Metadata size equals byte length, not character count.
7. Generated payload id is copied consistently into metadata.

#### BYTE payload framing
1. Body length 0 -> terminal frame only.
2. 1..256 KiB -> one DATA frame + terminal frame.
3. Exact 256 KiB -> one DATA frame + terminal frame.
4. 256 KiB + 1 -> two DATA frames + terminal frame.
5. Body at protocol max -> accepted.
6. Body > protocol max -> reject before frame allocation.
7. Every DATA frame carries same payload id/total size/type.
8. Offsets are monotonic and exact.
9. Nonterminal chunks have flags=0.
10. Final empty chunk has offset=total size and flags=1.

#### Ephemeral outbound cleanup
1. Raw `EphemeralFiles` strings have no deletion authority.
2. Each raw path must pass `ManagedEphemeralFile` validation before `OutboundRequest` construction succeeds.
3. Only a direct child of the system temp directory with the application-owned `rquickshare-clipboard-*.png` name may become managed.
4. Normal `Files` never acquire deletion authority.
5. Existing managed ephemeral file -> removed on drop or explicit cleanup.
6. Already missing managed file -> idempotent success.
7. Traversal, sibling, wrong-prefix, wrong-extension and ordinary user paths -> reject.
8. Removal failure -> warn/error without panic and without broadening the allowed boundary.
9. Frontend cancellation/reset asks Tauri to remove only managed ephemeral files; Tauri delegates the same core validation instead of duplicating policy.

#### Clipboard UI
1. Nonblank clipboard text -> emit Text, ensure discovery.
2. Blank text -> try image.
3. Text backend error -> try image.
4. Image saved -> emit EphemeralFiles, ensure discovery.
5. Image backend error -> inline error, emit no payload.
6. Discovery already running -> do not restart it.
7. Clipboard error is cleared before a new attempt.

#### Managed clipboard-image path
1. File must live under application temp root.
2. Filename must match the application-owned PNG prefix.
3. Sibling/parent traversal or arbitrary temp file -> reject.
4. Missing managed file removal -> idempotent success.
5. Existing managed file removal -> delete exactly that file.

### Stage 5 extension - Positive/N1-N12 matrix

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Modern protobuf compatibility | C | P | P | NA(pure encode/decode) | C(round-trip) | NA(pure) | P(legacy/missing fields) | P(malformed enum/fields) | NA(no time semantics in scoped fields) | NA(no UI) | P(sensitive flags must preserve semantics) | NA(no billing) | NA(no storage) |
| Outbound text classification/preparation | C | P | P | NA(pure) | NA(no retry) | NA(pure) | NA(no persisted data) | P(untrusted text input) | NA(no time) | NA(no UI) | P(text must not be logged as secret content) | NA(no billing) | NA(no storage) |
| BYTE payload framing | C | P | P | NA(pure framing) | P(re-send/idempotent frame construction) | NA(pure) | P(malformed sizes) | P(peer/protocol boundary) | NA(no time) | NA(no UI) | P(sensitive/header invariant) | NA(no billing) | NA(no storage) |
| Ephemeral file cleanup | C | P | P | NA(no network) | P(double cleanup) | P(drop vs frontend cleanup) | P(missing file) | C(prefix/path ownership boundary) | NA(no time) | C(cancel/reset) | C(do not delete arbitrary user file) | NA(no billing) | P(remove failure/read-only path) |
| Clipboard UI flow | C | C | NA(no numeric limit) | P(plugin/backend failure) | P(repeated paste) | P(rapid retry) | P(stale clipboard/backend state) | NA(no auth) | NA(no time) | C(error/retry/reset) | P(no clipboard content in logs) | NA(no billing) | P(temp-file failure) |
| Outbound text state machine | P | P | P | P(peer disconnect) | P(cancel/retry) | P(cancel vs accept) | P(malformed consent) | P(protocol misuse) | NA(no time) | P(cancel) | P(text secrecy/log review) | NA(no billing) | NA(no persistent storage) |
| Wi-Fi SAE receive | P | P | P | P(truncated transfer) | NA(no retry) | NA(single session parser) | P | P(malformed credentials) | NA(no time) | P(reject/cancel) | P(password must not be logged) | NA(no billing) | NA(no storage) |

### Stage 6 extension - Interaction matrix

9. Clipboard content x backend state x discovery state:
   - text/image/empty x text-plugin success/failure x image-command success/failure x discovery idle/running.
   - Exactly one outbound payload may be emitted per user action.

10. Outbound text size x chunk boundary x classification:
   - plain/URL/phone x 0/1/chunk/chunk+1/max/max+1 bytes.
   - Classification must not affect byte framing or size validation.

11. Ephemeral ownership x cleanup trigger:
   - managed/unmanaged path x frontend clear/drop/double cleanup x exists/missing/removal error.
   - Arbitrary user files must never be deleted.

12. Protocol generation compatibility:
   - legacy absent fields / modern populated fields x encode/decode.
   - New optional fields must not change defaults of old peers.

### Stage 7 extension - Coverage estimation

Additional deterministic scenarios for the consolidated Full Quick Share scope:

- text classification/preparation: 6-9 cases;
- BYTE framing boundaries: 5-7 cases;
- ephemeral cleanup security/idempotence: 5-7 cases;
- clipboard component interactions: 2-4 cases beyond existing coverage;
- outbound text state-machine integration: 3-4 cases;
- SAE receive regression: 1-2 cases;
- protobuf backward/default compatibility: 2-4 cases.

Estimated delta: **24-37 scenarios**, preferably table-driven where the branch/rule is identical.

### Stage 8 extension - Coverage map

| Production file | Important functions/branches | Current state | Remaining reason |
|---|---|---|---|
| `core_lib/src/proto_src/wire_format.proto` | modern optional fields/enums/default compatibility | Partial contract coverage | Legacy/default-field compatibility and malformed boundaries remain |
| `core_lib/src/hdl/outbound.rs` | text classification, metadata, BYTE framing, ephemeral cleanup, text consent/send | Partial | Pure helpers covered; state-machine and boundary matrix incomplete |
| `core_lib/src/hdl/inbound.rs` | SAE credential parsing | Partial | Explicit SAE regression missing |
| `app/main/src-tauri/src/cmds/clipboard_image.rs` | path ownership, image save, removal | Partial | Path abuse/removal branches deterministic; clipboard backend smoke-bound |
| `app/main/src/composables/ContentStatus.vue` | clipboard text/image/error/discovery interaction | Good | Blank-text image fallback + already-running interaction remain |
| `app/main/src/vue_lib/utils.ts` | ephemeral cleanup during clear/reset | Missing direct coverage | Deterministic unit seam already exists through VM invoke contract |

The final Mode B gate must use this extension together with the original Stage 3-8 inventory. Hardware smoke does not replace these deterministic cases.

## 2026-10-12 source reconciliation

These mappings come from current test source, not historical coverage percentages. Execution results belong to the offline work log. A listed test proves only its assertions; remaining cases are not silently marked covered.

| Historical gap | Existing regression evidence | Remaining scope |
|---|---|---|
| Port non-finite inputs | `tests/unit/example.test.ts`: NaN and both infinities | Settings/backend failures and event races |
| Receiver/mDNS UTF-8 | `blea.rs` receiver-data boundary; `utils.rs` multibyte-name truncation | BlueZ advertiser lifecycle |
| BWU router lifecycle | `bwu.rs`: empty/unknown IDs, dropped receiver, duplicate race, cancel, exact-ID precedence and old-ID alias | Route lifetime during full migration |
| Weave pure framing | `gatt.rs`: packet-size boundaries, wrong command/inverted range, confirm encoding, frame-size/trailing-byte boundaries | Reassembly/fragmentation/session lifecycle |
| Migratable TCP | `migratable.rs`: TCP round trip and shutdown forwarding | BLE-to-TCP crypto-state continuity |
| Filename/Wi-Fi abuse | `inbound.rs`: filename byte boundaries and malformed Wi-Fi table including invalid UTF-8 | Complete file-write/storage-failure matrix |
| Plain framing/BWU flags | `inbound.rs`: oversized read, empty send, one-shot flags | Truncated prefix/body and maximum send/read review |
| mDNS malformed records | `utils.rs`: Base64/UTF-8/truncated/compact tests | OS discovery/cache behavior |
| LAN selection | `utils.rs`: pure private preference, public fallback, IPv6/loopback/link-local/tunnel rejection and order | Real interface-enumeration failure |
| Listener/UI errors | `manager.rs`: no-pending, malformed, unrelated route, alias, non-consuming route and error truncation | Incomplete-prefix deadline, EOF, orchestration |
| Modern proto defaults | `tests/modern_protocol.rs`: legacy defaults plus modern/QR/resume round trips | Malformed/unknown enum review |
| Text helpers | `outbound.rs`: classification shapes, title/payload bounds, empty/UTF-8 handling | Accepted/rejected/cancelled text state-machine integration |
| BYTE chunk boundaries | `outbound.rs`: empty/exact-chunk/chunk+1/max and reconstruction/terminal marker | Sensitive/header and integration review |
| Managed ephemeral ownership | `outbound.rs`: name boundaries, idempotent remove, forbidden request, normal-file no-delete, drop cleanup | Removal-error/race matrix |
| Desktop cleanup command | `cmds/clipboard_image.rs`: shared core managed-path guard | Native clipboard backend smoke; execute existing test in CI |
| Clipboard/reset | `content-status.test.ts`: blank-text image fallback/running discovery; `utils.test.ts`: cleanup success/failure/no-delete/state reset | Rapid retry and plugin/storage combinations |
| SAE metadata | `inbound.rs`: `introduction_preserves_sae_wifi_security_type` | Full SAE transfer/rejection/privacy |

There is no separate Tauri `managed_temp_path` implementation now: the command delegates to core `ManagedEphemeralFile`. Do not plan duplicate validation in a nonexistent helper.

Desktop inventory: two `notification.rs` preview tests, two `logger.rs` capped-writer tests, three `cmds/open_download_destination.rs` path tests and one `cmds/clipboard_image.rs` cleanup test. These eight assertions were omitted from preflight/Mode B before the offline continuation. Notification body/action/lifecycle and settings persistence are not comprehensively covered by them.

Remaining deterministic priorities: text consent/send/cancel integration, explicit Weave/framing branches without broad transport extraction, and scoped notification/settings/event logic after preregistering its contracts. Fresh core/desktop/Vue coverage and meaningful mutation review remain required. The offline analyzer has a separate inventory in `offline-maintenance.md`.

The corrupted numeric `visibility` panic remains a settings defect. Recovery visibility changes privacy/discoverability and requires the user's policy decision. Hardware findings below remain authoritative for physical gates.

## Hardware smoke findings - 2026-10-06

Real Pixel-to-Linux and Linux-to-Pixel smoke on Ubuntu 26.04 KDE/Wayland produced the following evidence:

- Linux -> Pixel clipboard-image transfer succeeds over the normal LAN path and managed clipboard temporary files are removed after completion.
- Pixel -> Linux file receive establishes the BLE Weave session and completes UKEY2, but the offered Wi-Fi LAN bandwidth upgrade times out after 15 seconds. No TCP connection reaches the application's listener before the timeout, so the transfer falls back to BLE and progresses in roughly 509-byte payload chunks.
- The peer sends `BANDWIDTH_UPGRADE_RETRY` / `UPGRADE_FAILURE` after the failed Wi-Fi handoff. These frames are currently logged as unhandled and should be treated as expected fallback diagnostics rather than protocol-fatal errors.
- Two Pixel -> Linux text attempts fail before a receiver GATT/Weave session is opened. No `TextMetadata` reaches `InboundRequest::process_introduction`, so the current evidence does not support changing the inbound text parser.
- The completed-file `Open` action passes a local destination path to the frontend shell URL opener. This violates the shell opener scope and produces an invalid-URI/open failure. Local receive destinations need a dedicated backend capability rather than broad shell URL permissions.

Next deterministic/live checks:

1. Add FEF3 slot-0 read, Weave write/notify-subscription, and negotiated packet-size diagnostics without logging payload contents.
2. Keep inbound text parser unchanged until the next instrumented hardware run proves that a sharing frame reaches it.
3. Verify Linux firewall/listener reachability for the advertised BWU TCP port before changing the BWU router or protocol.
4. Fix local received-destination opening through a backend-only command that computes the configured/default download directory itself; do not accept an arbitrary filesystem path from the frontend.
5. Re-run Pixel -> Linux text and repeated Pixel -> Linux file receive without restarting the application.

### Follow-up smoke evidence

A later Pixel -> Linux text attempt reached the receiver successfully and exposed a separate state-machine bug:

- The text `IntroductionFrame` is decoded and the UI is moved to `WaitingForUserConsent`.
- Before the user clicks Accept, the peer sends another completed BYTES payload with a different payload ID. The current receiver sees `text_payload.is_some()` and incorrectly assumes every completed BYTES payload is the announced text body, then terminates the BLE session with `Unexpected text payload id`.
- Reference Quick Share receiver behavior (NearDrop) first matches the completed BYTES payload ID against the announced text payload ID; non-matching BYTES payloads continue through the transfer-setup frame decoder. rQuickShare must preserve that distinction.
- The frontend currently remains in `WaitingForUserConsent` after this GATT session error, so subsequent Accept clicks are sent into a dead session. GATT inbound failures must surface `Disconnected` to the frontend.

The same run also proved the Wi-Fi BWU reachability failure is host-firewall related on the smoke machine: UFW is active with default incoming deny, while rQuickShare is listening on a random high TCP port (40119 in the retest; 42971 in the earlier run). No rQuickShare rule is present. For the current smoke, use the existing fixed-port setting plus an explicit UFW rule; do not change the user's firewall automatically from the application.

### Successful post-fix Pixel smoke

A later hardware run with the fixed port reachable through UFW confirms the transport and text-state fixes:

- Cold BLE receiver path: Android reads FEF3 slot 0, opens the Weave notify session, completes UKEY2, accepts the advertised Wi-Fi LAN upgrade, connects to the fixed TCP port, completes LAST_WRITE / SAFE_TO_CLOSE, then the inbound session migrates from BLE to Wi-Fi LAN.
- Once the TCP socket reaches rQuickShare, introduction parsing and consent presentation are fast (roughly 0–1 second on direct TCP sessions).
- The remaining perceived pre-consent delay is before or during Android discovery/transport selection. On the observed cold path, slot-0 read -> GATT notify subscription took about 9 seconds and Wi-Fi upgrade negotiation took about another 8 seconds before consent. Treat this as a separate discovery/transport-latency optimization task; do not weaken the now-working protocol sequence without a measured experiment.
- Warm/direct TCP inbound file transfer completed about 1.5 MB in roughly one second after acceptance, confirming that the previous 509-byte BLE crawl was caused by the blocked BWU port rather than file-transfer chunking.
- Inbound text now completes successfully after acceptance, with the full text payload reaching `Finished`.
- The repaired local `Open` action successfully opens the configured download directory.

Notification UX follow-up:

- Consent notifications currently contain only the sender name even though the channel metadata already includes text description, file names and total bytes.
- Consent notifications should show a bounded preview/type summary so the user can decide without opening the main window.
- Successful inbound text should raise a completion notification with a bounded preview and a `Copy` action that copies the full received text.
- Successful inbound files should raise a completion notification with file/count summary and a `Show file` action.
- Notification previews must be bounded to avoid oversized desktop notifications; the underlying payload must not be truncated for actions such as Copy.

### Reveal received item UX

Post-smoke UX follow-up for completed inbound files:

- The completed-file action should reveal the received item itself in the desktop file manager instead of opening the download directory at its initial scroll position.
- On Linux, use the standard `org.freedesktop.FileManager1.ShowItems` D-Bus interface so Dolphin/Nautilus can open the parent folder and select the file.
- The frontend must not regain arbitrary filesystem-open authority. It may pass only the normalized relative received path from trusted transfer metadata; the backend resolves the configured download directory, validates every relative component, canonicalizes the existing candidate under that directory, and constructs the file URI.
- If the file-manager D-Bus interface is unavailable or the named file cannot be resolved, fall back to opening the configured download directory.
- For multi-file completion, revealing the first received item is sufficient for the current UI; multi-selection can be added later.

### Terminal transfer history vs. outbound composer

Hardware smoke on 2026-10-07 exposed a deterministic frontend-state defect:

- A completed inbound transfer remains in `displayedItems` with state `Finished`.
- `HomePage.displayedIsEmpty` previously used `displayedItems.length === 0`, so terminal history incorrectly hid the outbound composer (`Select files`, `Select folders`, clipboard).
- If a newly discovered endpoint reused the same endpoint id as the completed transfer, `_displayedItems` replaced the live endpoint with the terminal transfer card, making the visible peer non-clickable for a new send.

Required invariants:

1. Terminal history states (`Finished`, `Cancelled`, `Rejected`, `Disconnected`) must not by themselves suppress the outbound composer.
2. An explicit outbound payload or a live endpoint/active transfer must switch the UI into nearby-device/transfer mode.
3. While an outbound payload is pending, a live endpoint must take precedence over stale terminal history with the same id.
4. Active transfer states must still take precedence over the endpoint so progress/consent UI remains visible.
5. Regression tests must cover terminal-history-only, active transfer, live endpoint, outbound-payload, and same-id endpoint/history collisions.

### Receiver advertising recovery after GATT activity

Hardware smoke on 2026-10-07 localized the remaining Pixel -> Linux startup delay before the transfer protocol/BWU path:

- Successful BWU itself is fast: TCP route and CLIENT_INTRODUCTION complete in about 200 ms and BLE -> Wi-Fi migration in about 500 ms total.
- Repeated inbound attempts show GATT slot-0 reads and receiver advertising re-registration separated by long gaps, while Linux -> Pixel outbound TCP is immediate.
- BlueZ can consume a connectable advertising set when a peer connects. The current ReceiverAdvertiser retains an AdvertisementHandle that still looks live and only forces a re-register every 30 seconds.
- The receiver advertisement must therefore be refreshed by real GATT activity, not only by the 30-second safety timer.

Required invariants:

1. A slot-0 read at offset 0 requests an immediate receiver-advertisement refresh.
2. Completion/termination of a Weave notify session requests another refresh so the next inbound transfer is discoverable immediately.
3. Refresh is ignored while visibility is Invisible.
4. Refresh drops only the advertising handle; it must not stop the GATT application, active inbound task, TCP listener, or ordinary FE2C advertiser.
5. The existing 30-second re-register remains as a fallback for adapters/events that do not produce a refresh signal.
6. Bursty/duplicate refresh requests are harmless and bounded; no unbounded queue is allowed.
7. Add unit coverage for refresh policy/state transitions where hardware-independent, and retain hardware smoke for repeated Pixel -> Linux transfers.

### GATT slot-0 to mDNS handoff

Hardware smoke on 2026-10-07 after event-driven advertisement refresh refined the remaining startup issue:

- Three Pixel -> Linux file transfers reached backend `Finished`, even though Android reported one attempt as failed. This indicates the Android-side bootstrap/UI timeout can expire before the Linux TCP session arrives.
- Fresh runs use two receiver paths: BLE Weave + BWU on some attempts, but direct inbound TCP on others. In the slow direct-TCP path, Pixel reads FEF3 slot 0 and only much later opens the advertised TCP service.
- Immediate receiver-advertisement re-registration from inside the slot-0 read is too aggressive because it mutates the active connectable advertising set while the peer is still in bootstrap.
- The slot-0 read is nevertheless the best deterministic signal that Android is actively discovering this receiver. It should immediately refresh the Nearby mDNS announcement so the direct-TCP endpoint is visible during the same discovery cycle.
- Receiver BLE advertising recovery after slot-0 should be deferred briefly and remain best-effort; Weave-session completion can still refresh it immediately because that GATT session is already ending.

Required invariants:

1. Slot-0 offset-0 read triggers an immediate mDNS `register_resend` signal.
2. Slot-0 read does not synchronously drop/re-register the active FEF3 advertisement handle.
3. A short delayed best-effort FEF3 advertisement refresh may be scheduled after slot-0 read so adapters that consume connectable advertising recover without disturbing current bootstrap.
4. Weave session completion continues to request immediate FEF3 advertisement refresh.
5. mDNS resend and FEF3 refresh use separate bounded channels; no UI/visibility side effects are introduced.
6. Invisible visibility still suppresses mDNS re-announcement and receiver advertising.
7. Existing 30-second FEF3 refresh stays only as a safety fallback.
8. Hardware smoke must measure slot-0 -> TCP-accept latency across at least three consecutive Pixel -> Linux attempts.

### Monitor-first Linux receive discovery

The 2026-10-07 startup-latency investigation showed that the previous 5-second scan / 25-second idle Linux tray policy can itself add up to 25 seconds before FE2C activity is observed. A fresh smoke run also showed the active scan stopping after its first window without a later restart, while the task error was not surfaced by the top-level runner.

Required invariants:

1. Linux prefers the BlueZ Advertisement Monitor backend for FE2C detection and initializes `btleplug` active discovery only after monitor setup/runtime failure.
2. The passive monitor filters AD type 0x16 at offset 0 for the little-endian Quick Share service UUID bytes `2C FE`.
3. A passive `DeviceFound` event emits the existing BLE activity signal and therefore requests the existing mDNS receiver refresh path.
4. Cancellation of the passive backend exits cleanly and must not trigger fallback.
5. Unsupported/disabled/terminated Advertisement Monitor is explicitly logged before activating the compatibility active-scan backend.
6. Active fallback preserves foreground continuous discovery and background 5-second scan / 25-second idle duty-cycling; it is compatibility behavior, not the preferred Linux path.
7. Any terminal `BleListener` error is logged by the RQS task owner; discovery backend failure must never disappear silently.
8. On a system where Advertisement Monitor is active, rQuickShare must not hold BlueZ `Discovering: yes` merely while sitting in the tray.
9. Hardware smoke must perform at least three consecutive Pixel -> Linux receives without restarting rQuickShare and record passive-monitor/slot0 -> TCP-accept timing as applicable.
10. Hardware smoke must also record whether the machine selected the passive monitor or active fallback and, for fallback, the exact logged monitor-unavailability reason.

### Monitor-first hardware smoke result - 2026-10-07

The Ubuntu 26.04 KDE hardware run on package SHA `e229ac8` completed three consecutive Pixel -> Linux image receives, all reporting `Finished` without restarting rQuickShare. It did **not** exercise the passive-monitor happy path:

- At 21:34:34 UTC, BlueZ rejected `RegisterMonitor` on `org.bluez.AdvertisementMonitorManager1` with `org.freedesktop.DBus.Error.UnknownMethod`. `BleListener` logged this and selected the active-scan compatibility fallback without user-visible failure.
- Active fallback matched a Quick Share FE2C advertisement at 21:34:37 UTC and emitted the existing mDNS refresh signal. Following `tray_show` at 21:34:39, foreground mode kept the active scan running; the slow first transfer cannot be attributed to the 5s/25s background duty cycle in this run.
- First receive used GATT/Weave: slot-0 offset-0 read at 21:35:26, mDNS resend immediately, deferred receiver advertising re-registration at 21:35:28, Weave notify subscription and BWU TCP acceptance at 21:35:48, Wi-Fi LAN migration at 21:35:52, consent at 21:35:54, `Finished` at 21:35:57. The measured slot-0 -> Weave gap is **22 seconds**; do not conflate this with transfer payload time or with the user-observed approximate 18-second startup.
- Second and third receives used direct inbound TCP instead of GATT: TCP accept at 21:36:07 -> `Finished` at 21:36:10, and 21:36:19 -> `Finished` at 21:36:22. All three files completed and no transport-level errors were recorded in this run.
- No text transfer occurred in this run; earlier Pixel -> Linux text regressions still require a current-build smoke check.

Remaining discovery-latency investigation:

1. Treat the slot-0 -> Weave delay as a separately measured cold-start issue; the monitor-first refactor did not resolve it on this machine because its BlueZ API is unavailable.
2. The 2-second FEF3 advertisement re-registration while Android is still in GATT bootstrap is a **hypothesis**, not a proven cause. Compare cold-start slot-0 -> notify latency with and without in-flight advertising re-registration in a controlled, reversible A/B experiment; do not disable it permanently from one sample.
3. Preserve fast direct TCP receive, visibility suppression, bounded mDNS resend, Weave-session completion refresh, and the 30-second recovery fallback. Do not loosen protocol/BWU invariants or change the host's bluetoothd configuration automatically.
4. Repeat at least three cold-start trials per variant with equivalent visibility and foreground state, plus warm/direct TCP receives and text receive. Record which discovery backend was active on every run.

### Cold-start FEF3 advertising recovery A/B experiment

The measured 22-second slot-0 -> Weave gap is not yet causally attributed to receiver-advertisement re-registration. Evaluate the 2-second slot-0 deferred FEF3 refresh without changing default/release behavior:

- Debug-only startup control: `RQS_DIAG_SLOT0_ADV_REFRESH=0` disables **only** the deferred FEF3 recovery request from a slot-0 offset-zero read. Unset/default or `=1` retains the existing deferred refresh.
- The experiment must always preserve slot-0 mDNS resend, normal visibility policy, immediate refresh after a completed Weave session, and the 30-second safety refresh. Release builds ignore the control.
- Emit an explicit startup log identifying which variant is active and a distinct slot-0 event log so every hardware trial can be associated with the correct treatment.
- For each variant, run at least three **cold** Pixel -> Ubuntu receives (restart receiver before each; same phone, network, visibility, foreground/background state) and at least one repeated/warm direct-TCP receive. Keep the successful transfer path and compare slot-0 -> notify/TCP acceptance, total Android UI time, fallback use, and failures.
- System BlueZ configuration must remain unchanged throughout. No release-policy change from a single faster observation. Restore the diagnostic flag to its default after experiment.
- Verify Android -> Ubuntu **text** at least once, since this smoke log contains only images. A/B comparison must not suppress text/clipboard regressions.

### A/B execution helper

The repository includes `scripts/receiver-ab-smoke.sh` to make cold-trial startup reproducible after installing the **debug** Debian package built from the corresponding `dev` HEAD.

```bash
bash scripts/receiver-ab-smoke.sh baseline
bash scripts/receiver-ab-smoke.sh skip-deferred
```

Run the commands in separate trials, not simultaneously. **Always use the tray Quit action after each transfer, never Ctrl+C**. This lets the server cancel its tasks and unregister the mDNS service before exit. Exit the previous rQuickShare tray process before starting the next variant. The helper refuses to start if a user-owned `rquickshare` process remains, sets `RQS_LOG=trace`, sets the A/B switch explicitly to `1` or `0`, and writes per-run timestamped logs to `~/Downloads/rquickshare-smoke/ab-logs/`. It prints the relevant BLE/GATT/BWU/TCP events after exit but retains the complete log. If the debug diagnostic startup marker is absent, discard that trial: a release binary or missing GATT initialization may have ignored the test treatment. Never compare baseline and variant sessions with different visibility, foreground/background state, networking, or receiver startup conditions.


### 2026-10-08 A/B smoke findings: seven Pixel attempts

Received archive: three baseline logs and four skip-deferred logs (the fourth is plain text). All seven inbound sessions reached backend `Finished`. Six image payloads and one text payload were recorded; the text was processed and completed, though Android-side timing is not instrumented.

| Variant | Run UTC | Start -> TCP accept | TCP accept -> Finished | GATT slot0 read | Outcome |
|---|---|---:|---:|---|---|
| Baseline | 18:36:27 | 23 s | 4 s | none logged | image Finished |
| Baseline | 18:37:04 | 8 s | 4 s | none logged | image Finished |
| Baseline | 18:37:43 | 64 s | 6 s | 18:38:01, **46 s** before TCP | image Finished following user-reported Android error/retry |
| Skip deferred | 18:39:15 | 30 s | 7 s | none logged | image Finished |
| Skip deferred | 18:40:02 | 10 s | 5 s | none logged | image Finished |
| Skip deferred | 18:40:23 | 12 s | 6 s | none logged | image Finished |
| Skip deferred | 18:40:56 | 42 s | 4 s | none logged | text Finished; user reported slow discovery |

The start-to-TCP interval is **not** a tap-to-device-discovery metric; trials did not record Android tap timestamps. The baseline outlier included a GATT slot0 read at 18:38:01, deferred FEF3 re-registration at 18:38:03, periodic re-registration at 18:38:33, then inbound TCP only at 18:38:47. This single observation cannot prove that re-registration caused the delay. No skip-deferred trial read slot0, so the A/B treatment was **not exercised on the same protocol path** and causal conclusions are invalid. The BlueZ AdvertisementMonitor `RegisterMonitor` API remains unsupported (`UnknownMethod`) in all trials, so fallback active scans were used.

Android displayed an increasing number of identically named `Alcotester` entries over restarts (up to four). At application startup, the endpoint ID is randomly generated (`core_lib/src/lib.rs`) and embedded into a fresh mDNS service instance name (`gen_mdns_name`). All seven logs end without the expected `tray_quit`, `MDnsServer: service unregistered`, or `Application stopped` messages. The old A/B script incorrectly instructed Ctrl+C, which can terminate the process before its `MDnsServer::run` unregister/goodbye completes. This is a **plausible primary explanation**, not a confirmed Android cache bug. Do not stabilize the endpoint ID as a quick fix: that would introduce a linkability/privacy tradeoff. The smoke helper now requires tray Quit and warns when graceful unregister is not logged.

Follow-up criteria:
1. Reinstall the new helper only by pulling `dev` (no new package needed for a shell-script-only change). On Pixel, dismiss stale device lists or restart Quick Share discovery between separate cold-start trials.
2. Run at least two trials in which the last log includes `tray_quit` and `MDnsServer: service unregistered`. Verify whether ghost `Alcotester` entries stop accumulating without any Android-side reset; record the display before and after Quit.
3. Record exact phone-side tap/discovery/connect timestamps or a screen recording for long-running attempts, separately from the host process-start timestamp.
4. To evaluate the FEF3 hypothesis, compare at least three **slot0-reaching** cold GATT attempts per variant. If only direct TCP attempts occur, mark the A/B experiment inconclusive instead of picking a winner.
5. Maintain a text-transfer test as a separate scenario. Backend `Finished` validates text delivery on Linux but does not validate phone-side UI latency.
6. If graceful mDNS goodbyes are logged but ghosts remain, investigate Android mDNS cache/advertisement TTL and server identity before changing protocol identity.

### 2026-10-08 graceful-exit follow-up and discovery confounder

The next target-device run used the refreshed launcher from `dev`; the attached terminal capture and cumulative application log both include two **baseline** foreground-process runs (the application was actually minimized in the tray, so the *active fallback BLE scan ran in background duty-cycle mode*). The startup marker showed `RQS_DIAG_SLOT0_ADV_REFRESH=1` in both. Both cleanly exited through the Tauri tray Quit handler and logged `MDnsServer: service unregistered: OK`.

| Baseline run (UTC) | TCP accepted | Outcome | Exit | Periodic FEF3 refresh | Active fallback scan restart |
|---|---|---|---|---|---|
| 19:13:22 | 19:13:59 (37 s) | image Finished 19:14:03 | 19:14:13 clean unregister | 19:13:52 | 19:13:52 |
| 19:14:19 | 19:14:51 (32 s) | first transfer Cancelled at 19:15:27; retry TCP at 19:15:28, Finished at 19:15:32 | 19:15:36 clean unregister | 19:14:49 | 19:14:49 |

The second first-request state reached `WaitingForUserConsent` at 19:14:51 and remained until Android canceled it at 19:15:27; no `AcceptTransfer` was logged for that first request. The second request was explicitly accepted at 19:15:31 and completed one second later. Do not report this as a protocol crash.

**Confounder:** Every initial TCP connection arrived shortly after **both** the 30-second FEF3 advertisement refresh and the 5s/25s active-FE2C fallback scan restarted. Both timers are aligned at boot in this log. The evidence cannot distinguish receiver advertising exhaustion from BLE scan duty cycling, Android-side discovery delay, or mDNS behavior. Neither run read slot0, so the 2-second slot0 A/B experiment remains inconclusive. These start-to-TCP measurements also include arbitrary user interaction time.

**Confirmed:** the Linux mDNS unregister path completes on a proper tray Quit. Earlier duplicate Android entries cannot be attributed to failure to unregister on these two clean exits; their persistence/absence on the Android screen is still unknown. Previous 19:09:15 process log had no clean exit marker before the next startup at 19:13:22, so stale Android cache remains plausible for earlier duplicates. The intentional Tauri tray Quit uses exit code -1 (reported by Bash as 255); the smoke launcher now labels that code as a *normal* shutdown only when `tray_quit` and mDNS unregister are both logged.

### Debug-only periodic receiver advertising A/B

A second diagnostic knob `RQS_DIAG_RX_PERIODIC_ADV_SECS` now selects the **FEF3 receiver advertiser safety-refresh** interval in debug builds only: 5–120 whole seconds, default 30. Unsupported inputs fall back to 30; release builds ignore this override. The debug log states the selected interval. It does not change the FE2C discovery scan duty cycle, GATT slot0 behavior, mDNS register/resend, visibility handling or post-Weave refresh.

The smoke launcher now supports `baseline` (FEF3 periodic 30s), `periodic-10` (FEF3 periodic 10s), and the original `skip-deferred` (slot0 deferred disabled, FEF3 periodic 30s). **A freshly built and installed debug .deb is required for the new periodic-10 variant.**

Do not infer causation from comparing minimized baseline 30s to visible-window periodic-10 10s. To separate timer effects, keep visibility and window state fixed across variants; explicitly record whether `BleListener: active fallback scan stopped` occurs during each trial. Prefer first conducting a *foreground/continuous-scan* baseline with Show from the tray versus the background/duty-cycle baseline, then conduct a 30s vs 10s periodic comparison at the same window state. Record phone-side discovery/tap timing separately. Preserve Mode B hardware gates and the default release interval pending replicated results.

### Desktop notification and Android completion synchronization gate

For reducing the user's manual hardware smoke burden without dropping Mode B checks, see [hardware-test-automation.md](hardware-test-automation.md). It contains the provisional 33-scenario automation matrix, Ubuntu runner/ADB architecture, planning estimates and access/security constraints.


New user report (2026-10-08): the Linux-side desktop notification vanishes very quickly while the Android sender still displays a waiting state. Do **not** assume this is the same event as backend `Finished` without a synchronized phone video or timestamps.

Code audit:
- `app/main/src-tauri/src/notification.rs` constructs request and completion notifications via `notify_rust::Notification` on Linux but specifies no explicit expiration policy. The desktop notification server controls their lifespan. The request's `Accept`/`Reject` action waiter may outlive the visible popup; the in-app `WaitingForUserConsent` card remains an alternate acceptance route.
- The separate Vue `useToastStore` has a hard-coded 3000 ms auto-dismiss for in-window toasts, but those toasts are not the native inbound transfer notification. Avoid conflating them.
- `core_lib/src/hdl/inbound.rs` sets `State::Finished` when file payload chunks are fully assembled (also for finished text byte payloads), emits the state to the UI, **then** sends a final encrypted Nearby `Disconnection` frame. This ordering can allow Linux UI completion to precede Android sender completion. It does not prove Android waits because of this ordering.
- The 2026-10-08 hardware log contains a `WaitingForUserConsent` first session which Android cancelled after 36 s and immediately retried. Another earlier session completed at the Linux backend. Phone-side action/ack state was not captured, so root cause must not be asserted.

Release-blocking acceptance checks:
1. With Linux window hidden, Android sends a file. Keep the phone waiting 20+ seconds: actionable Accept/Reject must remain reachable and operable; provide an in-app fallback and record KDE notification lifetime.
2. Verify Accept, Reject and phone-side cancellation as distinct outcomes; no actionable stale prompt may remain after the session ends. Notification lifecycle cleanup is required if persistence is introduced; simply setting `Timeout::Never` without dismissal on peer cancellation is insufficient.
3. For one image and one text payload, capture both phone sender status (screen recording/timestamps) and Linux event timestamps: `WaitingForUserConsent`, user action, `Finished`, final disconnection send, socket close, Android UI completion. A Linux `Finished` event alone cannot certify remote UI completion.
4. Do not reorder encrypted connection teardown or declare sender success until a protocol-specific regression test establishes the correct wire lifecycle.

Remaining hardware matrix for progression (counts are individual scenario executions, not disjoint tests; scenarios may share successful trials):
- Request notification and cancellation: 3 outcomes.
- Sender/receiver completion comparison: 2 payloads.
- Cold discovery background/foreground: 3 each (6).
- FEF3 periodic 30s/10s matched window-state A/B: 3 each (6; baseline can be reused from preceding matched trials).
- FEF3 slot0 deferred on/off: 3 slot0-reaching attempts each (6, conditionally needed if GATT path is implicated).
- mDNS duplicate-device behavior after two clean restarts: 2 cycles.
- Current build feature smoke: incoming text + URL and outgoing image + text + nested folder: 5 paths.
- Warm consecutive inbound receives without restart: 3 sessions.
- A fresh debug Debian package gate: 1 build. Final strict Mode B: complete-suite 10 consecutive green executions plus mutation/coverage and Stage-12 YES/NO matrix review.

These counts describe a controlled smoke plan, not a claim that each is a uniquely missing unit test. The 10x final Mode B gate is distinct from the standard one-pass dev preflight, which was green at `1e2f690`. Moving to the next product feature may proceed on `dev` after the targeted discovery/notification blockers are characterized; promoting the resulting feature-complete build to `master` requires final signoff.

