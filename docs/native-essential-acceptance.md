# Essential installed/peer acceptance

Updated2026-10-17. Native29 is installed on the existing KDE/Wayland host with
the frame-prefix interruption repair; UI is unchanged from accepted native28.
Current UI iteration is user-accepted: flat incoming consent, compact device
surface, automatic result location, aligned Transfers and latest idle/movement
polish. This is general feedback, not a measured per-case checklist. The user
confirmed fast Android discovery and successful sending on native25. Earlier
native13 one-file outbound/code match remains historical. Broader transfer,
failure and lifecycle entries are UNTESTED unless explicitly recorded below.
See [native29 evidence](evidence/native29-installation-2026-10-17.json) and
[priorities](native-essential-priorities.md).

## Preconditions

Available peer reported2026-10-17: Pixel6a, Android17; Quick Share version could
not be found. Google Quick Share for Windows is unavailable. Keep its rows
UNTESTED. Local deterministic sources and SHA-256 manifest are prepared under
`outputs/rquickshare-peer-check-2026-10-17` in the conversation workspace; these
are preparation only. See [current reliability slice](native-frame-interruption-2026-10-17.md).

Use the exact installed native29 package/hash from
[evidence](evidence/native29-installation-2026-10-17.json), with the actual Android
and Google Quick Share for Windows versions recorded before checking each peer.
The package targets Ubuntu26.04 amd64; unavailable peers/platforms remain UNTESTED.
Use disposable files/destination and preserve content/hash evidence locally
without publishing identifiers, filenames, contents or PINs. No new installation
is needed for this checklist. Real device actions require the user's availability.

## Minimal ordered checks

| Check | Action | Required observation |
|---|---|---|
| 1 — discover and consent | With each available Android/Windows peer, find Linux and peer in both directions. Send one small file; compare the verification code and explicitly accept, then separately reject a request. | Correct intended device; no content accepted before consent; rejection leaves no received file. Record an unavailable platform as untested, not passed. |
| 2 — ordinary content | In both directions send one file, several files, a nested folder and text/link. Compare exact bytes/hashes and relative folder names; copy received text explicitly. | Correct content and destination; links do not auto-open or replace clipboard. Existing files remain intact; name collisions are resolved or rejected safely. |
| 3 — interrupted receive | In a disposable destination cancel midway through a large file; separately interrupt the peer/network. Retry explicitly. Verify the implemented atomic receive repair from native-receive-reliability.md before release signoff. | No incomplete file under an ordinary final name; no false full success, unintended overwrite or duplicate send. Retained completed members match the selected partial-outcome contract. |
| 4 — card and multiple recipients | Use tray/card near the clock, collapsed/expanded Devices and two recipients when available. Close the card during a send; decline/cancel one recipient. Lock/unlock and exercise notification inhibition. | Admitted work survives card close; member outcomes remain independent; lock/notification policy is respected; progress/cancel and received-file location are usable. |
| 5 — desktop/lifecycle | Use file-manager Send, enable/disable optional login start, reopen from the launcher and tray, then quit/restart under controlled idle conditions. | Recipient choice remains explicit; one daemon and one tray owner; no manual config repair or surprise automatic send. |
| 6 — package lifecycle | After ordinary transfer failures are resolved, verify install/upgrade/removal and settings recovery for the actual target OS. Test signed APT acquisition only after the exact release is approved/published. | Dependencies resolve, settings remain recoverable, and update ownership is the system package manager. No unsigned staging is represented as a trusted published channel. |

Stop a check on a concrete failure, preserve local evidence and fix its root cause
before repeating the affected check. Record PASS/FAIL/UNTESTED with candidate/peer
versions and a brief reason. This procedure supplies no current compatibility,
physical acceptance or release-publication claim by itself. Previews, account/QR,
resume and advanced payloads are outside these essential checks.

## Observed native29 two-file roundtrip — 2026-10-17

User-operated Linux -> available Pixel6a/Android17 -> Linux two-file transfer
passed. The user confirmed phone receipt, reverse receipt and automatic folder
reveal. At14:40:57Z, received read-only descriptors were acquired through the
actual authenticated Files1 capability API; bounded reads matched original sizes
and SHA-256 for both34-byte text and65792-byte binary fixtures. Basenames with
space/UTF-8 were preserved. Sources/results were not removed or overwritten.
See [native29 evidence](evidence/native29-installation-2026-10-17.json).

This is partial check2 evidence, not completion of the whole table. Phone Quick
Share version, per-case code confirmation, nested folders/text/clipboard, rejection,
cancellation, Windows and failure/lifecycle remain open. A separate source-only
[outbound metadata fix](native-outbound-metadata-2026-10-17.md) follows the observed
zero-count journal defect; its installed verification is pending.

## Observed installed checkpoint — 2026-10-16

Native12 upgrade and Layer Shell dependency installation passed with explicit
user approval;11 installed files match candidate bytes/modes/root ownership.
Normal/repeated launcher preserves one systemd daemon/client and one real KDE
tray item. User confirmed icon/card and available Android. Disk descriptor
Begin/Append/Abort leaves no source/transfer resources; no Send/Commit. The tmpfs
fixture exposed the existing seal-capable-source rejection and its pending
accounting decision. Check1-4 physical content/consent/interruption and check5-6
remaining lifecycle/removal/signed updates are not passed by these observations.

Subsequent native13 installed repair accepts named tmpfs files while retaining
anonymous-source guards; real /tmp append/abort passes. Its CLI has prepared the
known123-byte Android fixture, with no agent-initiated peer choice or Send. User
recipient/code/acceptance result and exact Android version remain pending. See
[native-tmpfs-source-review.md](native-tmpfs-source-review.md) for current evidence.

Historical native15 added the requested card/tray presentation repair. Installed
14-file/tray/icon/About checks and isolated rendering passed; user physical
KDE review remains pending. The user confirmed receiving the Android outbound
fixture with matching code on native13; daemon reports Finished. Reverse/content
hash/versions and remaining matrix checks are still open. See [card cleanup](native-card-cleanup.md).
