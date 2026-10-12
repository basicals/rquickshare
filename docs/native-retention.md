# Native current-run retention and ordered operations

2026-10-12. Mode A. The user approved option 1 in native-retention-review.md.
This implementation SSOT supersedes its awaiting-choice handoff. No persisted
history, implicit retry after restart, settings migration, GUI or packaging change.

## Frozen contracts and motivation

The additive Operations1 interface on the existing authenticated service/path has:
BeginSelection t -> s; CommitSelection st -> s; AbortSelection st -> empty;
SendSelection sst -> s; GetOperationState empty -> tt; GetReceipt t -> ss;
AcknowledgeThrough t -> empty. The interface XML is the wire fixture. Receipt
kinds are exactly begin_selection, commit_selection, abort_selection,
send_selection; result is a canonical upload/selection/transfer ID or empty.

Each actual connection owns a separate ordered ledger, bounded to 128 outstanding
receipts, and last-accepted/acknowledged-through u64 scalars. Sequence starts at 1;
zero/gaps/future receipt reads are InvalidInput. At/below acknowledged watermark
mutations and receipt reads are RequestExpired. Exact retained arguments replay;
changed kind/arguments are ReplayConflict. Failed operations do not consume a
sequence. Exhaustion and capacity are checked before ownership changes. Ack is
idempotent, cannot exceed accepted progress, and retires receipt metadata only.
The future client records results and acknowledges automatically, never asks the
user to clear history or rotate connections. Reconnection creates a fresh ledger;
no automatic replay of lost work on another connection or epoch.

Upload origin is typed, not a reserved string prefix. Manager1 AppendFiles and
GetUpload operate on either origin with unchanged FD/ownership/batch guards.
Manager1 Commit/Abort reject ordered uploads with InvalidState; Operations1
Commit/Abort reject legacy uploads with InvalidState. This prevents bypassing the
sequenced commit receipt. Existing legacy uploads/strings keep their old behavior.
Closed ordered uploads disappear only when no outstanding receipt references them;
preparations stay until commit/abort/disconnect. Ack never frees a committed unsent
selection or queued/active payload. Each origin has a bounded receipt population;
the four preparing uploads and global source budgets are shared.

Transfers retain weak source references, an explicit owner-closed barrier, an
outbound release barrier and a control-receipt barrier. Queued work has no core
owner; dispatch clears its barriers until the runtime explicitly reports Closed
and release, or a no-session failure/cancel. Inbound release is vacuous. Terminal
observation alone cannot satisfy closure. Lost observations disable retirement.
Legacy send replay references and unacknowledged ordered send receipts pin history.

Maintenance retains the latest 64 eligible terminal records, ordered by terminal
revision then ID, in addition to active/pinned records. For an explicitly reduced
internal transfer limit, the target is min(64, transfer_limit - 1), leaving one
admission slot. Default limits remain 256 transfers and 64 recent eligible records.
A batch removal checks and advances the shared revision before mutation; immutable
captures are unchanged. Maintenance runs on runtime progress and client ack or
disconnect, without timers. Missing retained control IDs are UnknownTransfer;
expired sequences remain RequestExpired after history removal. Original/download
files are never deleted. Bounds on FD/session/thread admission are unchanged.

## Plan and rollback milestones

1. [x] Analyze coordinator uploads/send ledger, runtime close/release/control
   futures, authenticated dispatch and immutable captures; freeze this contract.
2. [x] Implement separate pure ordered ledger and typed upload origin, extracting
   shared upload/enqueue helpers without changing Manager1 legacy semantics.
3. [x] Add truthful closure/release/control barriers and bounded retirement;
   integrate the serial runtime and additive authenticated dispatcher/fixtures.
4. [x] Add focused domain/runtime/private-bus regression checks; run affected
   Rust suite and strict formatting/Clippy once; record actual observations here.
5. [x] Update diary and handoff, commit reversible stages on dev.

The plan freeze and integrated implementation are separate atomic commits. The
ledger and closure changes share the retention invariant and are committed
together so an intermediate checkout cannot expose premature cleanup. Rollback additive
interface/ledger first, then retirement/barriers. No file/storage migration.
Risks: bypassed closure, late receipt targeting retired state, lost commit result,
legacy ID eviction, revision overflow and accidental source lifetime shortening.
Safety predicates and pinned-history capacity are retained rather than guessed.

## Scoped verification inventory and branch plan (before tests)

This is feature verification, not a repository-wide Mode B completion claim.
Existing upload/resource/auth/transport/snapshot cases remain the regression base.
Use crate-local cargo test --locked --all-features; CI workflows already run Rust
checks. No new dependencies, real peers, user bus or live discovery are required.

