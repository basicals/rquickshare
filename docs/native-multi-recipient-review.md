# Native multi-recipient sharing: ownership review

2026-10-16. Mode A, direct dev. Guide: /home/morozhenko/.codex/AGENTS.md,
sections3/3.1/11/12/13. This is the next independent parity slice while canonical
preview codec/mapping remains unestablished. No multi-send behavior/API/UI code
is authorized by this review until the significant choice below is selected.

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
2. [ ] M1: freeze exact bounded admission, partial-outcome/structural-failure,
   member identity/order, source retention, capacity/counter preflight and
   request/acknowledgement/reconnect contracts. Preserve supported single-send
   behavior or document the coordinated API extension explicitly. Bound recipient
   projection/receipt storage and disclose existing source mutation semantics.
3. [ ] M2: implement real coordinator/domain admission and lifetime/receipt logic;
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
Significant selection is pending under guide3/3.1; stop after presenting it.
