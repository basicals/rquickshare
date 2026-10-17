# Continuous-service retention and operation receipts

2026-10-12. Mode A. Analysis/review only; no retention policy or new API is
implemented by this document. Current executable/recovery checkpoint: `046153b`,
with measured verification in native-daemon-startup.md.

## Source findings

- Coordinator keeps every transfer in its bounded `transfers` map. Default limit
  256 applies to retained terminal records as well as queued/active transfers.
  Eventually register_inbound/send_selection return Capacity even after all workers
  finish and all source handles are gone. This is explicit designed backpressure,
  not an observed memory leak or a reason to remove bounds.
- The current Manager1 send ledger stores up to 128 arbitrary request IDs per
  authenticated connection. Identical accepted arguments replay the original
  transfer ID, conflicting arguments fail, and disconnect removes that ledger.
- Uploads keep at most 128 receipts per connection, including committed/aborted
  uploads. BeginSelection can reconcile a lost reply with the same request ID;
  CommitSelection replays its original selection. Closed receipts consume no source
  handles but still consume ledger capacity. Reconnect frees them and unsent work,
  but rotating a long-lived chooser connection is not the durable cleanup design.
- A terminal phase alone does not prove the core session/control owner has closed
  or the final reader reference has released. NativeRuntime handles ordered Closed
  events and owns release/consent futures. Removing coordinator records too early
  can turn a later legitimate Closed observation into UnknownTransfer/desync.
- Current immutable view pages can outlive live state changes. Archive removal must
  advance the same revision and leave already captured pages stable; old IDs must
  never alias a new transfer. Actual downloads/original files are outside metadata
  retention and must never be deleted by history cleanup.

Relevant sources: service.rs (clients, reservations, transfers, send replay),
service/uploads.rs (preparation/receipt lifecycle), service/runtime.rs (Closed,
receipts/releases/shutdown), service/bus/state.rs (immutable captures). Existing
tests deliberately require finite explicit Capacity and no silent replay eviction.
Simply relaxing those assertions would contradict the frozen contract.

## User-visible decision

Global AGENTS sections 3/3.1 require approval for significant data-model/public API
and product behavior changes. The prior diary explicitly leaves retention open.
The choice is history lifetime; automatic cleanup/acknowledgement must never become
a user action or a demand to restart/edit files.

| Option | Description | Benefits | Costs and risks |
|---|---|---|---|
| 1 — bounded current-run history (recommended) | Retain the latest 64 safely closed terminal transfers plus active work in memory; oldest eligible history disappears automatically; sequence/acknowledgement ledger prevents retired requests from being executed again | Bounded steady-state memory, no new persisted private state/migrations, fits current epoch/restart model; no manual clearing/reconnect | History is lost when the service stops; older eligible entries leave the panel; live/pinned work can still legitimately hit admission limits |
| 2 — persistent bounded history | Use a private transactional local journal for transfer outcomes/history, with the same bounded live operation protocol | Recent history can be inspected after service restart; better historical diagnostics | New storage format, corruption/migrations/write-failure policy, disk/privacy/cleanup budgets and more implementation; a journal cannot retain FD authority or resume an active payload automatically |

Recommend option 1 for the first native desktop. The accepted goal requires
background transfers to survive closing the GUI, not replaying payloads after a
service crash. In-memory recent history is sufficient for this first panel without
adding an independent database lifecycle. Option 2 remains a valid product choice
if persisted history is wanted now; it does not replace authenticated Google trust,
protocol resume support or real device evidence.

## Common proposed receipt design

Arbitrary forever-valid string IDs cannot be discarded from a bounded ledger while
still detecting every previously accepted ID. Introduce an additive ordered
operation interface for the future native client; do not silently change Manager1.
Each actual connection has monotonically increasing nonzero u64 operation sequences,
a bounded unacknowledged receipt window (128), last accepted sequence and cumulative
acknowledgement watermark. These scalars replace an unbounded retired-ID set.

Concrete proposed interface `io.github.morozhenko_apps.RQuickShare.Operations1`
on the existing service/path, with typed methods:

| Method | Input | Output | Meaning |
|---|---|---|---|
| BeginSelection | t | s | Next operation sequence creates one owner-bound upload |
| CommitSelection | st | s | Upload ID and next operation sequence commit once |
| AbortSelection | st | empty | Upload ID and next operation sequence abort preparation |
| SendSelection | sst | s | Selection, discovered endpoint, next sequence explicitly enqueue |
| GetOperationState | empty | tt | Last accepted sequence and acknowledged-through watermark |
| GetReceipt | t | ss | Operation kind and original result ID, empty ID for an empty result |
| AcknowledgeThrough | t | empty | Client has recorded results through this sequence; retire only receipt metadata |

