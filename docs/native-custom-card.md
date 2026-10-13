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

1. [ ] C1: implement one reusable full/compact GTK surface. Compact is the initial
   primary mode; full retains all existing Nearby/Incoming/Transfers functions.
   Add an accessible header menu, right-click menu and secondary Appearance settings.
   Preserve existing source, consent and recovery contracts. Build/review real
   compiled previews, supporting focused widget projection checks, then commit.
   Rollback: native presentation changes only; daemon/IPC stays compatible.
2. [ ] C2: implement a bounded presentation policy and actual desktop safety adapter.
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
