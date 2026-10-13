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
2. [ ] N2: canonical GTK presentation/dismissal and stable keyed widget updates.
   Rollback GTK bindings without altering transfers/source selection.
3. [ ] N3: owned daemon activation worker and source-only package assets.
   Rollback activation before removing its package assets.
4. [ ] N4: explicit degraded backend, fixed-code visible CLI errors and reconnect;
   retain no-silent-source-replay and owned shutdown.
5. [ ] N5: one affected verification pass, actual observations, current acceptance
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
