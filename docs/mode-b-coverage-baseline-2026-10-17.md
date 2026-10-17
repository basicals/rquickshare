# Mode B exact-source ten-run and file coverage evidence — 2026-10-17

## Scope and evidence source

MODE B Stage 11 automated stability and instrumented snapshot only,
source `f8e5d2d01d62200fcfd570c7bdf1f9051874845d`. GitHub run:
https://github.com/morozhenko-apps/rquickshare/actions/runs/39066477128
Full 10x job `117495109038`, Native GTK job `117499392551`,
coverage job `117499829172`. All three job conclusions:
**success**. Verified from full logs, not just the workflow badge.

The independent ordinary Preflight for the **same source**,
https://github.com/morozhenko-apps/rquickshare/actions/runs/39066477112,
also completed **5/5** success (stable 696 once; eight
race-sensitive methods 10x, 80 invocations; integration 4+4).
This normal profile does not replace the full Mode B execution.

## Exact observed repetitions

| Suite | Attempts and results | Verification |
|---|---|---|
| Core Rust library | 10 × 704 passed, zero failed/ignored | Ten corresponding 704-test summaries |
| Core Rust integration | 10 × (4 + 4) passed, zero failed | Twenty four-test summaries |
| Legacy Tauri Rust | 10 × 10 passed, zero failed | Ten matching summaries |
| Vue/Vitest frontend | 10 successful invocations, 8 test files in each | Ten frontend markers / fail-fast workflow success |
| Offline Python publication/contracts | 10 × 54 passed, zero failed | Ten `Ran 54 tests` summaries, pinned `scripts/requirements-r2.txt` venv |
| Native GTK/X11 D-Bus | 10 × 52 passed, zero failed | Ten numbered markers and ten 52-test summaries |

The GTK verification includes Ubuntu 26.04 container, GTK4
source formatting/strict Clippy, compiled preview helper and
isolated display/bus. This is an offline X11 test harness,
**not** actual KDE/Wayland operation, a physical Android/Windows
peer test or an installed-package migration.

## Source-exact measured coverage

Coverage job completed after **both** ten-run jobs succeeded,
from the exact same workflow source. These are actual
`cargo llvm-cov`/Vitest V8 summaries, **not** a guarantee of
100% current product behavior or coverage of every source branch.
The Rust llvm-cov report columns are regions/functions/lines;
this invocation provides no meaningful Rust branch measure.

| Artifact / instrumentation | Region or statements | Functions | Lines | Other |
|---|---:|---:|---:|---|
| `core_lib` cargo llvm-cov all-features | 86.20% regions | 85.30% | **89.35%** | 37,042 regions; 26,281 lines; 2,800 missed lines |
| Vue/Vitest V8 src | 56.00% statements | 75.86% | **56.00%** | 88.04% branches |
| Legacy `app/main/src-tauri` llvm-cov | 24.98% regions | 19.29% | **23.86%** | 1,577 regions; 1,052 lines; 801 missed lines |
| Native `native_desktop` GTK4 — separate focused run | 65.35% regions | 63.09% | **67.00%** | 52/0 one-shot instrumented GTK cases; 33-file per-source report; still no Rust branch coverage |

The core aggregate includes instrumented Rust test modules
and `include!` fixture files such as
`hdl/inbound/receive_reliability.rs`. Thus **89.35%** is
coverage of the llvm-cov inputs, not a rigorously isolated
production-only or branch-coverage percentage. Likewise Vue's
56% denominator includes entrypoint/template/stub files, and
legacy Tauri's low result includes command/runtime startup
paths not exercised by headless unit tests.

### Core Rust llvm-cov reported file-by-file map

These are source file numbers reported verbatim by the
instrumentation for the 2026-10-17 head; short source paths
are relative to `core_lib/src` unless otherwise noted. The
`hdl/inbound/receive_reliability.rs` and
`hdl/outbound/metadata.rs` files are test-only included fixtures
and should not be treated as independent product modules.

