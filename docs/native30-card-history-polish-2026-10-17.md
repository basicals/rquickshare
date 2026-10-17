# Native30 card and history polish

Mode A,2026-10-17, dev after f20e6d9. Guide: /home/morozhenko/.codex/AGENTS.md.
The user reports successful Android -> Linux text receipt after explicit Accept,
with a noticeable unmeasured delay before consent appeared. Do not represent that
delay as fixed or measured. Native29 remains installed during source work.

## Approved changes and implementation plan

1. [x] Analyze tray activation, drawer presentation, sizing and shared history rows;
   record this plan before edits. Existing user choices authorize these UX changes.
2. [x] On opening from a primary tray click or the tray Open action, close the
   device drawer before presentation. Clicking an already visible card still hides
   it. Explicit Devices/Send/incoming review retain their existing expansion rules;
   closing the drawer does not cancel transfers or discard prepared sources.
3. [x] Rename visible Transfers menu/window references to History, preserving
   action names, IDs and D-Bus contracts. Add theme-aware horizontal and vertical
   cell borders to the shared six-column live/metadata layout, keeping measured
   column groups and reserved actions aligned. No new history data or controls.
4. [x] Diagnose the actual open/closed width requests and constrain the same card
   width in both states. Keep bottom anchoring, visible controls, wrapping/ellipsis
   and monitor clamping. Fix the width owner rather than adding per-row offsets.
5. [ ] Run affected native checks once and strict lint, inspect actual-widget
   captures; document results. Include the already checked outbound metadata repair
   in a distinct revision30 package, preserve29, then use a fresh idle/source gate
   for any upgrade. Occupied/prepared state requires preservation/user action.
6. [ ] Record package/installation and pending physical acceptance; commit logical
   source changes and final evidence separately. No mainline merge/public release.

No new significant architecture/API/data/security decision is required: these are
the requested existing-widget behaviors. History borders use shared CSS classes;
the selected application theme remains authoritative. Risk/rollback: avoid hiding
an incoming request's explicit consent, clipping actions, changing drawer state on
ordinary daemon redraw, or losing the lower-edge/position contract. Revert the
source slice or restore exact native29 after the same fresh idle gate.

## Focused verification

Extend existing GTK smoke and menu checks rather than adding a separate test
framework: hidden Open -> closed drawer, visible Activate -> hidden, next Activate
-> closed again; no prepared-source mutation. Compare actual GTK width requests
and allocations for collapsed, empty/list, pending consent and sending rows. Assert
grid cell classes/count and aligned coordinates with/without actions; parse CSS
without errors and inspect rendered History. Label changes keep ID5 and counters.
Use existing isolated display/private broker; no live test sends or clipboard read.
No repeated ten-run gate; checks affected by a concrete failure may rerun once.

## State and continuation

Initial request-only width probe passed1/0 in0.54s; it does not prove mapped layer
surface allocation or every row state. Keep the actual mapped-width assertion
and a private fixed-width single-child surface as the horizontal size owner:
420 logical pixels, matching the existing default/placement width. It delegates
vertical measurement/allocation/snapshot to the existing root. This avoids a
window default-size hint being overridden by drawer natural widths; content still
wraps/ellipsizes and actual consent/toolbar controls must fit the allocation.
No per-state resizing or hidden horizontal scrolling/clipped action workaround.

At14:55:14Z, installed29 Text1 snapshot/revision-bound bounded read matched the
intended26-byte UTF-8 fixture SHA-256
40c0efd6cc70764ec4b71c302d9a8c4aa5477f6655f60b0e7def998c92b569f1.
No body or clipboard was logged/read outside that checked capability. Clipboard/
copy/close and the reported delay timing are not inferred. Broader folder/cancel/
Windows/failure/lifecycle acceptance remains open.

## Measured source checkpoint

One private420px surface now owns horizontal measurement; the existing root owns
all content/vertical layout. The smoke test paints both open/closed surfaces at
frame boundaries and asserts actual width420 and a fitting root minimum. Inspected
captures are420x196 and420x140; direct consent buttons fit. This is isolated X11
rendering, not physical KDE/Wayland acceptance. Tray smoke covers hidden primary
open -> collapsed, visible primary -> hidden, reopen -> collapsed and menu Open
-> collapsed, preserving the same service StateView Arc.

Shared history rows use contiguous cells with theme-derived borders and existing
SizeGroups/reserved action slot. CSS parse, cell classes and exact column alignment
passed in the existing widget test. Full native run:50 passed/2 failed in0.77s;
only failures were stale expected `Transfers (3)/(0)` labels in the two menu tests.
Update those assertions to the explicitly requested `History` strings, retaining
IDs/counter checks. Affected menu rerun passed2/0 in0.04s; strict native Clippy
passed in7.69s. No full52/0 rerun claim or repeated stability loop. Existing private
display/layer-surface and core no-experimental unused-code warnings remain separate.

Next: unique revision30 locked release build, exact package History visual review,
then fresh idle/source/owner gate before any installation. If prepared sources
remain, preserve them and request user action. Rollback29 is retained unchanged.
