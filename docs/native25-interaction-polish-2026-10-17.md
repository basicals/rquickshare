# Native24 feedback: interaction and completion polish

2026-10-17, Mode A, dev only. Guide: /home/morozhenko/.codex/AGENTS.md,
sections3/3.1/11/12/13. Current installed native24 remains unchanged during
analysis. User approved the general visual direction, but reported discovery
still slow and supplied a real Transfers screenshot with duplicated controls.
This document supersedes the relevant earlier explicit-only result-opening and
recipient-first interaction descriptions, subject to the pending choices below.

## Analysis and requested contract before code

Existing Rows uses one Expander plus a redundant Details button; expanded cards
repeat peer/status/bytes already present in the heading. The journal has separate
metadata-only rows after daemon restart. Simplify live row content without
inventing content authority for those older metadata records.

The card currently reserves progress before incoming consent and puts code/type
in expanded content. Before transfer bytes start, use the middle of the peer row
for payload type/code; during actual transfer replace it with progress. Incoming
consent still requires the real code/manifest and explicit Accept; changing row
layout does not authorize automatic acceptance.

Successful incoming completion should display Completed and automatically reveal
the actual received file/folder, or show plain received text with Copy/Close.
This user request authorizes automatic result presentation; it does not authorize
executing files, opening links or changing the clipboard automatically. Reuse
existing Files1 checked FD/portal and Text1 capabilities. Only a newly observed
successful incoming completion in the current GUI lifetime can trigger the
result; baseline/reconnect/journal rendering must never reopen old results.
Retain lock/disconnect/epoch/stale-result guards and bounded joined task owners.
Multiple-file/folder selection is best effort through the supported portal;
never reconstruct opener authority from a label or journal metadata.

Two outbound flows are explicit:
- Global prepared source: keep source/add controls between the expanded peer
  list and bottom four-button panel. Cover bottom controls with the requested
  blurred selection curtain and a clear-source action. Clicking a device Send
  queues the retained source in click order. Clearing the prepared source must
  not revoke already admitted transfers or remove source files.
- No global prepared source: per-device Send toggles one animated four-action
  inline strip extending left: File, Folder, Text, Clipboard. Selecting an action
  closes it, binds that exact current endpoint and opens the corresponding input.
  Successful selection/editor submission immediately admits one send, without
  another preview/Send step. Cancellation admits nothing; stale/missing endpoint,
  ambiguous admission and failed preparation must not silently retarget/retry.
  No existing prepared set is overwritten by a per-device draft.

Clipboard representation and transient Completed lifetime require the explicit
choices below before production edits; these materially affect what is sent and
what remains on the card. No new broad refactor or persistent device identity.

## Discovery observations

User reports native24 Android discovery is still slow. Privacy-safe current-process
trace: advertisement started28ms after visible at127737ms; first subsequent GATT
slot0 read at154816ms (about27.1s after visibility). Another visibility window
started201914ms, advertisement started201933ms; subsequent first slot0 read at
220411ms (about18.5s). mDNS resend followed these reads within1ms. Trace does not
identify peers or record when the Android share screen opened, so those intervals
are not definitive end-to-end phone discovery measurements. Do not claim the
previous timer-starvation repair solved the on-air/GATT delay. No raw peer name,
address, file content or PIN is retained in this evidence.

Next diagnosis must distinguish Bluetooth advertisement scheduling/scanning,
GATT discovery/read and TCP initiation, with bounded opt-in stage evidence and
an explicit live reproduction. Do not lower retry intervals or reset the adapter
without locating the responsible stage. Existing periodic/deferred fixes remain
valid independent timer regressions.

## Product choices (approved A/A)

1. Clipboard: recommended supported local copied files/folders, image, then
   plain text/link, with deterministic structured-file > image > text priority.
   Use existing bounded source acquisition; reject unsupported/nonlocal file
   entries without sending a different fallback representation. Alternative:
   text/link and image only, explicitly reject copied file lists. The broader
   option covers the expected Clipboard action but adds file-list ingestion and
   mixed-format validation; priority must be visible/documented.