| Source file | Lines | Functions | Regions | Missed lines |
|---|---:|---:|---:|---:|
| `bin.rs` | 0.00% | 0.00% | 0.00% | 22 |
| `byte_payload.rs` | 98.89% | 93.33% | 98.51% | 1 |
| `clipboard_image.rs` | 97.26% | 90.00% | 92.31% | 4 |
| `errors.rs` | 0.00% | 0.00% | 0.00% | 4 |
| `file_reader.rs` | 96.88% | 100.00% | 98.69% | 5 |
| `file_source.rs` | 99.65% | 100.00% | 96.51% | 1 |
| `hdl/ble.rs` | 5.59% | 11.11% | 5.10% | 135 |
| `hdl/ble_monitor.rs` | 21.31% | 40.00% | 15.87% | 48 |
| `hdl/blea.rs` | 54.96% | 65.62% | 57.66% | 159 |
| `hdl/bwu.rs` | 99.56% | 100.00% | 99.80% | 1 |
| `hdl/frame_header.rs` | 100.00% | 100.00% | 100.00% | 0 |
| `hdl/gatt.rs` | 27.45% | 44.68% | 25.54% | 452 |
| `hdl/gatt/slot.rs` | 100.00% | 100.00% | 100.00% | 0 |
| `hdl/inbound.rs` | 90.48% | 69.89% | 87.25% | 430 |
| `hdl/inbound/receive_reliability.rs` | 99.82% | 100.00% | 99.80% | 2 |
| `hdl/mdns.rs` | 53.29% | 54.17% | 43.43% | 135 |
| `hdl/mdns_discovery.rs` | 0.00% | 0.00% | 0.00% | 90 |
| `hdl/migratable.rs` | 100.00% | 100.00% | 100.00% | 0 |
| `hdl/mod.rs` | 80.00% | 100.00% | 62.50% | 1 |
| `hdl/outbound.rs` | 94.21% | 76.32% | 92.01% | 182 |
| `hdl/outbound/metadata.rs` | 99.47% | 100.00% | 99.67% | 2 |
| `hdl/receiver_header.rs` | 92.16% | 75.00% | 94.64% | 8 |
| `identity.rs` | 97.59% | 96.15% | 97.21% | 4 |
| `lib.rs` | 51.13% | 71.43% | 43.65% | 152 |
| `managed.rs` | 92.31% | 92.86% | 90.82% | 12 |
| `manager.rs` | 96.50% | 94.00% | 96.42% | 18 |
| `manager/session.rs` | 98.35% | 100.00% | 95.62% | 4 |
| `memory_source.rs` | 97.56% | 100.00% | 97.62% | 2 |
| `native_daemon.rs` | 87.69% | 100.00% | 76.70% | 8 |
| `peer_authentication.rs` | 98.92% | 97.14% | 98.53% | 3 |
| `prepared_source.rs` | 100.00% | 100.00% | 100.00% | 0 |
| `preview_payload.rs` | 98.31% | 90.91% | 98.00% | 2 |
| `protocol.rs` | 96.00% | 75.00% | 96.30% | 1 |
| `receive.rs` | 94.08% | 83.33% | 90.52% | 9 |
| `received_files.rs` | 95.43% | 96.77% | 92.58% | 10 |
| `receiver_trace.rs` | 98.51% | 100.00% | 95.18% | 1 |
| `service.rs` | 98.48% | 100.00% | 96.11% | 11 |
| `service/activation.rs` | 95.73% | 100.00% | 88.37% | 7 |
| `service/batch.rs` | 100.00% | 100.00% | 97.08% | 0 |
| `service/bus.rs` | 89.73% | 89.52% | 85.67% | 162 |
| `service/bus/batch.rs` | 85.19% | 75.00% | 81.58% | 4 |
| `service/bus/files.rs` | 95.74% | 100.00% | 93.22% | 2 |
| `service/bus/state.rs` | 98.64% | 100.00% | 96.80% | 7 |
| `service/bus/text.rs` | 96.88% | 87.50% | 95.80% | 3 |
| `service/bus/transport.rs` | 89.74% | 80.00% | 87.17% | 12 |
| `service/client.rs` | 93.48% | 98.72% | 88.93% | 52 |
| `service/client/controller.rs` | 79.36% | 73.21% | 79.27% | 110 |
| `service/client/desktop.rs` | 92.07% | 84.00% | 87.21% | 61 |
| `service/client/files.rs` | 87.58% | 81.25% | 80.53% | 20 |
| `service/client/journal.rs` | 82.47% | 100.00% | 86.76% | 17 |
| `service/client/notices.rs` | 94.87% | 100.00% | 91.94% | 4 |
| `service/client/settings.rs` | 94.87% | 100.00% | 90.32% | 4 |
| `service/client/state.rs` | 96.20% | 97.92% | 91.87% | 22 |
| `service/client/text.rs` | 95.83% | 100.00% | 90.59% | 5 |
| `service/client/visibility.rs` | 93.75% | 100.00% | 88.41% | 3 |
| `service/client/watch.rs` | 92.89% | 95.65% | 85.30% | 16 |
| `service/journal.rs` | 85.08% | 95.65% | 82.03% | 54 |
| `service/native_settings.rs` | 96.06% | 97.37% | 91.64% | 17 |
| `service/notices.rs` | 98.90% | 100.00% | 97.44% | 1 |
| `service/operations.rs` | 95.97% | 100.00% | 93.70% | 11 |
| `service/private_file.rs` | 95.35% | 100.00% | 93.06% | 4 |
| `service/process.rs` | 79.85% | 73.81% | 78.22% | 79 |
| `service/retention.rs` | 98.33% | 100.00% | 94.79% | 1 |
| `service/runtime.rs` | 86.96% | 85.71% | 82.40% | 90 |
| `service/settings_service.rs` | 89.02% | 72.41% | 87.73% | 56 |
| `service/uploads.rs` | 99.21% | 100.00% | 96.55% | 3 |
| `service/visibility_service.rs` | 99.15% | 100.00% | 99.53% | 1 |
| `service/wakeup.rs` | 99.57% | 100.00% | 98.58% | 1 |
| `sessions.rs` | 98.16% | 97.73% | 97.03% | 8 |
| `sessions/details.rs` | 97.42% | 100.00% | 98.20% | 5 |
| `sessions/tasks.rs` | 97.96% | 100.00% | 93.51% | 1 |
| `sessions/text.rs` | 100.00% | 100.00% | 100.00% | 0 |
| `settings.rs` | 94.44% | 83.33% | 94.21% | 4 |
| `utils.rs` | 91.76% | 87.50% | 91.86% | 28 |
| `visibility.rs` | 96.84% | 96.67% | 96.24% | 6 |

