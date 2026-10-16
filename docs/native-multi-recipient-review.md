# Native multi-recipient sharing: ownership review

2026-10-16. Mode A, direct dev. Guide: /home/morozhenko/.codex/AGENTS.md,
sections3/3.1/11/12/13. This is the next independent parity slice while canonical
preview codec/mapping remains unestablished. Implementation follows selected option1. The user chose daemon-owned
queue/results on2026-10-16.

## Analysis

Coordinator::enqueue_selection consumes exactly one client-owned Selection after
all current single-target admission checks. Transfer owns Arc<PreparedSource>,
Reservation holds Weak ownership and resources are released after all readers.
The current queue defaults to10 sends,256 retained transfers and128 endpoints;
these are internal budgets, not public protocol capacities. Operations1 has an
ordered exact-retry ledger, with a scalar transfer result. Manager2, SDK/controller
and GTK nearby rows expose one recipient per Send. The native Transfers projection
has no explicit batch/member model or stable queued-recipient lookup.

A GUI loop over current Send cannot reuse the consumed selection. Reopening paths
for each target can lose the admitted source identity. Retaining and resubmitting
pinned descriptors can avoid path reopening, but unsent-intent ownership/recovery
would remain in that GUI. The daemon already owns admitted transfers beyond GUI
close; multi-recipient intent should have equally clear ownership.

PreparedSource is a shared capability, not a byte-for-byte snapshot of an external
mutable file. read_file_chunk uses positional reads and checks advertised size;
sharing must preserve those existing guarantees without inventing immutable
file-content or durable restart-resume claims. Ordinary peer sessions remain
separate, with separate consent, progress, cancellation and failure. This is local
orchestration, not a proposed Quick Share wire extension or auto-accept mechanism.

## Significant alternatives (selection required)

| Option | Description | Advantages | Costs / risks |
|---|---|---|---|
| 1 — daemon-owned bounded batch, recommended | One explicit GUI Send submits the selected recipients and prepared source. Daemon owns admitted batch/member outcomes and pinned shared source; exact retry returns the same admission result. Member transfer outcomes remain independent; no automatic resend of accepted members. GTK selects recipients and projects per-peer status. | Unsent/admitted work does not depend on a card/widget lifetime; one admission/retry authority; resources and per-peer failures are explicit and recoverable in the current daemon epoch. | New typed batch/receipt/status contracts and coordinated UI are required. Risks: duplicate sends after lost replies, partial outcomes, capacity accounting, stale recipient names/discovery and accidental cross-recipient cancellation. Freeze these before implementation. |
| 2 — GUI-owned sequence of ordinary sends | GTK/controller retains a pinned source and orchestrates preparation/send separately for each selected peer using existing daemon operations; records which targets were started. | Smaller daemon API change; reuses single-send admission and receipt primitives. | Pending recipient intent depends on GUI lifetime and reconnect state; repeated descriptor/preparation ownership and per-peer retries sit in the presentation owner. Risks: lost unsent targets on GUI exit, duplicate attempts after uncertain results, additional handle/resource reservations and diverging UI recovery paths. |

Recommend1 for clear ownership, correct retry behavior, bounded accounting and
maintenance, independently of implementation volume. Option2 can be deliberately
implemented but is a poorer fit for daemon-owned background sharing. Do not
silently start either, expand queue budgets, add parallel-radio policy, auto-retry
failed peers or claim disk-durable batches/resume as part of this choice.

User-visible selected scope in both options: nearby recipient multi-selection,
explicit selection summary and one Send action; separate status/cancel per recipient.
Merely checking a recipient or choosing source files never sends. Full/Compact,
lock/DND/preview/disconnect gates and incoming PIN/Accept remain intact. Single
recipient remains supported. No hidden immediate sends while building the set.

## Staged plan after selection

1. [x] M0: inspect actual selection/transfer/resource/receipt/UI owners and record
   options and recommendation. Documentation-only; no runtime checks repeated.
2. [x] M1: freeze exact bounded admission, partial-outcome/structural-failure,
   member identity/order, source retention, capacity/counter preflight and
   request/acknowledgement/reconnect contracts. Preserve supported single-send
   behavior or document the coordinated API extension explicitly. Bound recipient
   projection/receipt storage and disclose existing source mutation semantics.
3. [x] M2: implement real coordinator/domain admission and lifetime/receipt logic;
   inventory branches before deterministic unit/integration regressions. No
   domain mocks or GUI-owned substitute for the selected authority.
