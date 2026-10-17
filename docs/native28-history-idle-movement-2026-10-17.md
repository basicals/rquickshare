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
- [x] A3: card result-action removal and one measured history column contract;
  footer actions, no healthy-state prose, preserved error/recovery controls.
- [x] A4: common draggable-target/cursor predicate and capture-phase admission.
- [x] A5: implement only the user-selected idle policy after response.
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

## Approved idle policy before implementation

User selected option1:15 seconds, pointer outside, only a free card. Pause/reset
eligibility while any nonterminal transfer, prepared source, input/preparation,
command/reconciliation, source/visibility pending state, dialog/popover or owned
secondary editor/result/settings/about window exists. Current minute visibility
alone is not a transfer; hiding never stops its daemon-owned expiry. Empty healthy
cards and completed rows can hide; errors/disconnect remain visible for recovery.
Use one local monotonic one-shot timer, reset on pointer/key/button/scroll input,
visibility changes and blocked->eligible transition. State/progress redraws do not
reset an already eligible idle deadline. Pointer inside prevents scheduling.
The callback rechecks all guards before the existing card-only hide path; no
service polling, cancellation, visibility mutation or synthetic daemon idle state.
Timer is removed on close/hide; no hidden/background tick loop. Tests use explicit
time/deadline inputs and direct eligibility guards rather than waiting15s.

## Implementation and measured verification checkpoint

Card manual result actions are limited to Transfers; automatic completion reveal
is unchanged. Shared measured six-column rows reserve date/action positions for
both live and metadata-only records. Footer owns Refresh/Clear; healthy retention
text is a tooltip and errors remain visible. Movement uses one ancestry predicate
for capture-phase drag and cursor feedback; inert scroll content is allowed,
actual controls/selectable text are excluded. Ordinary device labels are inert.
Approved card-only idle uses one monotonic15s one-shot, guarded at expiry and
cancelled while occupied; redraws do not renew an eligible deadline. Visibility
callbacks defer rearming to the GTK idle boundary to avoid model-borrow reentrancy.

Initial affected check19/0 in0.76s passed. After adding actual row-allocation and
drag-target assertions plus idle guard table, affected panel20/0 in0.56s passed.
Strict lint caught deprecated allocation() in the new test and a helper below the
test module; replace with compute_bounds() and move the helper before the module.
Assertions remain unchanged. These are scoped implementation checks, not complete
Mode B or real KDE drag/idle acceptance. No repeated ten-run development gate.

Final lint repair check: affected widget1/0 in0.54s, strict native lint3.47s.
The action column additionally reserves112 logical pixels even if all records
are metadata-only; SizeGroups still grow with actual controls/theme sizing. This
keeps the empty action slot present instead of collapsing it when the last live
capability disappears. Recheck only the affected widget and native lint for this
small allocation change before packaging; do not rerun unchanged core checks.

Reserved-slot affected widget1/0 in0.70s and strict native lint2.10s passed.
Implementation is split into independently reversible history-layout, movement
and idle-lifecycle commits. Full affected panel20/0 is before the test-only lint
repair/minimum-slot addition; final widget covers those actual column allocations.
No core/BLE changes or full-repository/Mode B claim. All check terminals closed.
Packaging revision28 uniquely identifies the new GUI bytes.

## Verified package and idle upgrade gate

Release package6656952 bytes, SHA-256
db330c61122505b815c38a97c2ed16212568c56dabe690a938af5fd127974ec0,
binary source2c14d26. Locked builder verified inventory14 files, dependencies/ELF/
checksums and offline extracted commands. Exact package Transfers and incoming
previews were visually inspected: no upper guidance/title, footer Refresh/Clear,
aligned data with reserved action space, direct incoming consent retained. Synthetic
rendering does not prove real KDE movement/idle acceptance.

Upgrade pins native27 package/hash and known owners2808363/2808365. Require fresh
global idle/zero-preparation gate before joining only those two units; APT local
authorization then normal launch,14-file readback and one14-entry tray/API checks.
Preserve startup preference and all received files/journal. No diagnostic override,
BLE/protocol change, cloud/signing/public release or master merge.
