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

## Pending product choices

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
- [ ] P2: freeze clipboard/completion contracts, branch/risk inventory and rollback.
- [ ] P3: simplify history rows; move source controls and implement prepared curtain.
- [ ] P4: implement the endpoint-bound inline action strip and one-shot acquisition;
  preserve the daemon FIFO, immutable source snapshots and explicit receiver consent.
- [ ] P5: stage-aware incoming rows and guarded once-only completion presentation.
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