2. Completed indicator: recommended retain the transient result until the peer
   list/card is hidden, then keep it only in Transfers. Alternative: automatically
   remove after a short visible interval. The first avoids missed status and
   timing guesses but may retain several completion rows while the list stays
   open; the second is cleaner but can disappear before the user sees it.

## Ordered milestones after choices

- [x] P1: inspect screenshot, live-row/composer/reveal ownership and privacy-safe
  discovery events; record explicit requirements and missing decisions.
- [x] P2: freeze clipboard/completion contracts, branch/risk inventory and rollback.
- [x] P3: simplify history rows; move source controls and implement prepared curtain.
- [x] P4: implement the endpoint-bound inline action strip and one-shot acquisition;
  preserve the daemon FIFO, immutable source snapshots and explicit receiver consent.
- [x] P5: stage-aware incoming rows and guarded once-only completion presentation.
- [ ] P6: scoped receiver diagnosis/repair based on demonstrated evidence.
- [ ] P7: run affected checks once, record actual results, atomic commits and a new
  verified native package/known-owner idle upgrade for physical acceptance.

Planned focused checks before tests: no duplicate history expand/status controls;
row code/type-before-bytes and progress-during-bytes; source placement/curtain clear
preserving queued payloads; action strip toggle/cancel/stale endpoint; direct send
exactly once after input; clipboard priority/bounds/failure; completion success-only,
no baseline/reconnect replay, locked/disconnected stale authority, explicit text
Copy only. External portal/clipboard dependencies use existing test seams; domain
logic is real. No whole Mode B or repeated ten-run development gate.
Rollback each independent UI/flow/result/discovery commit. Do not weaken existing
source limits, receipt authority or code/manifest consent to simplify the UI.

## Approved implementation ledger and additional feedback

User chose A for both clipboard and completion lifetime. Clipboard supports local
file/folder lists, image, then text/link in that priority; unsupported/nonlocal
structured entries fail without a different-format fallback. Completed remains
on the card until the card/device drawer is hidden, then only in Transfers.

Additional explicit requirements: softly pulse the actual pending inbound peer
row green, without name-based association. Pulse Receive only with the device
drawer closed; the open drawer already has a visibility spinner/countdown. Move
that strip after the device list, before bottom controls, alongside prepared
source controls. Main Send opens its type popover without opening the drawer;
open devices only after a global source is successfully prepared. The contextual
per-device strip is a distinct direct-send flow and does not use the popover.

Implement P3 first: reorder source/countdown after the list; one real blurred
backdrop around the bottom controls with a clear-source curtain; deduplicate live
history metadata and expansion controls; incoming row pulse and drawer-aware
Receive pulse. Use a small GTK snapshot wrapper for actual blur (no CSS property
guessing), forwarding child allocation and owning its lifetime. Existing discard
releases preparation only; admitted queue payloads remain daemon-owned.

P4 uses exact epoch/endpoint intent and the explicit preparation completion's
Selection, never a render-triggered guess at the first source. Reuse existing
consuming Send for a one-shot contextual source and retained Queue for a global
source. One owned GUI wait and existing joined ingress worker hold preparation
permits across acquisition. Cancellation or stale authority never auto-sends.

P5 tracks live phase transitions by epoch/transfer ID, seeds the first snapshot
as a baseline and never launches desktop effects from history/reconnect replay.
Result handling uses existing checked content/file capabilities and lock guards;
received content is never executed, links are never opened, clipboard replacement
requires Copy. Directory-level result capability, if required beyond existing
file portal authority, must be reviewed before changing a security/API boundary.

Affected verification per logical milestone, once: existing isolated GTK smoke
plus explicit assertions for child order, main Send preserving drawer state,
curtain/clear state, pulse classes, single history expansion/status and staged row
summary/progress; preparation/clipboard priority/cancel/limits and one-shot intent
checks; result baseline/success/replay/hide/security cases. Preserve measured
results per milestone and commit before the next stage. No native install until
the new package passes checks and a fresh known-owner/global-idle gate.

