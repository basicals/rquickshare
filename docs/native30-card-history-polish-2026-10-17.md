# Native30 card and history polish

Mode A,2026-10-17, dev after f20e6d9. Guide: /home/morozhenko/.codex/AGENTS.md.
The user reports successful Android -> Linux text receipt after explicit Accept,
with a noticeable unmeasured delay before consent appeared. Do not represent that
delay as fixed or measured. Native29 remains installed during source work.

## Approved changes and implementation plan

1. [x] Analyze tray activation, drawer presentation, sizing and shared history rows;
   record this plan before edits. Existing user choices authorize these UX changes.
2. [ ] On opening from a primary tray click or the tray Open action, close the
   device drawer before presentation. Clicking an already visible card still hides
   it. Explicit Devices/Send/incoming review retain their existing expansion rules;
   closing the drawer does not cancel transfers or discard prepared sources.
3. [ ] Rename visible Transfers menu/window references to History, preserving
   action names, IDs and D-Bus contracts. Add theme-aware horizontal and vertical
   cell borders to the shared six-column live/metadata layout, keeping measured
   column groups and reserved actions aligned. No new history data or controls.
4. [ ] Diagnose the actual open/closed width requests and constrain the same card
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

Pending. Installed native29 two-file roundtrip remains independently hash-verified.
The text report is user evidence only until checked against the intended fixture;
clipboard/copy/close and delay timing are not inferred. Broader folder/cancel/
Windows/failure/lifecycle acceptance remains open.
