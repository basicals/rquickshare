# Daemon-owned custom-card delivery

2026-10-13. Mode A. User accepted A/option 1 from the background ownership review.
Guide: /home/morozhenko/.codex/AGENTS.md. Work on dev only, no installation/push.
This document freezes the implementation contract before production changes.

## Analysis and invariants

Coordinator serializes transfer observations and revision changes; SelectionBus
serializes authenticated same-UID calls and immutable connection-owned StateCache
captures. NativeClient validates the complete capture, and PanelController owns its
connection/action queue. GTK currently infers transient notices from phase changes;
it loses dismissal on restart and cannot distinguish pending terminal notices from
history. The chosen design makes that distinction authoritative in the coordinator.
No new disk schema, transport owner, source authority or account identity is added.

## Frozen notice contract

One retained record per retained transfer, bounded by the existing 256-transfer
bus projection limit. Eligible events: inbound AwaitingConsent with its current
round; Finished; Failed; Disconnected. Rejected/Cancelled do not create a completion
popup. Intermediate progress does not create records. Every exact eligible event
records its event revision at observation time. A different round/outcome replaces
that record; noneligible phase retires it. Dismissed records remain tombstones for
that event until phase replacement/transfer retirement, preventing regeneration.
Retirement removes the record with its transfer. Registry belongs to daemon epoch,
survives GTK restart, and intentionally does not survive daemon restart/reboot.

Add Notifications1 at the existing service/path, with methods:
- GetNoticeCount(snapshot_id s) -> count u.
- GetNoticePage(snapshot_id s, offset u) -> next_offset u, notices a(sstt).
- Dismiss(notices a(sstt)) -> empty.
A row is (canonical transfer token, kind, consent_round, event_revision). Kind is
request/finished/failed/disconnected; round >0 only for request, otherwise zero.
Epoch is embedded in the canonical transfer token. Event revision >0 and <= capture
revision. Reads use the existing immutable caller-owned capture, pages <=16, count
<=256; existing base capture memory reservation includes these bounded metadata.
Dismiss accepts 1..16 unique rows, prevalidates the entire batch and all canonical
identities before mutation, rejects descriptors/malformed input. Matching current
records become dismissed atomically; stale/unknown same-epoch events are no-ops.
It cannot respond/cancel/delete, affect a newer round, or mutate a captured view.
A changed dismissal advances StateChanged revision once; repeated dismissal does
not. Same existing UID/sender/path checks and 64KiB/8-inbox framing apply.

NativeClient exposes validated Notice values in complete StateView and idempotent
exact dismissal; PanelController serializes dismissal with existing actions. Never
use current transfer round in place of the identity the user actually dismissed.
GTK's local policy only schedules authoritative pending notices and keeps a
bounded exact dismissed-in-flight set until fresh daemon state arrives. Safety,
manual surfaces, source ownership and explicit consent/readiness remain unchanged.

Daemon wake uses org.freedesktop.DBus.StartServiceByName with the fixed package
Desktop service identity. One owned coalescing worker processes monotonically new
notice generations, no per-event task, path, shell, timer or respawn-on-Quit loop.
SelectionBus never blocks transport progress on GUI activation. Activation errors
retain notices, never terminate daemon or repeatedly retry the same generation.
Cancellation drops pending activation and joins/closes its owned connection.
Source-only D-Bus activation assets use a fixed installed executable; do not install
or activate the actual user bus during verification. Development daemon without
assets continues safely with pending notices for manual GTK startup.

The GTK lifecycle must explicitly distinguish preview, connected and unavailable
backend. A fixed-code error presentation route shows CLI failure without daemon
connectivity; it accepts no paths/free text and performs no file operations.
Reconnect is explicit, never silently replays an old preparation on a replacement
owner. Stable keyed GTK rows retain user focus/manifest expansion across progress
updates; mode changes can replace layout deliberately. This scoped lifecycle work
is included in the accepted proposal, without broad settings migration.

## Milestones / rollback

1. [x] N1: coordinator record lifecycle, Notifications1 XML/capture/read/dismiss,
   SDK/controller bindings, scoped checks. Rollback consumers before additive API.
2. [x] N2: canonical GTK presentation/dismissal and stable keyed widget updates.
   Rollback GTK bindings without altering transfers/source selection.
3. [x] N3: owned daemon activation worker and source-only package assets.
   Rollback activation before removing its package assets.
4. [x] N4: explicit degraded backend, fixed-code visible CLI errors and reconnect;
   retain no-silent-source-replay and owned shutdown.
5. [x] N5: one affected verification pass, actual observations, current acceptance
   gates, atomic commits and exported handoff. No ten-run development repetitions.

## Scoped verification inventory before tests

