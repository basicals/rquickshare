# Native22 feedback: card, transfers window and discovery

2026-10-17, Mode A. The user tested native22 and reported slow Android-to-Linux
discovery (reverse discovery is fast), interleaved device/completed/unlinked
operation rows, redundant outbound Send steps and overly separated device cards.
The requested UX supersedes earlier recipient-first and inline completed-group
presentation. Work only on dev; do not infer new source/code from screenshots.

## Analysis before changes

The card render currently inserts active transfers, device rows with nested
completed groups and unlinked Recent activity in one scrolling container. Send
opens recipients first; send_to opens the content menu if no source exists, and
prepared source/recipient/multi-send controls coexist. Direct row submission
already uses the coordinator-owned send_many path, so source ownership and
explicit receiver consent must be preserved while simplifying the entry flow.
The screenshot is product failure evidence, not proof of a specific GTK reorder
root cause. Separating completed rows removes the unwanted mixed presentation.

Daemon/SDK already publish bounded current-run transfer snapshots (not durable
history). Session endpoint associations are ephemeral metadata; equal advertised
names do not prove a stable device relationship. Persistent journal retention or
cross-restart remembered devices need an explicit architecture/product decision.

BLE receiver advertising and mDNS visibility/resend are separate from outbound
peer scanning. The initial passive90-minute journal scan returned no matching
known advertisement/refresh events. It neither proves that advertising is absent
nor locates the delay. Diagnose the on-air/GATT/mDNS stages before changing retry
intervals or adding periodic refresh workarounds. Do not alter current transfers,
visibility, network configuration or authorization during passive analysis.

## Requested product contract

- Flat device rows, no separate rounded island for each device.
- Card: devices/prepared source/current receive review; no Completed operations
  or Recent activity transfer history between devices.
- File-first send: explicitly choose content, then one Send at the intended
  discovered device starts admission. Do not introduce an extra sender-side
  confirmation; keep code display and mandatory receiver consent. Choosing a
  file or checking recipients alone must not transmit content.
- A separate Transfers window lists current and completed work in one consistent
  table/list and replaces Incoming requests/Active transfers tray entries.
  Keep explicit incoming review, cancellation and received-content actions valid.
- User-visible history lifetime and remembered-device behavior require choices
  below before implementation. Never match devices by display name.

## Pending decisions and recommended options

1. Transfers journal: recommended daemon-owned bounded persistent metadata
   (last30 days, at most1000 completed operations, explicit clear), surviving
   restarts without storing payload bytes or auto-resuming work. This gives
   honest recovery and one owner but requires schema/storage/API/retention work
   and protects potentially sensitive local names/metadata. Alternative: retain
   only existing current-run snapshots; completed rows disappear after daemon
   restart. It preserves the current storage boundary but cannot serve as a
   cross-restart journal. Wait for the user's choice before public contracts.
2. Device recents: recommended current discovered devices/current receive only
   for this slice; transfer records go to Transfers. No unverified persistent
   device association. Alternative: include remembered devices across discovery
   sessions/restarts, requiring a separately designed stable identity lifecycle;
   names or retained ephemeral endpoint IDs cannot safely implement it.

## Ordered plan after decisions

- [x] A1: inspect native22 source/SSOT/screenshots and read passive existing logs;
  record the requested corrections and missing persistence/identity decisions.
- [x] A2: freeze the chosen journal/device contract, ownership, compatibility,
  retention/error semantics, implementation milestones and rollback before code.
- [x] A3: implement file-first direct single-recipient Send and flat card device
  presentation, keeping admitted multi-recipient/source ownership unchanged.
- [x] A4: implement the selected daemon/window/tray separation, preserving live
  review/cancel and content capabilities. Do not invent durable trust.
- [ ] A5: trace receiver discovery stages with bounded privacy-safe evidence;
  fix only the demonstrated cause and record what needs a live phone check.
- [ ] A6: run appropriate affected checks once (no ten-run development loop),
  record measured failures/corrections and commit logical stages independently.
- [ ] A7: separately verified package and known-owner/global-idle installation
  before the user's physical acceptance. Preserve native22 rollback package.

Rollback: independent UI entry/rows/window/journal/discovery commits, preserving
received files and user configuration. If persistent storage is selected, its
migration and downgrade policy must be documented before implementing it.
No production code, tests, package, service or visibility change in this analysis.

## Approved contract and implementation ownership

The user selected the persistent daemon journal and clarified the card: only
rows representing devices, their status, a progress bar between name/icon and
action, and Send or incoming review/accept. This replaces the earlier optional
recent-device question; no persisted device-recognition system is requested.
An inbound peer without a verified discovery binding is represented by its
current inbound-transfer row, keyed by transfer ID, never merged by name.
Incoming review still exposes the code/manifest before the explicit Accept.

