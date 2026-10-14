# Native opt-in login-start and single GUI process ownership

2026-10-14. Mode A. User selected1: enabled login-start launches daemon and tray
client, card hidden. Guide /home/morozhenko/.codex/AGENTS.md. Direct dev, no install/
enable/live user manager/config/clipboard/peer, branch/push or sibling edits.

## Analysis and staged plan before code

Legacy default-off is established. Current CLI spawns a GUI child and drops its
handle after name acquisition; GUI activation directly execs that same binary.
Adding another login launcher would compete with those paths. Current daemon
Type=exec and LaunchGuard already survive joined settings-generation name changes.
Native settings JSON must not duplicate actual OS enablement state.

1. [ ] A1: fixed GUI systemd/D-Bus process ownership, typed shared activation and
   separate retained GUI guard. Preserve daemon process/typed ingress contracts.
2. [ ] A2: headless normal/CLI launch delegates to GUI service; explicit --service
   runs one foreground worker for installed manager/development. Event-driven tray
   readiness and process-lifetime signals/guard across reconnect; no spawned child.
3. [ ] A3: one bounded native login-settings owner for systemd reads/explicit edits,
   actual OS state/watch/readback and secondary Settings controls, no JSON/schema
   or daemon API changes. Keep active transfers and GUI lifetime unaffected.
4. [ ] A4: affected domain/private-broker/GTK/package checks once, strict lint/
   format/build, actual release .deb/extract smoke, SSOT/atomic commits/exact export.

Rollback UI/login adapter, then launch/GUI assets/core activation extensions. Keep
existing daemon activation/wire/session and file/text/image behavior. Significant
GUI launch ownership refactor is inside selected option1, not unrelated cleanup.

## Frozen process and launch contracts

rquickshare-desktop.service: Type=exec, ExecStart=/usr/bin/rquickshare --service,
Restart=no, PartOf=graphical-session.target, Wants=rquickshare-daemon.service,
After=graphical-session-pre.target dbus.socket rquickshare-daemon.service,
TimeoutStopSec=infinity, SendSIGKILL=no. Install WantedBy=graphical-session.target
is available only for explicit user enablement; no preset/enable/start on package
installation. Do NOT order After=graphical-session.target: the target's wanted-unit
ordering would create a cycle. Daemon remains independently owned after GUI Quit.

GUI Desktop D-Bus asset delegates to that unit via SystemdService and fail-closed
Exec=/usr/bin/false, matching the daemon's supported-manager boundary. Application
entry/plain command and Send/error ingress use fixed owned D-Bus activation, never
spawn/fork/shell. Plain explicit launch sends existing Desktop1.Show after readiness;
login worker/daemon wake send no Show and leave card hidden until existing safe
presentation policy permits it. This gives repeated manual launch a useful existing
card instead of starting a competing GUI. No receive/send/clipboard action replay.
A failed panel activation is not automatically retried merely to show its error.

--service is the explicit foreground GTK worker for manager/development; --preview
remains offline and --help returns before GTK/bus. A separate fixed GUI LaunchGuard
pins rquickshare-desktop-owner.lock in the validated private runtime directory
before initialization and across reconnect. Preserve daemon lock name/behavior;
share validation without accepting caller-controlled lock names. SIGINT/SIGTERM
cancel startup/tray wait and request GTK quit, then join actual owners before guard
release. GTK init requires an actual graphical environment; no guessed display or
silent ordinary-window fallback. Desktop session environment remains acceptance.

Subscribe to watcher name changes before lookup; if absent wait for actual matching
broker events, no sleeps/polling/retry timer. Resolve same-UID unique owner before
registration and pin it afterward as before. Startup cancellation closes the owned
connection. Broker activation success/name acquisition is followed by the existing
DesktopClient handshake; it is not an invented complete GTK/transport guarantee.

## Frozen login settings owner