| Artifact | Risk / branches and interactions | Level / planned methods | Categories |
|---|---|---|---|
| Coordinator notice lifecycle/dismiss | phases, new/same rounds, progress, exact/stale/duplicate batch, atomic invalid rows, epoch, retention, consent unchanged | unit 2 table-driven | Positive,N1,N2,N4,N5,N6,N7,N9,N10 |
| StateCache notice reads | 0/1/15/16/17/256, offset max/overflow, immutable capture after dismissal, wrong owner/replaced capture | integration 1 table-driven | Positive,N1,N2,N5,N7 |
| Notifications1 + SDK/controller | real private broker, complete validated capture, dismissal reconnect/no-op, malformed row decoder, unknown API, shared action queue | integration 2 | Positive,N1,N2,N3,N4,N5,N6,N7,N9,N10 |
| GTK canonical policy/keyed rows | pending terminal on cold start, exact dismissal, new round, DND/lock, retained focus/expansion, shared modes, lifecycle | pure + existing GTK smoke, 1 new method | Positive,N2,N4,N5,N7,N9,N10 |
| Wake worker/assets | coalesce/new generations, failure keeps pending, fixed identity, active GUI no respawn loop, cancellation and joined IO | private broker 1 + static assets | Positive,N3,N4,N5,N7,N9,N10 |
| Degraded GUI/error/reconnect | missing/replaced backend, finite codes, no source replay/send, unsafe desktop hides, shutdown | scoped private broker + GTK, 1-2 methods | Positive,N1,N3,N4,N5,N7,N9,N10 |

No time/date arithmetic, billing or new persistent storage: N8/N11/N12 are not
applicable to new notice state. Existing source/storage checks remain unchanged.
Use real coordinator/SDK/policy, fake only external bus/desktop/activation providers.
Assertions must kill removed exact-identity/owner/bounds/round checks and accidental
consent mutation. No real peers, session registration, install or transport launch.

## Observations

Plan/signatures recorded before code. Previous C1/C2 checks are historical evidence;
none establishes new N1-N4 behavior. Physical KDE startup/focus/lock/monitor/DND,
installed package upgrade/removal and real Google peer parity remain external gates.


### N1 observed checkpoint

Coordinator records eligible events at their actual observation revision, preserves
exact dismissed tombstones, retires records on cancel/shutdown/history pruning,
and never mutates consent through presentation dismissal. Notifications1 reads the
same immutable capture as transfer details. SDK validates bounded complete notice
pages and the controller serializes exact dismissal. Five new methods cover
coordinator lifecycle/atomic rejection, immutable page boundaries, real SDK restart/
controller dismissal and malformed wire batches. No domain subject is mocked.

Initial compilation failed on borrowed data from temporary message bodies; retaining
the actual body fixed both lifetimes. All 216 affected service methods then passed
once (6.48s), including the first four new methods. The subsequently added negative
wire method passed once (0.24s). Nightly formatting passed. Existing five core
no-default-feature warnings are unchanged; final lint and downstream GUI checks
remain N5. No installed bus/core transport/Google peers were exercised. All owned
command/private broker sessions exited. Terminal closed.


### N2 binding / scoped verification refinement before affected checks

GTK now schedules canonical pending event revisions, including pending outcomes on
cold start, instead of inferring completion from a previous renderer snapshot.
Its bounded local dismissal set bridges the exact daemon acknowledgement. One
controller command can contain at most 256 known identities, serialized into wire
batches of at most 16; this prevents filling the eight-action queue merely because
many coalesced known notices are hidden. Already queued exact dismissals remain
worker-owned if a GUI waiter is replaced/closed; failures are not claimed confirmed.
This changes neither source recovery nor the ordered transfer mutation ledger.

Keyed transfer/endpoint rows retain their GTK objects for progress/readiness
changes. Description/round/phase/owner replacement rebuilds the affected row;
retired keys are removed and other rows stay parented. Stable button names identify
controls, avoiding text-based matching. The existing single GTK smoke verifies
actual retained expansion/child identity, reordering and retirement using the same
layout function. This does not claim physical compositor keyboard focus. One new
pure policy method covers authoritative pending terminal state on cold start and
canonical retirement; the prior full safety/round policy table is retained.

First native run passed all 13 methods (0.24s) before adding the final cold-start/
keyed-layout checks, with one unused old phase field subsequently removed. Final
N2 affected checks follow. No ten-run loop or real desktop registration.


N2 final affected policy/widget checks passed 3/3 (0.25s). The first added assertion
required Debug for the event identity; deriving it preserved strict equality rather
than weakening the assertion. The new pure method and extended GTK smoke pass;
no physical keyboard focus claim. Native inventory is now 14 methods. All owned
command/display/private broker sessions exited. Terminal closed.


### N3 concrete activation adapter before verification