| Artifact | Planned checks | Branches / interactions | Primary categories |
|---|---|---|---|
| Ordered ledger + mutations | 8 | exact/conflicting all four receipts; retired/zero/gap/future; early/duplicate/cumulative ack; 128/129; failed admission; last-sequence/revision overflow; client isolation; preparing/closed upload dependencies, typed-origin bypass | Positive,N1,N2,N4,N7 |
| Retention predicates | 6 | 64/65, default/reduced budget; source held/released crossed with owner live/closed; control receipt; legacy/unacked pins then disconnect/ack; desync; exhaustion; terminal order and stale controls; thousands of operations | Positive,N2,N4,N5,N9 |
| Runtime barriers | 2 | real outbound release vs Closed order; queued/pending route loss/cancel and inbound close; late control receipt; existing joined shutdown | N5,N9 |
| Operations1 dispatcher | 2 | exact XML/enums/errors; real private broker ownership, bad signatures/FDs/unknown methods; lost reply/replay/ack and unchanged Manager1 behavior | N1,N7 |
| Immutable capture | 1 | capture before collection remains stable; fresh capture sees removal/revision | N5 |

N3 is covered by existing bounded transport/core-loss tests (no new network path).
N6 old contracts remain in existing fixture tests. N8 no wall-clock/locale logic.
N10 fixed redacted error fixture remains; no source names added to receipts.
N11 no billing. N12 no persistence; existing FD adoption failures remain.
Assertions must detect removed watermark/conflict/capacity checks, >= boundary
changes, premature owner/source/control cleanup and legacy pin deletion. Test
fixtures use real coordinator/runtime, owned private broker and regular files.
No sleeps, weak assertions, mocked subject, repeated completed tests or ten-run
Mode A gate. Any added scenarios or changed estimates are recorded below.

## Actual verification and continuation

Plan frozen in `52b5347` before production edits. The separate ordered ledger,
typed upload origin, checked shared enqueue, Operations1 dispatch and XML/error
fixtures are implemented. Legacy strings have no implicit eviction; their
original validation/admission error ordering was restored during final review.

Owner/release/control-receipt barriers are adapter-only bookkeeping, not public
snapshot changes; actual metadata retirement advances the public revision.
Missing terminal revision is ineligible rather than assigned a fabricated age.
The runtime records Closed after removing its binding, awaits each identified
outbound release, pins late consent receipts and explicitly closes work which
never acquired a core owner. History maintenance runs before admission dispatch
and after observed progress; acknowledgement/disconnect unpins it on the next
serial runtime step. No timer or user intervention is involved.

19 new regression tests: Positive 2, N1 1, N2 5, N4 5, N5 4, N7 2 (primary
category counts, no double counting). Cross-category checks also cover cancellation,
disconnect, privacy-safe receipts and old-contract compatibility. N3/N6/N10 remain
covered by existing transport, fixture and redacted-error checks. N8/N11/N12 add no
new applicable wall-clock, billing or persistent-storage behavior in this slice.
Existing source-adoption/storage failure checks remain unchanged.

Actual observations:

- Full locked all-feature core run: 448 unit/export tests and 8 integration tests
  passed, including the owned executable tests. No failures/ignored tests.
- After restoring legacy error ordering and excluding missing terminal revisions,
  the affected service run passed all 164 tests (284 other unit/export tests and
  eight integration tests filtered). No redundant full-suite repetition.
- Locked all-feature/all-target core Clippy with `-D warnings` passed.
- New continuous-use case performs 1,000 complete prepare/commit/send/cancel/ack
  cycles (3,000 accepted ordered mutations) on one connection. Every iteration
  asserts empty closed-upload/receipt maps, released source budgets, at most 64
  records and an existing original file; the first request stays expired.
- Real adapter tests exercise both Closed/release orders and a late consent
  receipt. Private-broker tests exercise exact typed replies, ownership, rejected
  attached FDs/signatures and lost/retried/expired results. Cached pages survive
  removal and a fresh view sees the increased revision and reduced record count.

Locked desktop no-default-feature/all-target Clippy with `-D warnings`, nightly
format checks for both Rust crates and `git diff --check` passed. An independent
XML parser verified all seven Operations1 input/output signatures and fifteen
unchanged Manager1 methods. No package versions or dependency locks changed.
Terminal closed; no owned terminal/process remains running.
No frontend/UI changes, physical-peer proof, live discovery/session changes,
installed activation or ten-run stability/coverage-percentage claim. The existing
bluer future-incompatibility notice remains. Next product stage: a native client
which automatically reconciles/acknowledges receipts, then GTK tray/settings/
notifications, Dolphin and package/device interoperability validation.

## Completed checkpoint

Implementation committed as `8b0a4f7` on dev after the documented checks. No push,
master promotion or user-session installation. The ledger, runtime barriers,
wire fixture and regression cases can be rolled back as one integrated change;
`52b5347` preserves the approved plan. Remaining product work is explicitly listed
above and in product-direction.md. Terminal closed.
