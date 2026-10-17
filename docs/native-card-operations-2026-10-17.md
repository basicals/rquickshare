# Native card layout, Send flow and device-operation grouping

2026-10-17. MODE A (feature/refactor), followed by scoped MODE B (testing).
The user selected **option B: group completed operations by device**, with
unlinked operations displayed separately. This is an explicitly requested UX
change. All implementation is on `dev`; do not promote to `master`, install,
sign, publish or alter the currently running native21 candidate as part of this
work. Use the supplied universal agent guide (analysis, documentation-first,
explicit architectural STOP FACTOR, and MODE B inventory/coverage gates).

## Repository analysis and verified limitations

- The single GTK card is built by `native_desktop/ui/panel.ui` and
  `native_desktop/src/panel/{rows,menu,movement}.rs`. The four-action bottom
  toolbar already follows the drawer. `nearby-scroll` has a fixed 300 logical
  pixel maximum; the root clips overflow and `resize_to_content` measures at
  render time only. Expander toggles do not explicitly refresh the window's
  natural height. A keyed transfer row is recreated on phase/details changes,
  losing its previous expanded state.
- The Send action calls `show_sending`, which opens the device drawer but also
  opens the payload-type popover when no source is prepared. The user requests
  device discovery/recipient selection first, then explicit source choice.
- `StateView.endpoints` contains ephemeral discovery endpoint ID/name.
  `GetRecipientPage` contains **transfer ID and advertised recipient name only**.
  It does not return the recipient endpoint ID or verified durable identity.
  Inbound transfer sender names are untrusted display metadata. Neither unique
  names nor icon/device types prove that two rows represent the same peer.
- Current visible rows use separate keys for endpoint and transfer. Terminal
  rows with text/file capabilities bypass the five-recent-row display filter.
  The core's eligible current-run retention limit is 64; it is **not** durable
  history and must not be changed merely to clean up the presentation.
- A finished transfer can have `DetailsStatus::Unsupported`. Its completed
  phase is authoritative about operation state, not recoverable text content.
  Review/Copy must stay enabled only for a live content capability.

## Frozen user-facing behavior

1. Keep the four-action toolbar at the bottom. Expanding/collapsing devices or
   details grows/shrinks the card **upward** at the same pinned lower edge. Keep
   a screen-bounded scroll region when content exceeds available space.
   Never clip consent buttons, warnings, recovery controls or the toolbar.
2. Details buttons must neither overlap neighboring controls nor lose the
   expanded state during a background progress/phase refresh.
3. Send exposes currently discoverable recipients before the source-choice
   menu. Existing explicit send/consent and one-to-ten-recipient admission
   remain unchanged. No implicit send, automatic accept or paired status.
4. Completed operations appear under the **same device row only with an
   authoritative, non-name-based peer relationship**. An inbound transfer
   with only an advertised name, or a transfer with no verified relationship,
   stays in an explicitly unlinked section. Do not combine by equal names;
   do not infer trusted pairing from discovery or transfer success.
   Completed rows can collapse without invalidating in-progress work or the
   receive-text/reveal capabilities. Received files/text are not deleted.
5. Show an honest terminal status separate from content-preview availability.
   Unsupported or expired content must not masquerade as an active consent
   prompt; no automatic clipboard replacement or opening received URLs.

## Architectural decision required before full B grouping

Current `GetRecipientPage` supplies only the recipient's display name. There
is **no safe existing stable peer key** exposed to the GTK client. The user
selected grouping behavior, but not the required public metadata extension.

- **A: no new wire/API (partial B only).** Group only where a verified key
  already exists; leave all unmatched transfers unlinked. Minimal risk and
  no wire compatibility work; this does not provide useful grouping for
  the current outbound transfer projection.
- **B: additive daemon-owned endpoint association (recommended).** Expose
  an optional, same-epoch endpoint ID from the authoritative sending runtime
  through a versioned/optional typed read API, retain exactly specified
  privacy and invalidation semantics, and group outgoing operations only
  while the exact endpoint association is provable. Inbound without verified
  binding remains unlinked. Requires an explicit API/contract choice and
  core, SDK, GTK and negative tests.
- **C: persistent device identity/trust.** A new durable identity and
  verification lifecycle would permit grouping across reconnects, but adds
  credentials, pairing/security, migrations and privacy requirements.
  This exceeds the current ordinary-sharing scope.

**STOP FACTOR:** do not change public D-Bus API or infer identity from the
display name before the user selects a safe association strategy. Independent
layout, Send flow and honest terminal-state repairs do not require this change.

## Ordered plan, rollback and status

- [x] A1: inspect SSOT, native GTK, core recipient projection, current tests
  and CI. Record the user's B selection and API limitation before editing code.
- [ ] A2: repair GTK expansion, height measurement and upward anchor; add
  bounded-height and state-preservation regressions.
- [ ] A3: make Send enter explicit device selection before source choice, keeping
  the existing source flows available and tested.
- [ ] A4: repair terminal text/status projection and safely separate unlinked
  operations; implement connected grouping only after the peer-association
  decision, otherwise mark the blocked portion OPEN.
- [ ] B1: complete and update affected artifact/branch inventories,
  Positive/N1-N12 and interaction/coverage matrices **before** adding tests.
- [ ] B2: implement complete planned contract/widget assertions; run native GTK
  test suite, strict Rust format/Clippy and whole affected CI when possible.
- [ ] B3: complete ten consecutive successful runs and record measured coverage
  for the affected production files, or mark MODE B completion gate OPEN.
- [ ] A5: update current-state/evidence with exact commits, CI URLs and measured
  results; verify dev/master divergence; do not merge or publish.

Rollback: discrete documentation, layout, Send, model/API (if approved), and
test commits. Preserve existing public D-Bus semantics and core source/session
ownership until separately approved. GTK pixel/UI acceptance requires a live
KDE/Wayland native21-or-later package; offline compilation is not hardware proof.

## Scoped MODE B plan (pending implementation)

| Artifact | Risk | Level | Required branches/interactions | Negative categories |
|---|---|---|---|---|
| GTK transfer expander + keyed row | P1 | GTK integration | expand/collapse, phase/progress replacement, incoming/finished/rejected and long/untrusted labels, button accessibility/focus | Positive, N1, N2, N5, N9, N10 |
| Window height/bottom anchor | P0 | GTK + pure geometry | short/long, empty/one/many rows, resized and clipped monitor, changed visibility, repeated expand/collapse | Positive, N2, N5, N8, N9 |
| Send recipient-first flow | P1 | GTK integration | no source vs prepared source, no peers, many peers, vanished selected ID, disconnected and busy | Positive, N2, N3, N4, N5, N7, N9 |
| Device/transfer grouping | P0 | Core+GTK contract | exact proven ID match vs name-only/unrelated ID, vanished discovery, session epoch change, overlapping transfers, consent and text retained | Positive, N1, N2, N4, N5, N6, N7, N10 |
| Terminal content projection | P1 | Unit+GTK | finished text with/without content capability, unavailable/malformed details, failed/cancelled/partial and reveal | Positive, N1, N5, N6, N9, N10, N12 |

N11 (monetization) is not applicable to this project. Storage N12 applies to
transfer/receive preservation but not pure row layout; no filesystem mutation
is introduced. Network N3 applies to the send/peer path, not GTK geometry.
The original `docs/mode-b-inventory.md` and `docs/mode-b-test-plan.md`
remain the repository-wide inventories and are not superseded by this scope.

**Execution/coverage at plan freeze:** no tests or package builds have been run
for these changes; CI was green on the pre-change dev/master shared head.
Physical peer, timeout/disconnect and live card screenshots are pending.
