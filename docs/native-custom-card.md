# Custom native card: full and compact presentation

2026-10-13. Mode A. User selected the fully custom popup presentation and requested
full/compact modes, a secondary settings menu and right-click actions. This
supersedes the previous native-notification-review.md system-notification proposal.
The global guide remains /home/morozhenko/.codex/AGENTS.md, including quality-first
recommendations and one-run development verification. Work stays on dev.

## Analysis and approved direction

The existing GTK4 Layer Shell panel already has a real bounded controller, safe
source gate, complete Manager2 details, consent-round checks and no automatic Send.
Its 420x498 tabbed view is appropriate as the expanded view, but showing it for
every incoming request would reproduce the oversized primary surface problem.
The user explicitly permits moving secondary controls out of the card.

The daemon remains authoritative for requests, transfer/consent state and resource
lifetime. GTK renders state and submits explicit current-round controls. Custom
presentation does not take ownership of org.freedesktop.Notifications or replace
other applications' notifications. No hidden system-popup fallback is introduced.
Closing a card dismisses presentation, never implies decline/accept/cancel. Explicit
Reject/Cancel still use existing checked SDK/controller operations.

## Plan recorded before implementation / rollback

1. [x] C1: implement one reusable full/compact GTK surface. Compact is the initial
   primary mode; full retains all existing Nearby/Incoming/Transfers functions.
   Add an accessible header menu, right-click menu and secondary Appearance settings.
   Preserve existing source, consent and recovery contracts. Build/review real
   compiled previews, supporting focused widget projection checks, then commit.
   Rollback: native presentation changes only; daemon/IPC stays compatible.
2. [x] C2: implement a bounded presentation policy and actual desktop safety adapter.
   Subscribe before initial read; observe KDE effective DND and screen lock with
   pinned same-UID owners. Unknown/stale safety state suppresses automatic popups.
   No path/content opening, DND mutation or screen-lock bypass. Current request is
   retained in the daemon and remains manually discoverable. Coalesce views and
   select one current request; close on round/phase/epoch changes. Owned IO tasks.
   Rollback: remove automatic binding before the policy/guard adapters.
3. [ ] C3: integrate daemon-owned wake/activation and nonterminal fixed CLI errors.
   Freeze concrete activation/error contracts and inventory before their code.
   Use fixed package-owned GUI activation, not a caller-supplied executable/path.
   No duplicate transport owner, implicit Send, arbitrary error text or polling.
   Keep uninstalled development startup explicit; package assets are not installed
   into the live session. Join wake/renderer IO and preserve queued daemon requests.
   Rollback: activation/error bindings before their source assets.
4. [ ] C4: record final affected checks once, category counts, current limits and
   handoff. Actual KDE DND/lock/monitor/scale/focus, installed package behavior and
   Google peers remain external acceptance. No false final-parity claim.

No broad settings migration, account/visibility changes, persistent settings schema,
clipboard/reveal/update feature or transport refactor is part of C1. Those retain
separate approved-review gates. Finish the actual card before expanding that scope.

## C1 presentation contract

- Compact shows one current incoming request first, otherwise one active transfer,
  otherwise a small ready state. Sender (advertised identity), verification code,
  file count/total and primary actions remain visible. Full manifest is available
  by expanding to Full; no requirement to show every filename by default.
- Ready/unsupported/unavailable detail status keeps its existing accept readiness.
  Both modes retain the exact current consent round and explicit Accept/Decline.
  An action awaiting the daemon/core receipt must not be shown as completed.
- Full retains tabbed nearby selection, incoming requests and transfer controls;
  every relative filename is available through the bounded existing expander.
  No dominant drop zone. Compact/full toggle does not mutate or drop preparation.
- CLI invocation explicitly presents the recipient flow in Full. Tray activation
  uses the selected display mode. Switching modes repositions/clamps the existing
  Layer Shell surface using its measured current height, never an ordinary-window
  fallback. Real compositor acceptance remains pending.
- Header menu and right-click menu expose full/compact, incoming/transfers, source
  selection, discard, recovery, hide, settings and panel Quit as appropriate.
  Primary consent, Cancel and recovery remain visible when needed; no essential
  recovery is available only through an undiscoverable context action.
