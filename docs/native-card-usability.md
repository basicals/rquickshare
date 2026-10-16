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

## Mockup steering — 2026-10-16

The user supplied /tmp/codex-clipboard-c6f9fbc7-9eae-42c2-a73a-ef9668bcb570.png
and explicit written corrections. The written corrections govern differences
from the illustration. Retain the current application/theme styling; the image
is a layout reference, not a request to reproduce its colors, invented device
records or logo. No image generation, Figma import or new bitmap assets needed.

This steering supersedes U5/U6 layout proposals, while additive selection, drag,
single Open/Hide, no routine Reconnect and deferred OK/Cancel requirements remain.
It also explicitly authorizes the main-card redesign that was previously outside
the essential-release scope. It does not authorize persistent history, a trust
database, enrollment or automatic acceptance merely because the image includes
a Paired and recent section.

### Actual model limitations

`StateView::endpoints` exposes only ephemeral endpoint id/name, without verified
persistent identity or device type. Transfer details have advertised sender/type;
`TransferView::recipient` records outbound recipient association. Inbound sender
names are untrusted display metadata, not an authenticated association to a
discovery endpoint. Do not merge incoming requests into a named endpoint row by
string equality, invent phone/laptop kinds for unknown discoveries, or present
an advertised/green status as paired/trusted. The existing certificate source is
empty and remembered trust remains separately gated in native-trust-review.md.

Use exact discovery ids and transfer capabilities for keyed rows and actions.
Current incoming requests and active transfers can appear within the same list,
with safe per-request detail expansion. A returning-peer group needs a genuine
identity source; hide unavailable grouping rather than creating fake records.
Any recent result display must remain daemon-session data, with its lifetime
made clear; no new durable history store is included in this layout slice.

### Mockup implementation plan before code

1. [x] M1: inspect the image, current keyed rows, recipient controls, typed
   state/identity limits and existing settings/menu owners. Record scope and
   deviations here, preserving native15 evidence and installed state.
2. [x] M2: resolve Receive semantics, conflicting tray appearance entries and
   the still-pending additive source owner choice. These are concrete behavior/
   API questions; implementation remains stopped until the user's response.
3. [ ] M3: freeze a presentation model for one keyed vertical list. Remove main
   StackSwitcher tabs and repeated Nearby/Available labels and branding. Header
   contains Settings, one hide/close control and view toggle, with a drag region.
   Keep bounded lists and existing source/recipient explicit multi-send behavior.
   Inline request/transfer actions expand exact code/manifest/progress/cancel/
   saved-file details without navigating to redundant tabs or losing state.
   Do not turn a decorative row Receive button into unchecked consent.
4. [ ] M4: Compact has exactly Receive, Send, Hide and Full view actions; no
   Settings shortcut. A request requiring review opens the full/details surface,
   keeping consent explicit. Status dots overlay device icons, occupy no separate
   status column, and have tooltip/accessibility descriptions. Unavailable,
   pending, active, completed and failed states must not rely on color alone;
   critical errors and PIN remain readable in details.
5. [ ] M5: shorten tray/context to Receive, Send files, incoming requests/active
   transfers with actual counts, the existing three live visibility choices,
   Settings, About and Quit, plus the previously requested single Open/Hide if
   retained after clarification. Remove routine Reconnect, duplicate image
   action and source-specific menu clutter. No invented counters. Appearance
   menu entries follow the pending conflict resolution; no theme/icon redesign.
   Sending entry can expose file/folder/text choices within the sending surface,
   preserving essential existing capabilities without a separate Image command.
6. [ ] M6: replace the narrow scrolling Settings column with a separate wider
   window, branded header and left navigation/right content. Use populated
   sections for General, Receiving, Advanced and About (last); do not add an
   empty Transfers category solely to imitate the image. Existing fields only:
   name, login start, appearance, visibility, receive folder and port. Recovery
   actions appear contextually in Advanced when needed. Each section fits
   naturally without an internal scroller. Common Cancel/OK footer owns the
   staged draft across sections; switching sections never applies values.
   Preserve the U6 validation/CAS/exact-retry/capability/partial-outcome rules.
7. [ ] M7: implement the approved additive source contract and integrated source
   controls, run scoped checks once, inspect real offline GTK frames including
   all Settings sections and four-button Compact, then distinct package/install
   verification under U7/U8. Physical KDE/peer results remain separate.

M3-M6 require no persistent history or Google account/trust implementation. Before
changing row/consent/settings state handling, document its concrete branch and
interaction checks; do not weaken existing safety, multi-recipient or recovery
assertions to make a new layout pass. Rollback each model/API/layout milestone
as a coherent atomic commit. Native15 remains the retained installed baseline.

