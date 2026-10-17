# Native GTK instrumented file coverage — 2026-10-17

## Exact evidence and purpose

A single **instrumented** Native GTK isolated test run has now
been executed under Ubuntu 26.04 with the same real GTK4,
X11/Xvfb, D-Bus, GSK cairo and workspace TMPDIR harness
used by the separate **successful 10× Native GTK**
stability gate. Workflow-only source
`3bb41cb6647820ff82ce2ec70f6943e7d7b11c9c`.
GitHub
[Native GTK instrumented coverage 39071770811](https://github.com/morozhenko-apps/rquickshare/actions/runs/39071770811)
job `117510700242`: **SUCCESS**.
Log confirms `test result: ok. 52 passed; 0 failed; 0
ignored` and a real nonempty llvm-cov per-file `TOTAL`
table; the workflow itself fails if either proof is absent.
It uses only read-only GitHub contents and does not start
user services or real peer discovery.

Separate original
[full-mode-b 10× run](https://github.com/morozhenko-apps/rquickshare/actions/runs/39066477128)
had already passed Core/Tauri/Vue/Python ten consecutive full
suite repetitions and Native GTK **52×10** before this extra
one-shot instrumentation. Neither workflow covers a genuine
KDE/Wayland desktop/notification, physical Android/Windows,
or actual BlueZ lifecycle.

## Measured aggregate

| Type | Total | Missed | Coverage |
|---|---:|---:|---:|
| LLVM instrumented regions | 16,927 | 5,865 | **65.35%** |
| Instrumented functions | 875 | 323 | **63.09%** |
| Instrumented lines | 10,549 | 3,481 | **67.00%** |
| Rust branch coverage | Not reported | Not reported | **Not measured** |

These are llvm-cov measured-input results, and they include
test-specific source `test_bus.rs` and test portions embedded
in normal modules. This is not a proof that 67% of user-visible
GUI behavior, KDE flows or all executable production branches
are tested. 10,549 lines is the instrumented reporter's
denominator rather than the number of physical lines in
the `native_desktop/src` tree.

## Per-source file/function/region coverage map

Source paths below are relative to `native_desktop/src`.

| Source | Instrumented lines | Missed lines | Covered lines | Functions | Regions |
|---|---:|---:|---:|---:|---:|
| `card_policy.rs` | 340 | 2 | 99.41% | 100.00% | 96.40% |
| `card_view.rs` | 58 | 0 | 100.00% | 100.00% | 100.00% |
| `cli.rs` | 327 | 20 | 93.88% | 89.66% | 92.81% |
| `desktop_guard.rs` | 241 | 12 | 95.02% | 97.22% | 90.05% |
| `ingress.rs` | 273 | 69 | 74.73% | 80.77% | 76.98% |
| `lifecycle.rs` | 43 | 8 | 81.40% | 77.78% | 82.67% |
| `login.rs` | 380 | 18 | 95.26% | 97.37% | 89.26% |
| `main.rs` | 505 | 439 | 13.07% | 14.89% | 11.61% |
| `panel.rs` | 1416 | 581 | 58.97% | 47.47% | 60.23% |
| `panel/backdrop.rs` | 42 | 3 | 92.86% | 100.00% | 91.04% |
| `panel/compose.rs` | 395 | 157 | 60.25% | 53.66% | 60.29% |
| `panel/countdown.rs` | 152 | 10 | 93.42% | 83.33% | 92.49% |
| `panel/idle.rs` | 226 | 31 | 86.28% | 70.83% | 87.73% |
| `panel/image.rs` | 368 | 142 | 61.41% | 55.56% | 63.08% |
| `panel/incoming_preview.rs` | 69 | 21 | 69.57% | 85.71% | 67.27% |
| `panel/menu.rs` | 369 | 96 | 73.98% | 55.56% | 72.33% |
| `panel/movement.rs` | 287 | 128 | 55.40% | 66.67% | 54.31% |
| `panel/outbound.rs` | 407 | 214 | 47.42% | 50.00% | 45.09% |
| `panel/presentation.rs` | 198 | 52 | 73.74% | 50.00% | 68.49% |
| `panel/results.rs` | 208 | 73 | 64.90% | 65.22% | 67.68% |
| `panel/reveal.rs` | 276 | 246 | 10.87% | 20.00% | 10.50% |
| `panel/rows.rs` | 813 | 319 | 60.76% | 54.35% | 63.43% |
| `panel/settings.rs` | 1014 | 362 | 64.30% | 56.67% | 64.23% |
| `panel/surface.rs` | 36 | 0 | 100.00% | 100.00% | 94.64% |
| `panel/text.rs` | 262 | 131 | 50.00% | 40.74% | 54.15% |
| `panel/transfers.rs` | 279 | 71 | 74.55% | 48.00% | 73.51% |
| `panel/tray.rs` | 384 | 33 | 91.41% | 80.00% | 91.82% |
| `placement.rs` | 131 | 0 | 100.00% | 100.00% | 98.17% |
| `portal.rs` | 235 | 41 | 82.55% | 76.19% | 72.81% |
| `preview.rs` | 180 | 138 | 23.33% | 35.29% | 17.57% |
| `test_bus.rs` | 40 | 0 | 100.00% | 100.00% | 100.00% |
| `tray.rs` | 258 | 31 | 87.98% | 82.50% | 81.47% |
| `tray/menu.rs` | 337 | 33 | 90.21% | 95.12% | 88.40% |

## Next risk/impact ordering (not percentage chasing)

The largest measured gaps, sorted by missed lines, are:

| File | Missed lines | Covered lines | Risk and next method |
|---|---:|---:|---|
| `panel.rs` | 581 | 58.97% | Panel event/controller wiring; distinguish genuine use from X11 backend startup |
| `main.rs` | 439 | 13.07% | GTK/Wayland entrypoint and lifecycle, primarily actual application smoke |
| `panel/settings.rs` | 362 | 64.30% | Settings mutation, conflict and error/recovery flow; inspect branch map first |
| `panel/rows.rs` | 319 | 60.76% | Rows and transfer status, bounded display and stale session state |
| `panel/reveal.rs` | 246 | 10.87% | High-value open-file/open-directory capability; isolate pure routing and ownership safety |
| `panel/outbound.rs` | 214 | 47.42% | Actual selected recipient, explicit consent and progress/error/cancel, potentially async integration |
| `panel/compose.rs` | 157 | 60.25% | Classify pure policy branches vs widget/OS-bound behavior before introducing fixtures |
| `panel/image.rs` | 142 | 61.41% | Classify pure policy branches vs widget/OS-bound behavior before introducing fixtures |
| `preview.rs` | 138 | 23.33% | Classify pure policy branches vs widget/OS-bound behavior before introducing fixtures |
| `panel/text.rs` | 131 | 50.00% | Classify pure policy branches vs widget/OS-bound behavior before introducing fixtures |
| `panel/movement.rs` | 128 | 55.40% | Classify pure policy branches vs widget/OS-bound behavior before introducing fixtures |
| `panel/menu.rs` | 96 | 73.98% | Classify pure policy branches vs widget/OS-bound behavior before introducing fixtures |
| `panel/results.rs` | 73 | 64.90% | Classify pure policy branches vs widget/OS-bound behavior before introducing fixtures |
| `panel/transfers.rs` | 71 | 74.55% | Classify pure policy branches vs widget/OS-bound behavior before introducing fixtures |

The most under-tested high-impact UI paths include
`panel/reveal.rs` (10.87% lines; 246 missed),
`panel/outbound.rs` (47.42%; 214 missed),
`panel/settings.rs` (64.30%; 362 missed)
and `panel/compose.rs` (60.25%; 157 missed).
A lower percentage alone is not evidence of a bug;
pure rendering/validation branches may be eligible for
deterministic checks without mocking product logic, whereas
lifecycle/Wayland/portal/actual file-reveal actions must
retain physical acceptance.

## Updated global Mode B gate

- [x] Full same-source legacy four-suite Rust/Vue/Python **10/10**.
- [x] Native GTK isolated native 52-case **10/10**.
- [x] Instrumented core Rust, Vue, legacy desktop Tauri and
  now native GTK4 aggregate plus per-source measurements.
- [ ] **Per-public-method/branch** map and every relevant
  positive/N1–N12/interaction variant to Stage 12 threshold.
- [ ] Targeted meaningful mutation/reliability coverage for
  uncovered GTK/integration paths.
- [ ] Actual installed near-clock tray, complete session
  lifetime, KDE/Wayland, BlueZ and Google Android/Windows
  interoperability using physical peers.
- [ ] Final post-hardware full 10× execution at final
  production SHA and release/upgrade/signing gates.

The Native GTK test run is one-shot instrumentation,
not an eleventh ten-run proof; all source modifications
after the previous 10× evidence must be revalidated.