- The secondary settings surface initially contains the implemented Appearance
  choice only. Display mode is retained for the current GTK process; durable
  preference storage joins the later native settings/migration contract. Do not
  overwrite legacy settings or fabricate toggles for unavailable capabilities.
- One GTK surface and one set of commands; no copied full/compact transfer domain
  logic. Use pure projection helpers and weak widget callbacks, explicitly own
  popovers/settings/completion tasks and close them on panel shutdown. Preview
  remains explicit/inert, with full/compact selectable for review.

## Safety adapter evidence / remaining analysis

KDE's ScreenSaver XML defines GetActive and ActiveChanged. Plasma notification
manager exposes effective inhibited state; verify the current exact D-Bus property
and change signal from primary source before implementing C2. This adapter is
KDE-specific and read-only. Do not infer unlocked/DND-off from provider absence or
parse user configuration timestamps as a substitute for authoritative state.

Sources checked during analysis:
https://github.com/KDE/kscreenlocker/blob/master/dbus/org.freedesktop.ScreenSaver.xml
https://github.com/KDE/plasma-workspace/tree/master/libnotificationmanager
https://github.com/wmww/gtk4-layer-shell

## Scoped Mode A verification inventory before code

C1: existing single real GtkBuilder smoke extended for both modes, menu/settings
routing, complete file review in Full, compact current-request/active priority,
inert preview actions, mode-switch source preservation and hide/quit teardown.
Pure projection tables cover no view, disconnected, pending/busy, Ready/unsupported
or unavailable incoming details, multiple requests, all transfer phases and source
selection. Extend existing placement cases for measured compact/full height.
Expected 2-3 affected/new methods; no per-widget E2E expansion or ten repetitions.
N1/N2 untrusted metadata/bounds; N4 duplicate callbacks; N5 current-round/view changes;
N7 readiness/preview admission; N9 hide/mode/close; N10 plain/redacted metadata.
N3/N6 reused controller state failures; N8 no new time arithmetic; N11 no billing;
N12 no new storage in C1. C2/C3 inventories will be refined from actual adapter code
before those implementations; no exhaustive coverage claim is made for this plan.

## Progress / observations

Analysis and C1 plan are recorded before production changes. No custom automatic
popup, desktop safety monitor or notification process activation is implemented
at this checkpoint. Previous 43 core / 9 native observations remain unchanged.

### C1 observations before final commit

One pure priority/mode method is added; existing widget/parser/placement methods
are extended. Initial native run: nine passed, one failed because the new widget
assertion used effective visibility while the parent window was intentionally
hidden. Corrected to inspect the widget's own visible property; actual display
is verified by compiled captures. Three affected methods then passed (0.16s).
After moving menu/settings code into panel/menu.rs and tightening explicit preview
argument admission, the two affected parser/widget methods passed (0.17s).
Final source hides source-selection controls outside Nearby and fixes singular
file wording; the affected widget passed (0.17s). No domain/SDK change or repeat
stability loop. Native inventory is now 10, combined core/native 522.

Initial captures: compact Nearby 420x170, Incoming 420x336, Transfers 420x219;
Full 420x497 for each page. Actual incoming images were visually reviewed. Final
affected captures/build/lint and expanded height placement table are recorded
after completion below. Preview metadata is synthetic and service controls inert.

## C2 contract and inventory before code

Read-only safety inputs (verified against current KDE primary implementation):
- org.freedesktop.ScreenSaver, /ScreenSaver, GetActive and ActiveChanged.
  KDE also registers /org/freedesktop/ScreenSaver, but use one canonical path.
- org.freedesktop.Notifications, /org/freedesktop/Notifications, Properties.Get
  for interface org.freedesktop.Notifications / property Inhibited; current Plasma
  emits PropertiesChanged with that interface/property and the effective value.
  No Notify/Inhibit/Lock/SetActive call is made by this adapter.

