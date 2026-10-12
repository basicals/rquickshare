# rQuickShare maintained fork — audit and Mode B hardening

## 2026-10-12: bounded current-run history and ordered operations implemented

The user selected retention option 1. Plan/contract frozen before code in
`52b5347`; [native-retention.md](native-retention.md) records the additive seven
Operations1 methods, 128 connection-owned receipts and cumulative acknowledgement
watermark. Expired requests never execute again. Typed upload origins preserve
legacy Manager1 behavior and prevent bypassing sequenced commit/abort receipts.
Closed ordered upload metadata is reclaimed after acknowledgement; active selections
and queued/active payloads survive it. Legacy replay references remain pinned.

The serial runtime now records owner closure, identified outbound release and late
control receipts before retiring oldest eligible history beyond 64. Terminal state
alone cannot trigger retirement; missing/lost observations remain pinned. Shared
revision changes invalidate new reads without changing immutable captured pages.
No persisted journal, file deletion, higher FD/session limits or user cleanup step.
The future native client still must perform reconciliation/acknowledgement itself.

19 new tests; core inventory is now 448 unit/export plus 8 integration = 456.
The full run passed, followed only by the 164 affected service tests after final
compatibility refinements. Core and desktop strict Clippy, both Rust format checks,
XML contract parsing and diff checks passed. Exact observations/category counts
are in the implementation SSOT. No unchanged frontend checks or ten-run gate;
no physical peers, installed services or user settings were changed. Terminal closed.

Next: native client with automatic receipt acknowledgement and GTK tray/settings/
notifications, Dolphin, packaging and physical Google compatibility. The older
awaiting-retention-choice review below is historical and superseded.

## 2026-10-12: continuous-service retention review prepared

[native-retention-review.md](native-retention-review.md) analyzes the retained
256-transfer/128-receipt limits and prepares the next new API/data-model choice.
Recommended option 1 keeps 64 safely closed current-run terminal records in memory;
option 2 persists bounded history in a local transactional journal. Both use a
proposed additive ordered/acknowledged operation protocol so retired sequences
cannot be executed again, while Manager1 replay contracts stay compatible. History
cleanup requires owner closure and real reader release, not terminal phase alone.

Await user confirmation of history lifetime under AGENTS sections 3/3.1 before
changing behavior/wire contracts. No implementation, user-session change or test
rerun in this review. The executable/recovery slice below remains the current code.

## 2026-10-12: native executable and hidden visibility recovery implemented

The user selected option 2 and rejected manual JSON repair. Plan committed before
code in `823cf68`; [native-daemon-startup.md](native-daemon-startup.md) records
shared bounded read-only settings, effective Hidden recovery with a durable warning,
explicit legacy UI choices/save and persistence-before-core ordering. Failed save
retains the warning/live mode and restores the cached field; automatic retries are
disabled. Whole-file corruption still fails without resetting settings. Complete
atomic settings migration/repair remains a separate gate.

The native Linux executable reads prior settings, supports validation-only check,
local Unix session addresses, fresh epochs, a shared exclusive owner guard and
SIGINT/SIGTERM-driven joined cleanup. Updated Tauri/developer/native launchers share
the guard. Manager1 now has 15 methods including fixed GetStartupWarnings. No native
GTK client, installed activation/autostart, retention policy or peer claim was added.

25 new checks. Actual complete/affected results, one corrected private-directory
fixture assumption, compilation repairs and primary category counts are in the
startup SSOT: current inventory 437 core, 10 desktop, 46 frontend. All affected
checks and final strict Clippy/format/diff passed. No final one-call complete-suite
or ten-run stability claim. Real owned process tests prove signals during waiting
authentication and cleanup after an occupied-port startup failure; no live BlueZ/
mDNS success or graphical/device behavior was tested. No user config/service was
changed and no push/promotion occurred. Terminal closed.

Next: continuous-service retention review, native client/GTK notifications/settings,
Dolphin, package and physical Google compatibility. The previous privacy choice
below is answered and superseded by this checkpoint.

## 2026-10-12: native executable startup analysis

[native-daemon-startup-review.md](native-daemon-startup-review.md) records the next
executable plan and the existing outstanding visibility error decision. The old
runner uses Visible defaults without desktop settings; an invalid stored unsigned
visibility reaches a panic, while getter failures default to Visible. The daemon
must preserve the old AppData store without writes/resets. Recommended option 1
rejects invalid saved visibility before network startup; option 2 starts hidden
with an explicit degraded-state warning. User confirmation is required for this
privacy/product policy under AGENTS sections 3/3.1 and the existing diary.

