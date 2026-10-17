# Native inbound storage-failure regression matrix

2026-10-17. MODE B — Testing; direct dev, source-only, essential priority 2.
Do not touch the user's workstation/Pixel, native29 installation, service,
release/signing, `master` or physical filesystem; no automatic retry.
This document is the pre-implementation SSOT for a focused risk slice.
The repository-wide MODE B 10-repeat/coverage completion remains OPEN.

## Repository scan and selected gap

The test SSOT is the user-provided universal agent guide (`agent.md`);
repository references to `/home/morozhenko/.codex/AGENTS.md` are not
available through the GitHub connector. Read
`docs/native-essential-priorities.md`,
`docs/native-receive-reliability.md`,
`docs/native-transport-interruption-2026-10-17.md`,
`core_lib/src/hdl/inbound.rs`,
`core_lib/src/hdl/inbound/receive_reliability.rs`,
`core_lib/src/receive.rs`, `core_lib/src/receive/tests.rs`, and
`core_lib/src/received_files.rs`.

Framework: Tokio native Rust unit/integration tests; real Linux kernel
O_TMPFILE, `FileExt::write_all_at`, sync/link, managed session receipt
and actual FILE frame handler. CI: `Preflight` core all-feature tests,
nightly rustfmt, strict core/Tauri Clippy, frontend and Python contracts;
`Native GTK UI verification` is independent. The immediately prior
outgoing TCP reset source `ad2adf04` passed 701 core library tests and
8 integration; its Preflight 39058711970 subsequently completed five
successful jobs, GTK 39058711967 passed.

Source guards: inbound rejects invalid offset/overflow/premature last
chunk; `write_all_at` must succeed before advancing local progress;
Linux final publication checks exact inode size and `sync_data`,
links without overwrite, records completed receipt, then syncs parent.
`record_termination` closes all remaining anonymous files and returns
a partial receipt only for completed members. Sender retries are
never initiated by this path.

Existing regressions: first published file plus second `/dev/full`
write, cancellation, premature EOF, occupied destination; separate
Root tests cover drop, root rename/symlink confinement, collision and
directory fsync rejection after linking. **Not independently tested** in
one actual handler matrix: write errors `ENOSPC` vs read-only descriptor
`EBADF`, an altered staged-file size at publish, and each error both
with zero completed members and after a completed first member. This
is an evidence gap, not a preclaimed production defect.

## Risk/invariant analysis

Critical: a storage error cannot yield Finished, a corrupt final name,
a receipt for a failed member, or overwrite any existing destination.
Acknowledged bytes track only fully successful `write_all_at` calls,
not durable peer delivery. A late inode-size failure may have all
payload bytes acknowledged yet is **not** a completed file. Completed
first-member contents and read-only acquisition stay available on
failure, independently of a failed second. Unpublished anonymous
inodes and pinned publication handles are dropped on terminal cleanup.

This test controls only *external storage descriptors*: real /dev/full
returns ENOSPC, an actual read-only file descriptor returns EBADF, and
`set_len` changes the anonymous staged inode before publication. The
handler, receipt, session and publishing domain code are never mocked.
The test never changes permissions or content of user paths. A separate
system test of a genuinely read-only user-selected mount/full filesystem
is not implied by /dev/full or an injected descriptor.

## Exhaustive scoped inventory and branch map

The complete testable artifacts for this **storage failure path** (rather
than whole repository) are below. Other APIs/branches retain their
previous ledgers and are explicitly outside this slice.

| Production file / branch | Risk | Test level | Status/planned cases |
|---|---|---|---|
| `hdl/inbound.rs::process_offline_frame`: nonfinal FILE write success/failure and counter update | critical | real handler | N12: ENOSPC, EBADF × first member absent/published (4) |
| Same: final FILE after correct length, metadata length mismatch, no premature receipt/Finished | critical | real handler+kernel inode | N12: set_len corruption × absent/published first (2) |
| `hdl/inbound.rs::record_termination`, `finish_received`: terminal cleanup, no double publication | critical | session integration | all 6 |
| `receive.rs::Publication::publish`: size check before no-overwrite link, file and directory sync | critical | kernel filesystem | size fault in 2 cases; existing Root tests cover other branches |
| `received_files.rs::PendingFiles::finish_partial`, `ReceivedFiles::open`: exact saved subset, read-only descriptor, absent failed member | high | actual receipt | all 6 |
| `sessions.rs::observe_state`: original manifest and exact per-file progress despite failure | high | native managed session | all 6 |
| `receive.rs::Root::stage_file`, `accept_transfer`: O_TMPFILE admission and missing-parent/accept errors | high | previously tested | existing tests retained |
| Native GTK/IPC projection | medium | contract/widget CI | unchanged; no new rendering contract |
| Outbound `file_reader.rs`, `manager/session.rs`, `service/operations.rs` | medium | existing tests | no new behaviors or retry |

