# Current implementation and continuation state

Updated2026-10-16. Mode A; direct dev, no branch/worktree/push. This is the current
status index. Dated entries in other documents remain historical evidence; they
must not override this index or the linked feature contracts. Goals remain native
tray/card UX and full Google Quick Share parity, not an achieved compatibility claim.

[Completion audit](native-completion-audit.md) separates implemented manual flows,
remaining product work and current-package physical acceptance. The basic native
shell is substantial but not release accepted or UI-complete.
[Live visibility/quick controls](native-live-visibility.md) option1 is implemented
and scoped source checks passed. Native9 offline package and Full/Compact
rendering passed; public artifact export is next. Native8 remains preserved.

## Documentation maintenance plan and rationale

Six copied top-level distribution checkpoints drifted: they still required an
isolated signer and unknown fingerprint after the user selected this workstation
and initialized production keys. Product-direction also opened on an obsolete
domain-choice checkpoint. Consolidate current facts here and link the entry points
instead of copying another mutable status paragraph. Preserve dated observations.

1. [x] D1: replace duplicated checkpoints with links to this index, add an explicit
   historical-log notice, and correct the current capability summary.
2. [x] D2: prepare the next device-name ownership/lifecycle review from actual
   core/settings/transport readers. Significant UX/API implementation waits for
   user selection; no protocol/runtime changes in this documentation slice.
3. [x] D3: validate links/removed stale checkpoint claims and whitespace once;
   commit each logical documentation change. No unaffected Rust/package rebuild.

Risk: readers may mistake old dated checkpoints for current state. Mitigation:
entry links and explicit historical labels. Rollback is a documentation-only
commit; no runtime, saved schema, key custody, remote object or package changes.

Measured documentation check: seven entry points and21 relevant local links passed;
at that checkpoint archive verification passed and sidecars were still pending.
All four cloud artifacts are now verified as recorded below and in JSON evidence.
The device-name review explicitly gates implementation on selection.
Whitespace check passed. No production code changed or unaffected checks repeated.
The user subsequently requested continuing the recommended live-update option1.
[Live device name](native-device-name.md) N1–N5 are complete in source, scoped checks
and a verified uninstalled native4 package. The subsequent completed slice is
[temporary visibility ownership](native-visibility-review.md): mDNS previously
owned expiry, so its failure could leave Bluetooth temporarily visible indefinitely;
a stale tick could also overwrite a newer mode command. Core-owned policy is
selected by the user and implemented: core-owned deadline, read-only transport/
shell state, generation-safe expiry and owned mDNS SDK shutdown. Source checks
and the corrected native6 uninstalled package passed. V5 public source/package/
unsigned-APT/request export is verified; exact provenance and hashes are in the
visibility contract/evidence. Continue installed/physical acceptance
separately; do not restart the already completed name slice.

Next completed foundation: [authenticated peer/trust contract](native-trust-review.md).
User selected option1. Bounded role/token-bound P-256 signature/HKDF/certificate
verification and inbound/outbound PairedKey result gates are implemented; explicit
Fail stops before Introduction. Twelve new methods, scoped checks and native7
offline verification passed. Production certificate source is explicitly empty:
operational acquisition/authorization/rotation and remembered trust remain open.
No trusted-device UI, account access or auto-accept is implemented. Native7 public
source/artifact export is verified; exact receipt/hashes are in the contract and
evidence. Do not claim Google Account
or physical peer acceptance from the offline fixtures.

[Certificate-provider review](native-certificate-provider-review.md) researched
the next dependency. No supported third-party scope/client path was established.
An independent optional Linux provider uses an explicitly unofficial master-token/
Google Play services exchange, not normal desktop OAuth. The user selected option1:
supported authorization only; continue independent parity. No sign-in, credentials
or account API calls. Native8 is the latest measured package. The next scoped
prerequisite [bounded byte ingress](native-byte-ingress.md) is implemented: both
handlers now share request-owned16-payload/5MiB reservation limits, stable headers,
exact completion and release-before-dispatch. Nine new methods and scoped checks
passed as documented; native8 includes this fix, while native7 remains immutable.
[Preview decoder boundary](native-preview-review.md) option1 is selected and its
offline native foundation is implemented: private Glycin/Bwrap helper, hard
resource limits, validated local pixel IPC, cancellation/reaping and Full-card
fixture. Ten new Rust methods/scoped native checks and the native8 uninstalled
Ubuntu26.04 amd64 candidate passed; extracted PNG/JPEG and Full/Compact rendering
were verified. D6 source/package/unsigned-APT/request public export is verified;
exact archive/binary provenance and hashes are in the decoder contract/evidence. Remote
preview encoding/mapping, daemon IPC/production binding and persistent resume
contracts remain open. Do not ask for the decoder boundary again or claim full
preview compatibility from local fixtures.

## Implemented source and remaining acceptance

