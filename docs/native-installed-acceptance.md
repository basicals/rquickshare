# Installed native acceptance: host readiness and rollback

2026-10-16. Mode A, direct dev. Guide: /home/morozhenko/.codex/AGENTS.md,
sections3/3.1/11/12/13. Continue the essential current-peer/installed-desktop gate
after native12 source/offline verification. No preview or advanced parity work.

## I1 historical pre-install host state

Read-only inspection found Ubuntu26.04.1 amd64, KDE/Wayland and an installed
rquickshare0.11.5 legacy package. Native daemon/desktop units and native libexec
binaries are absent. No rQuickShare process was observed at this checkpoint.
Bluetooth is active; the real user broker advertises SystemdActivation. The ADB
executable exists but no existing ADB server listener was observed; do not start
one or infer that an Android device is available. User/device availability was
asked separately and is not assumed.

One actual apt-get --simulate install of native12 passed dependency resolution:
upgrade rquickshare0.11.5 to0.11.5+native12, install libgtk4-layer-shell0 1.3.0-1,
remove0 packages. Other45 upgradeable packages are not part of this operation.
The simulation does not establish actual install/service/desktop/peer success.
No apt update/install, user-manager reload, start/enable/stop, radio mutation or
account access was performed.

The local legacy debug .deb has matching package version but differs from the
installed /usr/bin/rquickshare. Its other four regular payload files match. Do
not call it an exact rollback image. The installed package has five regular files,
no conffiles and only the md5sums control file, with no maintainer scripts. The
installed executable is404,079,544 bytes, SHA256
a5d7ff1aba60f88ba31b35305741dd35c4c317786b1256a5d8b3880ef758f933.

## Plan before recovery artifact creation

1. [x] I1: inspect host/installed assets/activation support and simulate native12
   dependencies read-only. Identify actual peer availability separately.
2. [x] I2: prepare a private local recovery .deb from exactly the five installed
   regular payload files, preserving installed modes and package/dependency
   metadata, with root-owned archive entries and newly calculated checksums.
   Verify exact archive inventory/bytes/modes and compare live files again.
   This is a reconstruction of installed state, not the original release/build
   provenance or permission to execute that application. No user data/config copy.
3. [x] I3: record concrete upgrade/rollback commands and artifact hashes for
   review. Installation replaces the user's desktop application and requires
   administrator authentication; obtain explicit approval for that exact step
   before changing the installed system. Recheck live transfer/process ownership
   immediately before any approved change; never kill a user/ambient process.
4. [ ] I4: after approved install, check package inventory and dependency state,
   then actual activation/card/tray/idle shutdown with controlled own work. Peer
   acceptance requires an available user-operated Android/Windows receiver and
   explicit code/consent actions. Record unavailable parts as untested.

Recovery artifact lives outside Git under the local Codex workspace, contains
only already installed application files/metadata, and is not published. Keep
native12 and old immutable artifacts intact. Failed reconstruction leaves the
installed package untouched and blocks calling rollback ready. Dependency
simulation is not repeated until package/host changes or an actual install step
requires current state. No unrelated suites/builds or ten repetitions are needed.

Risks: the old debug .deb is not the installed binary; uncoordinated replacement
could strand a running old owner; administrator authentication may require the
user at the PC. Address those concrete gates, not broad new trust/feature work.
Rollback of this preparatory work is docs/private artifact removal only; no system
change has occurred. All owned inspection commands exited: Terminal closed.

## I2 actual recovery result

A private installed-state recovery package was built and verified successfully:
/home/morozhenko/Documents/Codex/rquickshare-acceptance/2026-10-16/rollback/
rquickshare_0.11.5_installed-snapshot_amd64.deb

Size80,994,242 bytes, SHA256
a9dc9185d4c8208fd3511c691c4d1445d9d37bae6f676a72fa601b5f7d6b1eb3.
All five regular file bytes/modes and root archive ownership match the inspected
installed application; live installed files were rechecked unchanged after package
creation. Control metadata preserves installed version/architecture/dependencies;
checksums are recalculated. No conffiles, user settings/data or maintainer scripts
were included. Private directory0700 and package/receipt0600; not published or
included in the Git source archive. This is an installed-file reconstruction,
not original release provenance. It has not been installed as a rollback test.

## I3 reviewed installation and rollback commands

Native candidate:
/home/morozhenko/Documents/GitHub/rquickshare/native_desktop/target/packages/
rquickshare_0.11.5+native12_amd64.deb
SHA256 b4ef947385b46a2b16f2dc625f1cfc216205672b1df75478f821bb4a857962f0.

After explicit approval and an immediate active-owner recheck, use the system
package manager to install this exact local package and its declared dependency.
No unrelated upgrade, APT-source/key change or publication. Authentication stays
in the system prompt, never in chat. The concrete command is:

```sh
pkexec /usr/bin/apt-get install --yes /home/morozhenko/Documents/GitHub/rquickshare/native_desktop/target/packages/rquickshare_0.11.5+native12_amd64.deb
```

Then reload the user's unit metadata without enabling login start, validate the
installed files/package and test explicitly owned native activation/GTK lifecycle.
Do not start a second owner or interrupt someone else's transfer. Actual radio
startup/peer sends are separate observable acceptance steps, not offline fixtures.
If administrator authentication is unavailable, retain the prepared state and
continue independent source inspection; never bypass it or handle the password.

If an approved test must roll back, first join only test-owned idle native clients/
services and use apt-get --allow-downgrades on the exact private recovery package.
Keep user settings/received data. The newly installed Layer Shell dependency may
remain; do not remove shared dependencies automatically. Rollback execution itself
is not yet performed or claimed successful.

