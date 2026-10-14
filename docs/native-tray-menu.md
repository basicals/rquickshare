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
2. [x] T2 bind GTK action/readiness projection, bridge ordered events to existing
   actions, and direct ContextMenu fallback to the card's existing popover. Preserve
   primary activation, hide/Quit ownership and lock safety; strengthen GTK smoke.
3. [x] T3 run affected native checks/lint/build once, record observations, update
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
17 rows (18 nodes including root; image extension2026-10-14); recursion depth -1/0/positive valid values cannot amplify the flat tree.
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

T2 review fixes before code: production Layer Shell menu projection requires a
known unlocked safety value even before the first GTK safety callback. Stop the
ordered action bridge after accepted Quit/Reconnect rather than executing remaining
queued local UI actions. Existing inert GTK smoke now exercises actual placement;
guard Layer Shell calls by the panel's existing layer flag so Xvfb review never
calls compositor APIs on an ordinary preview window. Production geometry/safety
policy remains unchanged; this removes a concrete unguarded adapter call, not a
new platform fallback or physical placement claim.

T2 observations: existing checked GTK actions now consume one ordered bounded
mailbox; menu projection follows current readiness/mode/safety, and direct tray
ContextMenu opens the existing popover. All 20 native methods passed once (0.55s),
then only the affected GTK method was rechecked after the Layer Shell preview guard
(0.52s). Native strict Clippy passed (4.46s), native build passed (11.61s), nightly
format/diff passed. Compile review found an immutable Ref dropped before row use;
retain it through rendering and publish another immutable projection. A test-only
interface constant is cfg(test), with no warning suppression. Existing core feature
warnings and isolated Xvfb DRI3 messages remain. All commands/brokers/tasks exited.
Terminal closed. No actual Plasma menu/Wayland focus proof.

T3 final contract review: when atomically rejecting EventGroup, report every ID
whose event was not dispatched, including valid clicks in the rejected group.
Reporting only malformed IDs would falsely acknowledge the omitted valid prefix.
Keep bounded all-or-none admission and standard failed-event IDs; strengthen the
real wire assertion. Inject expected UID into the menu adapter to exercise actual
credential denial on a private broker, rather than relying only on a UID helper.
No new trust boundary, action or UI choice is introduced.

Safety invalidation must render current button/action readiness before publishing
the tray projection. Otherwise a progress update while locked could leave actions
disabled after unlock until another unrelated core event. Reuse the existing render
on safety updates and extend the one GTK smoke with an actual projected state across
lock/unlock; presentation-only fixtures never gain a service mutation handle.

T3 observations: the affected real menu method passed (0.07s) after receipt/UID
corrections, with a correctly wrapped variant-FD rejection, actual broker credential
denial and group/property/count bounds. The affected GTK smoke passed (0.39s) after
immediate safety readiness refresh; no Layer Shell-on-preview warnings remain.
Final native strict all-target no-deps Clippy passed (3.65s), build passed (4.66s),
nightly format/diff passed. Earlier full 20-method native result remains applicable
to unchanged methods. No redundant complete rerun/ten-run gate/core suite run.
Two new methods: primary N1=1,N7=1; secondary bounds/concurrency/privacy/interruption
coverage is explicit above. Existing core feature and Xvfb DRI3 warnings remain.
All owned broker/display/task/command processes exited. Terminal closed.

All T1-T3 source stages complete. No live menu registration, package installation,
user configuration access or push. The standard /Menu object is a static safe
read model with GTK-owned enabled/mode state. Primary activation still opens the
card; direct ContextMenu uses its existing popover, while Plasma may import the
DBusMenu natively. Physical Plasma renderer, right-click positioning, menu keyboard
focus, mixed scales and Wayland activation remain manual acceptance. Source checks
do not establish installed Google interoperability or native feature parity.

Next native gaps: clipboard/text/reveal, autostart/updates and package equivalence;
account/QR/full Google parity and actual peer matrices remain open. Latest settings
contract/verification remains native-settings.md, original Tauri remains current.
Rollback GUI menu binding first, then export; preserve daemon settings/notice/source
authority. Work stays on dev with retained upstream references and atomic commits.


2026-10-14 image extension: native-image.md records Send image ID17 and private
u32 enabled projection (Send text stays ID16; other IDs unchanged). The flat tree
now has17 children/18 nodes. Property/event/FD/queue/auth/readiness contracts stay
unchanged; the event group limit remains16. Actual menu broker and GTK projection
checks passed with the full25-method native suite (0.50s). This is no Plasma signoff.
