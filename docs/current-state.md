# Current implementation and continuation state

Updated2026-10-16. Mode A; direct dev, no branch/worktree/push. This is the current
status index. Dated entries in other documents remain historical evidence; they
must not override this index or the linked feature contracts. The active goal is reliable ordinary nearby sharing with native
tray/card UX. [Essential priorities](native-essential-priorities.md) supersede
the broader execution order: previews are excluded and full feature parity is a
long-term inventory, not an essential-release gate.

[Completion audit](native-completion-audit.md) separates implemented manual flows,
remaining product work and current-package physical acceptance. The basic native
shell is substantial but not release accepted or UI-complete.
[Live visibility/quick controls](native-live-visibility.md) option1 is implemented
and scoped source checks passed. The latest completed slice is
[multi-recipient sharing](native-multi-recipient-review.md): daemon-owned atomic
admission, exact receipts, GTK selection and per-peer status/cancel. Native11
offline package and corrected Full/Compact rendering passed. The latest source
and offline candidate is [receive reliability](native-receive-reliability.md),
with atomic per-file publication and explicit retained-file partial results in
native12. Exact source/package/handoff evidence is recorded in that contract. Native9 remains an immutable earlier checkpoint.

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
or account API calls. Native12 is the latest measured package (see current artifact below). The next scoped
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
contracts remain unimplemented. Preview development is now excluded by the user;
retain this as historical foundation evidence. Do not ask for the decoder boundary again or claim full
preview compatibility from local fixtures.

[Preview wire review](native-preview-wire.md) now records complete public Google
source inspection at both the earlier pin and current main. No codec/mapping
fixture was established. The scoped internal inbound routing fix is implemented and its focused checks
passed: declared previews are bounded opaque data, isolated from sharing control
parsing and discarded. Production preview IPC/display is unimplemented and excluded from the active
scope. Bounded opaque routing remains input-safety behavior. Native9 remains immutable and does not include this source fix.

## Implemented source and remaining acceptance

| Area | Current evidence | Remaining work |
|---|---|---|
| Native ownership | Rust daemon, authenticated typed D-Bus, GTK4 tray/card, Full/Compact, separate menu/notifications, live visibility status/quick modes, core-owned temporary visibility and owned SDK cleanup implemented | Installed KDE/Wayland lifecycle, placement, lock/DND/focus/scale and physical visibility acceptance |
| Sharing | Daemon-owned1..10-recipient admission/results, file/folder preparation, consent/PIN, progress/cancel, atomic per-file receive publication/explicit partial results, received text explicit Copy, outgoing text/clipboard/PNG, received-file reveal and PairedKey authentication foundation implemented | Current native package with physical Android/Windows peers; operational certificate provider, no full compatibility signoff |
| Settings | Daemon-owned schema2/Settings2 store, migration/backup/recovery, live device name and GTK editor implemented | Real installed migration/recovery and Google/KDE name refresh |
| Desktop integration | Dolphin Send assets, descriptor handoff, systemd activation and opt-in daemon+hidden-card tray login start implemented | Installed context-menu/session/autostart, upgrade/removal/live-transfer acceptance |
| Distribution | Verified uninstalled Ubuntu26.04 amd64 native12 package with live visibility controls and private sandbox helper; public source/package/unsigned-APT/unapproved-request handoff verified; publisher and narrow HTTPS Worker route implemented | Exact release approval, bucket-scoped publisher credential, real signed public APT acquisition and installed updates |
| Signing recovery | Production primary/subkey initialized on existing Linux PC; non-release proof, encrypted local restore and all four independent cloud artifact downloads passed | Recovery checkpoint complete; hourly follow-up deleted; preserve archive password and backup custody |
| Further parity | Modern wire fields, folder support, native live name and core temporary deadline exist | Installed visibility controls, physical discovery withdrawal and suspend acceptance, Ordinary transfer/desktop acceptance first; account identity, resume, streams, Wi-Fi/app and QR/Apple capabilities are deferred; previews are excluded |

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