4. [ ] M3: authenticated typed native IPC, exact-retry SDK/controller, safe
   per-recipient projection and GTK selection/summary/status/cancel. No new peer
   wire format, account mode, automatic retry or unrequested trust behavior.
5. [ ] M4: affected checks once, actual observations in SSOT, atomic commits,
   changed complete candidate build and exact handoff. Preserve native9 immutable
   artifacts and keep installed/physical-peer and release gates separate.

Rollback: remove client/UI exposure before reverting the new daemon admission;
preserve existing one-recipient transfer and receipt contracts. No persistence
migration or durable background queue is assumed. No package rebuild for M0.
Option1 is selected; implementation follows the frozen M1 contract below.


## M1 frozen implementation contract (before code)

Local admission is atomic, remote transfer outcomes are independent. One Send
requests1..10 unique discovered endpoint IDs in explicit order (existing default
queue budget10, no capacity increase). Validate the whole request, ownership,
Running/synchronized state, all endpoints, queue/transfer budgets and the full
revision-ID range before consuming the prepared Selection or allocating a member.
Unknown/stale peer, invalid/duplicate ID, insufficient capacity or exhaustion
leaves the selection and queue intact. No partial local send is hidden behind an
error. After admission, each ordinary peer session independently accepts/rejects,
progresses, fails or cancels; no group-wide rollback or automatic retry. Retry of
a terminal failed peer remains a new explicit preparation/send as in single-send.

The daemon creates one ordinary Transfer per recipient, in request order, sharing
the existing Arc<PreparedSource>. The original Reservation is charged once until
all queued/active readers release it. No reopened path, copied payload, extra
long-lived source holder, new parallel-radio policy, disk queue or restart resume.
GUI close/disconnect releases only unadmitted client preparation; admitted queued
and active transfers retain source authority. Source external-mutation guarantees
stay those of existing positional/size-checked reads, not immutable byte snapshots.

Extend the one ordered operation ledger with SendMany and vector transfer receipt.
Exact same sequence/selection/ordered IDs returns the same vector; changed retry
is ReplayConflict, acknowledged sequence RequestExpired. Check enough revision
space before creating any member. Receipt pins every member until acknowledged;
retired/acknowledged entries cannot start new work. Do not create another replay
ledger or an unbounded Batch history owner. Rust OperationResult loses Copy and
adds Transfers(Vec<TransferId>); coordinated local consumers migrate together.
Existing single-send methods/semantics and all existing D-Bus tuple signatures stay.

New authenticated Batch1 facade on the existing path:
Send(selection:s, recipients:as, sequence:t)->transfers:as;
GetReceipt(sequence:t)->(kind:s, transfers:as);
GetRecipientPage(snapshot:s, offset:u)->(next:u, rows:a(ss)).
Use existing Operations1 state/acknowledgement and Manager2 capture lifetime.
Operations1 scalar GetReceipt explicitly rejects a batch receipt (InvalidState);
batch SDK uses Batch1 receipt endpoint, old clients cannot issue that new command.
Bound recipient array decoding before collecting more than10 IDs, validate strings
<=256 ASCII graphic and duplicates. No descriptors, caller-provided addresses or
names. Same-UID sender authentication, connection ownership, epoch/counter syntax,
frame/inbox limits and unknown-method checks apply unchanged.

Capture recipient display names from daemon discovery at admission, not from the
caller. Bound names to256 bytes, no controls; empty advertised name becomes
Unnamed nearby device. Names are presentation, never identity/authentication.
Store a private optional name on each outbound Transfer; do not remove it when
next_send consumes its endpoint. Keep recipient rows in the same immutable
Manager2 capture, aligned one-to-one with its transfer rows. Existing capture
memory accounting includes owned name strings/row storage; retain current budgets.
New client TransferView adds optional recipient name with redacted Debug. Validate
row ID alignment/count/name and forbid a recipient on inbound rows. Existing
single-send rows may have no captured name; existing payload detail/PIN behavior
is preserved. No separate batch ID or persistent user history is introduced.

SDK retains exact SendMany in the existing one pending mutation slot, validates
all returned unique canonical IDs/count/order, recovers a lost response through
the shared sequence state and Batch1 receipt, then acknowledges once. Daemon epoch
change never silently replays. PanelController queues one action and publishes
member transfers via the existing watch; no GUI loop over SendSelection.