Use two fixed owned monitor connections/tasks with the shared 64KiB/16-FD socket,
queue eight and cancellation. Subscribe to exact broker owner changes before
lookup; obtain actual same-UID unique owners; subscribe to their exact signal
path/interface before reading. Validate reply/signal sender and recheck ownership
before publishing. Provider absence/failure/loss is Unknown, never false. A valid
signal triggers a fresh authoritative read rather than installing an old queued
signal body as current state. No sleep/poll or per-event task. Provider ownership
events allow a fresh generation to recover; transport failure stays Unknown until
the presenter restarts. Close joins both tasks and closes their connections.
This is observation of desktop state, not an atomic screen-lock guarantee; physical
compositor privacy remains mandatory. The surface stays on the normal Top layer,
never the session-lock protocol or an above-lock overlay.

Pure presentation policy retains only current bounded transfer records, keyed by
daemon epoch/ID/consent round or terminal outcome. Initial pending requests may
present; historical terminal records at initial connection do not create popups.
New completion/failure/disconnect transitions may present once. Closing/hiding
dismisses presentation of current known records, never their daemon state; a new
round/request creates a new notice. Progress cannot repeatedly reopen a dismissed
card. Prefer pending incoming over completion notices; show one surface, with
the remaining requests available in Full. Epoch/phase/round retirement invalidates
old presentation keys. Bounds inherit the validated StateView transfer cap.

Unknown lock or observed locked state hides sensitive surfaces. Automatic display
also requires known DND-off and a healthy synchronized running service. DND does
not replace an explicitly opened manual view. Manual tray/CLI interaction and
local preparation must not be interrupted or navigated by an automatic notice.
Automatic notices use the selected Compact/Full mode and retain current controls;
no invented sender/PIN/files or automatic consent. If placement fails, retain the
request and expose a fixed failure rather than opening an ordinary window.

Expected three new methods: pure lifecycle/queue/dismiss/epoch/guard tables;
private broker provider initial/read/change/invalidated/owner loss/replacement;
missing or wrong-UID provider and owned shutdown. External provider fixtures may
implement their published contract; do not mock the actual policy/controller.
Extend the existing GTK smoke for guard suppression and pure presentation mapping.
N1 malformed provider values, N2 bounded records, N3 provider failure, N4 repeated
events/dismissal, N5 round/owner races, N6 new epochs, N7 sender/UID, N9 hide/close,
N10 no private logging apply. N8 no timers, N11 no billing, N12 no new storage.

Additional primary source confirmation:
https://github.com/KDE/kscreenlocker/blob/master/interface.cpp
https://github.com/KDE/plasma-workspace/blob/master/libnotificationmanager/server_p.h
https://github.com/KDE/plasma-workspace/blob/master/libnotificationmanager/server_p.cpp

### C1 completed / actual verification

Full and Compact share real incoming/progress card builders and the same checked
controller commands. Menus and Appearance live in panel/menu.rs; only implemented
appearance controls are offered. Source buttons are visible on Full Nearby only;
right-click/header actions retain their original readiness checks. Full manifest
review remains bounded and plain text. Mode changes retain the shared preparation
permit; the GTK test verifies this gate preservation, not a new physical prepared-
selection E2E. CLI explicitly opens Full recipient choice. Close is idempotent and
owns the settings window/popover teardown. No source, ledger or daemon API change.

Final affected widget passed 1/1 (0.17s); parser/widget after module extraction
passed 2/2 (0.17s); expanded height/edge/origin placement table passed 1/1. Earlier
successful unaffected native methods are retained as measured observations, with
no full-suite repetition after every UI edit. Strict native all-target Clippy
passed (1.93s), final build passed (2.84s), nightly format/diff passed. Dependency
no-experimental warnings are unchanged. All final six compiled captures match the
sizes above; final incoming images were visually checked (source controls hidden
on Incoming, singular file wording, no dominant drop zone). They are explicit
synthetic previews with disabled service controls, not KDE placement/privacy proof.
All owned display/broker/command sessions exited. Terminal closed.

One new method by primary purpose: N5 presentation priority, with N2 phase/count
and mode parsing tables. Existing widget/parser/placement methods were extended
for N1/N7/N9/N10 and geometry. No timing, billing or storage introduced (N8/N11/N12).
C2 safety/policy implementation is the next unfinished step; its contract above
was frozen before code. Persistent native settings remain a later migration gate.