No production APT release has been signed/published. Current native12 package SHA256
b4ef947385b46a2b16f2dc625f1cfc216205672b1df75478f821bb4a857962f0 was built
from88c1f94003fc35c665568024273d446b5ced35ed and verified offline. It includes daemon-owned
multi-recipient sharing and the defensive announced-preview routing fix.
Unsigned APT metadata expires2026-10-23T18:08:35Z; canonical
first-release request 34d1ca73c10956a269196ee42005996abcfbb09012a9dafbe3406bc3fd7c7e05
is unapproved. Exact source/check/artifact evidence is in
native-receive-reliability.md and evidence/native-receive-reliability-2026-10-16.json.
Native11 remains the immutable multi-recipient checkpoint.
Native10 is a preserved intermediate package whose visual review exposed the
Nearby-footer navigation defect corrected in native11; it was not handed off.
Native9 remains the previous live-visibility checkpoint.
Native8 is the isolated decoder checkpoint and remains immutable.
Native7 is the verifier checkpoint, native6 the core visibility checkpoint, native4 the name checkpoint;
intermediate native5 was superseded by explicit SDK shutdown. Prior artifacts
remain preserved. Do not confuse a source-commit assertion with proof
of build provenance or report later documentation-only commits as binary rebuilds.

## Latest implemented parity slice

[Multi-recipient sharing](native-multi-recipient-review.md) selected option1:
the daemon admits1..10 recipients atomically using one prepared source and bounded
per-peer descriptor reservations. Existing ordered receipts recover exact results;
GTK never loops over sends or silently drops a selected vanished peer. Separate
transfer rows retain recipient names and independent Cancel. Core8 affected methods
and native39 passed; final navigation-specific GTK1/0 and strict lints passed.
Native11 includes these changes and the defensive preview routing foundation.
Remote preview codec/mapping, production preview IPC/card binding and full Google
interoperability remain unestablished. Installed/radio/current-peer acceptance is
separate from these offline source/package observations.

## Current essential work order

The authoritative ordered queue and completion criteria are in
[native-essential-priorities.md](native-essential-priorities.md):

1. Current Android/Google Windows discovery and ordinary two-way sharing acceptance.
2. Safe interruptions, storage failures, partial-file handling and explicit retry.
3. Installed KDE/Wayland tray/card, consent, progress/cancel and recovery.
4. Installed file-manager Send, optional login start and single-process ownership.
5. Straightforward package lifecycle and the selected signed APT update path.

Without physical peers, inspect existing failure/cleanup/retry paths, fix concrete
scoped defects, prepare targeted acceptance steps and check package lifecycle
requirements. Do not substitute new accounts/QR/preview work for missing physical
evidence. Previews are excluded; resume and advanced parity are deferred.
Development checks run once per affected change/failure. No installation or release
approval is implied by this ordering.

## Active reliability finding — 2026-10-16

Source review found that incoming files are created under final names before
content arrives and request drop has no incomplete-file cleanup authority.
Exclusive creation and short-EOF validation already exist, but do not prevent
visible empty/truncated files after failure. The initial finding was source-only. The selected repair and offline regressions
are now implemented; this is not physical-peer acceptance.
[Receive reliability](native-receive-reliability.md) records the significant
publication/partial-outcome choice: recommend atomic publication per file, preserve
complete members and expose partial outcome; alternative is one all-or-nothing
containing transfer directory. The user selected option1; implementation now stages unnamed files, publishes
complete members without overwrite and exposes terminal partial receipts through
the existing native file capability. Core focused checks, native39/0, strict lints and the native12 offline package
passed. Exact public source/artifact handoff follows the receive contract; native11
and earlier artifacts remain immutable.
[Essential acceptance](native-essential-acceptance.md) provides the short current
Android/Windows/KDE check procedure; all checks remain unexecuted. Native11 and
its immutable public artifacts are unchanged.

## Installed acceptance preparation — 2026-10-16

[Installed acceptance](native-installed-acceptance.md) read-only readiness matches
native12 to this Ubuntu26.04.1/KDE Wayland host. Native12 is not installed: legacy
0.11.5 is installed, native services absent, no rQuickShare process observed at
inspection. Dependency simulation passed (upgrade application, add Layer Shell,
remove0). Because the old local debug .deb differs from the installed executable,
an exact five-file installed-state recovery package was prepared privately and
verified. Exact recovery/candidate hashes and pending installation/authentication
are in evidence/native-installed-acceptance-2026-10-16.json. No system change or
physical-peer acceptance occurred; peer availability remains unconfirmed.