Analysis also confirms the legacy UI does not claim the native bus name; an
independent shared launch guard is needed for the already accepted single-owner
invariant. No production code, user settings, process/service or package was
changed. No unchanged suite was rerun; this is not executable validation.

## 2026-10-12: bounded native state reads and invalidations implemented

The user selected stable snapshot pages (option 1). Plan/contract committed before
code in `9546431`; [native-dbus-state.md](native-dbus-state.md) records exact typed
signatures, view ownership, 16-record pages, checked 48-KiB bodies/64-KiB frames,
fixed 512-KiB capture reservations and 8-MiB aggregate cache. Successful replacement,
release, disconnect and close reclaim metadata without discarding transfers or
receipts. StateChanged is directed only to authenticated clients and coalesced by
the shared revision; effective discovery label/address updates now advance that
revision before mutation. Fourteen Manager1 methods and one signal are implemented.

18 new methods. The complete unit/export invocation passed 410/415 and exposed
five test Signature-display assumptions; exact wire-form comparison corrected
them. All 46 affected bus methods and four integration methods then passed. The
369 unaffected methods were already green. One later notification-test ordering
fence change passed its affected method. Final Clippy/format/diff checks passed.
Inventory is 419 checks, with no claim of a final single 419-method full run or ten
repetitions. The state SSOT preserves actual failures, outcomes, category counts,
limits and rollback. No installed service/UI, hardware certification or remote run.

Next: executable/process lifecycle, terminal/replay retention before continuous
deployment, native client/GTK, Dolphin and actual Google peers. The older state
contract choice below is answered; historical pending wire-state entries are
superseded by this checkpoint. No new significant decision is made in this slice.

## 2026-10-12: native D-Bus send and transfer controls implemented

[native-dbus-controls.md](native-dbus-controls.md) records the plan committed before
code (`7527abd`), additive Manager1 fixture and verification. DiscardSelection,
SendSelection, Respond and Cancel now delegate through the same serial authenticated
adapter into the existing coordinator/runtime. Selected files are never reopened
by path; explicit send moves ownership to the service; control replies acknowledge
admission rather than core processing or peer completion. No new trust, consent,
retention, settings or UI policy was introduced.

One full core run passed **397 unit/export + 4 integration = 401**, including eight
new regression methods. Final Clippy, formatting and diff checks passed. Tests
include actual private-broker send/replay/disconnect, foreign selection rejection,
accept/reject intent preservation and another same-UID panel cancelling queued
work; wire-level unit checks cover signatures/FDs/tokens, capacity and uncertainty.
Measured results and category counts are in the control SSOT. No package install,
live registration, hardware run, push or master promotion occurred. Terminal closed.

Next: concrete bounded snapshot/discovery/invalidation contract; executable,
continuous-service retention, native GTK panel and Dolphin. The old five-method-only
checkpoints below are historical; the current Manager1 exposes nine methods.

The concrete next public API choice is recorded in
[native-dbus-state-review.md](native-dbus-state-review.md): stable bounded typed
snapshot pages (recommended option 1) versus a larger bounded complete JSON reply
(option 2). It addresses aggregate state exceeding a symmetric 64-KiB message,
read consistency, cache ownership and invalidation. Implementation awaits the
user's choice under AGENTS sections 3/3.1; no old architecture approval is pending.


## 2026-10-12: transactional native D-Bus selection ingress implemented

The user chose the architecturally preferred option 1 from [native-dbus-selection-review.md](native-dbus-selection-review.md). [native-selection-transactions.md](native-selection-transactions.md) records the frozen Manager1 methods, owner-bound preparations, atomic commit/replay, shared aggregate quotas, safe CLOEXEC receipt and predecode frame/FD limits. A serial same-UID zbus adapter now hosts the retained native runtime and joins it on requested stop/bus loss. Exact per-caller disconnect subscriptions avoid unrelated session traffic; all broker authority replies are sender-checked. The library does not register itself on the user's bus or replace the UI.

35 new regression methods. One full local run passed **389 unit/export + 4 integration = 393**; the later fixture-only relocation passed all 22 affected bus methods. Final Clippy/format/diff passed; no-default-features compiled with the 9 existing inactive-feature warnings. Actual outcomes, the corrected serializer test assumption, source FD cleanup proof, category counts, rollback and remaining work are recorded in the selection SSOT. CI test jobs now declare their private-broker prerequisite; no remote CI/package/hardware run is claimed. No user package install, system policy, live service registration, push or master promotion.

Next: remaining snapshot/discovery/send/control wire methods and native executable; bounded retention review before continuous deployment; approved GTK clock-area chooser and Dolphin ingress; physical Google compatibility. Earlier pending-D-Bus statements below are historical and superseded by this entry.

