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
- [x] A2: repair GTK expansion, height measurement and upward anchor; add
  bounded-height and state-preservation regressions.
- [x] A3: make Send enter explicit device selection before source choice, keeping
  the existing source flows available and tested.
- [ ] A4: repair terminal text/status projection and safely separate unlinked
  operations; implement connected grouping only after the peer-association
  decision, otherwise mark the blocked portion OPEN.
- [x] B1: complete and update affected artifact/branch inventories,
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

## MODE B expansion: affected-artifact inventory before adding tests

The following covers the production files touched by this slice. It does not
replace the whole-repository MODE B inventory or claim complete release signoff.

| Production artifact | Testable rules and branches | Risk | Level | Planned direct cases |
|---|---|---|---|---:|
| `panel/rows.rs` | unchanged-key row reuse, changed-key replacement, expansion restored true/false, incoming default, terminal/active partition, discovery-key isolation, empty/device/active/recent ordering, unlinked-row disclosure, row control sensitivity and update, GTK title ellipsis | P0/P1 | GTK widget and pure | 13 |
| `panel.rs` | monitor present/absent and short/tall cap, content growth/reduction, terminal unsupported vs ready, nonterminal unsupported consent remains blocked, content/reveal capability retention, device drawer visible/hidden and bottom bar visibility | P0/P1 | GTK widget + geometry | 11 |
| `panel/menu.rs` | Send idle/no payload, Send prepared payload, discovered/no devices, picked device/no source, picked device/prepared source, stale/unknown endpoint, busy/disconnected, repeated send never starts a transfer | P1 | GTK/action | 10 |
| `panel/movement.rs` (unchanged contract) | bottom margin invariant with expanded/collapsed content, too-small display and overflow-reject, moved/pinned/restore | P0 | geometry/unit | 7 existing + interaction extensions |
| `panel/presentation.rs` (unchanged contract) | consent remains explicit and survives card Hide and Expand; no side effects from layout | P0 | GTK/domain contract | existing + 2 interactions |
| `service/client/state.rs` (unchanged contract) | endpoint name and transfer recipient name are different untrusted fields, reject malformed/invalid rows; no phantom identity binding | P0 | protocol/contract | existing + association gate pending API decision |

### Executable branch map and interactions

- `same_description`: equality for all retained metadata, inequality for each
  changed identifying/status/content field; progress-only updates must update
  their label without rebuilding the row.
- `cached_transfer`: existing row unchanged; existing row changed while
  expanded or collapsed; unseen incoming default expanded; unseen outgoing
  default collapsed; restored state must survive phase and sender updates.
- `render_rows`: 0/1/many endpoints crossed with 0/active/terminal transfers;
  active consent is never hidden in Recent; terminal with readable text or file
  capability remains accessible behind one explicit expander; unlinked transfer
  must not attach to same-name endpoint.
- `resize_to_content`: monitor unknown vs known; height 0/1/below-reserve/
  above-reserve, high DPI logical units and content below/at/above scroll cap;
  pinned bottom does not move when the upper panel height changes.
- `show_sending` / `send_to`: no source vs prepared source crossed with
  endpoint valid/vanished and busy/idle. Recipient choice precedes content
  picker and no action automatically admits a transfer.
- `transfer_card`: terminal unsupported vs ready, finished vs failed or
  cancelled, inbound vs outbound; content review only on fresh content revision.

### Positive and negative matrix

Legend: P = planned targeted check, E = existing regression to retain,
NA = not applicable to the pure UI slice. All cells are explicit.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Rows / grouping | P | P(long names) | P(0/many) | NA(no network) | P(reorder) | P(phase race) | P(stale state) | P(no name association) | NA(no clock) | P(collapse) | P(no forged identity) | NA(no billing) | NA(no write) |
| Window geometry | P | P(invalid dims) | P(screen bounds) | NA(no network) | P(repeat resize) | P(rapid toggle) | NA(no schema) | NA(no auth) | P(logical scaling) | P(hide/reopen) | NA(no PII) | NA(no billing) | NA(no storage) |
| Send flow | P | P(invalid ID) | P(0/10 peers) | E(disconnect) | P(repeated click) | P(disappear) | P(stale selection) | E(explicit consent) | NA(no clock) | P(cancel chooser) | E(no payload log) | NA(no billing) | E(source failure) |
| Terminal details | P | P(unsupported) | P(long text) | E(loss) | NA(read only) | P(phase change) | P(expired revision) | E(capability) | NA(no clock) | P(review denied) | P(no implicit clipboard) | NA(no billing) | E(partial file retention) |

Important mutations: drop the phase/sender equality guard, replace a selected
endpoint ID with its display name, force every expander closed on refresh, omit
`pin_surface_bottom`, increase scroller minimum above viewport, open source
popover before a peer is chosen, or represent Finished+Unsupported as a pending
consent prompt. At least one specific assertion should fail for each mutation.

Coverage map (required per affected production file):