One toolkit-independent native owner, queue8 and watch snapshot, owns a dedicated
bounded connection to the existing same-UID org.freedesktop.systemd1 unique owner.
This OS lifecycle concern stays separate from daemon transfer/settings ownership.
No second JSON flag, privileged shell/systemctl invocation, manager activation,
user-file copying or new daemon API. State reads are allowed without transport
availability; changes require current explicit unlocked Settings action.

Read only fixed rquickshare-desktop.service. GetUnitFileState enabled/disabled gives
persistent actual state; masked/runtime/linked/static/unknown/missing/custom units
remain unavailable/unknown, never coerced to false. LoadUnit inspects without start;
validate exact known unit object identity, vendor FragmentPath (/usr/lib or merged
/lib/systemd/user path), no DropInPaths and LoadState=loaded before edits. Do not
force/overwrite/mask/unmask custom units. Manager owner changes invalidate authority.
Subscribe to UnitFilesChanged before initial read and call manager Subscribe; close
connection releases subscription. Re-read on actual signals/explicit reload.

Explicit Apply carries expected observed boolean and desired boolean. Re-read before
mutation; stale value conflicts instead of blindly overwriting. EnableUnitFiles
([fixed unit],runtime=false,force=false) or DisableUnitFiles([fixed unit],false),
then read back actual state before acknowledgement. Never Start/Stop/Restart unit,
Reload manager or touch current transfers. A concurrent external change can still
race (manager has no CAS revision); report actual observation, not durable exclusion.
Failure/lost reply becomes unknown until explicit/owned readback; never auto-repeat
a write. Dropping a GUI completion does not abandon admitted work. Shutdown retains
owner, cancels broker wait/closes connection and joins; accepted OS changes cannot
be retracted, next launch reads actual state. No guessed UI success or boolean retry
ledger pretending the external manager has application receipts.

Secondary Settings shows Start at sign-in plus separate Apply/Reload and fixed
status. Unknown uses an inconsistent checkbox and disabled Apply. Initial-off
means unenabled vendor unit, not an application write at first run. Changes affect
future sign-ins, not live transfers/current GUI. Existing Save settings keeps its
own transactional semantics. One retained login completion waiter, lock/close guards,
no per-widget daemon controller or automatic enabling from rendering/opening settings.

## Verification inventory before tests

A1: typed daemon/desktop activation, real shipped asset with external fake manager,
current owner/no start, absent/unsupported/failure/cancel/UID; independent daemon/
GUI guards, duplicate guard, safe files and retained release. Existing core checks.
A2: launcher cold/warm/no child, exactly fixed unit, explicit worker/preview/help
argument separation; actual late tray watcher/cancel/owner pin, signal shutdown
ownership and unchanged GTK card-hide/readiness, no transfer replay on reconnect.
A3: real private fake-manager contract with actual login controller: enabled/off/
unknown/masked/custom/dropins, wrong UID/owner/FD/path, stale expected value, denied/
lost reply/readback, busy/close/dropped completion, UnitFilesChanged, no Start/Stop/
force/runtime mutations. Pure tables for finite state/errors; fake external OS only.
A4: extend existing single GTK smoke; real staged release package exact new unit/
activation/Install semantics, offline help/settings smoke, XML/desktop/YAML/format/
lint. No real user manager/login/install or ten-run Mode B gate. Physical KDE
sign-in/logout/readiness/lock/DND/scale/focus and upgrade/removal remain acceptance.

Applicable Positive,N1,N2,N3,N4,N5,N6,N7,N9,N10,N12; N8/N11 not applicable. Record
actual results/failures per stage and final source/version artifact provenance.

Official upstream references inspected2026-10-14:
https://raw.githubusercontent.com/systemd/systemd/main/man/org.freedesktop.systemd1.xml
https://raw.githubusercontent.com/systemd/systemd/main/man/systemd.special.xml
Manager documents fixed unit-file read/enable/disable, no-force/runtime parameters,
changes lists and subscription lifecycle. These operations differ from Start/Stop.
Graphical session target stops PartOf services; independently enabled wants links
select login services, pre-target concerns environment setup. Installed behavior
is not proven by reading specifications or a fake external manager.
