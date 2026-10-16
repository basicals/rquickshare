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
- [x] U2: obtain the significant source-extension decision and resolve what the
  single File tray entry contains. No production implementation before answers.
- [x] U3: header/background drag on the existing surface with bounded monitor-relative
  geometry, preserving lock/consent/focus and tray reactivation placement. Record
  geometry invariant tests and actual KDE acceptance separately.
- [x] U4: coherent source entry/action layout, plus Add beside Clear for an
  existing file selection using the approved owner contract. Cancellation,
  invalid names, duplicate names, size/count/FD limits and lost responses must
  preserve the previous selection. Existing transfers keep their source snapshot;
  no selection/edit action sends anything.
- [x] U5: simplify both exported tray and GTK context menus. One visibility-aware
  Open/Hide action, no Full/Compact menu entries, no routine Reconnect entry.
  Keep appearance toggle on the card; exceptional connection recovery stays
  contextual and uses the existing no-replay owner lifecycle. Freeze the File
  entry after the user's clarification; retain essential folder/text capabilities.
- [x] U6: lay out Settings as a wider labeled grid without an internal scroll
  container; About at the bottom, Cancel/OK in the footer. Editing fields is
  local until OK, including appearance/login start. Cancel/close discards an
  unsubmitted draft. Serialize validated native save then the requested login
  mutation, display each acknowledged/uncertain outcome, and keep the dialog
  open on failure. Preserve exact retry/CAS/directory-FD/recovery ownership and
  idle-only service restart. Freeze detailed commit/close state handling before
  implementing this milestone. Do not silently remove recovery for corrupt
  settings or an unavailable service.
- [x] U7: extend affected existing contract/GTK checks, run appropriate checks
  once, inspect isolated actual widget frames, and record observed results.
  No ten-run development loop or full compatibility claim.
- [x] U8: atomic scoped commits, distinct native16-or-later candidate, offline
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
3. [x] M3: freeze a presentation model for one keyed vertical list. Remove main
   StackSwitcher tabs and repeated Nearby/Available labels and branding. Header
   contains Settings, one hide/close control and view toggle, with a drag region.
   Keep bounded lists and existing source/recipient explicit multi-send behavior.
   Inline request/transfer actions expand exact code/manifest/progress/cancel/
   saved-file details without navigating to redundant tabs or losing state.
   Do not turn a decorative row Receive button into unchecked consent.
4. [x] M4: Compact has exactly Receive, Send, Hide and Full view actions; no
   Settings shortcut. A request requiring review opens the full/details surface,
   keeping consent explicit. Status dots overlay device icons, occupy no separate
   status column, and have tooltip/accessibility descriptions. Unavailable,
   pending, active, completed and failed states must not rely on color alone;
   critical errors and PIN remain readable in details.
5. [x] M5: shorten tray/context to Receive, Send files, incoming requests/active
   transfers with actual counts, the existing three live visibility choices,
   Settings, About and Quit, plus the previously requested single Open/Hide if
   retained after clarification. Remove routine Reconnect, duplicate image
   action and source-specific menu clutter. No invented counters. Appearance
   menu entries follow the pending conflict resolution; no theme/icon redesign.
   Sending entry can expose file/folder/text choices within the sending surface,
   preserving essential existing capabilities without a separate Image command.
6. [x] M6: replace the narrow scrolling Settings column with a separate wider
   window, branded header and left navigation/right content. Use populated
   sections for General, Receiving, Advanced and About (last); do not add an
   empty Transfers category solely to imitate the image. Existing fields only:
   name, login start, appearance, visibility, receive folder and port. Recovery
   actions appear contextually in Advanced when needed. Each section fits
   naturally without an internal scroller. Common Cancel/OK footer owns the
   staged draft across sections; switching sections never applies values.
   Preserve the U6 validation/CAS/exact-retry/capability/partial-outcome rules.
7. [x] M7: implement the approved additive source contract and integrated source
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

## Implementation checkpoint before final verification

The source/SDK extension is committed as228785e with scoped core checks in its
own SSOT. Native acquisition and mockup widgets are implemented but not installed.
Main navigation now opens the single combined list and expands exact request/
active rows; there are no user-visible tabs or name-based identity joins. Five
recent terminal results are retained in the list, plus any result still exposing
received content/partial files. Four Compact actions, icon-overlay status with
accessible descriptions, header-only bounded movement and shortened tray are
implemented. Tray Open/Hide projects actual visibility and counts actual state;
an event-driven idle publication avoids borrowing GTK's model reentrantly while
visibility changes. No elapsed-time polling or automatic operation replay.

