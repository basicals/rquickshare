# Current implementation and continuation state

Updated2026-10-15. Mode A; direct dev, no branch/worktree/push. This is the current
status index. Dated entries in other documents remain historical evidence; they
must not override this index or the linked feature contracts. Goals remain native
tray/card UX and full Google Quick Share parity, not an achieved compatibility claim.

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
and a verified uninstalled native4 package. The next scoped analysis is
[temporary visibility ownership](native-visibility-review.md): mDNS currently
owns expiry, so its failure can leave Bluetooth temporarily visible indefinitely;
a stale tick can also overwrite a newer mode command. Core-owned policy is
selected by the user and implemented: core-owned deadline, read-only transport/
shell state, generation-safe expiry and owned mDNS SDK shutdown. Source checks
and the corrected native6 uninstalled package passed. Final source export is the
remaining V5 handoff command. Continue installed/physical acceptance
separately; do not restart the already completed name slice.

## Implemented source and remaining acceptance

| Area | Current evidence | Remaining work |
|---|---|---|
| Native ownership | Rust daemon, authenticated typed D-Bus, GTK4 tray/card, Full/Compact, separate menu/notifications, core-owned temporary visibility and owned SDK cleanup implemented | Installed KDE/Wayland lifecycle, placement, lock/DND/focus/scale and physical visibility acceptance |
| Sharing | File/folder preparation, consent/PIN, progress/cancel, received text explicit Copy, outgoing text/clipboard/PNG, received-file reveal implemented | Current native package with physical Android/Windows peers; no full compatibility signoff |
| Settings | Daemon-owned schema2/Settings2 store, migration/backup/recovery, live device name and GTK editor implemented | Real installed migration/recovery and Google/KDE name refresh |
| Desktop integration | Dolphin Send assets, descriptor handoff, systemd activation and opt-in daemon+hidden-card tray login start implemented | Installed context-menu/session/autostart, upgrade/removal/live-transfer acceptance |
| Distribution | Verified uninstalled Ubuntu26.04 amd64 native6 package, fresh unsigned APT and unapproved signing-review request; publisher and narrow HTTPS Worker route implemented | Exact release approval, bucket-scoped publisher credential, real signed public APT acquisition and installed updates |
| Signing recovery | Production primary/subkey initialized on existing Linux PC; non-release proof, encrypted local restore and all four independent cloud artifact downloads passed | Recovery checkpoint complete; hourly follow-up deleted; preserve archive password and backup custody |
| Further parity | Modern wire fields, folder support, native live name and core temporary deadline exist | Tray/current-visibility UX, physical discovery withdrawal and suspend acceptance, trust/account identity, previews/resume, streams/multiple recipients, Wi-Fi/app completeness, QR/Apple/service research and versioned peer evidence |

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

No production APT release has been signed/published. Current native6 package SHA256
5b4f07fc3546ac0dc493ed1d69740baae7c7b8c188d30eb4d61c30480a930692 was built
from32fc64f99bfd37b382cd732eaa97ff31c863fe05 and verified offline. The current
unsigned APT metadata expires2026-10-22T20:09:55Z; canonical first-release request
551a9499ab5ee893cdc3cdd4ed6a58009cdaf0762428f3f3f1ebe6bca2f96821
is unapproved. Exact artifact/source/check evidence is linked in
native-visibility-review.md and evidence/native-visibility-2026-10-15.json.
Native4 is the prior name checkpoint; intermediate native5 was superseded by
explicit SDK shutdown. Prior artifacts remain preserved. Do not confuse a source-commit assertion with proof
of build provenance or report later documentation-only commits as binary rebuilds.

## Work that can proceed without physical peers

1. Continue the remaining parity slices after the completed live name and core
   visibility policy: secure local trust, previews/resume, multi-recipient and
   account/QR research. Freeze each significant contract before code; local trust
   must remain distinct from Google account identity.
2. Prepare concrete unsigned release review/provenance and selected publisher
   credential handoff independently of desktop testing; do not bypass release
   approval or invent credentials to publish.
3. Research remaining protocol/account/QR requirements against primary sources
   before proposing their significant contracts. Do not equate modeled protobuf
   fields or local substitutes with Google interoperability.

Physical installed KDE/Wayland, BlueZ/discovery and Google peer checks stay explicit
acceptance work. They are not reasons to rerun unrelated completed checks while
waiting for the user. Development checks run once per affected change/failure.