The bounded journal uses one versioned atomic metadata document beside the
existing private native settings directory, owned by the daemon, not GTK.
At1000 records a small bounded document permits one complete atomic transaction
without another database/runtime dependency. Reuse the existing safe private
file primitives (0700 directory,0600 regular single-link file, no-follow pinned
FD operations, temporary exclusive file, fsync/rename/directory fsync). Extract
only these internal primitives without changing settings persistence behavior.
Schema1 stores random transfer ID, untrusted peer display label, direction,
coarse status, payload kind/count/bytes and UTC start/end timestamps. No file
paths/names, PINs, message contents, keys, stable identity or executable actions.
Active records from an earlier daemon become Interrupted on recovery; work is
never resumed. Keep at most1000 most recent completed entries for30 days; current
live work remains in the existing bounded coordinator and is displayed separately.
Backward wall-clock changes must not expire new records or produce negative ages.

One joined IO owner serializes journal load/update/clear; filesystem IO does not
run on GTK or inside synchronous coordinator methods. Terminal transitions are
queued in order with bounded backpressure, no dropped records or unowned tasks.
Metadata writes coalesce unchanged progress; live progress uses existing snapshots.
Normal shutdown joins the owner. Store failure is visible in Transfers while
sharing stays available; corrupt/unknown schema is preserved, not silently reset.
Manual Clear deletes only completed journal metadata, never current work/content;
it is fenced by a content token with an idempotent receipt so a lost reply cannot
clear later records on retry. Explicit corrupt-journal reset preserves an exact
private recovery copy before replacement. Unsafe paths are never followed.

Add a same-UID authenticated Journal1 read/clear API with bounded immutable
connection-owned pages and token fencing. Existing Manager2/Batch1 contracts
remain additive. The SDK validates page count, unique IDs, fields and epoch;
missing Journal1 produces an explicit unavailable history, not a disconnected
sharing service. GTK uses a separate themed Transfers window and existing
controller connection/consent/content actions; it never reads the store itself.
The tray has one Transfers entry replacing Incoming/Active entries. The window
shows current transfers plus completed metadata, not nested completed devices.

Focused verification inventory before implementation:
- Private file extraction: preserve existing settings store/security/rollback
  tests; journal tests inject only filesystem side effects and wall clock.
- Journal positive/edges: load/restart interrupted, terminal updates/idempotence,
  clear fencing/retry preserving newer entries,30-day boundary/backward time,
  0/1/1000/1001, malformed/unknown schema, unsafe permissions/symlink/hardlink,
  write/fsync/rename errors, corruption recovery, redacted Debug/no payload.
- IPC/SDK: same-owner read/clear, stable bounded pages, wrong/stale token/offset,
  older daemon, malformed/duplicate row, no cross-snapshot mutation.
- GTK: file-first disclosure, cancel never sends, one row Send admission,
  no completed operation in card, no name-only inbound association, inline
  0/partial/complete progress, separate window close/reopen and live consent.
- Discovery: passive stage evidence first; any changed rule receives one focused
  regression run and a live-phone timing check, not guessed timing constants.

A2 is complete for the approved scope. Record actual checks per milestone;
no whole-repository Mode B/coverage or repeated ten-run claim is planned.

### Recipient interaction refinement (explicit user instruction)

Remove recipient checkboxes and the aggregate Send button entirely. Each Send
beside a discovered device independently admits the current immutable prepared
source to the daemon FIFO. The prepared selection remains available for later
clicks; queued/active copies retain their own source snapshot. Dispatch one
outbound recipient at a time, waiting for its terminal status and joined source
release before starting the next. Cancel/reject/failure advances the FIFO; closing
GTK does not drop admitted work. The existing bounded queue returns an explicit
capacity error rather than silently dropping clicks. Do not add a GTK-owned
queue or infer recipient order from alphabetic device sorting.

Source-reuse root cause found: existing Batch1.Send consumes the prepared
selection, so repeated per-device clicks cannot reuse it. Add Batch1.Queue with
a distinct ordered-operation kind, retaining the unsent selection and sharing
its immutable source snapshot with admitted queue entries. Keep Batch1.Send
consumption semantics intact. Queue/replay/source budgets stay in Coordinator;
GTK uses Queue, not a reconstructed local source. Add retained-source/FIFO/retry
regressions before installation. Clearing or extending the prepared selection
must not change already queued snapshots.


## Service implementation and measured checks

Daemon journal, safe private-file primitive reuse, additive Journal1 read/clear,
SDK checked pages/cache/invalidation and retained-source Batch1.Queue are in
source. FIFO now waits for terminal status plus owner/source release. Existing
consuming Batch1.Send remains unchanged. No persistent trust or payload storage.