## 2026-10-12: native coordinator/core runtime implemented

[native-core-runtime.md](native-core-runtime.md) records the completed serial coordinator/core mapping, bounded pending dispatch, discovery-only routes, truthful consent processing receipts and awaited shutdown. [native-core-events.md](native-core-events.md) records ordered bounded TCP/BLE lifecycle events and readiness without polling. Startup retains its owner across cancellation; shutdown closes admission before adopting buffered inbound sessions. The legacy Tauri app remains unchanged. Runtime implementation: `c7674e8`; ordered events/admission: `fb316c2`. The next new public D-Bus selection choice is recorded in [native-dbus-selection-review.md](native-dbus-selection-review.md); no bus contract is implemented before that choice.

28 regression methods added. One completed full core run passed 350 unit/export plus 4 integration tests; final runtime changes passed 23 affected methods. Final Clippy/format/diff checks passed; no-default-features compiled with 9 existing inactive-feature warnings. Current inventory is 354 unit/export plus 4 integration, with no claim of one final-source 358-method run. Full measured details and remaining physical/API/UI gates are in the runtime handoff. No daemon registration, system install, push or master promotion occurred. Older pending-runtime statements below are historical checkpoints superseded by this entry.

_Last updated: 2026-10-12_

## Native coordinator continuation — 2026-10-12

The user superseded development repetition requirements: one appropriate verification run, no ten-run loop or redundant polling. Global AGENTS section 12 records this preference. Previous measured repetitions remain in their original verification sections.

The accepted native service design now has a reusable Linux coordinator: caller-owned unsent selections, service-owned queued/active sends, bounded aggregate outbound-source reservations including sender duplicates, per-client idempotency, revisioned snapshots/consent, cancellation acknowledgement and draining. A terminal event cannot release admission quota while a worker still holds the original payload. Core event loss pauses sends/consent and remains explicit after reconnect; no invented resynchronization. This is internal state logic, not an activated daemon or authenticated IPC endpoint.

Plan, complete scoped inventory, 53-method index, category counts, results and next adapter gates are in [native-service-coordinator.md](native-service-coordinator.md). One full core run passed (233 unit/export + 4 integration tests), with format/Clippy and diff checks passing. Existing desktop/frontend/analyzer code was unchanged and its suites were not repeated. No physical compatibility or clock-area UI acceptance is claimed. No live service, package registration or system settings were changed.

Follow-up core inspection found outbound errors correlated to the network address instead of the request ID. The bounded correction uses one outbound failure builder, preserves metadata and terminal outcomes, and passed one targeted manager invocation (9 tests, 3 new). The user then approved the substantial TCP supervision refactor. The manager now bounds and joins its TCP workers, keeps accept responsive during outbound transfers, reserves BWU classifiers and cancels pending/active work on shutdown. [native-transfer-supervision.md](native-transfer-supervision.md) records 26 new scenarios, actual execution results including one corrected FIN/RST test assumption, and two real-manager text transfers. The [managed dispatch slice](native-managed-dispatch.md) now implements native outgoing cancellation, queue-closure health input and partial-start rollback. One full core run passed 288 tests, followed by 44 affected manager tests after the final fairness correction; format/Clippy passed. Common inbound consent processing acknowledgement is now implemented in [native-session-controls.md](native-session-controls.md); the live coordinator/core adapter remains pending. No live daemon or Google parity claim.

This document is the working log for the `morozhenko-apps/rquickshare` maintenance pass. It is intentionally kept separate from release notes: it tracks audit scope, upstream triage, CI gates, fixes, and the final Mode B verification before fast-forwarding `dev` into `master`.

Native continuation checkpoint: TCP supervision `ee2139a`, managed outgoing dispatch/startup rollback `2354c1d`. The user approved option 1 in [native-session-control-review.md](native-session-control-review.md). The [common session controls](native-session-controls.md) now provide fresh capabilities and owned consent/receipts for TCP and BLE, bounded joined GATT tasks and cancel-safe stale BWU-route pruning. Its 42 new methods and exact full/affected execution observations are recorded in the verification section. No new permission is pending for this completed slice; live service/API/UI and hardware parity gates remain open.

The 2026-10-12 offline continuation, recovered diary references, implementation plan and validation results are recorded in [offline-maintenance.md](offline-maintenance.md). Dated hardware evidence remains in [mode-b-inventory.md](mode-b-inventory.md).

The subsequent user correction establishes the native tray/notification-area redesign, Tauri migration following NativePair and full Google parity as the target. See [product-direction.md](product-direction.md); maintenance completion does not complete that goal.

## Current CI policy

