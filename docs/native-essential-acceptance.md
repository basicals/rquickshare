# Essential installed/peer acceptance

2026-10-16. Short procedure prepared from the native11 checkpoint and updated for the native12 receive repair.
Status: partially executed; native13 installed after the tmpfs source repair,
launch/tray/basic card confirmed.
Physical transfers and the remaining desktop/lifecycle matrix are unexecuted.
This is current-package acceptance, not an ADB/hardware
automation project. See [priorities](native-essential-priorities.md) and
[receive reliability finding](native-receive-reliability.md).

## Preconditions

Agree on the actual Linux package/desktop and available Android/Google Quick Share
for Windows versions before installation. Native12 targets Ubuntu26.04 amd64; use its exact final package/checksum
from the receive-reliability evidence after candidate handoff. The exact package is now installed on the matching host; offline checks plus
scoped installed observations are recorded in [installed acceptance](native-installed-acceptance.md).
They do not complete the physical checks below. Record exact package SHA256, OS/desktop and peer
versions. Installation/activation and physical device actions require the user's
availability/authorization; no such operation was performed to prepare this file.
Use disposable test files/destination, not personal documents. Record evidence
locally without publishing device identifiers, filenames, contents or PINs.

## Minimal ordered checks

| Check | Action | Required observation |
|---|---|---|
| 1 — discover and consent | With each available Android/Windows peer, find Linux and peer in both directions. Send one small file; compare the verification code and explicitly accept, then separately reject a request. | Correct intended device; no content accepted before consent; rejection leaves no received file. Record an unavailable platform as untested, not passed. |
| 2 — ordinary content | In both directions send one file, several files, a nested folder and text/link. Compare exact bytes/hashes and relative folder names; copy received text explicitly. | Correct content and destination; links do not auto-open or replace clipboard. Existing files remain intact; name collisions are resolved or rejected safely. |
| 3 — interrupted receive | In a disposable destination cancel midway through a large file; separately interrupt the peer/network. Retry explicitly. Verify the implemented native12 repair from native-receive-reliability.md before release signoff. | No incomplete file under an ordinary final name; no false full success, unintended overwrite or duplicate send. Retained completed members match the selected partial-outcome contract. |
| 4 — card and multiple recipients | Use tray/card near the clock, Full/Compact and two recipients when available. Close the card during a send; decline/cancel one recipient. Lock/unlock and exercise notification inhibition. | Admitted work survives card close; member outcomes remain independent; lock/notification policy is respected; progress/cancel and received-file location are usable. |
| 5 — desktop/lifecycle | Use file-manager Send, enable/disable optional login start, reopen from the launcher and tray, then quit/restart under controlled idle conditions. | Recipient choice remains explicit; one daemon and one tray owner; no manual config repair or surprise automatic send. |
| 6 — package lifecycle | After ordinary transfer failures are resolved, verify install/upgrade/removal and settings recovery for the actual target OS. Test signed APT acquisition only after the exact release is approved/published. | Dependencies resolve, settings remain recoverable, and update ownership is the system package manager. No unsigned staging is represented as a trusted published channel. |

Stop a check on a concrete failure, preserve local evidence and fix its root cause
before repeating the affected check. Record PASS/FAIL/UNTESTED with candidate/peer
versions and a brief reason. This procedure supplies no current compatibility,
physical acceptance or release-publication claim by itself. Previews, account/QR,
resume and advanced payloads are outside these essential checks.

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
