# Native outgoing text / clipboard ownership review

2026-10-13. Mode A. Direct dev checkout. Guide /home/morozhenko/.codex/AGENTS.md
sections 3.1,11,13. Status: user authorized continuation after the concrete recommendation; implement A. This is an
analysis/proposal, not implemented source or a new compatibility claim.

## Remaining plan and priority

Received Text/URL R1-R4, settings S1-S6 and tray T1-T3 are implemented in source.
Continue the native migration with outgoing text/clipboard, then clipboard images,
received folder/file reveal, autostart/update/package lifecycle and installed
migration acceptance. Do not retire Tauri before package/feature/physical gates.
Protocol/parity remains: custom name, temporary visibility, trustworthy identity,
previews/resume/streams/multi-recipient/Wi-Fi/app payloads, Google Account and QR,
and actual Android/Windows/KDE matrices. See full-feature-roadmap.md for details;
core/legacy implementation markers are not native GTK/Google acceptance.

## Source analysis

- service.rs Selection, Reservation, Transfer.resource/source and SendDispatch
  are Arc/Weak<OpenedSelection>. Their lifetime proves resource accounting remains
  reserved until the actual core reader releases ownership, beyond queue removal.
- service/runtime.rs dispatch always constructs OutboundPayload::OpenedFiles.
- service/operations.rs uses one bounded connection-local ordered receipt ledger
  for Begin/Commit/Abort/Send. Acknowledged retired sequences cannot create new work.
- service/uploads.rs stages descriptor-based files before atomic CommitSelection.
- NativeClient and PanelController own one uncertain-operation/preparation ledger;
  GTK/CLI gates prevent concurrent source replacement. Prepared selection only
  enables recipient choice; Send is separate and explicit.
- hdl/outbound.rs already supports Text, URL/phone classification, byte framing,
  peer consent and cancellation. Empty text is rejected. Legacy clipboard image
  support creates a managed temporary PNG; that is not native clipboard ownership.
- Native bus transport limits a frame to 64KiB. A 128KiB text cannot be sent as one
  unbounded string call. Reuse bounded staged ingestion and ordered acknowledgements;
  UI capture/privacy rules alone would not solve this transport constraint.

## Options requiring selection

| Option | Description | Advantages | Costs and risks |
|---|---|---|---|
| 1 / A — unified prepared source (recommended) | Generalize the existing source capability to Files/Text, preserve one owner, queue, resource reservation, ordered preparation/Send, cancel/recovery and UI recipient flow. Text stages bounded bytes, never a fake file. | Single lifecycle/invariant implementation; CLI/files/text cannot silently replace each other; quota and late-release ownership stay explicit. Future images use the file boundary. | Substantial refactor of tested file-source/coordinator/runtime/SDK code and native API extension; regression protection must prove existing file/FD behavior and real reader lifetime. |
| 2 / B — separate text preparation/dispatch domain | Keep file capability model and add separate text resources, receipts and preparation/queue controls feeding the existing core transport owner. | Less direct change to the file-source structures; independent removal of text-specific code. | Two preparation/recovery/quota paths plus coordination gates; overlapping file/text/CLI actions can diverge, steal the source slot or release accounting early. Later unification duplicates work. |

Recommend 1/A for explicit ownership, predictable recovery and maintainability,
not implementation speed. No second RQS/transport owner under either option.
The accepted received-text Copy policy does not authorize this significant change
to source lifecycle and public native preparation contracts. Await selection before
production code. The user asked to ask before substantial refactoring; do not hide
this change as a button-only addition or infer selection from general continuation.

Implementation SSOT: [native-outbound.md](native-outbound.md). Historical proposal follows.

## Proposed implementation milestones if A is selected

1. Document exact typed PreparedSource contract and rollback. Keep legacy file/DTO
   interfaces compatible. Carry the SAME source capability through the managed core
   request until its actual reader/send owner exits; extracting a nested file Arc
   and dropping the outer reservation capability would incorrectly release quota.
2. Generalize coordinator reservations/selections/dispatch to typed Files/Text.
   Preserve file descriptors, folder naming, owner checks, queue/transfer ceilings,
   source release, terminal retention and idle restart. Text bytes are immutable,
   bounded and redacted; no arbitrary paths, synthetic file or deletion authority.
3. Freeze staged text ingestion and captured selection-summary/API contracts before
   coding. Proposal: typed text Begin with expected byte count, <=16KiB byte appends,
   exact sequence/replay checks, then existing atomic Commit/Abort/Send ownership.
   No FDs for text routes, all bodies within current frame limits. Unknown/mixed upload
   kinds fail closed. Retain bounded fingerprints/receipts, not copied private bodies
   in operation history. Define aggregate text memory admission across every stage.
4. Extend the SDK/controller's one preparation/uncertain-operation ledger. Dropped
   waits preserve exact explicit recovery; reconnect releases unsent caller sources
   and never recaptures clipboard or implicitly sends. Ordinary PanelState/snapshots
   contain kind/size/readiness only, no text body/title. File CLI uses existing gate.
5. Freeze concrete GTK send presentation before its code: explicit clipboard capture,
   protected plain preview, preparation and recipient choice; Send is a separate
   action. No background clipboard subscription/read/copy or automatic URL launch.
   Preserve an existing preparation and current settings/read recovery. Test only
   isolated clipboard/display; no live user clipboard during development.
6. Actual focused core/SDK/GTK checks once, atomic commits and SSOT/exports. New
   failures/changes justify only affected reruns. Real Google/KDE acceptance stays
   separate. Clipboard images follow after their encoding/memory/FD ownership review;
   do not silently reuse legacy path-taking ephemeral cleanup in native code.

## Risk/invariant verification plan before implementation

Cover typed source accounting and actual reader release, descriptor/file/folder
compatibility, owner/epoch/upload-kind mismatch, byte/chunk/count boundaries,
partial staging/cancellation/storage-independent text, exact duplicate versus body
substitution, lost Begin/Append/Commit/Send replies, disconnect/restart and CLI/text
competition. Use actual coordinator/managed core and private brokers, not mocked
source/queue logic. Protect private Debug/state/signal/notice redaction and no implicit
clipboard capture/Send. Expand the exact scoped branch inventory in the implementation
SSOT before writing supporting tests; no whole-product Mode B completion claim.

## Rollback and handoff

No production changes/tests/devices/clipboard operations in this review. Revert this
documentation commit to remove the proposal. Proposed implementation rollback:
GUI/SDK consumers, then wire/ledger and typed runtime/coordinator, then core source
support; keep file-owned descriptor authority and legacy behavior throughout.
Current implementation checkpoints remain c75b490/3e135f4/43e2398/8f1fd73.
All review shell commands completed. Terminal closed. No branch/push/install.