- The repository has exactly two long-lived branches: `dev` and `master`.
- `dev` is the only active development branch. Normal work lands directly on `dev` with fast preflight: Rust formatting, Rust tests, Clippy, frontend lint/typecheck/unit tests.
- Long-lived feature branches and internal pull requests are not part of this fork's workflow.
- Package/build smoke is manual only. No Tauri packaging runs on every development commit.
- Release preparation is manual and must not create release branches.
- Mode B final gate runs the complete Rust and frontend automated suite 10 consecutive times.
- `master` is release-only and is updated by fast-forward from validated `dev` after the final gates and hardware smoke are green.

## Current gate status

Offline continuation (2026-10-12): all four suites passed ten consecutive local runs on final implementation `2051314` (122 core + 8 desktop + 41 frontend + 25 analyzer checks per iteration), after an earlier loop on `29c3dc8`. Fresh line measurements are core 48.05%, desktop 21.31% and frontend 54.69%, with measurement limits and ten caught analyzer mutations recorded in `offline-maintenance.md`. Physical-device completion, broader Mode B review and final promotion remain open.

The Pixel/BLE foundation and current Full Quick Share work are consolidated into the maintained two-branch workflow. On 2026-10-07 the user explicitly requested an **interim snapshot** fast-forward of `dev` at `0bc9b50` into `master` (400 commits ahead, zero behind). Both refs were verified identical after the non-forced update. This promotion is **not** a final Mode B completion or hardware release signoff: the remaining cold GATT startup latency and current-build Pixel -> Linux text smoke are still open. The complete 10x Mode B gate has not been claimed as passed.

- The final implementation head `e229ac8` passed all five preflight jobs (run `37687010250`) and manual Linux debug `.deb` package smoke (run `37688585293`). The promoted `0bc9b50` differs only by subsequent documentation describing Pixel smoke results.
- The debug-only slot0 and periodic advertising A/B controls and launcher are implemented on `dev`. Controlled slot0-reaching trials and matched foreground/background comparisons remain open; ordinary/release behavior must not change based on inconclusive evidence.
- Historical pre-smoke Mode B is green after the BWU test hardening: stable suite 1x plus selected async transport/BWU tests 10x (run `37475497104`). The current SSOT is stricter, so a complete-suite 10x run is still required before completion.
- Targeted mutation audit now reports 22 caught, 18 unviable, 1 missed and 0 timeouts across 41 mutants. The only survivor is the `MigratableStream::poll_flush -> Ok(())` mutation, which is equivalent/unobservable for the current concrete `DuplexStream` and `TcpStream` transports; the real delegation remains in production code.
- Residual `unwrap`/`expect`/panic review of the 11 changed Rust modules found no such calls in production regions; remaining occurrences are under `#[cfg(test)]`.

There is no standing `dev -> master` PR. Promotion to `master` is a deliberate fast-forward after the documented release gates pass.

## Completed work

### CI and toolchain

- Removed automatic package builds from normal `dev` pushes.
- Moved package smoke to a manual workflow.
- Moved release packaging to a manual workflow.
- Updated GitHub Actions away from Node 20-backed actions and configured Node 24 for frontend CI.
- Removed the deprecated `arduino/setup-protoc` action; Linux CI installs `protobuf-compiler` directly.
- Removed deprecated/redundant cache/action usage found during validation.
- Aligned Rust formatting/Clippy toolchains with the repository's actual toolchain requirements.
- Removed duplicate Rust formatting work and duplicate internal PR validation.
- Enabled cancellation of superseded runs.

### Repository metadata / maintenance

- Aligned crate/package license metadata with the repository GPLv3 license.
- Disabled automatic release-please activity for the maintained fork; release preparation is manual.
- Replaced the stale Tauri 2.2 dependency set with current Tauri v2 packages (Tauri core/API/CLI 2.12.1 plus current compatible plugin minors).
- Regenerated Cargo.lock and pnpm-lock.yaml reproducibly on GitHub-hosted runners and removed the temporary sync workflows afterwards.
- Updated the frontend toolchain conservatively inside its existing major lines: ESLint 9, Tailwind 3, TypeScript 5, Vite 6 and Vitest 3.
- Removed the unused Vue DevTools/Electron development stack that was responsible for a large obsolete vulnerability tree.
- Added weekly/manual dependency security auditing for both Rust lockfiles and frontend dependencies.
- Current Rust cargo-audit is clean for both lockfiles.
- Current JavaScript production/runtime audit has no known vulnerabilities.
- Full JavaScript dev-tooling audit has one accepted high-severity build-only advisory: Tailwind CSS 3.4.19 pulls `braces 3.0.3` through its watcher/glob chain; the advisory currently has no patched `braces` release. This dependency is not shipped in the Tauri runtime and build inputs are trusted repository files. The runtime audit remains release-blocking; the full toolchain audit remains visible/reporting.

