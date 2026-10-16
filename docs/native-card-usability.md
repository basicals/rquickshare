# Card, tray and settings usability follow-up

2026-10-16. Mode A, direct dev. Guide: /home/morozhenko/.codex/AGENTS.md,
sections 3/3.1/11/12/13. This supersedes the pending generic native15 visual
review with the user's concrete usability feedback. Native15 remains installed;
no source/runtime/package change has been made for this follow-up yet.

## Analysis

The card is an undecorated Layer Shell surface. `placement::place` deliberately
snaps to the nearest monitor edge; no drag controller exists. Ordinary toplevel
window-manager movement cannot simply be assumed for this surface. A scoped
header gesture can move its existing anchors/margins within its current monitor
without replacing the tray/card architecture. Reopening from the tray can retain
the existing clock placement policy; persistent or cross-monitor coordinates are
outside this requested slice.

The source row has separate small Files/Folder buttons and hides both once a
selection is admitted. `Panel::choose` and the SDK controller reject another
selection. The SDK releases its upload preparation after commit; immutable daemon
sources retain read-only capabilities and can be shared by multi-recipient
transfers. Adding files therefore needs a source-ownership/API decision, not a
button wired to discard/reopen the previous paths. Names never authorize reopening.

Tray/context duplicate Open/Hide, Full/Compact and file/folder/image entry points.
The exported projection has no actual card visibility field. Reconnect explicitly
closes/joins the GUI client and recreates it; it does not replay operations. This
is exceptional recovery, not a routine setting. Keep that safety contract rather
than introduce an automatic retry/replay loop as incidental cosmetic work.

Settings currently use a narrow scrolling column, immediate appearance preview,
separate native Save and systemd login Apply, plus reload/retry/restart/reconnect
controls. Close hides and retains the editor draft. Native settings have CAS and
exact retry; login start is a separate systemd owner with confirmed readback.
There is no cross-owner atomic transaction. An OK flow must report partial or
uncertain outcomes truthfully and must not pretend Cancel rolls back acknowledged
writes. The user's explicit request authorizes deferred application and Cancel;
it does not authorize transport restarts while active work exists.

## Requested contract and proposed milestones before code

- [x] U1: inspect guides, settings/source/controller/placement/menu owners; record
  native15 limitations, pending choices and rollback here.
- [ ] U2: obtain the significant source-extension decision and resolve what the
  single File tray entry contains. No production implementation before answers.
- [ ] U3: header drag on the existing surface with bounded monitor-relative
  geometry, preserving lock/consent/focus and tray reactivation placement. Record
  geometry invariant tests and actual KDE acceptance separately.
- [ ] U4: coherent source entry/action layout, plus Add beside Clear for an
  existing file selection using the approved owner contract. Cancellation,
  invalid names, duplicate names, size/count/FD limits and lost responses must
  preserve the previous selection. Existing transfers keep their source snapshot;
  no selection/edit action sends anything.
- [ ] U5: simplify both exported tray and GTK context menus. One visibility-aware
  Open/Hide action, no Full/Compact menu entries, no routine Reconnect entry.
  Keep appearance toggle on the card; exceptional connection recovery stays
  contextual and uses the existing no-replay owner lifecycle. Freeze the File
  entry after the user's clarification; retain essential folder/text capabilities.
- [ ] U6: lay out Settings as a wider labeled grid without an internal scroll
  container; About at the bottom, Cancel/OK in the footer. Editing fields is
  local until OK, including appearance/login start. Cancel/close discards an
  unsubmitted draft. Serialize validated native save then the requested login
  mutation, display each acknowledged/uncertain outcome, and keep the dialog
  open on failure. Preserve exact retry/CAS/directory-FD/recovery ownership and
  idle-only service restart. Freeze detailed commit/close state handling before
  implementing this milestone. Do not silently remove recovery for corrupt
  settings or an unavailable service.
- [ ] U7: extend affected existing contract/GTK checks, run appropriate checks
  once, inspect isolated actual widget frames, and record observed results.
  No ten-run development loop or full compatibility claim.
- [ ] U8: atomic scoped commits, distinct native16-or-later candidate, offline
  package verification and existing authorized local repair after an idle-owner
  check. Never overwrite native15 or stop an unknown/active owner. Record real
  installed checks and leave physical drag/chooser/OK/Cancel observations explicit.

## Source decision pending

Recommended: daemon-owned atomic extension/replacement of an unsent immutable
file selection using bounded capability upload and exact receipts. Publish the
new source only after validating the whole aggregate. Existing queued/running
transfers retain their immutable snapshot; failure retains the previous unsent
selection. This changes the source API and requires focused core/SDK/UI changes.

Alternative: a GTK-owned capability draft before daemon preparation, with all
files admitted together later. This can reuse ordinary source admission, but
changes source lifetime and CLI/desktop ingress ownership, and unsent drafts
depend on the GUI. It requires a broader refactor than a daemon-owned extension.
No path-based reconstruction or discard-before-success shortcut is acceptable.

## Risks, verification and rollback

Moving a Layer Shell surface differs from native toplevel dragging: scaled
coordinates, moving-widget gesture offsets, changing content height and monitor
bounds need explicit checks. Physical KDE behavior cannot be inferred from Xvfb.
Source extension must preserve quotas, sender ownership, immutable transfer
sources, retry identity and descriptor-only authority. Settings must distinguish
unsent edits from admitted saves and separately acknowledged login changes;
failure cannot be hidden by closing the window. Large fonts/small screens remain
an acceptance concern even after removing the unnecessarily small scroll window.

Keep each source/API, presentation and settings milestone independently reviewable
with its SSOT update and conventional commit. Roll back consumers before a new
source endpoint; preserve all prior settings/received files. Retained native15
and native13 packages provide installed rollback only when idle. Signing, APT
publication, Drive, previews and advanced peer features are outside this slice.

Observed checks: analysis and documentation only. No new tests/build/install,
settings mutation, menu event, source preparation or transfer. All owned one-shot
commands exited: Terminal closed. Implementation is pending the scoped decisions.
