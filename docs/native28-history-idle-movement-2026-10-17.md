# Native28 history, idle dismissal and movement

2026-10-17, Mode A, dev only. Guide: /home/morozhenko/.codex/AGENTS.md.
User confirmed native27 incoming inline consent works as intended. New feedback:
remove Show in folder from the card because completion already reveals the result;
keep manual result access in Transfers. Screenshot crosses out redundant title/
retention text and moves Refresh/Clear completed to the bottom. Different live
versus metadata rows and optional buttons currently shift table fields. The card
should hide after inactivity; some background drag zones show a hand but do not move.

## Analysis, approved scope and pending decision

Rows::cached_transfer appends result actions on all pages. Restrict manual result
access to Transfers while keeping automatic result presentation unchanged. Live
history and metadata-only history currently use different independent horizontal
boxes; optional action buttons compete with dates/columns. Use one internal
shared column layout for peer, direction, content/progress, status, timestamp and
reserved action slot across both containers. GTK SizeGroups synchronize actual
measured widths, including empty action placeholders. This is presentation only:
no persistence/API/identity changes and no invented content actions for old metadata.
Normal retention guidance moves to a tooltip; retain visible loading/error notices.
The title already exists in the window decoration. Move Refresh/Clear to the footer.

Movement currently rejects every ScrolledWindow ancestor, including empty space
and ordinary device labels, but assigns an inherited grab cursor to the whole
root. Share one ancestry predicate between pointer cursor feedback and drag
admission; allow inert scroll content, exclude actual controls, scrollbars,
selectable labels and popovers. Use capture-phase drag admission so scroll
containers cannot swallow approved background drags. Preserve monitor clamping,
shared bottom edge and clicks/scrolling on controls; no global pointer grab.

Autohide requires user choice: recommended15s without input with pointer outside,
only when no request/transfer/prepared source or open menu/dialog exists; alternative
also hides with requests/transfers. The asynchronous question is pending. Do not
implement a timing/product policy until answered; independent explicit UI and
movement repairs may proceed. Hide means card only, never stop/cancel daemon work.

## Milestones and verification inventory before code

- [x] A1: read screenshot, current row builders, movement and presentation ownership.
- [x] A2: record approved independent changes and pending idle policy.
- [ ] A3: card result-action removal and one measured history column contract;
  footer actions, no healthy-state prose, preserved error/recovery controls.
- [ ] A4: common draggable-target/cursor predicate and capture-phase admission.
- [ ] A5: implement only the user-selected idle policy after response.
- [ ] A6: affected checks once, exact-package visual inspection, reversible commits,
  distinct package and fresh idle-gated upgrade; installed/evidence/export receipts.

Focused checks: card has no manual result action but history retains it; equal
history column allocations with and without buttons, live and metadata rows;
footer positions and error visibility; draggable inert scroll labels versus
buttons/menu/text/scrollbar targets and outside-root rejection. Idle tests are
planned after policy choice, using explicit monotonic times rather than sleeps.
Do not bypass input/receipt/consent guards, weaken assertions or run ten dev loops.
Risks: losing manual partial-result capabilities, column overflow, stealing control
clicks, hiding pending consent or breaking position retention. Reuse existing
checked callbacks/SizeGroups/GTK event contracts. Roll back each independent UI
commit or the preserved native27 package after an idle gate. No BLE/protocol,
public release, cloud access or unrelated cleanup.