### Protocol / interoperability

- Added currently used Nearby frame types 8–12 to the protobuf enum.
- Added/retained compatibility for compact mDNS endpoint information and fallback device naming.
- Reviewed upstream interoperability PRs before integrating equivalent fixes instead of blindly merging stale branches.

### Linux BLE lifecycle

- The primary Linux receive-side FE2C detector is BlueZ Advertisement Monitor, not an always-running discovery session.
- The monitor filters 16-bit Service Data advertisements for the Quick Share/Nearby service UUID 0xFE2C and emits the existing BLE activity signal used to refresh mDNS.
- Advertisement Monitor is preferred because it can receive targeted advertisement events without holding an active BlueZ discovery session, avoiding the Bluetooth HID auto-reconnect regression described by upstream issue #429.
- If Advertisement Monitor is unavailable at startup or its event stream terminates unexpectedly, the listener falls back to bounded active discovery rather than disabling inbound discovery. The fallback keeps the existing Linux tray policy of approximately 5 seconds scanning followed by 25 seconds idle.
- Fallback activation and monitor termination must be logged. BLE discovery tasks must never fail silently.
- The active-scan fallback remains continuous while the application is foregrounded and duty-cycled only while backgrounded on Linux.
- Made BLE advertising follow receive visibility rather than outbound discovery mode.
- Added foreground/background lifecycle signalling.
- Unified scan/advertising lifecycle so send discovery and receive visibility do not fight each other.

The monitor-first architecture supersedes duty-cycling as the normal Linux receive path. Duty-cycling is retained only as a compatibility fallback for BlueZ/controller combinations where Advertisement Monitor cannot be used. This covers the same problem areas reported upstream in issues/PRs such as #429, #430 and #404 while removing the intentional 0-25 second receive latency window from the normal path.

### Inbound hardening

- Reject unsafe received filenames instead of joining remote path components directly into the download directory.
- Reject absolute paths, traversal components, separators/control characters and overlong received names.
- Reserve unique destinations for multiple inbound files with the same filename.
- Reject negative or oversized byte payload sizes before allocation.
- Reject byte chunks that exceed the peer-declared size.
- Reject file payload data before the user has accepted the transfer instead of reaching an `unwrap()`.
- Guard offset arithmetic against overflow.
- Reject duplicate payload IDs.
- Reject negative file sizes and total-size overflow.
- Use exclusive destination creation and fail clearly if the configured download directory is unavailable.
- Reject invalid peer P-256 public keys instead of panicking.

### Outbound hardening

- Reject negative or oversized peer-controlled byte payload sizes before allocation.
- Reject byte chunks that exceed declared payload size.
- Replace peer-controlled connection/consent `unwrap()` paths with checked errors.
- Reject invalid peer P-256 public keys instead of panicking.
- Avoid outbound payload-ID collisions.
- Reject non-UTF-8 filenames cleanly instead of panicking.
- Guard total outbound transfer size against overflow.

### Error handling / UI

- Surface transfer errors instead of reducing every failure to a generic unexpected disconnect.
- Stop receiver loops cleanly when their channel closes.
- Normalized transfer state text and Vue markup so lint is meaningful rather than permanently noisy.
- Added a browser Clipboard API fallback when the Tauri clipboard plugin fails on Linux/Wayland, with unit coverage for primary/fallback/failure paths.
- Capped the active on-disk log at 5 MiB per session while keeping stdout logging alive; oversized previous logs are still rotated at startup. Unit tests cover the capped writer.
- Enabled a restrictive production/development Content Security Policy and `freezePrototype` in the Tauri WebView. Production network access is limited to Tauri IPC plus the GitHub API used for release checks.

## Upstream PR / issue triage

Reviewed or identified as relevant:

- PR #442 — missing download-directory handling. Not applied verbatim: silently recreating a configured removable/missing destination is not always desirable; the maintained fork currently fails clearly when the selected destination is unavailable.
- PR #439 / issue #431 — compact mDNS endpoint info and newer frame types. Equivalent compatibility work is included.
- PR #430 / issue #429 — continuous BlueZ discovery interference. Equivalent duty-cycle work is included.
- PR #404 — BLE advertisement based on visibility. Equivalent lifecycle work is included.
- PR #418 — filesystem / transfer error propagation. Equivalent error-surfacing work is included where applicable.
- PR #408 — Tauri JS/Rust dependency alignment. Superseded: the fork now uses current Tauri v2 (2.12.1 core/API/CLI with compatible current plugins) and reproducibly regenerated lockfiles.
- PR #380 / issues #423 and #195 — clipboard fallback. Equivalent behavior is included with dedicated unit tests.
- PR #334 — cancellation during active file sends. Equivalent cancellation handling is included.
- PR #333 — Wi-Fi credential payload parsing. Equivalent handling is included with declared-length validation.
- Issue #268 — unbounded active log growth. Fixed with a 5 MiB runtime file cap.
- PR #420 — large transport/protobuf refactor. Deferred until the maintained fork is green because it is too broad to merge as a bugfix.