| File | Functions | Branch evidence before this change | Remaining |
|---|---|---|---|
| `native_desktop/src/panel/rows.rs` | `same_description`, `transfer_row`, `cached_transfer`, `render_rows`, `arrange` | badge and keyed-list smoke | explicit changed projection, expanded-state, section split, all boundaries |
| `native_desktop/src/panel.rs` | `resize_to_content`, `transfer_card`, `details_card_mode`, `render` | full-card smoke, static 300px cap | dynamic monitor bounds and terminal state labeling |
| `native_desktop/src/panel/menu.rs` | `show_sending`, `send_to`, `set_drawer` | tray action/status smoke | recipient-first positive and no-source/unknown-peer negative |
| `native_desktop/src/panel/movement.rs` | `pin_surface_bottom`, `clamp_moved_surface`, `bottom_margin` | pure 720/1080/2160 geometry tests | actual GTK expanded/hide/reopen geometry |
| `native_desktop/src/panel/presentation.rs` | `apply_presentation`, `hide` | safety, reset and notice tests | explicit interplay with nested Recent |
| `core_lib/src/service/client/state.rs` | `capture_state`, `transfer_view`, recipient page | typed and bounded read checks | authenticated endpoint association (blocked choice) |

Expected count in the new targeted slice: around 34 explicit positive/negative
scenario assertions plus existing GTK, geometry and core regressions; adjust the
actual count to the implemented inventory, not the other way around. Do not
claim 100% branches, 10 successful runs or exact coverage until measured.

## Incremental source checkpoint (2026-10-17; verification pending)

- `panel/rows.rs`: expanding transfer rows now pins before toggling and
  remeasures after GTK idle allocation; phase/status replacement recovers the
  previous expander state. Header requests receive a fixed right-aligned review
  control. The GTK Expander is configured to resize its top-level window.
- `panel.rs`: nearby scroll height adapts to the current monitor (120..760
  logical pixels after reserving 220), and finished/failed transfers with
  unsupported metadata display a neutral explanation rather than the
  inappropriate pre-consent warning. Optional `RQUICKSHARE_UI_DEBUG=1` prints
  numeric geometry, scroll maximum and placement state only, never payloads,
  filenames, advertised names or PINs.
- `panel/menu.rs`: Send first opens the nearby recipient list; choosing a
  current endpoint with no prepared source opens the existing source menu.
  No automatic transmission, discovery identity or entitlement is implied.
- `panel/rows.rs`: current transfers, discovered endpoints, and unlinked
  completed activity are separate. Completed entries are nested in a collapsed
  `Recent activity · device not verified` expander and remain available for
  explicit text/file review. This is the safe fallback for option B, **not**
  verified grouping by device. Endpoint association remains blocked by the
  documented API decision.
- `panel/tests.rs` and `rows.rs`: added assertions for scroll cap, Send
  device-first flow, terminal unsupported metadata, and expander state. The
  isolated native GTK CI now exists on dev and runs the GTK suite ten times.
  Whole-repository Mode B coverage and native installed acceptance remain OPEN.


## Selected architecture: B now, migration path to C (2026-10-17)

The user explicitly approved **B with a future migration to C**. This lifts the
prior API STOP FACTOR for the scoped, read-only endpoint association only.
No persistent peer identity, account, pairing, trust, auto-consent, migration
or new credentials are approved by this decision.

### Root cause and identity semantics

Outbound coordinator transfers use `endpoint: Option<String>` for queued
dispatch, which is explicitly cleared when dispatch takes ownership or fails.
That mutable field cannot be used as historical identity. Inbound requests
currently have only unverified advertised sender name/type. A separate,
immutable `associated_endpoint: Option<String>` is attached on outbound
admission and retained until that transfer record is released from current-run
history. It is **ephemeral**, scoped to its coordinator epoch, not an
authenticated or persisted device identity, and must never be used to authorize
sending or acceptance. Neither name equality nor reuse across daemon epochs
creates an identity. Preserve cancellation, dispatch and retention semantics.

Expose optional transfer association via a new additive
`Batch1.GetTransferEndpointPage(snapshot, offset) -> (next, a(ss))`.
Rows are ordered exactly with existing `GetTransferPage`: transfer ID and the
admitted endpoint ID or empty string; only outbound has an association.
This remains an authenticated, bounded, immutable **capture** owned by a bus
client; no added payload access, no credentials. Old readers need not call the
new method; old writers continue using existing Send/GetRecipientPage.
For a new reader talking to an older service, a missing optional method must
degrade to *no association*, never to name-based guessing. Any malformed
response, divergent transfer ID/order, wrong epoch, invalid endpoint or
non-empty inbound association must fail closed (protocol error), not silently
group untrusted state. Snapshot retention budget must account for its rows.

GTK nests terminal outbound entries under the exact currently discovered
endpoint row only when `associated_endpoint` equals an endpoint ID in the
same synchronized state snapshot. All other transfers remain in a collapsible
unlinked activity section. Grouping is presentation only: daemon transfer IDs,
consent, cancel, results, source and file/text capabilities remain independent.
Device groups collapse while retaining expanded transfer controls/focus when
possible. Disappearance/reappearance of a peer does not confer trust.