Standard source checked: https://dbus.freedesktop.org/doc/dbus-specification.html#bus-messages-start-service-by-name
StartServiceByName(s,u) returns 1 for started and 2 for already running. The daemon
uses a dedicated bounded bus connection and one coalescing worker. publish_changes
only submits the greatest pending event revision; revisions never decrement, so
same-event errors or GTK Quit cannot trigger a respawn loop. No transfer runtime
waits for activation. Worker cancellation drops a blocked request and joins/closes
its connection; startup cancellation also cleans the reserved service name.

The scoped external adapter seam permits deterministic failed/blocked activation
barriers. One new method checks the actual private broker missing/already-owned
service replies plus coalescing/repeated generations/cancellation with a bounded
provider fixture. It does not replace or mock coordinator/notice policy. Source
asset points to /usr/bin/rquickshare and is packaged/removed with the native binary.
No live session activation or installation is performed.


The first N3 test found an actual adapter defect: the private reference broker
returns ServiceUnknown without an activation file even when the name is owned.
The adapter now checks the fixed name's current same-UID owner first; an existing
GUI needs no activation file. Missing owner uses StartServiceByName. The assertion
that an existing GUI is usable is retained. The original test helper's private
module access also failed compilation; a local bounded scenario deadline replaces
that helper dependency. No assertions are weakened and no production timeout is
introduced. These observations remain recorded rather than replacing the failure
with a claimed initial pass.

## N4 concrete lifecycle/error contract before implementation

Add Desktop1.ShowError(code s)->empty with finite codes invalid_selection, busy,
recovery, backend_unavailable and panel_unavailable. Same actual-sender/UID checks
apply; unknown code is InvalidInput. DesktopState carries the last fixed code and
an error revision, separate from recipient-show revision; no paths or arbitrary
text cross this route. Error display never acquires, sends, answers or cancels.
A server may exist without a transfer controller: it can display fixed errors,
while Begin/import fails Unavailable before acquiring a preparation permit.

GTK startup failure to connect the daemon enters an explicit unavailable backend
state, distinct from inert preview. Tray/privacy/error ingress still work. Provide
an explicit Reconnect action and state that unsent preparation is released and
files must be chosen again. Reconnect joins/closes the complete previous client/
GUI ingress lifecycle before creating a fresh client in the same process; no new
OS dialog process or mutable cross-generation controller is needed. Display mode
is retained during this explicit reconnect. It never starts the daemon or replays
source data/uncertain sends. Acquiring/staging files disables reconnect until that
owned work finishes or is explicitly aborted. Accepted daemon transfers survive.

CLI failure prints its existing fixed stderr message and sends one finite error
code to the same GUI service, starting the fixed same executable only if absent.
This does not retry acquisition/import/send. If the graphical platform/session is
itself unavailable, presentation can fail explicitly; no system-popup fallback.
Scoped checks: one real private-broker absent-backend/error/invalid-code/import-
rejection scenario, one CLI finite-code/startup scenario, plus the existing GTK
smoke for unavailable versus preview and explicit reconnect intent. Core/SDK are
real; only desktop/broker/activation providers are fixture-owned. No live install.


N3 affected method passed once after the ownership check correction (0.01s),
including actual private-broker replies and deterministic failure/coalescing/
cancellation. All owned broker/command processes exited. Terminal closed. The
activation source asset is not installed and real startup remains a package gate.


### N4 implementation checkpoint before final native checks

DesktopServer can serve finite ShowError without a transfer backend, and rejects
Begin/import Unavailable before acquiring the gate. Its actual private-broker
method passed 1/1 (0.03s): every code, unknown/malformed codes, no source admission,
UID denial, old GUI owner replacement and joined shutdown. Eight existing desktop
ingress methods also passed (0.44s) after the optional-controller binding change.
An initial enum delimiter typo failed parsing and was corrected before these runs.

CLI failures carry a typed code from their origin; no classification by message
substring and no raw paths/text in ShowError. GTK distinguishes unavailable mode
from explicit inert preview. Reconnect is visible in both modes and performs a
joined same-process lifecycle restart with the selected mode; it cannot implicitly
replay a source/transfer. Old windows are explicitly destroyed on close. Local
acquisition/staging disables reconnect. The added CLI fixture checks real source
acquisition/import failure and fixed-code routing on a private broker with no
transfer backend; it never launches a GUI or transport. The existing GTK smoke
now checks unavailable presentation/reconnect readiness/intent as well as preview,
keyed expansion and safety. Final affected native checks follow.


All 15 native methods passed once after N4 implementation (0.20s), including real
CLI acquisition/error routing and the unavailable/preview/reconnect GTK assertions.
Review of same-process reconnect also found a display-global CSS provider lifetime:
the panel now explicitly removes its own provider on close before destroying its
window. This prevents accumulating providers across repeated reconnects and never
removes another application's provider. The affected widget check and strict native
lint follow this owned-resource correction. No unrelated settings cleanup.