### Legacy desktop Tauri coverage map

These files are under `app/main/src-tauri/src`.

| Source file | Lines | Functions | Regions | Missed lines |
|---|---:|---:|---:|---:|
| `cmds/change_download_path.rs` | 0.00% | 0.00% | 0.00% | 13 |
| `cmds/change_visibility.rs` | 60.56% | 45.45% | 68.22% | 28 |
| `cmds/clipboard_image.rs` | 49.49% | 35.29% | 41.89% | 50 |
| `cmds/discovery.rs` | 0.00% | 0.00% | 0.00% | 23 |
| `cmds/get_hostname.rs` | 0.00% | 0.00% | 0.00% | 3 |
| `cmds/open_download_destination.rs` | 65.19% | 40.00% | 63.25% | 55 |
| `cmds/send_payload.rs` | 0.00% | 0.00% | 0.00% | 14 |
| `cmds/send_to_rs.rs` | 0.00% | 0.00% | 0.00% | 9 |
| `logger.rs` | 29.66% | 55.56% | 32.03% | 83 |
| `main.rs` | 0.00% | 0.00% | 0.00% | 250 |
| `notification.rs` | 10.66% | 12.00% | 13.41% | 176 |
| `store.rs` | 0.00% | 0.00% | 0.00% | 97 |

### Vue/Vitest coverage priority details

Measured summary: 56% lines/statements, 88.04% branches,
75.86% functions. Named file rows in the reporter include:
`App.vue` 0%, `HomePage.vue` 0%,
`main.ts` 0%, `ItemSide.vue` 0%,
`ContentStatus.vue` 99.13%,
`Heading.vue` 100%, `SettingsModal.vue` 98.69%,
`SideMenu.vue` 86.84%,
`clipboard.ts` 100%, `network.ts` 100%,
`utils.ts` 62.72%, `useToastStore.ts` 16.66%.
The generated declaration files `env.d.ts` and
`useCommsHelper.d.ts` appear in the report but are not
runnable product branches. It would be wrong to chase
100% in declarations or to mistake a 0% app bootstrap
for a failed tested clipboard protocol.

