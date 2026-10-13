# Native desktop context-menu ingress ownership review

2026-10-12 review; 2026-10-13 decision. Mode A. Option 1 selected.

The user delegated selection to architectural quality and explicitly requires
recommendations to prioritize quality over implementation volume or speed.
Select option 1 because it preserves a single preparation/operation owner without
adding daemon authority-transfer transitions. Implementation SSOT:
[native-desktop-ingress.md](native-desktop-ingress.md). Pending-choice language
below records the review history, not the current authorization state.

## 2026-10-13 implementation checkpoint

Option 1 is implemented and verified as recorded in native-desktop-ingress.md.
CLI local acquisition, Desktop1 FD import, shared gate, recipient presentation and
package-owned Dolphin source assets exist. Live installation, native notifications
and visible nonterminal errors remain release gates. The review below preserves
its earlier analysis; it is not a pending ingress decision.

## Historical state at review

The uninstalled GTK tray shell is implemented in native_desktop and consumes
PanelController. Its own chooser acquires sources locally and the controller's
exclusive daemon connection creates/sends selections. Manager2 and Operations1
are implemented; no old native clients or Manager1 compatibility layer exists.
The CLI does not yet implement `send -- <path-or-file-uri>...`. No Dolphin entry
is installed. Source capability acquisition, strict batches, exact replay and
connection-owned preparations already exist; reuse them rather than path reopening.

## Why a user decision is required

The accepted descriptor boundary in native-desktop-design.md forbids daemon path
opening. native-dbus-selection-review.md explicitly says: "A CLI-to-other-client
authority handoff is not granted by this proposal." A committed selection is
owned by its creating daemon-bus connection, so passing its token to GTK cannot
authorize Send. Historical path-activation wording in the alternative-A section
is not a grant to bypass the accepted capability boundary.

The remaining choice changes a public IPC contract and ownership/cleanup rules.
AGENTS.md sections 3/3.1 require stopping for that decision. It is not a tooling,
naming or small internal refactor question. Keep the finished shell reviewable and
commit it before asking; do not implement either ownership alternative meanwhile.

## Alternatives

| Option | Contract / description | Advantages | Costs / risks |
|---|---|---|---|
| 1 — acquire in CLI, import capabilities into GTK (recommended) | CLI resolves local roots and acquires read-only files; bounded staged GUI IPC transfers FD/name batches. GTK validates/adopts them and uses its own existing controller connection to create the daemon selection. CLI exits only after the GUI import receipt. Recipient choice and explicit Send remain in the panel. | Preserves daemon selection ownership and Manager2/Operations1; no daemon adoption API or privileged path-opening deputy; file-manager source authority stays with its ingress process. | Requires a new narrow authenticated GUI import endpoint, bounded staging/replay/cleanup and cold-start lifecycle. Interrupted import must not duplicate preparation. GTK must have adopted resources before CLI exit. Same UID is not sandbox consent. |
| 2 — prepare in CLI, transfer daemon selection ownership to GTK | CLI creates the selection on its daemon connection, then an explicit atomic service operation adopts it onto the GUI's verified daemon connection. GTK selects the recipient and sends. | Existing CLI preparation/batching runs directly against the daemon; GUI receives a selection reference after verified adoption. | Changes daemon ownership and ordered mutation contract; introduces adoption replay, target identity, both-client disappearance and ambiguous-reply rules. A bare token is insufficient. More service state/authority transitions and rollback surface. |

Recommendation: option 1. The GUI already owns the pending-selection connection,
and descriptors can cross an ingress boundary without granting authority over
another daemon client's resources. The extra GUI import protocol is isolated from
the stable service ledger. Option 2 is viable but adds service lifecycle complexity
without a current need to share prepared selections between independent clients.

Neither option sends on invocation, evaluates shell text, downloads remote URLs,
reopens caller paths in the daemon, deletes originals or changes Google identity.
Use the existing panel recipient flow. Busy/prepared-selection behavior, exact
wire signatures/limits, sender checks, recovery and startup are frozen in an
implementation plan after this direction is selected, before production code.

## Next authorized work after selection

1. Record the selected option, freeze the bounded ingress/ownership invariants,
   introspection and scoped risk/test inventory before code.
2. Implement local path/file-URI parsing and capability acquisition, Unicode/
   percent/space handling, complete-selection validation, explicit visible errors.
3. Implement the selected handoff with actual private-broker/controller checks for
   ownership, limits, interrupted replies, disconnect and no implicit Send.
4. Only after the actual CLI/chooser flow works, add package-owned Dolphin assets
   using local protocols and separate arguments, with uninstall/rollback. Do not
   install into the live user session during offline development.

Independent remaining product gates stay in product-direction.md. No promise of
complete Google compatibility from these local IPC or GUI checks.
