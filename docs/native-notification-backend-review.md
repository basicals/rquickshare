# Background custom-card ownership and API review

2026-10-13. Mode A. Review only; awaiting a new public-contract/data-model decision.
Recommend A for architectural quality. The user-selected fully custom card, Full/
Compact modes and settings/right-click separation remain fixed in both options.

## Concrete implemented state

C1/C2 in native-custom-card.md implement the actual GTK surface, checked existing
controls, bounded transient presentation policy and owned read-only KDE safety
monitors. GTK can present current daemon requests and observed completion/error
transitions while its process is running. Unknown lock/DND suppresses automatic
presentation; current requests stay in the existing daemon coordinator. There is
no takeover of the system notification service or system-popup fallback.

GTK Quit still closes the graphical process. The daemon does not yet wake a renderer
for a later background event. Initial historical terminal transfers deliberately
do not generate popups, so simply launching GTK on completion is insufficient to
identify an undelivered new completion. Transient presentation dismissal is also
currently scoped to the GTK process. Dolphin errors still use fixed stderr when
the GUI/backend is unavailable. These are release-blocking C3 gaps.

This requires actual daemon notification state or a supervised presentation channel;
an unauthenticated path/token handoff or best-effort unacknowledged event is not a
durable implementation. The global guide sections 3/3.1/13 require a decision for
the new public contract/data model or substantial process refactor before code.
The preceding UI selection authorized the custom surface, not either detailed
background-delivery contract below. C3 production code is not implemented yet.

## Alternatives

| Option | Contract | Advantages | Costs / risks |
|---|---|---|---|
| A — daemon notice read model (recommended) | Add a bounded in-memory notice registry to the authoritative service state and expose versioned immutable reads plus exact-notice dismissal. GTK renders it and uses the existing consent/cancel APIs. Daemon wakes the fixed package-owned GUI service on new pending notices. | One recoverable source of truth across GTK restarts; state and notice belong to the same captured revision; no second presentation transport; fits the accepted thin D-Bus-client architecture. | Extends local API and capture/state lifecycle; dismissal and notice retirement must not affect consent or newer rounds. Requires a deliberate degraded-GUI lifecycle for backend-unavailable errors. |
| B — daemon-supervised renderer with a private acknowledged channel | Daemon owns notice records and a bounded private IPC stream to a supervised GTK renderer, with sequence/ack and reconnect recovery. Existing transfer reads/controls remain on D-Bus. | Isolates rendering failures and avoids adding notices to the public transfer read model; renderer protocol can evolve separately. | Adds another IPC/lifecycle and coordination with the existing tray/CLI panel; must prevent duplicate surfaces, distinguish delivery from dismissal, handle child failure and keep source selections with their original SDK owner. |

Choose A because notifications are a projection of already authoritative transfer
state. Keeping their identity/dismissal alongside that state permits coherent reads
and recovery without reconciling an additional private protocol. Work volume and
speed are not the reason for this recommendation. B is viable when a separately
supervised renderer is a product requirement; no such independent renderer is
currently required by the selected card UX.

## Reviewable A proposal before signature freeze

- Registry is in daemon memory for its current epoch; it survives GTK shutdown,
  not a daemon restart/reboot. No new disk schema, account identity or cloud service.
  Bound records by existing retained transfers; no new record on progress ticks.
- Notice identity includes daemon epoch, transfer ID, kind, consent round when
  applicable and notice event revision. New round/outcome retires the old identity.
  Initial history is distinguishable from pending undismissed events. Terminal
  retirement prunes records under the existing bounded transfer retention rules.
- Proposed additive Notifications1 surface: bounded notice metadata/page reads on
  an existing caller-owned immutable capture; exact-identity/bounded-batch dismissal.
  Freeze concrete XML, enums/budgets and errors after direction approval, before
  code. Preserve Manager2/Operations1 source and transfer authority. No arbitrary
  filenames/text, commands, recipients, executable paths or file opens in this API.
- Closing/hiding acknowledges presentation dismissal only. It never accepts,
  rejects, cancels or deletes a transfer. Old/duplicate dismissal cannot dismiss
  a newer round. User response still checks actual current consent readiness and
  the core receipt; recording intent is not reported as completed reception.
- Daemon wake uses a constant package-owned GUI activation identity, not an IPC
  path/executable argument. Track attempts by current notice generation, without
  timers or immediate respawn loops after user Quit/activation failure. Provider
  safety is still checked by the renderer before any sensitive surface is mapped.
- Uninstalled development remains explicit and isolated. Do not install activation
  into the user's bus as a test, launch another RQS, or treat package-source assets
  as installed delivery proof. Actual installed startup/upgrade/removal stays gated.
- GTK needs a deliberate backend-unavailable state that can show a fixed error
  without creating a transfer client. A narrow fixed-code presentation route and
  explicit reconnection replace ad-hoc dialog executables or raw diagnostic text.
  Source/preparation owner replacement must retain the existing no-silent-replay
  contract. This is a scoped GTK lifecycle refactor, not settings migration.
- Full/Compact modes share stable state and controls; retain user interaction
  during progress updates. Current row rebuilding can reset expanded/focused
  widgets; keyed GTK row reconciliation is required before final focus acceptance.
  Avoid an unrelated broad UI rewrite or changes to consent/transport behavior.

## Implementation stages after approval / rollback

1. Freeze notification identity, immutable reads/dismissal XML, capture and retention
   invariants; document actual scoped artifact/branch inventory before tests.
2. Implement daemon registry and private-broker reads/dismissal using the real
   coordinator. Snapshot/notice revision must agree; core receipts remain authoritative.
3. Bind GTK to canonical notices and backend health, preserve keyed widgets, and
   route fixed CLI errors without paths. Keep source gate/selection ownership.
4. Add owned event-driven GUI activation and source-only package assets. Test
   activation failure/recovery with explicitly owned private fixtures; no live bus.
5. Run affected checks once and record observations. Roll back activation/GTK
   consumers before daemon API/state additions. Physical KDE DND/lock/overlap/
   focus/scaling and real Google peers remain acceptance gates.

Either option requires scoped negative checks for new/retired rounds and epochs,
late/duplicate/out-of-order delivery, renderer loss, exact dismissal, retention,
UID/owner/sender validation, bounded queues, backend absence, source preservation,
no implicit response/send and joined shutdown. No Mode B exhaustive or ten-run
claim is made by this review.