The client serializes mutating operations through its owner. First accepted sequence
is 1, the next is exactly last+1. Exact retained retries replay the same result;
different kind/arguments fail ReplayConflict. A retired sequence fails an explicit
RequestExpired error and never becomes a new operation. A future/gapped/zero
sequence fails InvalidInput. Failed admission does not consume a sequence; overflow
fails explicitly before mutation. Acknowledgement is idempotent and cannot exceed
accepted progress. It acknowledges receipt acquisition, never peer completion.

Lost replies are reconciled via GetReceipt/GetOperationState before issuing more
work. Begin, Commit, Abort and Send need this protocol together: acknowledging Begin
alone must not allow a lost Commit reply to lose its original selection ID. Upload
metadata stays while preparing or while a corresponding unacknowledged ordered
operation still needs it. Once closed and no receipt depends on it, it is reclaimed.
The new interface's receipt namespace is distinct from Manager1 strings; do not
encode sequences into guessed reserved legacy string prefixes.

The native client acknowledges automatically after recording each accepted result
in its current connection state. No user click, periodic timer or connection
rotation is required. Queued/active transfers stay service-owned after acknowledge
or client disconnect. No source handle, active selection, transfer or download file
is freed by acknowledgement. Same-UID authentication, connection-bound preparations
and fresh daemon epochs remain unchanged. No bare-token cross-client ownership
handoff, persisted live-transfer recovery or implicit resend after restart.

Manager1's existing string methods keep their original retained-receipt/Capacity
contract for compatibility. The native client uses Operations1 without an automatic
fallback to unsafe ID reuse. Legacy receipts pin their referenced transfer records
until that owner disconnects; deliberately non-acknowledging/legacy clients remain
bounded and can exhaust their available budget. This is not a same-UID flood
availability guarantee.

## Safe transfer retirement proposal

- Keep active/admission limits separate from the recent-history target. Do not
  solve retention by raising descriptor/session/thread/decoder bounds.
- A record is eligible only after truthful terminal observation, ordered core-owner
  closure acknowledgement, actual source-reference release, no pending control/
  receipt dependency and no legacy replay reference. Queued cancellation without
  core dispatch has its own explicit no-session closure path.
- Add the minimal runtime/coordinator closure/release seam. Mark availability
  before a late Closed/receipt callback can target a retired record. Do not infer
  closure from a phase or invent a terminal state on lost events.
- Retire oldest eligible terminal records beyond 64 by terminal revision, with ID
  tie-break; apply checked revision updates before mutation. Immutable existing
  captures remain unchanged. Uncertain/incompletely released records stay pinned;
  Capacity remains truthful if all records are pinned/active.
- Cancellation/consent on a retired ID fails UnknownTransfer; clients refresh an
  old view rather than resurrecting history. No new ID is reused. Retired ordered
  requests still fail RequestExpired by watermark even after the record is gone.
- Option 2 adds a bounded archive/schema and atomic journal transactions around
  this same safety predicate. Old epoch records are historical, not live control
  authority; interrupted transfers are uncertain, never automatically reopened by
  path or resubmitted. Storage budgets/error/migration details must be frozen in
  its selected implementation SSOT before code.

## Staged work after choice

1. Freeze the chosen lifetime, exact interface XML/errors, operation/receipt state
   model, bounds, closure seam and complete scoped negative/branch inventory.
   Record motivation before implementation; preserve Manager1 fixtures.
2. Implement the pure ordered operation ledger and closed-upload cleanup, using
   actual coordinator operations and independent namespaces.
3. Integrate closure/release acknowledgement and safe bounded terminal retirement
   with the real runtime; add Operations1 authenticated dispatcher/private-bus
   tests. If option 2, add the separately planned journal/transaction stage.
4. Verify thousands of sequential acknowledged operations without reaching a
   historical quota, lost/repeated/conflicting replies, early/late/duplicate ack,
   gaps/overflow, foreign callers, terminal-but-reader-held/owner-live races,
   legacy replay pins, queued cancel, disconnect, desync, shutdown and stable views.
   Use real domain/runtime logic and external deterministic fixtures only.
5. Run affected checks once, record actual observations and make reversible atomic
   commits. No unchanged suites or ten-run development repetitions.

Rollback removes the additive interface/ledger first, then retirement/closure
changes. Existing Manager1 and the legacy GUI remain available; no settings/source
migration in option 1. Option 2 needs its own nondestructive journal rollback plan.
GTK chooser/settings/notifications and Dolphin follow; this is not Google parity.

## Approved choice

The user selected option 1. Implementation contract and progress now live in
[native-retention.md](native-retention.md). The review below is historical.

## Handoff (historical)

Await the user's history-lifetime choice before implementing new public contracts
or changing retention. The existing executable stays unchanged, with known finite
ledgers and no production continuous-use signoff. No tests rerun, packages installed,
user session changed or sources modified in this review. Terminal closed.