Initial compile stopped on misplaced controller arms and an extra Path reference;
these were corrected before execution. A test compile then exposed three fixture
API/import mistakes, corrected without changing production contracts. The first
service execution had five failures and one old concurrent-dispatch test waiting
for a second job before finishing the first. Only the owned test run was stopped
with its terminal interrupt. The two journal failures used non-private fixture
parent directories; fixture directories now explicitly use0700, with production
storage validation unchanged. Backpressure fixtures now saturate the real external
transport queue rather than violating the newly selected single-outbound policy;
FIFO checks assert no next job before terminal status and joined source release.

Final affected service run:308 passed/0 failed in8.05s (367 unrelated filtered).
No ten-run loop or complete repository/coverage claim. Native GTK checks, passive
receiver diagnosis and package/installation have not yet run for this source.
All completed terminal tasks exited; installed native22 remains unchanged.


GTK implementation observed: device rows are flat and contain no checkboxes;
there is no aggregate Send control. Live outbound progress is associated only
with its exact current endpoint; inbound/unbound live work has its own peer row.
Terminal transfers never enter the card. Code/manifest review remains explicit.
Transfers opens a separate themed window, preserves current content controls,
merges older metadata without duplicating live IDs, and exposes fenced Clear.
Tray now has14 rows with one Transfers entry instead of Incoming/Active entries.

Measured GTK run initially42 passed/2 failed: one old summary-wording assertion
and an event test still clicking the removed Incoming entry. After correcting
those expected contracts, its independent group scenario also needed its Transfers
action explicitly re-enabled after testing disabled-action rejection. Final GTK
run44 passed/0 failed in0.67s. Native strict all-target/no-dependency Clippy passed
in5.15s. Actual offscreen card and Transfers-window frames were inspected: one
flat device list and a separate metadata list. Removed unwanted automatic text
selection from device/header labels. Offscreen Layer Shell/DRI warnings are
expected for Xvfb and are not KDE/Wayland acceptance.

Core strict Clippy first identified a redundant borrow introduced while extracting
the unchanged private-directory helper; remove that borrow and rerun only lint.
No protocol/runtime behavior changed in that correction. A5 receiver timing and
A7 package/physical acceptance remain open; installed native22 remains unchanged.

## Receiver discovery timing root cause and scoped repair

Native daemon startup has no log subscriber, explaining the missing BLE/GATT/mDNS
stage evidence. Add opt-in RQUICKSHARE_RECEIVER_TRACE=1 diagnostics containing
only allowlisted stage names and monotonic elapsed milliseconds. Default is off;
no peer addresses/names, service data, PINs, paths or payloads. A temporary per-unit
runtime environment override may enable it during the authorized idle upgrade;
never change the global user-manager environment or shipped autostart defaults.

Code inspection found two independently reproducible timer-starvation defects in
ReceiverAdvertiser: select rebuilt sleep(periodic_secs) after every other event,
and every Deferred request replaced the previous two-second deadline. Therefore
neither recovery was guaranteed to run amid continuing events. Preserve absolute
per-advertisement periodic and earliest deferred deadlines, clearing both only
when the advertisement handle is replaced/stopped. Keep the existing30-second
release fallback and two-second deferred interval; no new timing magic or consent
change. Refresh touches only the advertisement, not the separate GATT/session
owner. Paused-clock regressions must prove repeated events cannot postpone either
recovery and stop/restart resets their lifetimes. This fixes those code defects;
real Android discovery latency still needs a measured phone reproduction.


Receiver verification: BLE advertiser/schedule9 passed, trace allowlist1 passed;
all use deterministic clock/format rules, no hardware or timing claim. Strict
core all-feature/all-target lint passed13.06s; the actor's original Debug/Clone
traits remain intact. Routine transfer-phase text was removed from the card's
global header because live device rows already convey it; pending-control/error
messages remain. Final affected GTK/widget and daemon lifecycle checks follow.

## Final journal retry correction before installation

The native23 internal package built and verified, but is not installed. Final
review found Read retried a failed journal write on every normal state refresh,
including progress events. That contradicts the explicit Refresh recovery action
and can amplify disk failures. Within the approved journal recovery contract,
make ordinary reads observational except retention pruning; add authenticated
Journal1.Retry with the same bounded response/capture validation, invoked only
by Refresh. It preserves the same metadata/security boundary and never retries
sharing operations. Both paths share one owner and validator; no timer/backoff.

Before implementation: test the real Store with injected failing persistence,
assert a normal read does not attempt a write after storage recovers, then an
explicit retry saves pending records. Exercise Retry through the existing real
private-bus SDK journal scenario. Run affected journal/SDK scopes and lint once.
Preserve the verified, uninstalled native23 package; increment to native24 for
this changed artifact. Rollback is an independent correction commit.

Final pre-correction checks observed: affected GTK widget1/0 in0.52s, daemon
lifecycle integration4/0 in0.02s, native strict lint8.11s passed. Native23 package
builder verified the package; no installed-state or Android acceptance implied.
