# Native file input: approved descriptor boundary

_2026-10-12. Mode A. Status: planned before implementation._

## Scope and decisions

The user approved descriptor/capability input in [native-desktop-design.md](native-desktop-design.md). This first offline slice implements reusable Linux resource acquisition, a core handle-based outbound entrypoint and position-independent bounded file reads. It does not install/start a daemon, change the wire protocol, migrate settings or create a UI. Service orchestration, D-Bus publication, GTK and file-manager packaging follow separately.

Keep the legacy serialized `OutboundPayload` variants intact. Add an in-process-only owned-selection variant, excluded from Serde/TypeScript so a JSON caller cannot mint file handles. The daemon will construct this variant from validated D-Bus FDs; desktop path/URI adapters will acquire resources first. No path-opening service API is introduced.

Use a Linux-only `core_lib::file_source` module with safe rustix wrappers (no application unsafe code). The core already supports Unix file handles. Other platforms retain their legacy input surface; do not claim their native descriptor integration is complete.

### Resource contract

- `OpenedFile` owns a read-only regular file, a validated relative destination name, and the size observed at acquisition. Reject writable/O_PATH handles, directories/special files, unsafe names and protocol-unrepresentable sizes. Names do not identify paths to reopen.
- `OpenedSelection` owns a nonempty bounded collection. Acquire local roots with `openat2`, no symlinks/magic links; enumerate folders relative to held directory handles with `BENEATH` confinement. Validate every acquired object, never rely on a prior directory-entry type check. Renaming/replacing a root name cannot redirect later descendant opens.
- Initial acquisition limits are explicit caller-supplied budgets: files, directory entries, nesting depth and aggregate bytes. A zero budget is invalid. Default native budgets: 256 files, 4096 visited entries, depth 64, 1 TiB total. These are resource limits for this foundation, not Google limits or a final parity claim. The later service must also enforce aggregate budgets across selections. Exhaustion fails transactionally; do not reopen paths as a fallback.
- Folder enumeration uses a held directory FD per active depth, not one FD per descendant directory. At most the selected-file budget remains held for a successful selection. Release on error/drop; do not delete sources. Root-name collisions and duplicate relative destination paths fail explicitly.
- Acquisition errors are atomic: missing/unreadable/unsafe sources invalidate the selection; no silent partial send. Symlinks and special files invalidate native folder selections. Legacy path preparation's existing skip behavior remains until the desktop migration replaces it.
- `read_chunk_at` uses positioned reads from an explicit byte offset, independent of shared open-file position. Never read past the acquired size; premature EOF or a changed size is an error, not a successful zero-byte chunk. Same-size concurrent changes remain possible: no immutable snapshot claim.
- Source metadata/errors must not dump absolute paths or file contents in the new resource layer's Debug/errors. This does not certify existing application logs as private.
- Legacy wire metadata remains equivalent. The existing path route still exists for rollback; its ancestor check/open race is not declared fixed merely because the new handle route exists. New native operations must use the handle route exclusively.

## Plan, risk and rollback

1. F0: record approval, module/type contracts, inventory and limits; persist general maintenance preference. Documentation commit.
2. F1: implement Linux resource acquisition and validation, deterministic filesystem tests, bounded positioned reads. Commit independently.
3. F2: integrate owned-selection payload into outbound metadata/transfer handling. Reuse a bounded positioned-read helper in the existing sender to fix growth/truncation/offset behavior; zero-byte reads before the advertised end currently make no progress. Preserve crypto/framing, originals and text/ephemeral paths. Commit separately.
4. F3: run format/Clippy, full existing suites plus new tests ten consecutive times; update CI so resource tests run with core tests, record results/mutations/remaining work. Commit handoff docs.

Risks: Linux kernel support for openat2 (fail closed on unsupported/restricted calls), symlinked ingress paths intentionally rejected, source contents remain mutable, selection resource budgets require future service-wide accounting, and actual Google peer behavior still requires hardware. No settings/wire migration. Roll back integration then resource commits; existing Tauri remains usable. Do not delete the old app until native parity checks pass.

