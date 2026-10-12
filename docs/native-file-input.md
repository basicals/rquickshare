# Native file input: approved descriptor boundary

_2026-10-12. Mode A. Status: resource/core implementation complete; final validation in progress._

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
- Destination components are bounded to 255 UTF-8 bytes, complete relative names to 16 KiB, and received manifests to the same depth budget as local folder walks. Reject drive separators, empty/dot components and file-versus-directory prefix collisions. Roots with the same display name fail instead of silently merging trees.
- Core preparation duplicates each selected file handle once, so the integration may temporarily retain up to twice the selected-file budget plus active directory traversal handles. Future service-wide accounting must include these duplicates and all queued/active selections; a per-selection cap alone is insufficient.
- Legacy wire metadata remains equivalent. The existing path route still exists for rollback; its ancestor check/open race is not declared fixed merely because the new handle route exists. New native operations must use the handle route exclusively.
- Serde/TypeScript payload contracts are unchanged. Rust consumers that exhaustively match `OutboundPayload` must handle the new Linux-only variant; review crate release/version notes before publishing it externally. This commit is on `dev`, not a release promotion.

## Plan, risk and rollback

1. F0: record approval, module/type contracts, inventory and limits; persist general maintenance preference. Documentation commit.
2. F1: implement Linux resource acquisition and validation, deterministic filesystem tests, bounded positioned reads. Commit independently.
3. F2: integrate owned-selection payload into outbound metadata/transfer handling. Reuse a bounded positioned-read helper in the existing sender to fix growth/truncation/offset behavior; zero-byte reads before the advertised end currently make no progress. Preserve crypto/framing, originals and text/ephemeral paths. Commit separately.
4. F3 (historical completed plan): run format/Clippy, full existing suites plus new tests ten consecutive times; update CI so resource tests run with core tests, record results/mutations/remaining work. Commit handoff docs. The subsequent user instruction requires one appropriate development run; ten repetitions apply only to an applicable final stability gate. Preserve the measured results below rather than repeating them.

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

Core entrypoint integration also uses real loopback TCP sockets and explicit established-session test keys, exercising the actual constructor, encrypted introduction, consent, file-frame and cancellation handlers. Planned cases: exact introduction/file bytes/terminal frame, source growth/truncation before send, and cancellation before first file read. The peer fixture verifies HMAC, decrypts AES-CBC and checks message sequence numbers. This is local encrypted-framing coverage, not a physical peer or full UKEY2 handshake claim. Domain handlers are not mocked. Descriptor reception must use atomic close-on-exec support in the future IPC adapter; adoption reinforces CLOEXEC but cannot retrospectively make receipt atomic.

## Negative and interaction matrix

Every inventory row has scoped checks for Positive plus applicable categories below; non-applicable categories are explicit. Full protocol branch coverage is not part of this finite resource slice.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Names/budgets | ✔ | ✔ | ✔ | ✖ local | ✖ stateless | ✖ stateless | ✔ malformed names | ✔ traversal | ✔ Unicode | ✖ no UI | ✔ redaction | ✖ no billing | ✖ no I/O |
| FD/root/walk | ✔ | ✔ | ✔ | ✖ local | ✔ duplicate roots | ✔ replacement | ✔ stale paths | ✔ symlinks/types/modes | ✔ Unicode | ✔ drop cleanup | ✔ redaction | ✖ no billing | ✔ missing/read errors |
| Positioned reads | ✔ | ✔ | ✔ | ✖ local | ✔ repeat offsets | ✔ shared offset/source change | ✔ truncation | ✔ bounds | ✖ byte reads | ✔ drop ownership | ✔ redaction | ✖ no billing | ✔ read errors |
| Core integration | ✔ | ✔ | ✔ | ✖ unchanged transport | ✔ independent repeated reads | ✔ shared handles | ✔ source changes | ✔ no JSON handles | ✔ names | ✔ originals survive drop | ✔ sanitized new types | ✖ no billing | ✔ EOF |

