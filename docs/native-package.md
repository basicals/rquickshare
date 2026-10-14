# Native package and systemd user activation

2026-10-14. Mode A. User selected option1 in native-package-review.md: a systemd
user service owns the daemon process; D-Bus requests its launch; GTK remains a
client. Guide /home/morozhenko/.codex/AGENTS.md. Direct dev, no install/enable/live
user manager/peer/config/clipboard, branch, push or NativePair edits.

## Analysis and plan before code

Existing LaunchGuard spans process lifetime and joined settings generations. The
bus name is released/reacquired at idle settings restart; never supervise that
process with Type=dbus. Existing NativeClient passive connect and typed snapshot/
epoch handshake stay intact. NativePair supplies the direct Debian tooling model,
not a working activation contract. Current workflows package Tauri only.

1. [ ] P1: fixed process/activation assets and explicit owned activation adapter,
   preserving passive SDK connect, UID/owner checks, cancellation and settings restart.
2. [ ] P2: native GTK launch/reconnect opts into activation, package-owned application
   entry and direct Debian builder/verifier; preserve legacy release jobs.
3. [ ] P3: isolated broker/domain/package checks, strict affected lint/build/format,
   native manual artifact workflow, actual observations, exact archive and handoff.

Rollback GTK activation first, then adapter and service assets; package builder
is independently reversible. No saved-data/API schema change, updater, login-start
setting or Tauri retirement. Significant new behavior is limited to selected
installed daemon ownership and explicit application launch activation.

## Frozen ownership and activation

Unit rquickshare-daemon.service under /usr/lib/systemd/user, Type=exec,
ExecStart=/usr/libexec/rquickshare-daemon, Restart=no. No Install section or enable
operation: package installation alone does not start or enable background sharing.
PartOf=graphical-session.target ties its shutdown to the graphical session. Normal
SIGTERM enters existing joined cleanup. TimeoutStopSec=infinity and SendSIGKILL=no
avoid forcibly killing transfers/workers on a guessed deadline; an uninterruptible
external IO can delay shutdown. No system service, root, sudo or shell child owner.

D-Bus activation name io.github.morozhenko_apps.RQuickShare delegates exclusively to
that unit through SystemdService. Exec=/usr/bin/false is a deliberate fail-closed
sentinel required by the service description format; an unsupported broker must
not silently start a second directly managed daemon. The adapter requires the
broker's SystemdActivation feature before requesting missing-service activation.
Existing owner needs no activation and is checked for a unique name and same UID.
Missing owner: request the fixed name once, validate reply1/2, then resolve/check
actual owner. Other broker errors, unknown/malformed replies, wrong UID or missing
post-activation owner fail. Concurrent callers rely on the broker/manager's normal
coalescing, never start private processes. No retries/sleeps or watch-driven restart.

A separate bounded connection owns activation IO and closes on success, failure
or cancellation. No operation ledger/transfer replay; the caller then connects the
unchanged passive SDK and checks live owner/epoch/readiness. Cancellation cannot
retract an already accepted manager start, so it never stops an existing/new shared
daemon. Normal GUI launch and explicit Reconnect opt in; preview remains offline.
The current GUI startup is synchronous before GLib ownership; no detached bootstrap
worker is introduced. GTK Quit still joins local clients, leaving daemon work alone.

## Native package contract

One package owns /usr/bin/rquickshare and /usr/libexec/rquickshare-daemon plus fixed
application, GUI/daemon D-Bus, unit and Dolphin assets. Native stage replaces the
same-name rquickshare package when explicitly installed; never install two owners
of its executable. Legacy packaging/release jobs stay intact pending acceptance.
Use a native version suffix to distinguish these artifacts. No maintainer scripts
that start/restart/enable services, modify user stores or delete received data.

Builder uses locked Rust binaries and dpkg-deb in a fresh temporary stage, no host
install. Default release; an explicit dev profile produces an honestly labeled
.debug package for offline pipeline validation. Determine shared-library Depends
from actual ELF via dpkg-shlibdeps, fail on missing metadata; include systemd user
session/dbus/BlueZ/portal runtime requirements explicitly. A private extracted
library metadata input may supply actual Debian shlibs records for local builds;
never ignore unresolved dependencies or infer compatibility from ldd alone.
GTK4>=4.12, Layer Shell and real host ABI requirements must be declared; no Ubuntu
24 compatibility claim for a binary linked on26.04. Support native amd64/arm64 only,
no cross-architecture relabeling. Build and verify SHA256, exact allowed file layout,
root ownership/modes, assets/identities, native version/architecture and dependency
metadata. Extracted smoke only --help (add native GUI help without GTK/bus), daemon
--help, and explicit fixture --settings PATH --check. No reading user configuration.
A staged package is not installed upgrade/removal/startup acceptance.

## Scoped verification inventory before tests

- Activation domain table: running owner skips start; absent owner supported starts
  once; unsupported/start failure/missing final owner/malformed owner/wrong UID;
  cancellation before/during external admission and owned connection close.
- Real private broker + actual SelectionBus/NativeClient: current owner, passive
  missing owner, unsupported non-activating broker, explicit activation cleanup;
  no owned service termination after GUI/client close. Fake only external broker/
  manager contract where real systemd is unavailable, never core/session domain.
- Existing settings generations/process tests protect name loss/restart/LaunchGuard.
  Unit/static checks assert process-based supervision, fixed exec, no enablement/
  restart/forced kill and activation sentinel/delegation.
- Package validation: exact assets/modes/ownership, dependency/version/arch integrity,
  checksum/malformed metadata/tampered inventory rejection, offline executable help
  and explicit settings fixture. Use table-driven Python utility tests for pure
  validation; one actual staging/build/extract smoke. No ten-run Mode B gate.

Applicable Positive,N1,N2,N3,N4,N5,N6,N7,N9,N10,N12; N8/N11 not applicable (no time or
billing policy). No coverage/stability/full interoperability claim. Record exact
checks and failures here after each milestone. Login-autostart/updater behavior and
installed KDE lifecycle/real Google checks remain separate decisions/acceptance.