## Positive / N1–N12 matrix

Every category for the selected storage path has disposition. P means
planned, E existing, X not applicable.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| FILE handler/write | E | E | E | E | E | P | P | E | X | E | E | X | P |
| Staging/publish | E | E | P | E | E | P | P | E | X | E | E | X | P |
| Receipt/session cleanup | E | E | P | E | E | P | P | E | X | P | E | X | P |

N1 covers existing malformed payload/unsafe destination guards; N2 size
boundary is planned here; N3 network failure separate previous verified
RST/EOF ledgers; N4 no automatic retry, existing command idempotence;
N5 first member complete vs zero; N6 truncated inode before publish;
N7 existing pinned root and no-overwrite/consent contract; N8 X because
no time/locale; N9 request termination and cancellation cleanup; N10
existing generic no-path error and redacted receipts; N11 X no billing;
N12 planned real kernel storage faults. Existing positive completion
remains green; no accidental second success.

## Interaction matrix and coverage estimate

2 first-member states (none published / first published) × 3 storage
failures (`ENOSPC`, `EBADF`, `set_len` mismatch) = **six mandatory
parameterized subcases**, one test method. Each asserts pre-terminal
ReceivingFiles, error type (kernel errno vs bounded generic publish
error), old and new progress, original saved manifest, no second final
name, optional exact first name, and after Disconnected exactly zero or
one read-only retained file. The invalid member never enters the
receipt and no FD/publication owner remains. `set_len` must only
occur after successful complete data frame; no body is re-sent.

Estimated new inventory: 1 table-driven test, 6 cases.
Mutation sensitivity: bypassing error propagation, acknowledging failed
write, removing pre-link size guard, treating complete byte count as
Finished before publication, dropping completed member, preserving
an abandoned anonymous FD, or creating an invalid receipt must fail
at least one exact-state or filesystem assertion. This is mutation
*review*, not executed mutation tooling or measured branch coverage.

## Plan and rollback

1. [x] Repository scan, ownership and critical contracts verified; plan,
   branch map, matrix and six test cases documented before code.
2. [x] Implement all six tests in existing handler reliability fixture;
   no production changes or new dependencies. Preserve all assertions.
3. [ ] Run exact-source CI including full Rust suite + 8 integration,
   format/core Clippy/Python/frontend and GTK; triage any red jobs.
   If production behavior is wrong, record a MODE A repair plan before
   changing runtime code.
4. [ ] Record verified results by category and per-production-file
   coverage remaining; update current-state. Mark MODE B full gate
   open without full 10-pass/coverage workflow; never claim physical
   disks/peer acceptance.

Rollback: revert only focused tests/docs. No permission flags, protocol
fields, file names, receive policy, retention, auth, UX, release or
installed service is changed.

## Results

Test commit `0bdf73bd` adds one real-handler table-driven regression to
`core_lib/src/hdl/inbound/receive_reliability.rs`. It checks all six
preregistered combinations. Exact kernel error codes are required
(ENOSPC=28, EBADF=9); the size-mismatch path must return the bounded
"Cannot finish received file safely" error before publishing. The two
read-only/ENOSPC variants use independently owned real external
descriptors, never user files. The received-root remains an isolated
temporary directory. The truncated-inode variant uses the actual
O_TMPFILE after valid data reception.

Affected source is currently under exact-source CI:
- Preflight: https://github.com/morozhenko-apps/rquickshare/actions/runs/39061071880
- Native GTK: https://github.com/morozhenko-apps/rquickshare/actions/runs/39061071881

Observed so far: Rust format, frontend and strict core Clippy succeeded.
Full core execution, legacy Tauri Clippy and GTK remain in progress
at this written checkpoint. No claim that the six new cases pass
or that Mode B whole-repo gates are closed until actual job evidence.