Upstream backlog classification for this maintenance pass:

- **Covered by equivalent fixes in this fork:** PRs #439/#433, #430, #418, #408, #404, #380, #334 and #333; issues #431, #429/#369, #423/#195, #421/#407, #268 and #440.
- **Covered by the Pixel receiver work integrated in `dev`:** issue #425 and the modern BLE/GATT receiver bootstrap path; older discovery reports #358/#311/#270 are smoke-test targets because they overlap with the same mDNS/BLE/TCP chain but do not provide enough evidence for separate code changes.
- **Linux packaged-smoke targets rather than speculative code fixes:** #422 (Wayland close/hide behavior), #357/#390/#328 (blank/partial WebKit rendering across mixed GPU/font/EGL causes), #307 (window reopen/GTK lifecycle), #426/#365 (package dependency variance), #325/#394 (generic transfer failures without a single reproducible root cause).
- **Feature requests, not Mode B blockers:** #435/#436 folder hierarchy, #432 multi-file UX, #434 AirDrop interoperability, #428/#363/#246 outbound clipboard/text, #388/#347/#310 custom device name, #387/#384/#224 trusted devices, #386/#383 sorting, #385/#382 tray indicators, #356/#395 localization, #370 visibility tray menu, #411 decorations, #405 update-check toggle, #375 sender image, #368 QR, #366 silent flag, #364 double-click tray, #354/#374 icon variants, #329 Flatpak, #326 Homebrew, #295/#412/#413/#414/#415/#416 Windows work, #264 dock behavior, #245/#182 notification actions.
- **Dependency/packaging PRs superseded or handled independently:** #424, #419, #417, #410, #402, #400, #399, #391, #371, #342, #276 and #241.
- **Deferred architecture work:** PR #420 transport/protobuf refactor. It is intentionally not merged into this hardening pass because PR #2 introduces the minimum transport abstraction required for modern Pixel receive without replacing the hardened state machine.

No remaining upstream item is treated as an automatic blocker solely because it is open; only reproducible defects that overlap the maintained Linux/Android scope can block Mode B.

## Modern Pixel receiver BLE/GATT bootstrap

Upstream issue #425 contains current 2026 evidence that modern Pixel Quick Share can leave Wi-Fi during receiver discovery. In that state, mDNS-only Linux receivers may never appear or may fail before opening the TCP connection.

The implementation is integrated in `dev` and ports the proven architecture from `martinalderson/rquickshare:feat/ble-receiver-connect-back` without overwriting the fork's hardened inbound code:

- receiver advertisement on service UUID `0xFEF3` using the same endpoint id and hostname as mDNS;
- visibility-aware connectable advertising with retry on transient BlueZ failures;
- Nearby GATT slot plus weave write/notify characteristics;
- bounded weave framing/reassembly and handshake validation;
- the existing hardened UKEY2 / Sharing receive state machine generalized over `AsyncRead + AsyncWrite`;
- a migratable BLE/TCP transport that preserves crypto keys and sequence counters;
- encrypted `UPGRADE_PATH_AVAILABLE` negotiation after UKEY2 establishment;
- validated plaintext `CLIENT_INTRODUCTION` / ACK on the new TCP socket;
- LAST_WRITE / SAFE_TO_CLOSE prior-channel handoff;
- LAN address selection that ignores Docker/VPN/tunnel interfaces;
- Wi-Fi-LAN transport swap for the actual payload;
- regression coverage for advertisement layout, weave handshake, framed lengths, LAN-interface filtering and BWU client-introduction validation.

The implementation now lives on `dev`. It must pass the final complete Mode B gate and real Pixel -> Linux smoke before `dev` may be fast-forwarded to `master`.


## Coverage audit

Coverage is measured explicitly instead of inferring quality from test count. The detailed Mode B scope, inventory, branch map, Positive/N1-N12 matrix, interaction matrix and coverage map are maintained in `docs/mode-b-test-plan.md` and `docs/mode-b-inventory.md`.

Historical baseline on the Pixel receiver branch (not current counts):