GTK nearby rows use bounded checkboxes and a selection summary with one explicit
Send button. Checking peers or choosing files never sends. Selection may precede
file preparation. Keep a checked vanished peer visible with its last name and an
unavailable marker, allow explicit uncheck, disable Send until all selected peers
are currently present. Do not silently drop recipients or auto-send on reappearance.
On a race, daemon rejection retains source/recipient choices for review. Clear
choices after confirmed batch result or its exact recovery, and on daemon epoch
change/close. Keep existing Full/Compact, preview/lock/busy/disconnect/ingress gates.
Display stable recipient on each transfer; per-transfer Cancel stays independent.
Compact continues directing recipient choice to Full; no additional large window.

## M1 focused regression inventory

| Artifact | Branches / interactions | Level / planned new methods | Categories |
|---|---|---|---|
| Coordinator/ordered batch |1/10 and invalid0/11 recipients; duplicate/invalid IDs/names; owner/epoch/running/sync; unknown peer; queue/transfer/revision preflight all-or-none; ordered exact/altered/acknowledged replay | unit /3 table-driven | Positive,N1,N2,N4,N5,N7,N10 |
| Shared source/lifecycle | one reservation across members; independent positional reads; disconnect; cancel one member; release only after all readers; receipt pins each terminal member | unit /2 | Positive,N2,N4,N5,N9,N12 |
| Bus/SDK/controller | actual private broker; ordered result/captured names; lost reply and ack recovery; malformed types/FD/oversized arrays/foreign owner; immutable capture and privacy; one controller command creates N transfers | integration /3 | Positive,N1,N2,N3,N4,N5,N6,N7,N9,N10 |
| GTK projection | checkbox-only never sends; limit/uncheck; vanished checked peer retained; explicit summary/Send; unavailable/pending/locked/preview gates; result/recovery clears choices; stable recipient labels and per-peer cancel | extend existing GTK smoke /1 helper | Positive,N1,N2,N4,N5,N7,N9,N10 |

Use real coordinator/resources/receipt logic, temporary files/private broker and
existing reply interruption seams. No subject mocks or sleeps. Existing transport/
file reader/source mutation/store suites protect unchanged peer/session and disk
logic. N8/N11 inapplicable (no new wall clock/billing); N6 concerns malformed IPC,
not a new persisted schema. Important assertions kill partial local admission,
shared-offset or per-recipient resource double charging, replay duplicates, wrong
member pins, lost names on queue dispatch, hidden vanished targets and automatic
send from selection. This is Mode A, checks once per change/failure, not Mode B
whole-product coverage or ten-run stability. Update observed counts after execution.

## M2 descriptor-accounting refinement before final source

The initial full645-method check passed644 and caught an incorrect SDK fixture
assumption: native upload reserves two handles per file (original plus one future
worker clone), not one. Inspecting prepare_outbound_selection also exposed the
new batch-specific extension needed before final code: N concurrent peer workers
can each clone every source descriptor. Shared Arc ownership alone does not make
those physical handles free. Keep one source/byte reservation, but preflight and
conservatively extend its handle slots to at least files*(N+1), retaining them
until the last shared reader/queued owner drops. Check the aggregate existing
reserved_handles budget before ID allocation or source consumption. Do not raise
the default1024-handle budget or weaken assertions to hide cloning. Text has no
file handles. Extend the validation matrix with handle exhaustion and assert the
exact N+1 reservation in source/SDK lifetime cases. This is scoped admission
accounting, not a shared-file type or transport architecture refactor.

## M2 observed verification (2026-10-16)

Implemented atomic coordinator admission, shared descriptor reservation, common
operation ledger/vector receipts and receipt pins. The coordinated Batch1 bus,
SDK/controller and captured recipient projection are implemented for integration
verification before the GTK exposure is committed. No parallel transport policy
or persisted batch owner was added.

Initial compile found missing recipient zvariant Type metadata and a test helper
match arm; both were corrected. First full core run:645 methods,644 passed, one
SDK reservation assertion failed (13.96s). All637 pre-existing methods passed in
that run. The failure led to the descriptor-accounting refinement above. After
that source/test change, all8 affected new methods passed (0.29s),637 filtered.
Strict all-feature/all-target Clippy initially caught two byte-slice style errors
in the new domain assertion; corrected without changing its values. Final strict
Clippy passed (see command log for elapsed build time). No repeated full-suite or
ten-run stability claim. Logs: /tmp/rqs-multi-core.log,
/tmp/rqs-multi-affected.log, /tmp/rqs-multi-core-clippy-final.log.

Terminal commands completed and their owned sessions exited: Terminal closed.
M3 GTK widgets are in progress; native9 and installed applications are unchanged.