P3 observed: source/countdown moved after peer list, actual GTK snapshot blur
curtain disables underlying controls and reuses explicit source discard; main
Send preserves drawer state. Pending inbound rows pulse softly, Receive pulses
only when the drawer is closed. History uses its single expander, without Details;
peer/status/byte duplicates are hidden, manifests exposed directly after one
disclosure. Header carries type/code before transfer and progress during bytes.
No installed binary changed.

First affected GTK smoke failed an old tray-Send-opens-drawer assertion; changed
it to the approved closed-drawer plus visible-popover contract. Follow-up compile
found an invalid Image enum assumption: protocol images are Files, so removed
the nonexistent branch and use filename type. Final affected real widget smoke
passed1/0 in0.66s, including source/countdown order, blur input blocking and
drawer-aware Receive indication. Isolated Layer Shell warnings are expected,
not physical KDE proof. No ten-run/full Mode B claim. Terminal commands exited.

P4 source checkpoint: one animated four-action per-device strip, repeat click
closes it; main type menu preserves the drawer. File/folder chooser and text
editor bind exact epoch/endpoint and use the returned Selection; successful
context input uses existing consuming Send, global preparation opens devices
after completion and uses retained Queue on each row click. Clipboard uses
bounded file lists (256 roots/256KiB), PNG worker validation or bounded UTF-8,
with files > image > text and no fallback after the chosen format fails. Cut
clipboard entries are read as sources, never moved/deleted. Hide/lock/epoch
changes invalidate input generation; dropped waits never replay a send.

The joined ingress owner now returns explicit selection completion rather than
guessing from a later watch snapshot. Before submitting the contextual Send,
refresh both controller state and ingress state from their owners: GUI watch
delivery may lag the completed acquisition. Prepared source is preserved after
failed/stale-target admission for an explicit user decision. No daemon API or
protocol change in P4.

Measured verification: initial full native46 run had45 passes/1 failure; GIO
accepted a remote-host file URI as a local path. Added explicit empty/localhost
authority and percent-encoding validation without weakening the assertion.
Affected panel scope then16 passed/0 failed in0.67s (30 unrelated filtered),
including all8 MIME format combinations, local URI/count/size/malformed cases,
exact target/epoch/pending/disconnect guards and strip repeat toggling. Lint
found an unreachable payload fallback and a test helper after the test module;
removed the impossible branch and moved the helper. Strict all-target native
no-dependency lint passed1.42s. After the ingress-ordering guard and contextual
editor copy correction, affected actual widget smoke passed1/0 in0.50s. No ten
repetitions or whole Mode B claim; installed native24 remains unchanged.

## P5 result capability review before public/security-boundary changes

Existing Files1.AcquireFile returns only a checked regular-file FD by index. It
cannot provide a received top-level directory or one whole-batch result. The
portal OpenDirectory accepts a file/directory FD and opens its containing folder,
possibly selecting it; OpenFile also accepts directories but must never receive
an untrusted regular received file as an automatic action. Official reference
inspected2026-10-17:
https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.OpenURI.html#org-freedesktop-portal-openuri-opendirectory

Recommended option1: daemon resolves one typed completed-transfer result using
its original receive root and immutable receipt: one loose file -> checked file
FD, one common received folder -> checked directory FD, several loose roots ->
containing directory. SDK validates descriptor type and current epoch/revision;
GTK uses the matching directory-only/open-containing-folder portal action once.
No caller paths, no metadata-name authority, no content execution. This gives
correct folder/batch behavior but requires an additive result API, bounded
joined filesystem acquisition and replacement/rename/type validation. Any
unavailable result remains completed with a visible manual action/error.

Alternative option2: keep the current per-file API and automatically reveal only
the first complete file. It reuses checked authority and fewer contracts but
cannot consistently select the received folder or represent multiple loose roots;
the user may see the wrong level of a nested folder. Do not silently choose this
semantic shortcut for the requested whole-transfer result.