## Prioritization by missed production lines and operational risk

| Priority | Production surface | Reported missed lines / coverage | Recommended next action |
|---|---|---:|---|
| 1 | `hdl/gatt.rs` | 452 / 27.45% | Separate deterministic protocol seams from physical BlueZ/GATT/mDNS; do not invent a fake adapter |
| 2 | `hdl/inbound.rs` | 430 / 90.48% | Inspect uncovered branches; add only explicit contract tests backed by a branch map |
| 3 | `hdl/outbound.rs` | 182 / 94.21% | Inspect uncovered branches; add only explicit contract tests backed by a branch map |
| 4 | `hdl/blea.rs` | 159 / 54.96% | Separate deterministic protocol seams from physical BlueZ/GATT/mDNS; do not invent a fake adapter |
| 5 | `hdl/ble.rs` | 135 / 5.59% | Separate deterministic protocol seams from physical BlueZ/GATT/mDNS; do not invent a fake adapter |
| 6 | `hdl/ble_monitor.rs` | 48 / 21.31% | Separate deterministic protocol seams from physical BlueZ/GATT/mDNS; do not invent a fake adapter |
| 7 | `hdl/mdns.rs` | 135 / 53.29% | Separate deterministic protocol seams from physical BlueZ/GATT/mDNS; do not invent a fake adapter |
| 8 | `hdl/mdns_discovery.rs` | 90 / 0.00% | Separate deterministic protocol seams from physical BlueZ/GATT/mDNS; do not invent a fake adapter |
| 9 | `lib.rs` | 152 / 51.13% | Inspect uncovered branches; add only explicit contract tests backed by a branch map |
| 10 | `service/bus.rs` | 162 / 89.73% | Inspect uncovered branches; add only explicit contract tests backed by a branch map |
| 11 | `service/runtime.rs` | 90 / 86.96% | Inspect uncovered branches; add only explicit contract tests backed by a branch map |
| 12 | `service/client/controller.rs` | 110 / 79.36% | Inspect uncovered branches; add only explicit contract tests backed by a branch map |

The highest coverage holes in `hdl/gatt.rs`, `ble.rs`,
`mdns_discovery.rs` and `ble_monitor.rs` concentrate
in BLE/BlueZ/mDNS boundary code. Automated test seams may
increase meaningful coverage only where the logic can be
isolated without altering protocol behavior. Physical cold
discovery, slot0/BWU, integration with installed BlueZ and
Android/Google Windows are still **BLOCKED ON PHYSICAL
ACCEPTANCE**, not proofs that these modules are broken.

The more exercised inbound/outbound core paths have
90.48%/94.21% measured lines, but remain below 100% and
must still pass the full Mode B public-method/branch and
N1–N12 inventory; a high percentage alone cannot prove
no regressions.

## Completion gate and next bounded tasks

- [x] Full same-source four core/desktop/frontend/Python
  suites **10/10**, and native GTK **10/10**.
- [x] Same-source instrumented core, Tauri and frontend
  baseline with per-source file map.
- [x] Normal Preflight stable 1x / eight risk-oriented
  tests 10x with 5/5 complete jobs.
- [x] Instrument current **native GTK** file/function/line
  coverage in its isolated real X11/D-Bus runner; **67.00% lines,
  63.09% functions, 65.35% regions**, 52/0 tests,
  [33-file map](native-gtk-coverage-baseline-2026-10-17.md).
- [ ] Classify uncovered production code as deterministic
  testable vs hardware-only, with explicit per-function
  branch/N1-N12 matrix, before adding any tests.
- [ ] Full global public-method, business rule, N1-N12,
  boundary, interaction and mutation checks (Stage 12)
  still require evidence beyond percent coverage.
- [ ] Installed native29/native30 package, release
  signing/upgrade, real KDE/Wayland service/tray,
  Pixel/Windows BLE-to-TCP/send/receive acceptance;
  no user machine was touched by these GitHub jobs.

**Overall MODE B is not complete**, even though the strict
ten-consecutive-automated-suites Stage 11 stability subgate
has **passed** on the exact current code snapshot. Later
production/test modifications require their own revalidation;
doc-only commits do not change the tested source.