Settings has four populated sidebar pages and deferred fields/OK/Cancel. The
initial40-method native run passed39 and found an obsolete no-Choose-files label
assertion on a hidden source button; the button is now consistently Add files.
Affected GTK runs then exposed obsolete immediate appearance-preview expectations
and a lazy file-manifest assertion without expansion. These were updated to the
approved staged-mode and explicit-detail contracts, keeping exact PIN/filename
assertions after expanding the real manifest. Ten panel methods passed10/0 in0.60s.
An optional actual-widget capture passed1/0 in0.77s and produced four760x389
Settings section PNGs with no scroller. All four were visually inspected.

Final review found that the old `load(view, true)` also applied appearance during
Reload, despite the new staged edit policy. Before correction: make loading pure
editor population, apply card mode only from an acknowledged Save/Retry or the
existing authoritative daemon-state update. Reopening obtains current read-only
settings and never treats a picker change as an apply. Hide routine recovery
controls when healthy; retain them for actual errors/uncertainty. An unapplied,
restart-required or unconfirmed-name save must keep Settings open and must not
silently continue/close as fully applied. Then rerun only affected panel checks,
native strict lint and final visual/package validation. Native15 remains installed.

## Verified source milestone, native16 candidate not yet installed

The new layout/source-acquisition/settings code is complete. The exported menu
has15 entries including separators: one visibility-aware Open/Hide, Receive,
Send, actual request/active counts, three visibility choices, Settings/About/Quit.
No view choices, folder/image/text duplicates or routine Reconnect. Exceptional
visibility retry remains contextual on the card, not a permanent tray item.
The sending popover retains Files/Folder/Text/explicit Clipboard image capability;
there is one entry button instead of the two isolated Files/Folder buttons.

Compact uses four equally arranged icon/text quick actions; Full has Settings,
view toggle and hide/close with a drag area. Status overlays distinguish available,
unavailable, pending, sending, receiving, completed, cancelled and failure using
shapes plus theme-relative color and descriptions, without another status column.
Existing prepared sources expose Add files beside Clear. Real source changes use
the committed daemon-owned extension; text sources cannot be extended as files.

Settings uses a760px-wide natural-height window and General/Receiving/Advanced/
About sidebar; the four observed sections fit760x389 without scrolling. Reload
now only populates the editor; Save/Retry acknowledgements or the authoritative
state update apply appearance. Cancel resets the draft; section changes preserve
it. Controls/close are blocked while admitted work runs. Native unapplied/uncertain/
restart-required outcomes keep the window open; login errors require explicit
readback before another write. Existing daemon CAS/directory/recovery and systemd
ownership remain unchanged. No login-start write occurred during development.

Native checks: the full41-method suite passed41/0 in0.72s before the final helper/
readback changes; the final affected panel suite passed11/0 in0.90s, including the
single real GTK smoke and two new geometry/status mapping methods. Earlier
post-change failures were stale separate-page/immediate-preview/lazy-manifest
expectations and one helper double-borrow compile error; updated assertions check
the approved single list and staged editing while retaining exact code/manifest,
lock/hide, batch-send and recovery checks. No assertion was weakened to accept a
production defect. Geometry checks clamp all boundary positions; physical Wayland
dragging remains an explicit acceptance gate. Strict native all-target/no-deps
Clippy and formatting/whitespace run after final code. The same nine existing
core dependency warnings remain visible; no suppression or unrelated cleanup.

Observed debug frames: Compact420x141 and Full devices420x341 in the isolated
renderer, with transparent rounded corners. The incoming fixture was then moved
to the same expandable-row builder used by production, avoiding a visual claim
from the old separate Incoming page. Original decoder fixture remains isolated
historical code; no production thumbnail routing was added. Final package release
frames and installed checks follow; these debug observations are not KDE signoff.
All owned one-shot checks/renderers exited: Terminal closed. Native15 still runs.

## Native16 package/release visual gate

Built from1a9d675d45196ba11e57b2261830bd882c39f011. The strict offline builder
passed its14-file/owner/mode/asset/dependency/ELF/checksum/help/private-settings
checks. Package:rquickshare_0.11.5+native16_amd64.deb,6,502,012 bytes, SHA256
902748767e11c8e5c0307653c13e795737d13dc43364ed8e24dd25b1e4ea4ede.
The native15 package remains unchanged for rollback. Native16 is not signed or
published; no APT staging/request or key operation is implied.