Section3/3.1 requires the user's choice before the additive result capability.
P3/P4 are reviewable independent commits; P5 lifecycle/Completed persistence and
automatic text/result presentation, P6 discovery and P7 package/install remain
open. Do not mark native25 installed or auto-result complete. No new public API,
file-manager launch, clipboard capture from the live session or service restart
was performed in these source stages. All command sessions exited.

## P5 approved result contract before implementation

User selected option1. Add Files1.AcquireResult(transfer:s,revision:t)->(kind:y,fd:h)
as an additive same-UID authenticated method; AcquireFile remains unchanged.
Only complete Finished inbound Files with an exact immutable receipt qualify.
Kind0: one loose file (regular read-only FD), kind1: one common received top-level
directory (read-only directory FD, show containing folder/select this directory),
kind2: several roots (original receive-root directory FD, open directory contents).
A single nested file selects its received top-level directory. Derive from actual
collision-resolved receipt paths, never advertised names/current settings/caller
paths. Open directories beneath the pinned root without symlinks; validate every
receipt member through that pinned selected directory before returning it. Root
rename is supported; missing/changed/replaced files or unsafe parents fail closed.
Do not auto-open partial/cancelled/failed batches. Old per-file partial-reveal
remains available manually. One shared bounded/joined acquisition owner handles
both methods and revalidates client/transfer/revision after filesystem IO.

SDK requires exactly one read-only non-PATH FD, validates kind versus actual
regular/directory type and current Finished authority; no FDs in ordinary state.
Controller returns the private typed result. Portal OpenFile is used only for a
checked directory and only to display its contents; automatic received regular
files use OpenDirectory (select in containing folder), never their default app.
Reuse the owned Request/Response/cancellation protocol and the one ingress permit.

Track current-run phase transitions, seeded on initial/changed epoch to suppress
historical replay. Successful inbound completion adds a transient Completed row
and a bounded result queue. Serialize result presentation while input/review or
portal work is active; text windows retain Copy/Close and never overwrite each
other. Closing a result wakes the queue. Lock/disconnect/epoch changes prevent
unsafe presentation. Card/drawer hide clears transient completion rows; hiding
the card also cancels not-yet-presented results, while future active->Finished
transitions remain observable. Daemon journal retention is unaffected.

Focused inventory before new tests: receipt modes0/1/2, nested/multiple roots,
renamed root and replaced/missing/symlink members, readonly directory flags;
real broker/SDK AcquireResult success/foreign/stale/partial/malformed FD/type;
portal rejects regular FD for contents and preserves existing cancellation tests;
result tracker initial/epoch baseline, success-only once, queue order, eviction,
hide and no active-transfer cancellation; GTK staged summary/progress, Completed
retention and explicit-only text Copy. Run affected scopes once and record actual
failures/fixes; no full Mode B/ten-run expansion.

P5 observed: additive typed whole-result API/SDK/controller and shared joined
acquisition owner implemented. Receipt modes cover loose file, common directory
and multiple roots; every member is revalidated through the pinned directory.
SDK rejects unknown kind, regular/directory mismatch, writable and PATH FDs.
Partial receipt cannot authorize whole-result acquisition; manual file reveal
is unchanged. GTK tracks successful current-run completion once, retains Completed
until hide, and serializes text/result presentation through existing guards.
Only checked directories can reach portal OpenFile; regular files use containing
folder selection. No automatic text copy, URL open or payload execution.

Measured P5 checks: initial compile found a missing Phase import and a temporary
borrowed message body; fixed both. Receipt scope5/0, SDK initial3/0 and bus5/0
passed; after adding descriptor-type/security cases, affected full service310/0
in8.25s and receipt5/0 passed. Strict core lint found one redundant borrow in the
new pure descriptor validator; corrected without behavior change, final strict
core lint passed15.57s. Native50/0 in0.66s, including new transition/replay/hide
rules and private-portal directory-only cases; strict native lint passed4.17s.
No real desktop/clipboard effects or installed update during these tests.
P6 discovery diagnosis and P7 final package/physical acceptance remain open.
