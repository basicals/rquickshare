# Essential installed/peer acceptance

2026-10-16. Short procedure prepared from the current native11 checkpoint.
Status: not executed. This is current-package acceptance, not an ADB/hardware
automation project. See [priorities](native-essential-priorities.md) and
[receive reliability finding](native-receive-reliability.md).

## Preconditions

Agree on the actual Linux package/desktop and available Android/Google Quick Share
for Windows versions before installation. Native11 was built for Ubuntu26.04
amd64 and verified offline only. Record exact package SHA256, OS/desktop and peer
versions. Installation/activation and physical device actions require the user's
availability/authorization; no such operation was performed to prepare this file.
Use disposable test files/destination, not personal documents. Record evidence
locally without publishing device identifiers, filenames, contents or PINs.

## Minimal ordered checks

| Check | Action | Required observation |
|---|---|---|
| 1 — discover and consent | With each available Android/Windows peer, find Linux and peer in both directions. Send one small file; compare the verification code and explicitly accept, then separately reject a request. | Correct intended device; no content accepted before consent; rejection leaves no received file. Record an unavailable platform as untested, not passed. |
| 2 — ordinary content | In both directions send one file, several files, a nested folder and text/link. Compare exact bytes/hashes and relative folder names; copy received text explicitly. | Correct content and destination; links do not auto-open or replace clipboard. Existing files remain intact; name collisions are resolved or rejected safely. |
| 3 — interrupted receive | In a disposable destination cancel midway through a large file; separately interrupt the peer/network. Retry explicitly. Exercise the selected repair from native-receive-reliability.md before release signoff. | No incomplete file under an ordinary final name; no false full success, unintended overwrite or duplicate send. Retained completed members match the selected partial-outcome contract. |
| 4 — card and multiple recipients | Use tray/card near the clock, Full/Compact and two recipients when available. Close the card during a send; decline/cancel one recipient. Lock/unlock and exercise notification inhibition. | Admitted work survives card close; member outcomes remain independent; lock/notification policy is respected; progress/cancel and received-file location are usable. |
| 5 — desktop/lifecycle | Use file-manager Send, enable/disable optional login start, reopen from the launcher and tray, then quit/restart under controlled idle conditions. | Recipient choice remains explicit; one daemon and one tray owner; no manual config repair or surprise automatic send. |
| 6 — package lifecycle | After ordinary transfer failures are resolved, verify install/upgrade/removal and settings recovery for the actual target OS. Test signed APT acquisition only after the exact release is approved/published. | Dependencies resolve, settings remain recoverable, and update ownership is the system package manager. No unsigned staging is represented as a trusted published channel. |

Stop a check on a concrete failure, preserve local evidence and fix its root cause
before repeating the affected check. Record PASS/FAIL/UNTESTED with candidate/peer
versions and a brief reason. This procedure supplies no current compatibility,
physical acceptance or release-publication claim by itself. Previews, account/QR,
resume and advanced payloads are outside these essential checks.
