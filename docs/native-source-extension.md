# Atomic extension of an unsent native file selection

2026-10-16. Mode A, direct dev. User selected daemon-owned atomic updates in the
card-usability follow-up. Guide: /home/morozhenko/.codex/AGENTS.md. This is the
source/API milestone of the approved mockup, not a new peer wire protocol.

## Analysis and contract before code

The coordinator owns immutable PreparedSource values, connection-local selection
capabilities and ordered receipts. Existing uploads already stage/validate bounded
FD batches and survive lost replies; reuse this machinery for only the added
files. Do not clone the entire original selection into GTK, reopen display paths,
or discard the old selection before the replacement is accepted.

Add Operations1.CommitExtension(upload, selection, sequence) -> new selection.
This additive method is authenticated like other ordered operations, permits no
FDs in the commit call, and records kind commit_extension in the existing ledger.
Both capabilities must belong to the same caller/epoch; upload must be an ordered
preparing nonempty file upload and the base must still be an owned file selection.
Changed arguments at the same sequence are ReplayConflict; exact replies replay
the same new token even after the old token is removed. Retired receipts never
authorize new work. Existing CommitSelection and legacy selection calls retain
their contracts. Older daemons report incompatibility; coordinated daemon/client
installation is required and the GUI must not claim extension success.

Validate the complete old+delta names/count/bytes before mutation (256 files,
1 TiB per source, existing path/name bounds); duplicates fail without renaming.
Reserve aggregate resources for old plus staged delta plus retained old capability
clones before cloning. Existing 1024 handle and 128 MiB immutable-memory budgets
are unchanged, including the worst-case future worker clones. Temporary lack of
capacity is an explicit rejection; do not discard the base to free capacity.
Clone only held read-only capabilities, preserving original file sizes/memory
charges; reject a changed size while cloning. No filesystem paths are reopened.

Once all fallible validation/cloning and revision checks succeed, atomically move
the delta into a new immutable aggregate, record its reservation, replace the
unsent base selection and mark the upload committed. Already queued/active
transfers retain their original Arc and source reservation. No awaits occur in
the ownership transition. Abort/failure before commit releases only staged delta.
Client disconnect releases its preparations, never another client's resources or
admitted transfers. Selection edits never dispatch transfers.

SDK preparation keeps optional base selection alongside its delta source and
upload/sequence. Commit/recovery uses the same ordered mutation ledger; recovering
Begin/Append/Commit replies cannot manufacture another extension. Explicit abort
retains the old base before commit; if commit already succeeded, ordinary abort
semantics discard the newly admitted unsent selection, without cancelling sends.
GTK Add captures the exact current base before chooser acquisition; cancel keeps
it, and an intervening stale base is rejected by the daemon. The shared preparation
gate continues to exclude conflicting CLI/dialog work.

## Milestones, checks and rollback

- [x] E1: coordinator atomic extension and ordered ledger, exact kind/fingerprint.
- [x] E2: bus allowlist/decoder/XML and SDK mutation/recovery/controller support.
- [ ] E3: GTK acquisition supports optional extension target and Add file/folder
  source controls under the same gate/readiness guards. Main mockup layout follows.
- [ ] E4: scoped real-resource checks once; record failures and actual results.
  Core tests: aggregate contents, old transfer snapshot unchanged, old token
  retired/new token unique, exact replay/changed intent/retired sequence; duplicate
  names, count/byte/handle/memory limits, empty/text/aborted/committed deltas,
  wrong client/unknown or stale base, changed source size and abort preservation.
  Bus/SDK: strict signatures/no FDs, actual upload/commit, lost commit reply then
  exact recovery, unchanged legacy admission and no automatic dispatch.
  Existing GTK smoke checks Add/readiness/cancel integration; no broad Mode B
  project or ten-run repetition. Assertions validate exact source bytes/counts and
  resources, not just success statuses. Main categories Positive,N1,N2,N4,N5,N7,
  N9,N12; no new locale/time/billing/persistent schema (N8/N11/N6).

Risks: doubled retained capabilities during replacement, stale base race, replay
after old token removal, accidental text mixing or transfer snapshot mutation.
Rollback GTK/controller consumers first, then SDK/bus/ledger/coordinator; unchanged
old API remains usable. Keep source and later UI/settings commits separately
reviewable. No installed-service changes until a distinct verified package passes
the existing idle-owner gate. Native15 remains installed and retained.

Observed checks: plan/analysis only. No implementation or new test claim yet.

## E1-E2 actual implementation and core verification

CommitExtension validates/retains the old source, reserves worst-case aggregate
capabilities/memory, clones held old descriptors and moves staged added files into
one new manifest. Only successful commit removes the base token. Ordered receipt
fingerprints include both upload/base; SDK recovery and controller preparation
retain that intent. Existing send admission consumes its unsent selection, so
extending an already-sent token is a stale-base rejection; other transfers and
retained source Arcs are untouched. No path reconstruction or wire change.

Initial affected service suite passed293/0 in15.52s, including five new aggregate/
replay/negative/count/text/size/abort methods and the existing SDK reply-loss and
ack-loss tables extended for CommitExtension. Subsequent new bus/controller
fixtures initially failed compilation because the existing append helper takes
borrowed File pairs; corrected the fixture call, without changing assertions.
Four extension-filter methods then passed4/0 in0.98s, including strict no-FD/
signature/foreign-owner rejection and actual controller recovery before/after
BeginSelection, AppendFiles and CommitExtension. The added aggregate byte/memory
boundary method passed1/0 in0.16s. Strict core all-features/all-target Clippy passed
12.85s; only the existing bluer future-incompatibility notice remains.

Eight new methods total: primary Positive1, N1=1, N2=2, N4=1, N7=2, N12=1;
secondary interruption/race/capability cases are described in the plan. No claim
that all663 core methods were freshly run. Logs:/tmp/rqs-source-extension-core.log,
/tmp/rqs-source-extension-extra.log, /tmp/rqs-source-extension-budgets.log and
/tmp/rqs-source-extension-clippy.log. Formatting/whitespace passed. All owned
commands exited: Terminal closed. GTK acquisition changes are in progress and
their verification belongs to the subsequent native mockup milestone. Native15
remains installed; no real file selection, peer transfer or user settings change.
