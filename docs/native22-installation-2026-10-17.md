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
- [ ] I2: advance only the native package revision to22, commit provenance and
  build the locked release daemon, GTK and private helper. Verify package exact
  inventory, dependency metadata, ELF, checksums and extracted offline commands.
- [ ] I3: run the exact real sandbox fixture once on this host; record the actual
  result separately from GUI and interoperability acceptance.
- [ ] I4: fresh same-owner/global-idle preflight: no source reservations, handles,
  prepared bytes/selections or nonterminal transfers. Never discard user work.
  Join only the identified idle rQuickShare services; use the system package
  authorizer to install the exact package. User enters any password locally.
- [ ] I5: normal launch; compare all14 installed regular files, versions, one
  real tray and unchanged login-start state. Record source/package/install
  evidence, export the candidate and leave physical acceptance to the user.

Risk and rollback: a service restart clears in-memory terminal history and
window placement. No source file or received content is removed. Non-idle work
blocks the restart. If installation fails, restore normal launch of the actual
installed version and record the failure; retain both packages. Idle rollback
uses retained native21 through the package manager, not file replacement.
New grouping uses ephemeral live endpoint association only, never trust/name
matching; consent and clipboard policy are unchanged. No full Google parity.