Strict native lint rejected an unnecessary checked unwrap in canonical policy;
matching the optional event revision fixes the cause without allowing the lint.
The final affected policy/widget checks and strict lint are rerun for that change.
N5 also strengthens the existing real SDK/controller restart scenario from four
notices to 21, exercising controller dismissal across the wire batch boundary
(16+4 after the first dismissal). This verifies the N2 internal 256-identity command
binding rather than assuming a small batch proves chunked behavior.


N4 final affected native policy/widget checks passed 3/3 (0.18s); strict native
all-target Clippy passed (2.29s). Earlier full native 15-method and core desktop
8+1-method observations remain valid for their affected code. No repeated unchanged
suite or final stability claim. All owned display/broker/command sessions exited.
Terminal closed. N5 now checks the strengthened chunk boundary, affected process
startup and final core lint/build, then refreshes the handoff.


N5 actual final core service run passed all 219 methods with default features
(5.38s), including the 21-notice controller chunk scenario and changed process
startup. Strict core lint then found double-must-use emitted by the old async-trait
macro on the new private adapter. The adapter now uses native return-position
future bounds with explicit Send; no lint allowance or dependency update is added.
Only the affected wake checks and failed lint are rerun after this implementation
change, not the completed 219-method suite.

Scoped N5 verification refinement: an additional integration method injects a blocked
external activation provider into the real SelectionBus/Harness and NativeClient,
read/dismiss actual canonical notices while activation is blocked, then close the
bus and verify joined cancellation and unchanged consent. This directly verifies
that activation cannot block the serial service, complementing the coalescing
adapter method. Primary N5; secondary N3/N4/N7/N9. No real transport or user bus.


Final affected wake methods passed 2/2 (0.04s), including the real serial bus reading
and dismissing notices while activation is blocked. Its joined shutdown closes the
provider and leaves consent AwaitingConsent with no control intent. Final strict
core all-target Clippy passed (8.46s), with the existing bluer future-compatibility
notice only. No allowances or weakened assertions. The 219-method run plus this
new integration method cover 220 distinct affected service methods across recorded
runs; this is not a fresh single 220-method execution. Core inventory is 512 unit/
export + 8 integration; native inventory 15; combined 535 is an inventory, not a
whole-suite/coverage claim. Final binary builds/format/assets/captures follow.


## Final N5 observations / handoff

Both final binaries built without running them: daemon 17.61s; native GTK 8.29s.
Final nightly core/native format checks, git diff check and Notifications1/ShowError
XML/fixed activation asset checks passed. Six final compiled inert captures were
regenerated and the incoming Compact/Full images visually reviewed: Compact
420x170/336/219 for nearby/incoming/transfers; Full 420x497. No dominant drop zone.
These are synthetic disabled-control previews, not compositor/lock/focus acceptance.
All owned command/display/private broker processes exited. Terminal closed.

Ten new scoped methods across N1-N5: N2=1 (immutable page bounds), N3=3 (activation,
absent backend, CLI failure), N4=1 (exact replay/lifecycle), N5=3 (SDK restart/chunks,
canonical cold start, activation/bus independence), N7=2 (atomic/wire rejection) by
primary purpose, with Positive/N1/N6/N9/N10 interactions in their tables. Existing
GTK smoke is extended rather than multiplying UI tests. The CLI source-failure
case reuses existing filesystem acquisition; notices introduce no storage schema.
No new notice time/billing/storage arithmetic: N8/N11/N12 do not apply to registry
state. No complete Mode B, ten-run stability, mutation-engine or whole-535 coverage
claim. Detailed failed observations and affected reruns above remain recoverable.

Completed source commits: 55905bd canonical daemon records/API/SDK; 6c39be0 canonical
GTK policy/keyed rows; 49df758 owned fixed GUI activation/assets; 1c41eb7 degraded
backend/error/reconnect; 428ddf7 native activation futures and real blocked-bus
verification; 7563db7 cross-batch controller regression. Roll back GTK/activation
consumers before daemon notice API/state. Work remains on dev, without push,
installation, live tray/daemon/settings registration or Tauri retirement.

C3 is now implemented in source. Registry/dismissal survives GTK restart for the
current daemon epoch, not reboot/daemon restart. Package assets are source-only;
real installed startup/upgrade/removal, KDE lock/DND/overlap/focus/scale and actual
Google Android/Windows peers remain acceptance gates. Mode preference is current-
process only. A separate tray DBusMenu dropdown, durable native settings migration,
clipboard/reveal/autostart/updates and full Google feature/compatibility parity
remain outside this completed notification slice. No new significant decision is
pending inside N1-N5; future settings/package ownership reviews retain AGENTS rules.