Pending Receive choices: recommend reviewing a waiting request, otherwise using
the existing daemon-owned one-minute visibility command to become discoverable.
Alternative: only open waiting-request details and leave visibility unchanged.
Neither choice permits auto-accept, opening links or clipboard replacement.
The illustration includes Compact/Full tray entries, conflicting with the user's
earlier explicit removal request; ask which instruction to retain.

Observed mockup verification: image/source inspection and documentation only;
no production edits, tests, builds, service/settings changes or external requests.
All owned one-shot commands exited: Terminal closed.

## Selected mockup contracts

The user selected all recommended choices: Receive opens a waiting request for
review, otherwise explicitly requests the existing one-minute daemon visibility;
Full/Compact stays only on the card; additive file selection is daemon-owned and
atomic. No remaining decision gate blocks these selected milestones. The earlier
File submenu question is superseded by the mockup's single Send entry. Preserve
file/folder/text choices inside that sending surface, rather than in the tray.
Implementation starts with [source extension](native-source-extension.md), then
the documented presentation/settings milestones. No automatic acceptance/replay.

## Presentation and settings state contract before implementation

The main surface is one list. Internal Incoming/Active filters may remain as
navigation destinations for the shortened tray, without visible notebook tabs;
normal opening returns to the combined list. Incoming rows are keyed by exact
transfer capability, not joined to discovery names. A request's Receive/Review
action expands its code/manifest and explicit Accept/Decline controls. Active
rows expose progress/cancel in details; terminal results remain accessible for
the current daemon session, especially retained partial files and explicit Copy.
No full persistent history screen or empty Paired section is introduced.

Discovered rows have a generic device icon until a real typed device kind exists,
name, an icon-overlay status and Send action. Existing checkboxes/explicit batch
Send preserve multi-recipient capability. Row Send with a prepared source sends
only that exact available endpoint; without a source it selects that recipient
and opens sending choices, never auto-sending after a file dialog completes.
The sending surface offers Files, Folder and Text, replacing separate tray image
entries; a normal image file remains sendable. Add is available for file sources,
not text. All final actions reuse SDK safety/readiness/recovery guards.

Compact always contains four quick actions; receiving requests requires opening
full review, never accepting directly. Badge/count/exception text can describe
pending work without adding another quick action. Changing visibility uses the
existing daemon command/receipt and one-minute clock. Known status colors overlay
icons and include shape/tooltip/accessibility descriptions; no separate status
column or claim that a colored dot proves identity/trust.

Dragging uses a header-only gesture and current monitor-relative anchors/margins,
clamped against actual card dimensions. Account for the widget moving under the
pointer rather than repeatedly applying relative gesture deltas. No global input
capture or native toplevel protocol assumption. Drag marks manual presentation;
tray activation resets placement beside the tray. Content resizing clamps the
position; no persistent position schema or cross-monitor pointer tracking.

Settings keeps one draft across populated sidebar sections, branded header and
shared Cancel/OK. Appearance no longer previews while editing. Close/Escape acts
as Cancel before submission; reopening reads the current acknowledged state.
While an OK/save/login operation is admitted, prevent draft edits and premature
close; cancellation cannot revoke an admitted writer. Validate all native fields
and the requested login target before saving. Native save (when changed or an
import/apply is needed) runs first, then login start only if changed. Close only
after both requested owners acknowledge success; a partial/uncertain outcome
keeps the editor open with explicit status and the successful stage retained.
Retry native uncertainty uses the original SDK intent. Login uncertainty requires
readback, not automatic Set replay. Port changes retain explicit idle-only restart
in Advanced; OK never interrupts active transfers. Invalid/unavailable sources
keep contextual repair/reload paths. About is the last sidebar page, not a routine
card action. Removing routine Reconnect leaves the existing exceptional fresh
connection recovery on an unavailable card, with no automatic transfer replay.

Focused verification plan: existing GTK smoke exercises four Compact actions,
no main tabs, header/settings/hide, inline consent and retained-file controls,
add readiness, exact row actions, stage-only appearance/Cancel and sidebar/footer
layout; existing private tray checks cover actual counts and visibility-aware
Open/Hide with removed entries absent. Pure geometry covers clamping/negative
origins/size boundaries. Settings state checks cover native/login success, partial
failure, uncertainty, close inhibition and retry/readback. Renderer review checks
actual compact/full/request/settings sections and theme-relative rounded styling.
Each check runs once after its source changes; failures trigger affected reruns.