### C2 implementation review before final checks

Two owned read-only provider monitors and the pure bounded policy are implemented.
The first guard compilation exposed a generated typed-signal wrapper; use its
actual message for sender validation. Four focused priority/policy/provider methods
then passed (0.03s). After wiring the GTK presenter and extending privacy/round
assertions, all 13 native methods passed once (0.19s), including the existing tray
checks using their extracted shared private-broker fixture. No user bus was touched.

Manual CLI/tray presentation is recorded as an intent while initial lock state is
unknown/locked; geometry is validated but no sensitive surface is mapped. A later
verified unlock presents that explicit intent, without polling, rather than treating
startup observation latency as a fatal GUI failure. DND still suppresses automatic
cards, not explicit manual review. An open settings surface also prevents automatic
navigation. Unknown lock hides both card and settings; queued UI actions cannot
admit transfer commands while the lock observation is unsafe. File dialog source
acquisition retains its already explicit authority and never sends automatically.

Gate release and completion-waiter release now reevaluate presentation as well as
rendering. Otherwise a notice suppressed during local preparation could remain
hidden after the last busy flag cleared, until an unrelated state event arrived.
Closing/dismissing remains separate from daemon consent/cancellation. Presentation
records are transient at C2; background GTK activation and cross-renderer notice
ownership still require the C3 contract. GUI Quit does not yet cause a new renderer
to start for a background request. This is an unfinished source milestone, not
installed-product notification completion or physical lock-screen proof.

### C2 completed / final observations and continuation boundary

Final affected native widget/provider/tray fixture checks passed 5/5 (0.17s)
after busy-release reevaluation and owned broker teardown hardening. The provider
method was strengthened with a real late-reply race: old unique owner remains
alive, releases its names, new owner reports locked/DND-on, then the old unlocked
reply returns. The actual read helper rejects that reply as Unknown; the affected
method passed 1/1 (0.03s). This tests owner pinning rather than merely an old
connection closing. No sleeps or weakened assertions.

Source review also removed needless unmap/re-present for an automatic update on
the same monitor. Actual monitor changes still reconfigure the surface; automatic
progress updates do not request focus again. Unsafe safety state still unmaps.
This is source-checked and compiled, not physical keyboard/focus acceptance.
The status now follows the current transfer/pending core control instead of
showing the unrelated source-selection instruction on an incoming card. Pending
Accept/Reject/Cancel is described as in progress, not completed reception.

The last affected widget passed 1/1 (0.16s); final strict native all-target Clippy
passed (1.62s), build passed (2.48s), format/diff checks passed. Existing nine core
no-experimental dependency warnings remain unchanged. Production code did not
change core/daemon APIs, crypto or transport, so unchanged core suites were not
repeated. Previous compiled preview captures remain valid: C2 observes real desktop
state only in normal mode; explicit inert preview bypasses it. No new screenshots
or full-suite/stability loop was needed for those IO/policy changes.

C1/C2 add four methods: N5 presentation priority/lifecycle 2, N3 actual provider
lifecycle 1, N7 absent/foreign provider 1 by primary purpose. Tables also cover
N1/N2/N4/N6/N9/N10 as inventoried. Native inventory 13; combined core/native 525,
not a full-525 execution or measured branch coverage. Privacy of physical lock
transitions, actual compositor focus/scale/overlap and Google peers remain open.
All owned sessions and fixture processes exited. Terminal closed.

Next significant choice is documented in native-notification-backend-review.md:
recommend A, a bounded daemon notice read model, coherent immutable reads and
exact presentation dismissal. Alternative B is a daemon-supervised renderer with
private acknowledged IPC. Both retain the selected custom Full/Compact card.
C3 changes the public local API/data model or process contract and a scoped GTK
backend lifecycle; await that decision before C3 production code. Current transient
presentation is useful while GTK runs, but does not yet provide closed-GUI delivery
or visible backend-unavailable Dolphin errors. Keyed GTK row reconciliation is
also needed before final expansion/focus acceptance; current rebuild behavior is
explicitly carried into the reviewed C3 GTK lifecycle work, not claimed fixed.