- **Rust:** 50 tests pass with `cargo test --all-features`.
- **Frontend:** 25 Vitest tests pass across 6 test files.
- **Total:** 75 automated tests before the separate package/live smoke checks.

Historical measured coverage before later protocol/text/folder/desktop changes (not final current coverage):

- **Rust core:** 30.37% line coverage, 29.75% function coverage, 31.19% region coverage.
- `hdl/inbound.rs`: 31.67% lines, up from 14.58% before the introduction/state tests.
- `hdl/outbound.rs`: 30.67% lines, up from 2.38% before file-preparation and consent/wire tests.
- `utils.rs`: 61.34% lines.
- `hdl/bwu.rs`: 95.83% lines.
- `hdl/migratable.rs`: 81.40% lines.
- **Frontend:** 46.18% line/statement coverage, 82.71% branch coverage and 53.33% function coverage.
- `SettingsModal.vue`: 98.05% lines, 90.62% branches, 100% functions.
- `ContentStatus.vue`: 92.75% lines.
- `Heading.vue`: 100% lines/functions/branches.
- `SideMenu.vue`: 98.21% lines.

The historical low percentages include OS/service orchestration: BlueZ/GATT lifecycle, mDNS, Tauri startup/window/tray and `HomePage.vue`. The original coverage jobs omitted the desktop Rust crate entirely. The offline continuation adds desktop measurement; its result must be recorded separately rather than inferred from core/Vue coverage.

No arbitrary repository-wide percentage gate is being introduced in this pass. The useful gate is regression coverage for validated defects plus live Linux/Android smoke for OS-bound behavior. Coverage should rise further as the post-smoke refactors create testable seams.

## Remaining audit work

- Validate the new CSP/freezePrototype behavior in the packaged Linux UI smoke test.
- Keep the regression coverage map aligned with fixes; current coverage includes mDNS compact records, P-256 normalization, inbound path/size guards, Wi-Fi credential parsing, clipboard fallback, log caps, BLE receiver advertisement, weave framing/handshake, BWU routing/introduction validation and LAN-interface filtering.
- Complete `dev` real-device validation: Pixel -> Linux BLE/GATT -> Wi-Fi-LAN receive, repeat receive without restarting the app, Linux -> Android send, and explicit FE2C/FEF3 BlueZ coexistence check.
- Perform packaged install/start/window/tray/send/receive smoke checks on the target Ubuntu/KDE/Wayland machine.
- Run the final Mode B gate after the hardware smoke. The pre-smoke Mode B run is already green and is not a substitute for this final post-smoke gate.
- Fast-forward `dev` to `master` only after the above is complete.

## Mode B exit criteria

The maintenance pass is complete only when all of the following are true:

1. Normal `dev` preflight is green.
2. New regression tests for fixed defects are green.
3. The complete automated Rust and frontend suite passes 10 consecutive runs.
4. The run is recorded against the final post-smoke head so no later production or test changes invalidate the evidence.
5. Manual package build succeeds.
6. Linux application smoke test succeeds.
7. Android -> Linux receive path succeeds.
8. Linux -> Android send path succeeds.
9. No unresolved high-severity security/reliability finding remains in the audit.
10. `dev` can be fast-forwarded to `master` without merge noise.

## Refactoring audit

Refactoring is required, but the work is split by risk.

### Do after real-device smoke, before long-term maintenance work

1. **Extract a shared secure-session layer from inbound/outbound.**
   - `core_lib/src/hdl/inbound.rs` is about 2,059 lines.
   - `core_lib/src/hdl/outbound.rs` is about 1,403 lines.
   - Both implement parallel UKEY2 / D2D crypto responsibilities: key exchange finalization, encrypted frame send/decrypt, HMAC handling, sequence counters, keepalive and frame I/O.
   - This duplication makes protocol/security fixes easy to apply on one direction and miss on the other.
   - Recommended target: a shared `SecureSession`/crypto transport helper that owns derived keys, sequence counters, secure-message encode/decode and common frame I/O.
   - **Do not perform this extraction before Pixel smoke** because it touches the most sensitive protocol path.

2. **Split inbound state-machine responsibilities.**
   - `process_offline_frame` is roughly 250 lines.
   - `process_introduction` is roughly 200 lines.
   - `do_bandwidth_upgrade` is roughly 130 lines.
   - The current module combines connection negotiation, UKEY2, secure framing, consent, payload validation, filesystem output and bandwidth upgrade.
   - Recommended split after smoke: `secure_session.rs`, `inbound_transfer.rs`, `bwu.rs`/routing helpers, keeping one explicit state-machine coordinator.