| Area | Current evidence | Remaining work |
|---|---|---|
| Native ownership | Rust daemon, authenticated typed D-Bus, GTK4 tray/card, Full/Compact, separate menu/notifications, live visibility status/quick modes, core-owned temporary visibility and owned SDK cleanup implemented | Installed KDE/Wayland lifecycle, placement, lock/DND/focus/scale and physical visibility acceptance |
| Sharing | File/folder preparation, consent/PIN, progress/cancel, received text explicit Copy, outgoing text/clipboard/PNG, received-file reveal and PairedKey authentication foundation implemented | Current native package with physical Android/Windows peers; operational certificate provider, no full compatibility signoff |
| Settings | Daemon-owned schema2/Settings2 store, migration/backup/recovery, live device name and GTK editor implemented | Real installed migration/recovery and Google/KDE name refresh |
| Desktop integration | Dolphin Send assets, descriptor handoff, systemd activation and opt-in daemon+hidden-card tray login start implemented | Installed context-menu/session/autostart, upgrade/removal/live-transfer acceptance |
| Distribution | Verified uninstalled Ubuntu26.04 amd64 native9 package with live visibility controls and private sandbox helper; offline package/rendering verified, current public handoff export in progress; publisher and narrow HTTPS Worker route implemented | Exact release approval, bucket-scoped publisher credential, real signed public APT acquisition and installed updates |
| Signing recovery | Production primary/subkey initialized on existing Linux PC; non-release proof, encrypted local restore and all four independent cloud artifact downloads passed | Recovery checkpoint complete; hourly follow-up deleted; preserve archive password and backup custody |
| Further parity | Modern wire fields, folder support, native live name and core temporary deadline exist | Installed visibility controls, physical discovery withdrawal and suspend acceptance, trust/account identity, previews/resume, streams/multiple recipients, Wi-Fi/app completeness, QR/Apple/service research and versioned peer evidence |

Feature contracts and measured checks: [product history](product-direction.md),
[settings](native-settings.md), [desktop ingress](native-desktop-ingress.md),
[autostart](native-autostart.md), [package](native-package.md),
[APT](native-apt.md), [publisher](native-r2.md), [download route](native-download-route.md),
[local signing](native-local-signing.md), [recovery](native-signing-backup.md).

## Distribution binding already selected

Production base is https://download.morozhenko.com/dist/rQuickShare/ (case-sensitive).
The deployed rquickshare-download Worker reads only the dedicated private
rquickshare-native-apt bucket through that prefix. Existing app-dist/Picturix
routes were preserved. Domain and signer-host selection are complete; do not ask
for another host, browser login or replacement domain to resume independent work.

Signing uses protected authority and subkey-only profiles on the existing Linux
workstation, not independent-host isolation or offline primary custody. Public
primary6D8854762CF0C86881138A56695A0888D09FA616 and signing subkey
1F5150D98D94C43109FBA2503E7DE744C1014357 are initialized and verified.
This is not approval to sign a particular release request. The public keyring and
non-release evidence are committed; private profiles/passwords/ciphertext are not.

Recovery archive direct download matched20559 bytes and SHA256
a9de8d32972119de7ec747849c76c10e292543d0b984bb28ba0b948f0220f9bf.
It is the same ciphertext whose local restoration/signature/revocation checks
passed. The .sha256/receipt/instructions were independently downloaded and verified
on2026-10-15 at13:25:25–13:25:52Z, completing four-file cloud acceptance. The app
confirmed deletion of the hourly follow-up. Exact evidence and historical quota/
repair observations remain in native-signing-backup.md and its JSON.

No production APT release has been signed/published. Current native9 package SHA256
990251e7e89d483d1a282a782e6682ceb1a779e93a6fcacf83b8abfdc1c18b94 was built
fromb1ce2e21ea9e5d2f437d65206882e48d166e3491 and verified offline. Current
unsigned APT metadata expires2026-10-23T12:08:22Z; canonical first-release request
9d0732164d550ac4d1ccf4c6ab1b4ad960eff5bdd22fef8f20cc3425f7af2ab7
is unapproved. Exact artifact/source/check evidence is linked in
native-live-visibility.md and evidence/native-live-visibility-2026-10-16.json.
Native8 is the isolated decoder checkpoint and remains immutable.
Native7 is the verifier checkpoint, native6 the core visibility checkpoint, native4 the name checkpoint;
intermediate native5 was superseded by explicit SDK shutdown. Prior artifacts
remain preserved. Do not confuse a source-commit assertion with proof
of build provenance or report later documentation-only commits as binary rebuilds.

## Work that can proceed without physical peers

1. Continue the remaining parity slices after the completed live name and core
   visibility policy and PairedKey verifier: authorized certificate provider/
   rotation research, secure local trust, previews/resume, multi-recipient and
   account/QR research. Freeze each significant contract before code; a crypto
   verifier alone does not supply operational Google identity, and local trust
   must remain distinct from Google account membership.
2. Prepare concrete unsigned release review/provenance and selected publisher
   credential handoff independently of desktop testing; do not bypass release
   approval or invent credentials to publish.
3. Research remaining protocol/account/QR requirements against primary sources
   before proposing their significant contracts. Do not equate modeled protobuf
   fields or local substitutes with Google interoperability.

Physical installed KDE/Wayland, BlueZ/discovery and Google peer checks stay explicit
acceptance work. They are not reasons to rerun unrelated completed checks while
waiting for the user. Development checks run once per affected change/failure.
