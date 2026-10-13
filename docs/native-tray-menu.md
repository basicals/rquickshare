# Native tray menu

2026-10-13. Mode A. Guide /home/morozhenko/.codex/AGENTS.md. The user already
selected secondary right-click actions and now requested continuation. Direct dev
commits, no live tray install, push or separate agent. Settings slice is complete.

## Analysis and decision before code

Tray currently exports only StatusNotifierItem and treats ContextMenu as Activate.
GTK owns a checked SimpleActionGroup shared by its card/header menus. Watch channels
are correct for coalescing placement hints but would silently replace side-effectful
menu choices. Use a standard com.canonical.dbusmenu object on the same owned tray
connection, a fixed privacy-free menu tree and one bounded action-batch mailbox.
Reuse GTK actions and final safety/readiness checks; never add daemon control logic
to the tray. This implements the previously accepted UX, not a new transfer API.

Primary protocol references checked before code:
- https://specifications.freedesktop.org/status-notifier-item/latest/status-notifier-item.html
- https://raw.githubusercontent.com/a17r/libdbusmenuqt/master/src/com.canonical.dbusmenu.xml
- https://raw.githubusercontent.com/KDE/plasma-workspace/master/applets/systemtray/statusnotifieritemsource.cpp

## Stages / rollback

1. [x] T1 export fixed DBusMenu layout/properties/event methods and owned bounded
   command queue; StatusNotifierItem.Menu returns /Menu. Freeze/read/event validation,
   dynamic enabled/mode snapshot and revision signalling. Private-broker checks.
2. [ ] T2 bind GTK action/readiness projection, bridge ordered events to existing
   actions, and direct ContextMenu fallback to the card's existing popover. Preserve
   primary activation, hide/Quit ownership and lock safety; strengthen GTK smoke.
3. [ ] T3 run affected native checks/lint/build once, record observations, update
   checkpoints/export exact committed source. Installed Plasma menu/focus is a
   separate physical gate. Roll back GTK binding, then tray export; prior card
   menu and daemon settings remain independently usable.

## Frozen scoped contracts

Fixed flat tree: Open card, Send files, Send folder, Incoming, Transfers, Compact,
Full, Settings, Reconnect service, Hide card, Quit panel; fixed separators and root
IDs. No file/device names, status diagnostics, saved folder or consent buttons.
Full/Compact reflects the current temporary preview; persistence remains explicit
Settings Save. Quit closes only the graphical client. No stop/restart/save/autosend
or receive-accept menu command. ItemIsMenu stays false; primary activation opens card.

Menu reads/events authenticate actual same-UID unique caller, reject any FDs and
reuse existing 64KiB/16-FD predecode framing. Static root/leaf layouts have at most
15 rows; recursion depth -1/0/positive valid values cannot amplify the flat tree.
Property-name requests <=16 with each name <=64 bytes; IDs <=32. Unknown IDs or
unsupported event names fail explicitly. clicked dispatches; opened/closed/hovered
are presentation-only no-ops. Event data is an ignored non-FD variant; timestamps
are not authority. Grouped events <=16 are validated together before admission,
preserve order and report invalid IDs without partially dispatching valid prefixes.
Action mailbox holds at most eight batches; saturation rejects explicitly without
coalescing actions. No timeout, detached worker, unbounded per-click task or source
replay. Closing tray closes admission and its bus connection; GUI drains/drops only
its own queue during the joined graphical lifecycle.

GUI publishes one in-memory menu projection (enabled bitset plus current mode) via
watch, without filesystem/IPC work in callbacks. Initial actions are disabled until
GTK publishes readiness. Revision increments only for an effective change; owned
tray task emits LayoutUpdated(revision,0). Checked overflow fails the tray, never
wraps IDs. No state lock is held across broker or signal await. Revalidate enabled
actions at bus admission and GTK execution, including lock/disconnected/preparation
states. Hide/Quit remain available for local cleanup; locked/unknown desktop must
not open sensitive panels/settings/choosers. Old queued actions cannot become
authority after a safety change. ContextMenu fallback carries a presentation kind
with coordinates, not a side-effectful action masquerading as activation.

## Scoped verification inventory before tests

| Artifact | Branches/interactions | Check | Categories |
|---|---|---|---|
| Fixed menu projection | root/leaf/depth/property filtering/unknown ID; mode checks/enabled bits; no private fields | unit table + real wire signature/property checks | Positive,N1,N2,N10 |
| Real menu IPC | authenticated caller, FD rejection, disabled/unknown click, ignored event, valid/rejected group, eight-batch saturation/order | isolated private broker integration 1-2 methods | N1,N2,N4,N5,N7,N9 |
| Dynamic ownership | changed/unchanged projection/revision signal, watcher loss, connection close, no queue replay | extend existing tray fixture | Positive,N3,N5,N9 |
| GTK adapter | same actions, readiness, lock/unlock, temporary mode, ContextMenu fallback, panel close | extend existing single GTK smoke | Positive,N5,N7,N9 |

Assert exact rows/actions/order and unchanged state for rejections; kill missing
auth/readiness, dropped/coalesced events, mode mismatch and consent-bearing menu.
No domain mocks/sleeps, no competing GTK test thread. Storage/migrations/time/billing
unchanged (N6/N8/N11/N12 not applicable); settings verification remains historical.

T1 observations: fixed projection/export and bounded mailbox implemented. Four
affected tray methods passed (0.13s), including two new methods (primary N1 and N7)
and strengthened real watcher/revision/context checks. Initial signal visibility
compile error was corrected by exposing the owned signal emitter to its parent.
GUI consumer fields/methods are intentionally not yet wired and generate temporary
unused warnings until T2; no suppression added. Existing core no-default feature
warnings remain. All private brokers/commands exited. Terminal closed.