Interactions: root file/folder × valid/invalid name; relative tree × rename/symlink replacement; count/depth/entries/bytes × exact/over limit; read offset × zero/small/large source × shared-offset change × truncation/growth; normal files versus ephemeral cleanup ownership; legacy JSON versus in-process handles. Plan approximately 30–45 resource test methods with table subcases plus 5–10 core integration methods; document justified final counts, do not replace independent rules with one sample.

## Progress

- F0: plan/inventory recorded before code.
- F1/F2: complete in `c8d537e` around one owned-resource/core contract. They share one atomic feature commit because the private handle-adaptation seam is consumed by the outbound integration; the native reader contract also supplies the existing sender's bounded-read correction. No protocol framing or crypto production code changed.
- F3 mutation review: 18 copied-source mutants were killed by assertion failures, not compile failures. The first pass exposed missing in-root symlink coverage, which was added; manifest depth was also enforced for received handles during review. Baseline source was copied to scratch, never mutated in place.
- F3: complete for this offline implementation slice. The four complete suites passed ten consecutive times on `c8d537e`, with independently checked 40 suite invocations. Rust format, core/desktop Clippy, Vue typecheck, ESLint and diff checks passed. Coverage and category counts are below. The existing preflight/core jobs already execute these tests; no redundant CI job was added.
- No native hardware/UI compatibility claims. D-Bus coordinator and GTK implementation remain subsequent stages.

## Execution, coverage and continuation

| Suite | Tests per run | Consecutive passes |
|---|---|---|
| Core (`cargo +stable test --all-features`) | 180 unit/export assertions + 4 integration | 10 |
| Existing desktop (`cargo +stable test --no-default-features --bin rquickshare`) | 8 | 10 |
| Vue (`pnpm exec vitest run`) | 41 | 10 |
| Offline analyzer (`python3 -m unittest discover -s scripts/tests`) | 25 | 10 |
| Total | 258 | 10 |

New tests: 44 file-resource methods, 8 reader methods and 10 native/core integration methods, 62 total with table-driven subcases. The final scope grew beyond the preliminary estimate to cover real encrypted framing, storage fault injection, in-root symlink aliases and manifest depth. None are ignored/skipped. Ten passes are observed stability evidence, not proof that every scheduling/hardware failure is impossible.

Measured with cargo-llvm-cov 0.9.1 against the 180-test core library suite, excluding separate test files/generated bindings/target files:

| File | Lines | Functions | Regions | Remaining / interpretation |
|---|---|---|---|---|
| `src/file_source.rs` | 235/235 (100%) | 35/35 (100%) | 343/353 (97.17%) | Public methods exercised; not every propagated kernel/fcntl/stat/directory errno is separately injected |
| `src/file_reader.rs` | 155/160 (96.88%) | 18/18 (100%) | 226/229 (98.69%) | Includes inline tests/fixtures; real `File::metadata` error arm not forced, adapter metadata errors are covered |
| `src/hdl/outbound.rs` | 1605/2271 (70.67%) | 113/201 (56.22%) | 2325/3305 (70.35%) | Includes existing and new inline tests; broad protocol/state-machine paths remain outside this finite slice |

LLVM branch counters were not enabled; regions are not branches. These are not production-only percentages for files containing inline tests. Do not claim full Mode B completion or 100% protocol/security coverage. The scoped matrix above records planned risk categories; the index below maps every implemented method to a primary category. Individual methods also cover overlapping categories.

Executed assertion-killed mutations: remove name validation; allow writable/O_PATH handles; accept special files; omit CLOEXEC; allow child/root symlinks; remove BENEATH confinement; change file/entry/depth/byte boundaries; wrap total-byte overflow; omit budget validation; merge same-named roots; replace positioned reads with shared-offset reads; read beyond remaining bytes; accept zero progress; ignore source-size changes. The copied baseline passed before mutation. Compilation failures were not counted as killed mutants. Prefix collision and legacy JSON/TS invariants additionally have direct assertions; this was targeted mutation review, not exhaustive mutation-tool coverage.

No system packages or application registrations were installed. No daemon, GUI, Bluetooth/ADB or file-manager process was launched. All task-owned terminal commands exited; terminals closed. Only lockfile dependency edges changed: already locked rustix/serde_json/tempfile versions were reused; no dependency versions were upgraded.