Actual release GUI renders passed: Compact420x141, Full devices420x341,
Full inline incoming review420x349. All card corner alpha values are0. Reviewed
the three release frames and the final four760x389 Settings sections; no parser/
CSS/critical failure. Incoming release frame uses the production expandable-row
builder, shows the exact fixture code/manifest and explicit Accept/Decline, with
no payload thumbnail. Example records and the Preview-only status are inert
renderer data; they are not discovered/paired physical peers. Receipt and hashes:
native_desktop/target/mockup-native16-release/receipt.json.

Next authorized local repair gate: verify exact native15 package/installed version
and known service owners1925337/1925338, globally zero prepared-source resources,
no nonterminal transfers, one own tray and no busy preparation. Only then join
those idle services, upgrade the exact native16 package through system Pinentry/
Polkit authorization, reload units without enablement and normal launch. Require
all14 installed files to match, one tray with15 menu rows and unchanged login-start
state. Stop if owners/work differ; never force-kill or interrupt active work.
Physical KDE dragging/chooser/OK/Cancel acceptance remains separate after install.

## Native16 installation safely deferred by prepared work

At2026-10-16T22:58:26Z the preflight matched native15 and known daemon/client
PIDs1925337/1925338. The daemon was Running/synchronized with zero transfers,
but had one retained source reservation, two reserved handles and414 selected
bytes. The observer owns no selections; the existing GUI owns that preparation.
Do not infer a filename or discard/adopt another client's source from display
metadata. The idle-resource assertion stopped the upgrade before any service
stop, package operation or system mutation. Installed version remains native15;
login start remains disabled. Preserve the failed preflight as evidence of the
guard, not a package/install defect. Script:/tmp/rqs-native16-upgrade.py;
receipt:/tmp/rqs-native16-upgrade.json; log:/tmp/rqs-native16-install.log.

Asked the user to finish sending or clear the prepared selection, or defer the
installation. Do not poll unchanged work or delete it automatically. All source,
checks, release package and renderer artifacts are complete and can be exported
while waiting. Only a fresh idle gate after the user's response permits the
already-authorized local upgrade. No further architecture question is pending.
All owned commands exited: Terminal closed; normal native15 app services remain.

## Immutable export and continuation

Exported the verified6,502,012-byte native16 package/checksum and seven actual
widget frames to /home/morozhenko/Documents/ChatGPT/quickshare и nativepair/outputs.
Immutable497-file source archive:rquickshare-native16-source.zip, source commit
23adf00cb30ed9e826314451c0bcf27e6b6b9942, SHA256
faeedae18b84d9d6541c3f5671bb0412459f7da374d3c86980defdc527c673c1.
Binary source remains1a9d675d45196ba11e57b2261830bd882c39f011. Exact export receipt:
rquickshare-native16-export.json. Mutable source pointer, feature/evidence/current
state/progress/readiness copies are refreshed; native15 immutable package/archive
hashes were rechecked and preserved. The older unsigned APT/request is explicitly
not the current candidate. No cloud/key/signing/publication work occurred.

U3-U7 and M3-M6 are source/offline complete. U8/M7 installed/physical completion
still waits for the user clearing/sending the existing prepared work or choosing
to defer. Resume with a fresh known-owner/idle preflight, never by assuming the
old preparation disappeared. No further rebuilding/testing is needed unless
source changes or a new defect is found. Source remains direct dev; no push,
branch/worktree creation or unknown-process termination. Terminal closed.

## 2026-10-17 user-closed GUI and resumed installation gate

The user reported exiting the tray application and asked whether that released
the preparation. Fresh read-only inspection confirmed daemon PID1925337 still
Running/synchronized, desktop unit Inactive/dead with PID0, zero reservations,
handles, selected bytes and transfers. Exiting the client released its unsent
capability; no source file was deleted. Native15 remains installed at this gate.

The existing upgrade is authorized and now unblocked. Adapt only the idle-owner
preflight to allow this verified inactive desktop (PID0, no own tray) as well as
the previously known active owner. Recheck zero resources immediately before
joining the known daemon and installing exact verified native16. Do not start
the old GUI just to satisfy its previous tray predicate. Keep login start off;
normal post-install launch must restore one desktop/daemon/tray and14 matching
installed files. Preserve the earlier deferred receipt, no rebuilding/retesting
of unchanged code, and no automatic transfer/settings replay.

## Native16 installed verification — 2026-10-17 local date