3. **Split outbound transfer construction from secure transport.**
   - `process_consent` is roughly 230 lines and mixes consent state, file metadata, file I/O and payload transmission.
   - Text sending is implemented; the earlier two TODO locations are obsolete. Accepted/rejected/cancelled text state-machine coverage remains distinct from helper coverage.
   - Recommended split: transfer plan/payload source abstraction separate from crypto/channel transport.

### Medium-priority cleanup

4. **Break up GATT session orchestration.**
   - `gatt.rs` is roughly 492 lines; `weave_session` alone is about 227 lines.
   - Separate framing/reassembly, connection handshake and duplex bridge lifecycle once the Pixel path is proven on hardware.
   - Keep the current bounded queue, disconnect handling and BLE->TCP migration invariants covered during any extraction.

5. **Separate utility domains.**
   - `utils.rs` currently mixes mDNS encoding, P-256 normalization, HKDF, random generation, download-directory lookup and LAN interface selection.
   - Recommended modules: `mdns_codec`, `crypto_utils`, `network_utils`, `filesystem_utils`.
   - This is maintainability work, not a release blocker.

6. **Split desktop orchestration from UI rendering.**
   - `HomePage.vue` is roughly 382 lines and currently combines rendering with settings bootstrap, notification permission, transfer/endpoint/visibility listeners and drag/drop registration.
   - Recommended extraction after smoke: `useTransferEvents`, `useWindowDrop` and `useAppSettings` composables plus smaller transfer/device components.
   - `app/main/src-tauri/src/main.rs` is roughly 387 lines; tray/window lifecycle and receiver-task wiring can become separate modules later.
   - These are UX/maintainability improvements, not protocol blockers.

### Low-risk refactoring already completed

- Shared frame/payload size limits and validation were moved out of inbound/outbound into `core_lib/src/protocol.rs`.
- Duplicate payload-size regression tests were collapsed into one shared test.
- Inbound file-introduction validation was made transactional: the complete file set is validated/prepared before transfer state is mutated, avoiding partial state after malformed metadata.
- Outbound file metadata/file-handle preparation was extracted from the protocol state machine into a dedicated preparation helper with temp-file regression coverage.
- Both preparation helpers now return named result structs instead of opaque multi-value tuples.
- A poisoned `CUSTOM_DOWNLOAD` lock now recovers the configured path instead of silently dropping back to the default directory.
- BWU routing already has its own `BwuRouter` module instead of remaining embedded in the inbound state machine.
- BLE/TCP transport switching already has a small `MigratableStream` abstraction.

### Refactoring rule for this maintenance pass

No broad architectural refactor should be merged solely to improve style before real Pixel smoke. Before the smoke test, only pure helpers, duplicated validation, tests and isolated infrastructure may be extracted. Protocol state-machine refactoring starts only after the current behavior is proven end-to-end.

**Audit conclusion:** no additional architectural refactor is required before the real-device smoke. The next mandatory refactor for maintainability is the shared secure-session extraction, but it should begin only after the current Pixel send/receive behavior is proven so protocol regressions can be distinguished from structural changes.

## 2026-10-12: accepted compact tray panel and context-menu direction

The user selected the tray-panel-plus-native-notifications option in [product-direction.md](product-direction.md), explicitly rejected the current oversized drop-zone window, and requested a file-manager context-menu sender. This is accepted product scope; do not ask the A/B desktop-interaction question again.

[native-desktop-design.md](native-desktop-design.md) records the current Tauri/core coupling, proposed service API/security boundary, Dolphin adapter, phased implementation/validation and rollback points. The remaining review is the path-based versus descriptor-based file-selection contract. No native implementation, package registration or settings migration has started. The historical protocol-refactor guardrail above still applies; the newly authorized desktop migration must keep protocol behavior separable from shell changes.

Security follow-up: the user questioned the path-based recommendation. Source inspection identified a check/open race between candidate metadata inspection and later `File::open` in outbound preparation; no exploit was executed. The design now recommends handle/capability input for the durable service boundary and records its remaining content-mutation, folder-enumeration, resource-budget and authorization limitations. This is a revised recommendation pending approval, not an accepted architecture change or completed fix.

Subsequent user decision: descriptor/capability input approved, with a cross-project preference to fix brittle internal workarounds immediately and ask before substantial refactors. The global agent guide records that preference. [native-file-input.md](native-file-input.md) is the implemented file-boundary handoff: safe Linux resource acquisition, an in-process-only core payload, and bounded positioned reads. Implementation `c8d537e` passed all 258 checks ten consecutive times; 62 methods are new, and 18 copied-source mutants were caught by assertions. No native daemon/UI/context-menu installation or physical peer test occurred. The legacy Tauri path route remains a rollback path with its known acquisition race; it must not be used by the new service.