**Migration to C:** introduce an independently verified, stable
`device_identity` as a *new optional, separately typed association* in a future
versioned API. Map current ephemeral endpoints to that identity only after
verified device ownership/trust; never rewrite, persist or promote
`associated_endpoint` into a durable key. Gate security/privacy decisions,
consent, credential storage/migration, revocation and any auto-accept by a
separate user-approved ADR. No implicit pairing claims in UI.

### Implementation stages and rollback

- [x] C0: inspect Coordinator dispatch clearing, Batch1 snapshot/SDK,
  GTK keyed rows, existing CI and explicit compatibility requirements.
  Freeze the B/C split and regression inventory before production edits.
- [x] C1: retain immutable ephemeral association in core; project additive
  bounded, authenticated and snapshot-frozen Batch1 rows; update XML.
- [x] C2: SDK optional read/strict validation, compatible absence on old
  daemon only; never use untrusted display names as fallback.
- [x] C3: GTK nested, collapsible completed rows under matching live endpoint,
  unlinked fallback for stale/inbound/absent; preserve independent actions.
- [ ] C4: regression suite on coordinator, capture, bus/SDK and GTK;
  strict format/Clippy/CI, ten runs and coverage evidence.
- [ ] C5: report exact head, execution and remaining hardware gates.
  No master promotion, install or release publication.

Each stage is committed independently and reverted from presentation to SDK
to read API to coordinator retention. Retained records and files survive the
display change; no schema on disk is introduced.

### B-to-C Mode B delta inventory (planned before tests)

| Production file | Branch map / interactions | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `core_lib/src/service.rs` | outbound one/multi sends, release before/after terminal, retry, cancel, inbound, epoch and retention, mutating dispatch endpoint never erases association | P | P | P | NA | P | P | P | P | NA | P | P | NA | NA |
| `core_lib/src/service/bus/state.rs` | empty/1/256 pages, immutable capture after changes, ordered paired rows, memory charging, malicious/outbound alias and read auth | P | P | P | NA | P | P | P | P | NA | P | P | NA | NA |
| `core_lib/src/service/bus.rs` + XML | new method, unknown method, invalid signature/offset/foreign snapshot, exact bounded page/FD refusal and existing methods unchanged | P | P | P | NA | P | P | P | P | NA | P | P | NA | NA |
| `core_lib/src/service/client/state.rs` | old daemon/missing optional method, 0/one/many, wrong ID/order, stale epoch, inbound non-empty, unknown live endpoint, correct cap | P | P | P | P | P | P | P | P | NA | P | P | NA | NA |
| `native_desktop/src/panel/rows.rs` | matching live endpoint vs stale/absent, same-name different ID, active+terminal, multiple recipients, collapse, refresh/reconnect, text capability retention | P | P | P | NA | P | P | P | P | NA | P | P | NA | NA |

Tests must include at least: a shared display name with distinct endpoint IDs;
outbound terminal after `next_send` clears dispatch authority; inbound sender
with the same name remains unlinked; stale/missing endpoint falls back; a
same-epoch confirmed match displays once; all receipts/consent and cancel remain
unmodified; optional older daemon is handled without fake association.
Measure production-file coverage and verify the mutation variants before
claiming MODE B complete. Physical KDE/Android/Windows acceptance is separate.

### Implemented B-to-C checkpoint before final execution

Core `Transfer.associated_endpoint` is a distinct clone of the admitted
outbound endpoint; `Transfer.endpoint` remains mutable dispatch authority.
Inbound associations remain empty. Retained terminal records preserve the
association until the original current-run history eviction; no durable
identity, pairing or new acceptance behavior. `StateCache` freezes one
association row per transfer with the existing snapshot, and charges owned
copies against its bounded byte reservation. Authenticated Batch1 adds the
read-only `GetTransferEndpointPage`, leaving existing methods/signatures
unchanged. SDK validates matching transfer IDs, direction and safe byte
limits. UnknownMethod/UnknownInterface from older daemons maps only to empty
grouping, never to a guessed name. Error responses other than
`Incompatible` still fail the state read.

GTK groups terminal outbound rows under a currently discovered endpoint
with the exact same ID in the same state snapshot. Other operations remain
in Recent activity; the discovered device itself is not labeled paired.
The renderer retains row content/capabilities while reparenting between
GTK containers. A missing/changed association and incoming requests do
not silently match display names.

Focused regressions added to Batch core ownership, bus state page bounds,
SDK real-bus batch reads, SDK inbound/outbound and GTK strict-grouping/
reparenting. Current-stage verification is **not yet signed off**:
the previous new GTK CI first failed rustfmt (fixed), then ran a genuine
preview decoder fixture without its helper binary (CI fixed to build it).
Fresh exact-head compilation, ten-repeat suite, per-file coverage, physical
KDE/Android/Windows acceptance and MODE B Completion Gate remain OPEN until
observed evidence is recorded. The UI workflow is intentionally independent
of release signing and installed package changes.
