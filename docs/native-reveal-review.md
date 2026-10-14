# Native received-file reveal boundary review

2026-10-14. Mode A analysis only for the next slice. Clipboard image ownership and
capture are implemented in native-image.md. No native reveal production API/UI is
implemented by this review. Guide /home/morozhenko/.codex/AGENTS.md sections3/3.1
require selection for a new significant security/API/product boundary.

## Findings

Legacy Tauri reveal_download_item accepts a relative name and resolves it below
the current configured destination, then calls FileManager1.ShowItems or opens the
current folder. Its traversal/canonical checks do not associate the target with a
particular completed transfer, collision remapping or original receive directory.
Native must not turn an advertised peer name or caller-supplied path into opener
authority. Settings may change after receipt; the directory/file entry may move,
disappear, become a symlink or be replaced. Same UID is not sandbox consent.

Native receive already pins a Root directory authority before consent, safely
materializes relative destinations on Accept and holds actual destination handles
while writing. InternalFileInfo contains actual collision-resolved file_url; the
file entry/handle is removed on final chunk. SessionSnapshot/runtime/coordinator
retain advertised details and received Text but no completed actual-file receipt.
Typed transfer details are intentionally insufficient to authorize filesystem IO.

The audit also found premature file EOF could publish Finished. That independent
declared-size bug is repaired under the existing maintenance authorization in
inbound-file-completion.md; it is not a reason to introduce reveal before selection.

## Concrete options requiring selection

1. Recommended: transfer-bound received-file receipts and explicit Show in folder.
   At actual successful completion record bounded private actual root/relative
   destination/fingerprint metadata; derive availability from completed inbound
   Files and current terminal revision. Keep root authority separate from watched
   labels. Request by current transfer/revision/manifest index, never a caller path.
   Resolve only beneath that original root without symlink/magic-link traversal,
   revalidate the recorded identity and acquire a short-lived read-only capability.
   GUI invokes portal OpenDirectory with that FD only after explicit action and
   fresh unlock/epoch checks. No payload/default application execution or URL open.
   Advantages: survives settings changes, uses actual collision-resolved results,
   shares daemon/history ownership and a descriptor-native desktop adapter.
   Costs: new bounded receipt/lease/API/controller lifecycle and negative cases.
   Risks/limits: moved/deleted/changed targets become unavailable; fingerprint
   revalidation is not immutable content attestation. External file-manager
   selection is best effort and not atomic with later filesystem changes. Do not
   retain every completed file FD indefinitely: it can pin deleted disk contents.
   Freeze exact quotas/schema/rollback after selection, before implementation.

2. Current receive-folder action only, explicitly labeled Open receive folder.
   Acquire the current configured directory capability and open that folder on a
   user action; no per-transfer file receipt or promise of locating its file.
   Advantages: less retained state and no completed-file lookup boundary.
   Costs: user searches manually; changing settings can lead to a different folder
   than an older transfer used. Risks: must never present this as showing the actual
   completed file; retains the per-transfer reveal parity gap.

Recommend1 for correct authority and recoverable ownership, independently of work
volume. Both are explicit desktop actions; neither auto-opens received data. This
selection does not authorize a separate Open file/default application feature.

## Proposed option1 stages and verification before code

1. Freeze receipt identity, private root/lease quotas, availability/terminal revision,
   bounded typed API, safe resolution and lifecycle invariants. Rollback docs only.
2. Capture actual receipt before file handles retire; propagate via managed session
   and runtime, bounded coordinator/history lifetime. No advertised-name authority.
3. Same-UID typed query/acquisition and serialized SDK/controller with owned FD
   cleanup; no body/path in watched state, no auto-replay of uncertain desktop action.
4. Explicit row action and owned portal adapter; current unlock/epoch/revision checks,
   bounded pending request/response lifecycle and cancellation. Existing transfer
   Send/Accept and received text remain independent. Rollback GUI then API/receipt.
5. Actual request/kernel/private-broker/isolated GTK/portal checks once; cover
   collisions, nested files, original root renamed/settings changed, symlinks,
   deletion/replacement, incomplete/failed transfers, stale revision/epoch, missing
   portal, lock/close/disconnect, quota/retry and descriptor release. Physical KDE
   file-manager selection is a separate acceptance gate.

## Primary desktop reference

https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.OpenURI.html
inspected2026-10-14: OpenDirectory accepts a file/directory FD and asks the file
browser to show its containing directory, possibly selecting it. It uses
FileManager1.ShowItems and falls back to opening the directory when that service is
absent. It is distinct from OpenFile/default application execution. The separate
freedesktop wiki fetch returned403; do not claim that page was inspected.

## Handoff

Await user selection1/2 before new reveal production code. Existing image/EOF work
and exact measured checks are recorded in their own SSOT. No real desktop portal,
user files/clipboard/config, install, push, branch or physical peer operation here.