Next stages: service-wide resource accounting and descriptor-safe D-Bus reception; authoritative revisioned service state/consent/reconnect/lifecycle coordinator; core service ownership adapter; then GTK4 tray feasibility/prototype, clipboard/settings migration, context-menu CLI/package adapter and physical Google peer gates. The old Tauri path race remains in the rollback path until that entrypoint is migrated; source contents remain mutable even through FDs. Full Google account/QR/Apple capability parity remains separate required work.

## New-method primary-category index

| Category | Count | Scope |
|---|---|---|
| Positive | 10 | Valid metadata/tree/bytes and legacy compatibility |
| N1 | 8 | Unsafe/empty/colliding names and invalid budgets |
| N2 | 12 | Lengths, offsets, counts, depth, byte limits and overflow |
| N3 | 0 | Not applicable: transport failure policy unchanged; loopback is only a fixture |
| N4 | 1 | Repeat positioned reads; service-command replay remains a later stage |
| N5 | 8 | Renames, unlink, subtree replacement, concurrent writes and shared offsets |
| N6 | 3 | Changed source sizes; no persisted schema introduced |
| N7 | 10 | Access modes/types, symlinks, confinement, CLOEXEC and no JSON handle minting |
| N8 | 1 | Unicode/portable names; no wall clock or timezone rule |
| N9 | 3 | Cancel/drop ownership; no GUI flow introduced |
| N10 | 2 | Redacted diagnostics and private failure messages |
| N11 | 0 | Not applicable: no billing or entitlements |
| N12 | 4 | EOF, interrupted reads and storage/metadata failures |

Primary categories sum to 62; table subcases and overlapping risks are not double-counted.