## Complete scoped artifact / branch inventory before tests

| Artifact | Executable branches / rules | Risk / level | Planned scenarios |
|---|---|---|---|
| Relative-name validation | empty; absolute; dot/dot-dot; separator/backslash; NUL/control; invalid UTF-8 ingress; valid Unicode/nested components | Traversal/privacy; unit | Table-driven valid/invalid names and nested manifests |
| Budget validation/accounting | zero each field; file/entry/depth/byte boundary; overflow; duplicate names; empty selection | Resource abuse; unit/integration | Each budget at limit and above, atomic failure/drop, collision |
| FD adoption | read-only regular; writable; O_PATH; directory; special file; zero-length; size outside protocol | Security; integration | Real descriptors for every local type/access branch; sparse boundary files where supported |
| Root acquisition | regular/directory; missing; unsafe root name; symlink root/ancestor; open error; no roots | Confinement; integration | Real filesystem paths, spaces/Unicode, empty dirs, symlink/missing sources |
| Folder walk | deterministic sorting; file/dir; unsafe name; duplicate roots; symlink/special; confinement; nesting/entry limits; error rollback | Confinement/lifecycle; integration | Nested trees, replaced root/ancestor via deterministic seams (no sleeps), symlink target outside, FIFO, bounded resources |
| Positioned reads | offset before/at/after end; empty buffer; short read; EOF; size changed; I/O error; offset interference | Integrity/liveness; unit/integration | Original contents after rename/replacement, shared-offset changes, append/truncate, zero-sized files, explicit EOF |
| Core handle metadata | new variant; name/type/parent/size; per-file payload IDs; total bytes; original ownership; JSON cannot construct | Interoperability/security; core integration | Metadata exact assertions, held handles survive name removal, serialization rejection; legacy generated bindings unchanged |
| Sender bounded chunk helper | remaining less/equal/greater than buffer; offset overflow/negative; zero progress; positioned offset | Liveness/integrity; unit/integration | Boundaries, append/truncate, cancellation unaffected; no protocol mocks |

External transport is not mocked as the subject: acquisition and reader tests exercise real disposable local files, kernel descriptors and production functions. Filesystem storage is necessary for these integration contracts; no real network, clocks, sleeps or Bluetooth. Tests use isolated temp directories and explicit mutation points, not probabilistic race loops.

## Negative and interaction matrix

Every inventory row is planned for Positive plus applicable categories below; non-applicable categories are explicit. Full protocol branch coverage is not part of this finite resource slice.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Names/budgets | ? | ? | ? | ✖ local | ✖ stateless | ✖ stateless | ? malformed names | ? traversal | ? Unicode | ✖ no UI | ? redaction | ✖ no billing | ✖ no I/O |
| FD/root/walk | ? | ? | ? | ✖ local | ? duplicate roots | ? replacement | ? stale paths | ? symlinks/types/modes | ? Unicode | ? drop cleanup | ? redaction | ✖ no billing | ? missing/read errors |
| Positioned reads | ? | ? | ? | ✖ local | ? repeat offsets | ? shared offset/source change | ? truncation | ? bounds | ✖ byte reads | ? drop ownership | ? redaction | ✖ no billing | ? read errors |
| Core integration | ? | ? | ? | ✖ unchanged transport | ? independent repeated reads | ? shared handles | ? source changes | ? no JSON handles | ? names | ? originals survive drop | ? sanitized new types | ✖ no billing | ? EOF |

Interactions: root file/folder × valid/invalid name; relative tree × rename/symlink replacement; count/depth/entries/bytes × exact/over limit; read offset × zero/small/large source × shared-offset change × truncation/growth; normal files versus ephemeral cleanup ownership; legacy JSON versus in-process handles. Plan approximately 30–45 resource test methods with table subcases plus 5–10 core integration methods; document justified final counts, do not replace independent rules with one sample.

## Progress

- F0: plan/inventory recorded before code.
- F1/F2/F3: pending.
- No native hardware/UI compatibility claims. D-Bus coordinator and GTK implementation remain subsequent stages.