The fresh gate passed with the GUI inactive, the known daemon owner and globally
zero source/transfer resources. Only the owned idle services were joined. The
exact verified local native16 upgrade returned apt exit0; normal launch restored
daemon PID2139034 and desktop PID2139035. All14 installed regular files match the
archive bytes/modes/root ownership. Login start remains disabled, no unrelated
package upgrade/removal, settings mutation or received-file deletion occurred.

Real KDE watcher inspection found one Active rquickshare item with15 menu rows,
the expected action order and all removed duplicate/view/reconnect IDs absent.
Two explicit Open/Hide events were checked by LayoutUpdated-driven readback,
restoring the original card visibility. No sleeps, repeated unchanged polling,
selection/send/consent event or unknown-process operation was used.

One installed API smoke used this observer's two private temporary files (3+7
bytes), held read-only descriptors and an independent caller connection. Actual
Operations1.CommitExtension atomically produced one2-file/10-byte/4-handle source;
an exact replay returned the same token and no transfer was created. Explicitly
discarded only that observer's new source and closed its connection; resources
returned to zero and fixture bytes matched until the observer removed its own
temporary directory. This does not establish physical Add chooser/peer behavior.

Receipts:/tmp/rqs-native16-upgrade.json, /tmp/rqs-native16-installed-tray.json,
/tmp/rqs-native16-installed-extension.json; install log:/tmp/rqs-native16-install.log.
Tracked evidence preserves the earlier deferred observation separately. No source
change/rebuild or unaffected development-suite repetition was needed. All owned
observer/installer commands exited: Terminal closed. Normal app services remain
running intentionally. Source/package/installed scope is complete; physical KDE
dragging, chooser and Settings OK/Cancel review, plus Android/Windows acceptance,
remain explicit pending checks. No signing/APT publication or Drive/key work.

## User-confirmed installed dragging — 2026-10-17

The user answered "Yes, the card can be dragged" to the specific native16 full
card/header movement check on their KDE/Wayland session. Mark physical dragging
passed on this installed host. This is a user observation, not an automated
compositor test, multi-monitor/scale survey, settings acceptance or peer result.
No code/settings/service changes or development-test reruns were needed. Next
manual UI gate is staged Settings Cancel/OK, then additive file chooser behavior;
ordinary Android/Windows acceptance remains in the separate essential matrix.

## Native17 bottom-edge and Compact order follow-up — plan before code

The user confirmed Settings edits remain unsaved until OK on native16. Record
that specific user observation; no broader fault-recovery claim follows from it.
They then requested Compact order Receive, Send, Full view, Hide and a shared
bottom edge between Full/Compact, especially after dragging Full.

Analysis: drag currently anchors Left/Top, and clamp_moved_surface preserves its
top margin when the natural height changes. This necessarily lifts the Compact
bottom edge. Subtracting the height delta on each render would accumulate errors
when several updates occur before GTK allocates the new height. Use the native
Layer Shell bottom anchor as the position owner instead of such corrections.

1. [ ] B1: swap the two Compact widget children, keeping the same actions and
   four-action invariant. No menu/daemon/settings contract change.
2. [ ] B2: drag writes Left/Bottom margins. Before changing a visible card's mode,
   convert a Top anchor to Bottom once using its current allocation; preserve an
   existing Bottom margin. Pin that position across content renders/mode changes,
   preventing apply_mode from reapplying the old tray hint. Rename the internal
   dragged flag to position_pinned because mode switching also pins geometry.
   Keep horizontal anchoring, logical coordinates, monitor bounds and lock/hide
   guards. Explicit tray placement starts a fresh tray-relative position.
3. [ ] B3: add focused pure geometry boundaries/Full-Compact roundtrip assertions
   and extend the existing GTK smoke for exact quick-action order. Run affected
   checks and strict native lint once, then inspect the final Compact frame.
   No full core suite or ten-run repetition; rerun only failures/affected changes.
4. [ ] B4: commit source/docs, build/verify distinct native17 and preserve native16.
   Upgrade only after the existing known-owner/global-idle gate. Record14 actual
   installed files, one tray and unchanged login state; physical bottom alignment
   needs the user's KDE check. Update artifacts/evidence and continuation index.

Invariant: while pinned and the resized card fits above its existing bottom edge,
monitor_height - bottom_margin stays constant across repeated renders and mode
roundtrips. If expansion would cross the monitor top, clamp once to keep the
whole card visible; screen-fit takes precedence over an impossible position.
Do not persist a position or add a timer/global pointer/window-manager fallback.
Rollback is the native17 source commit plus retained native16 package when idle.