| Method | Primary category |
|---|---|
| `given_portable_names_when_validated_then_unicode_and_nested_paths_are_accepted` | N8 |
| `given_unsafe_names_when_validated_then_every_unsafe_form_is_rejected` | N1 |
| `given_name_at_total_length_limit_when_validated_then_limit_is_inclusive` | N2 |
| `given_protocol_size_boundaries_when_validated_then_only_representable_sizes_are_accepted` | N2 |
| `given_total_size_boundaries_when_adding_then_overflow_and_budget_excess_are_rejected` | N2 |
| `given_zero_budget_when_constructing_selection_then_each_zero_field_is_rejected` | N1 |
| `given_invalid_manifest_names_when_descriptor_is_adopted_then_no_name_bypasses_validation` | N1 |
| `given_inheritable_descriptor_when_adopted_then_close_on_exec_is_enforced` | N7 |
| `given_default_budgets_when_inspected_then_all_resource_caps_are_explicit` | Positive |
| `given_read_only_regular_file_when_adopted_then_metadata_and_bytes_are_preserved` | Positive |
| `given_writable_descriptors_when_adopted_then_both_write_access_modes_are_rejected` | N7 |
| `given_path_only_descriptor_when_adopted_then_it_cannot_be_used_as_read_authority` | N7 |
| `given_directory_or_special_descriptor_when_adopted_then_non_regular_types_are_rejected` | N7 |
| `given_empty_regular_file_when_adopted_then_zero_byte_read_completes` | Positive |
| `given_acquired_file_when_path_is_replaced_then_original_handle_is_used` | N5 |
| `given_acquired_file_when_unlinked_then_the_held_source_remains_readable` | N5 |
| `given_shared_file_description_when_other_holder_seeks_then_positioned_reads_are_independent` | N4 |
| `given_source_size_changes_when_reading_then_growth_and_truncation_fail` | N6 |
| `given_same_size_write_when_reading_then_no_immutable_snapshot_is_claimed` | N5 |
| `given_offset_and_buffer_boundaries_when_reading_then_only_advertised_bytes_are_returned` | N2 |
| `given_empty_root_list_or_empty_folder_when_acquired_then_empty_selection_is_rejected` | N1 |
| `given_nested_folder_when_acquired_then_tree_names_sorting_and_totals_are_preserved` | Positive |
| `given_regular_root_when_acquired_then_source_does_not_depend_on_absolute_name` | Positive |
| `given_missing_or_unnamed_root_when_acquired_then_errors_are_private_and_atomic` | N10 |
| `given_symlink_root_or_ancestor_when_acquired_then_resolution_is_rejected` | N7 |
| `given_symlink_descendant_when_acquired_then_no_outside_bytes_are_selected` | N7 |
| `given_symlink_to_selected_subtree_when_acquired_then_in_root_aliases_are_also_rejected` | N7 |
| `given_fifo_root_or_descendant_when_acquired_then_no_blocking_read_is_attempted` | N7 |
| `given_pinned_root_when_ancestor_path_is_replaced_then_original_subtree_is_used` | N5 |
| `given_replaced_descendant_when_opened_relative_to_root_then_symlink_escape_fails` | N5 |
| `given_relative_escape_when_opened_from_root_then_absolute_and_parent_paths_fail` | N7 |
| `given_file_budget_when_acquiring_then_exact_limit_succeeds_and_excess_fails` | N2 |
| `given_entry_budget_when_walking_then_roots_and_empty_directories_are_counted` | N2 |
| `given_depth_budget_when_walking_then_exact_depth_succeeds_and_excess_fails` | N2 |
| `given_byte_budget_when_acquiring_then_zero_files_and_exact_sum_are_handled` | N2 |
| `given_duplicate_destinations_when_building_selection_then_collisions_fail` | N1 |
| `given_same_named_roots_when_acquired_then_colliding_tree_destinations_fail` | N1 |
| `given_file_directory_prefix_collision_when_building_selection_then_both_orders_fail` | N1 |
| `given_non_utf8_or_control_source_name_when_acquired_then_metadata_is_rejected` | N1 |
| `given_private_selection_when_debugged_then_paths_names_and_contents_are_absent` | N10 |
| `given_selection_when_dropped_then_original_files_are_never_deleted` | N9 |
| `given_received_handles_when_bounded_then_count_and_bytes_are_enforced_without_paths` | N2 |
| `given_received_manifest_depth_when_bounded_then_exact_depth_succeeds_and_excess_fails` | N2 |
| `given_cloned_handle_when_original_position_changes_then_owned_bytes_remain_accessible` | N5 |
| `given_premature_eof_when_reading_then_zero_progress_is_an_error` | N12 |
| `given_interrupted_storage_when_reading_then_operation_retries_without_sleep` | N12 |
| `given_storage_errors_when_reading_then_permission_io_and_disconnect_errors_propagate` | N12 |
| `given_metadata_error_when_reading_then_no_data_read_is_attempted` | N12 |
| `given_offset_beyond_end_when_reading_then_no_storage_call_is_attempted` | N2 |
| `given_complete_or_empty_source_when_reading_then_no_storage_call_is_attempted` | Positive |
| `given_short_storage_read_when_reading_then_only_actual_bytes_are_returned` | Positive |
| `given_large_storage_source_when_reading_then_buffer_and_remaining_limits_are_applied` | N2 |
| `given_native_selection_when_introduced_and_accepted_then_exact_file_frames_complete` | Positive |
| `given_prepared_source_changes_when_consent_arrives_then_no_false_completion_occurs` | N6 |
| `given_native_transfer_cancelled_before_read_when_accepted_then_original_is_preserved` | N9 |
| `given_native_files_when_prepared_then_protocol_names_types_sizes_and_parents_match` | Positive |
| `given_unlinked_native_source_when_prepared_then_no_path_reopen_occurs` | N5 |
| `given_native_source_size_changed_when_prepared_then_transfer_is_rejected` | N6 |
| `given_native_payload_when_serialized_or_deserialized_then_json_cannot_carry_handles` | N7 |
| `given_legacy_payloads_when_round_tripped_then_existing_json_contract_is_preserved` | Positive |
| `given_native_payload_when_dropped_then_original_source_has_no_cleanup_capability` | N9 |
| `given_prepared_native_handles_when_shared_position_changes_then_readers_stay_independent` | N5 |
