# Native package and daemon activation review

2026-10-14. Mode A analysis, after completed native-reveal.md. Direct dev only.
Guide /home/morozhenko/.codex/AGENTS.md sections3/3.1 requires a user choice for the
new installed process owner/activation boundary. No package production change,
activation asset installation, live manager call, autostart or transport start here.

## Inspected current state

- native_desktop/packaging has only fixed GUI D-Bus activation and Dolphin menu
  assets. The native GUI occupies /usr/bin/rquickshare, conflicting with the legacy
  packaged binary. No native daemon unit/activation/package builder exists.
- All existing manual package workflows still build Tauri. GTK4>=4.12 and Layer
  Shell runtime/development dependencies need native package validation; extracted
  local build libraries do not prove target distro dependency availability.
- NativeClient::connect gets an existing owner; it does not activate a missing
  daemon. Launching the GUI currently needs a separately manually started daemon.
- LaunchGuard owns the process across joined settings generations. The daemon
  serve_native_settings loop deliberately releases/reacquires its bus name for
  explicit idle restart after settings changes. GUI Quit leaves transfers owned by
  this separate process. No GUI-child daemon or competing transport may be added.
- NativePair docs/BUILD_AND_PACKAGING.md and README.md were read in the sibling
  checkout. It builds/verifies a Debian package directly, placing its daemon in
  /usr/libexec, with no Node/WebKit/Tauri packaging dependency. Its documentation
  explicitly defers installed systemd service/activation. It is an architectural
  and packaging example, not evidence that its service lifecycle is implemented.
  No files in that repository were changed.

## Choice awaiting user selection

1. Recommended: systemd user service owns the daemon process; D-Bus activation
   requests that fixed service. GTK/CLI remain clients. Use process lifetime rather
   than bus-name lifetime for supervision (explicit Type=exec candidate); current
   SDK generation/handshake remains readiness authority. Package fixed native GUI,
   daemon, service/activation/menu/application assets together. Benefits: explicit
   start/stop/status ownership independent of GUI, one manager, diagnosable failures
   and a clear future user-controlled login-start policy. Costs: systemd user
   manager/session integration becomes a supported-platform dependency. Risks:
   incorrect unit shutdown/session/environment or name/epoch handling can interrupt
   work; verify in isolated fixtures then installed KDE acceptance. No silent second
   direct-exec strategy when a supported manager fails.
2. Direct session D-Bus activation owns daemon startup, without a systemd unit.
   Package the fixed daemon Exec asset and use the same typed SDK afterward.
   Benefits: fewer manager assumptions, independent GUI and process still retained
   by its own existing ownership. Costs: no managed service status/stop boundary;
   login-start, failure recovery and package removal must use a separate explicit
   lifecycle design. Risks: later ad-hoc process-control mechanisms or duplicated
   startup authority unless carefully kept separate. No GUI child/fork/shell wrapper.

Prefer1 for a KDE/Debian/Ubuntu target and durable ownership/diagnostics, regardless
of implementation volume. Both preserve daemon-owned transfers after GUI Quit and
same-UID authentication. Neither installs/enables anything during development or
silently replays Send/Accept/clipboard actions. Login autostart policy and updater
ownership are not silently settled by this choice; freeze them before their code.

## Systemd lifetime trap and primary references

Official upstream documentation inspected2026-10-14:
- https://raw.githubusercontent.com/systemd/systemd/main/man/systemd.service.xml
- https://raw.githubusercontent.com/d-bus/dbus/master/doc/dbus-specification.xml

The systemd source documents Type=dbus as active while BusName is held, terminating
remaining service processes on name loss. Inference from that rule plus current
serve_native_settings: copying a conventional Type=dbus unit would terminate the
intended in-process settings restart. Preserve process supervision explicitly,
rather than changing existing epoch/restart semantics merely to fit a unit. Type=exec
confirms executable startup, not application readiness; keep those claims distinct.
D-Bus documents SystemdService delegation to the named unit when the broker supports
systemd activation. Verify actual target support; do not assume a private ordinary
broker exercised a real user manager. freedesktop rendered pages timed out; only
these upstream source documents were successfully fetched.

## Proposed stages after selection, before implementation

1. Freeze selected process owner, fixed identities/paths, activation admission,
   environment/session and shutdown invariants, retained epoch/restart behavior,
   native package collision/rollback and supported distro dependencies. No implicit
   visibility/trust default change, service enablement or old settings deletion.
2. Add package-owned daemon activation/unit assets and a cancellable owned explicit
   activation adapter. Preserve passive unavailable state where activation is not
   requested. Authentication/unique owner/epoch checks still follow broker startup;
   no detached process, shell, retry loop or automatic replay. Plan test seams first.
3. Build native .deb into a staging root using locked binaries and Debian tooling;
   verify metadata, dependency resolution, file layout/modes, checksum and packaged
   read-only help/check behavior. No host install. Preserve legacy release workflow
   until native package gates justify changing it. Roll back builder/assets before
   removing supporting SDK behavior.
4. Add/update scoped native CI/manual artifact gate, documentation and exact export.
   A staged package is not an installed lifecycle or Google compatibility pass.
5. Separately review login-start/user controls/updater policy, then implement only
   confirmed behavior. Installed upgrade/removal, KDE session start/logout, old-owner
   collision, actual Dolphin actions and real peer checks remain acceptance.

Verification inventory: missing/current/foreign/stale owner, failed activation,
coalesced simultaneous startup, cancellation/join, early bus name before service
ready, name loss across settings restart, existing LaunchGuard conflict, GUI Quit
independence, package paths/modes/dependencies/removal ownership. Use private brokers
and explicit fake external manager boundaries, actual SDK/process domain logic.
No magic sleep or ten development repetitions. Record actual measured results and
failures in the next implementation SSOT. No new tests executed for this analysis.

## Handoff

Received-file reveal is complete at2ed984c; its exact checks remain native-reveal.md.
This review is awaiting choice1/2. Do not implement installed activation ownership
until selected. No live user service, package installation, peer/config/clipboard
operation or NativePair modification occurred. All terminal commands exited.
Terminal closed.
