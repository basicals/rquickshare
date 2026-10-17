# Native22 local installation candidate

2026-10-17, Mode A. The user explicitly requested installing the merged card
changes for hands-on testing. This authorizes one local upgrade, joined restart
of the known idle application services and normal launch. No signing, public
release, APT publication, cloud access or login-start change.

## Analysis and plan before packaging

Source checkpoint b968378 contains the merged card/endpoint association work.
Installed version is 0.11.5+native21, daemon PID2201915 and desktop PID2201916;
login start is disabled. Preserve native21 package/source archives for rollback.
The substantive source passed Preflight and the isolated GTK executable suite;
do not repeat unaffected whole suites. The prior full-library preview sandbox
failure occurred in a container; run the exact real-helper scenario on this host
once, with the existing isolated broker. Never disable sandboxing or assertions.

- [x] I1: inspect packaging, prior installed-upgrade procedure and known owners.
- [x] I2: advance only the native package revision to22, commit provenance and
  build the locked release daemon, GTK and private helper. Verify package exact
  inventory, dependency metadata, ELF, checksums and extracted offline commands.
- [x] I3: run the exact real sandbox fixture once on this host; record the actual
  result separately from GUI and interoperability acceptance.
- [x] I4: fresh same-owner/global-idle preflight: no source reservations, handles,
  prepared bytes/selections or nonterminal transfers. Never discard user work.
  Join only the identified idle rQuickShare services; use the system package
  authorizer to install the exact package. User enters any password locally.
- [x] I5: normal launch; compare all14 installed regular files, versions, one
  real tray and unchanged login-start state. Record source/package/install
  evidence, export the candidate and leave physical acceptance to the user.

Risk and rollback: a service restart clears in-memory terminal history and
window placement. No source file or received content is removed. Non-idle work
blocks the restart. If installation fails, restore normal launch of the actual
installed version and record the failure; retain both packages. Idle rollback
uses retained native21 through the package manager, not file replacement.
New grouping uses ephemeral live endpoint association only, never trust/name
matching; consent and clipboard policy are unchanged. No full Google parity.


Initial build observation: the daemon built, but GTK compilation stopped because
ambient pkg-config has no GTK4 development metadata. The documented existing
private sysroot is /home/morozhenko/Documents/Codex/2026-10-12/new-chat/work/gtk-native/root.
Reuse its PKG_CONFIG_SYSROOT_DIR, pkg-config metadata and library search paths as
in prior native builds; no host package/config changes. This is an environment
recovery, not a source defect or passed build. Re-run only the affected build.


Build and host verification observed: native22 release package is 6502732 bytes,
SHA-256 a3fcdc0118782de16e50b276f2260b39cce9c8fd5d1cdd23704ea703f3803194. Binary/package source is
0f34808 (only later documentation differs). Offline builder verification passed
all14 files, dependencies, architecture, checksums and extracted commands.
The exact real Bubblewrap/Glycin PNG/JPEG fixture passed once on this host;
the earlier container failure remains historical and is not retrospectively
changed into a pass. Existing default-core dependency warnings remain.

Reproducible local build environment: set PKG_CONFIG_SYSROOT_DIR to the private
root above, PKG_CONFIG_PATH to $PKG_CONFIG_SYSROOT_DIR/usr/lib/x86_64-linux-gnu/pkgconfig,
and LD_LIBRARY_PATH to $PKG_CONFIG_SYSROOT_DIR/usr/lib/x86_64-linux-gnu. Run
python3 scripts/native_deb.py build --shlibs-file /tmp/rqs-native13-layer-shell.shlibs
--library-dir "$LD_LIBRARY_PATH". This is a task-local environment, not installed
runtime search configuration. Actual GTK4/Layer Shell versions:4.22.4/1.3.0.


Historical authorization wait: the fresh known-owner/global-idle gate passed
and the two idle rQuickShare units were joined. The exact native22 package then
waited for local administrator authentication. The user confirmed the system
password window; installation subsequently completed successfully.

## Installed observations and handoff

Apt exited0 and upgraded0.11.5+native21 to0.11.5+native22. Normal launch restored
one daemon (observed PID2495491) and one desktop (2495492). All14 installed regular
files match package bytes, modes and root ownership. Login start remains disabled.
Preflight found zero prepared resources and no nonterminal transfers; two prior
terminal records were released by the documented restart, not received files.

Passive post-install readback found exactly one Active rquickshare tray, the
expected15 menu rows, valid running Visibility2 and the new authenticated Batch1
endpoint-association read. No agent send, clipboard change, automatic acceptance
or discovery-mode change was performed. A later passive snapshot had one transfer
row; only counts are recorded, with no peer names, source contents or PINs.
See evidence/native22-installation-2026-10-17.json for measured receipts.

I1-I5 are complete for local package/install verification. Physical KDE layout,
recipient-first chooser, expanded details and live endpoint grouping are ready
for user testing, not marked accepted. Real Android/Windows and broader Mode B
acceptance remain open. The exact preview sandbox fixture passed on this host;
that does not prove the previously failing CI container or full native suite.

Only the native package revision and documentation changed from the previously
checked production source. The actual locked package build/verification and
single affected host sandbox test were run; unchanged core/GTK suites were not
repeated. The final documentation receipt uses [skip ci] to avoid repeating
unchanged CI/ten-run GTK work on this local packaging checkpoint. No CI pass
is claimed for that receipt; the package revision was checked in the real build.
All observer/installer commands exited (Terminal closed). The normal installed
application services intentionally remain running for the user's tests.
