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

1. [x] P1: fixed process/activation assets and explicit owned activation adapter,
   preserving passive SDK connect, UID/owner checks, cancellation and settings restart.
2. [x] P2: native GTK launch/reconnect opts into activation, package-owned application
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

P1 implemented: bounded separate activation connection, verified broker replies,
explicit SystemdActivation requirement for missing owners, fixed once-only start,
same-UID stable unique owner and unconditional connection close. Passive NativeClient
is unchanged; GTK wiring belongs to P2. Unit supervises process, never enables or
restarts it, and preserves joined shutdown. First compile found the existing test
watchdog helper was module-private; widened only its test-only visibility for reuse.
Four activation methods then passed0.04s/build30.34s. Strict lint exposed the
async_trait macro's generated duplicate must_use annotation on the new seam;
replaced this private statically dispatched seam with native impl-Future trait
methods, avoiding boxing/macro rather than suppressing lint. Final affected service
262/0 passed8.38s/build28.24s; strict core lint11.62s. Inventory575 core. Existing
restart/process and actual private-broker ownership tests are included in the
service filter. Real installed systemd delegation/coalescing remains acceptance;
fake external admission tests do not prove a real manager. Terminal closed.

P2 implementation state before actual package smoke: explicit native GTK startup/
Reconnect uses the new adapter, then the unchanged passive SDK. Preview remains
offline. Native --help returns before GTK/display/runtime/bus; extracted smoke can
therefore identify the real GUI without showing a window. Failure notice now refers
to installation/Reconnect rather than requiring manual daemon startup.

Direct Debian builder/verifier and source application entry are implemented. Four
Python methods use real tiny dpkg-deb archives and table-driven metadata/checksum/
layout/mode/link/asset/control-script abuse. Initial fixture runs found directory
modes inherited the host umask; builder now normalizes its own stage directories
to0755 and control/file modes explicitly, preserving strict verification. Fixture
postinst is executable so the verifier, rather than dpkg's earlier syntax check,
rejects it. A subsequent valid fixture was rejected because its local loop shadowed
the output directory variable and placed the artifact inside its own payload;
corrected only that fixture variable, retaining exact inventory assertions. Final
4/0 passed0.627s. Strict native all-target no-deps lint passed7.76s; the known nine
core no-default-feature warnings remain separate. Release build of both binaries
and actual stage/extract/help/settings smoke are in progress, not yet a pass.

The manual native-package-smoke workflow builds inside official ubuntu:26.04,
independently of its Ubuntu24.04 Actions host, with native prerequisites and one
core/native/package quality gate before release artifact upload. It never installs
the resulting package or enables a service. Source only: no remote run/push claim.
Official Docker Ubuntu listing confirms26.04 tag; official Ubuntu package listing
confirms libgtk4-layer-shell-dev1.3.0-1 for resolute. Inspected2026-10-14:
https://hub.docker.com/_/ubuntu
https://packages.ubuntu.com/resolute/libgtk4-layer-shell-dev
The separate GTK development web fetch failed; installed host and private Debian
metadata provide actual local GTK4.22.4 evidence, not an Ubuntu24.04 ABI claim.


P2 actual release artifact: daemon release build2m46s and native GUI release1m49s
passed; the actual .deb/checksum/extract/GUI --help/daemon --help/private settings
--check all passed without install or bus/transport activation. ELF library metadata
came from installed host records plus the actual extracted Layer Shell .deb shlibs;
no unresolved dependency was ignored. The host dpkg reported merged-/usr libc
loader-diversion notices and a stage-layout warning. Before final P3 artifact check,
move the stage under the conventional debian/rquickshare directory so dpkg-shlibdeps
recognizes package ownership. Preserve the independent host diversion notice.

P3 additional actual cold broker check passed1/0 in0.05s/build24.20s: real isolated
systemd-activation-capable dbus-daemon reads the shipped activation asset, requests
exact rquickshare-daemon.service, then a fake external manager admits the actual
SelectionBus. The actual NativeClient handshake/state and independent close pass.
This verifies real broker wire/delegation, not installed systemd process management.
Upstream systemd src/core/dbus.c signal_activation_request inspected2026-10-14:
https://raw.githubusercontent.com/systemd/systemd/main/src/core/dbus.c
It loads the requested unit and queues a start job; that adapter does not require
Type=dbus. Inference: Type=exec can preserve the current name/epoch lifecycle while
the broker still waits for the name. Installed unit environment remains acceptance.

Final verifier also reads actual ELF class/endian/executable type/machine before
smoke, rejecting relabelled binaries regardless of filenames. Final Python5/0
passed0.965s; new cold-fixture strict core lint6.30s. Native application desktop-file
validation, both nightly format checks, whitespace and manual workflow YAML/static
trigger/container checks passed. Remote workflow was not run. Source archives can
build without .git: SOURCE_DATE_EPOCH is respected, otherwise defaults to the source
manifest mtime; no unrelated parent Git repository is queried. This affects only
artifact timestamp normalization, not runtime behavior.


P3 staging follow-up: the conventional directory alone did not remove the dpkg
stage warning. Read the installed primary Dpkg::Path implementation: package-root
recognition requires an existing DEBIAN subdirectory. Create that owned directory
before dependency analysis, then write final control metadata afterward. This fixes
the actual root cause; do not suppress the warning or assume absolute paths are
wrong. Repeat only affected staging/dependency/archive smoke; binaries stay cached.

P3 build invalidation repair plan before code: the staging-only rerun still entered
expensive core release compilation. Inspected core_lib/build.rs: no explicit rerun
inputs, and every run truncates/rewrites tracked bindings/index.ts inside its own
default watched package. This self-invalidates the next Cargo invocation and reruns
prost generation unnecessarily. Stopped only this owned build (exit130), no test or
user process. Under the cross-project maintenance authorization, add explicit proto
and bindings input watches and compiler environment watches; write the sorted index
only when bytes differ. Preserve generated exports/protocol output and current
contracts. No runtime change or significant refactor. Rollback build.rs independently.
The actual final build is required after this change; then one immediate same-command
Cargo build verifies cache freshness/index mtime rather than repeating business
suites. Record observed Fresh/build time and exact payload/checksum before export.