At the I1-I2 checkpoint, installed state remained legacy0.11.5. The user's physical-device availability
question is still unanswered; generic continuation does not establish an available
peer or completed Android/Windows/KDE acceptance. No installation or activation has
occurred in I1-I2. All owned preparation commands exited: Terminal closed.

## I3 installation authorized — 2026-10-16

The user explicitly approved replacing the old installed application with native12.
Immediately before invocation, candidate SHA256/private rollback SHA256 matched
and no rQuickShare process was present; installed version remained0.11.5. Started
the exact documented pkexec/apt-get local-package command. Administrator password
is entered only in the system authentication window; no password is read/logged
by the agent. Installed/activation/peer success remains pending command completion.

## I3 installed result and I4 scoped execution plan

The approved apt operation exited0: rquickshare0.11.5 was upgraded to
0.11.5+native12 and libgtk4-layer-shell0 1.3.0-1 was installed. No package was
removed and the45 unrelated upgradeable packages were left untouched. All11
installed regular payload files match the candidate bytes, modes and root
ownership. Both package states are install-ok; installed CLI help and the daemon
settings-only check passed. The latter explicitly started no service. User unit
metadata was reloaded without enabling login start.

Normal installed launcher activation then succeeded: systemd owns the daemon
from /usr/libexec/rquickshare-daemon and GTK from /usr/bin/rquickshare --service.
Both units are running; desktop login start remains disabled. A private no-auto-
start D-Bus observer found a synchronized running daemon, zero transfers, sources,
handles and reservations, available/running visibility, and exactly one active
rquickshare item registered with the real KDE StatusNotifierWatcher. No peer
identifier or personal file metadata was recorded. These are IPC observations,
not visual confirmation of card placement or physical-peer acceptance.

Next scoped checks, before any further runtime action:
1. Capture current service PIDs and invoke the normal launcher once. Require the
   same owners/PIDs and exactly one own tray item; read the own menu projection.
2. On an owned private D-Bus connection, Begin/Append one disposable read-only file
   descriptor/Abort. Do not Commit or Send; require an aborted result and unchanged
   daemon resources. This checks installed staging cleanup only, not a complete
   file-manager send. Remove the owned fixture and close the observer.
3. Record observed results and leave visual/current-peer checks pending until the
   user confirms available devices. Do not toggle visibility/login start, request
   device authentication or terminate ambient processes. The installed application
   is the user's explicitly requested background app, not a test watcher.

No architecture/API/UX change or new build/test suite is needed for these checks.
Failure stops the affected step and preserves evidence; the private downgrade
package remains available, but no rollback is executed. All installation and
inspection terminal commands exited: Terminal closed.

## I4 actual installed observations — 2026-10-16

Normal repeated launcher exited0 and retained daemon PID1901044 and desktop
PID1901045, with exactly one own active tray item. The real own DBusMenu contains
21 rows with ordinary source/navigation/appearance actions enabled; visibility
is Visible, Full is selected, reconnect/retry are correctly disabled in the
healthy state. No menu events or persistent setting mutations were performed.
The user confirmed the tray icon and card on click and an available Android peer.
This does not establish precise placement/focus, Full/Compact, peer version,
verification code or transfer acceptance.

An owned read-only file descriptor from the repository target disk filesystem
was appended and aborted through the installed Desktop1 endpoint. Ack1/aborted;
no Commit or Send; final daemon source/handle/byte/transfer resources were all0.
The private observer and disposable fixture were closed/removed. The first
fixture under /tmp was rejected with InvalidInput and also aborted/removed.
A scoped kernel probe established that /tmp ordinary named files support
GET_SEALS with SEAL only, whereas this disk filesystem returns EINVAL. Source
inspection traced rejection to the deliberate broad seal-capable-memory contract
in native-image.md. See [tmpfs source review](native-tmpfs-source-review.md):
changing this accounting distinction requires a concrete user choice, not a
silent weakening of anonymous-source invariants. No production code changed.

I4 remains partly complete: installed activation, owner reuse, tray/menu IPC,
user-confirmed card existence and disk-descriptor abort passed. Android actual
transfers, visual placement/Full/Compact/lock/DND, committed file-manager flow,
login start, idle shutdown/restart and removal/recovery remain unexecuted.
No Google interoperability claim follows from nearby endpoint counts. No radio
pairing, peer Send, ADB server, clipboard change or release signing/publication
was performed. Installed app services remain running as the approved application;
no test server/watcher/terminal remains. Terminal closed.

Documentation verification:54 affected local links resolve; evidence JSON parses
and whitespace check passed. No production code/build or unrelated test suite was
changed/repeated for this installed evidence checkpoint.

## Subsequent native13 repair — 2026-10-16

User-selected tmpfs source distinction is implemented, verified and installed as
native13. Exact package/source/upgrade/installed-regression evidence is in
[native-tmpfs-source-review.md](native-tmpfs-source-review.md) and
evidence/native-tmpfs-source-2026-10-16.json. Native12 observations above are
historical and remain intact. Current installed version is0.11.5+native13, one
daemon/client/tray running with login start disabled. The real installed CLI has
prepared a known123-byte Android fixture with no auto-Send; retain that source
while the user performs recipient/code/acceptance actions. I4 is still partially
complete: actual current-peer content and remaining desktop/lifecycle checks open.

## Subsequent native15 card cleanup

User-requested eight-point card/tray cleanup is installed as native15. Exact
14-file package/install verification, one tray/client/daemon, actual icon/About
projection and renderer corner/geometry results are in [native-card-cleanup.md](native-card-cleanup.md).
Native14 was reviewed but not installed; native13 remains a rollback artifact.
Native13's user-confirmed Android outgoing smoke is retained, with remote hash/
exact version/reverse transfer still unverified. Physical KDE cosmetic feedback
and the rest of I4 remain distinct pending checks, not blanket acceptance.
