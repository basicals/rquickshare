# Native clipboard image source review

2026-10-14 status: user selected1/A; source/accounting/capture/review implementation
is complete in [native-image.md](native-image.md). The review and primitive-only
observations below are historical; use that SSOT for actual PNG/SDK/GTK verification.

2026-10-13. Mode A. Work on dev. Status: user selected1/A on2026-10-14. Implementation SSOT: native-image.md.
The proposal below is historical. Guide /home/morozhenko/.codex/AGENTS.md sections3.1/11: significant new
storage/resource/security boundary requires explicit selection. This review follows
completed native-outbound.md O1-O4; it does not reopen accepted text decisions.

## Analysis and actual local probe

Native now has one Files/Text preparation/queue/recovery capability. Images should
reuse Files and existing no-path, read-only descriptor ingestion, recipient choice
and explicit Send. Legacy Tauri clipboard_image.rs allocates named temporary PNGs;
its path-based ManagedEphemeralFile deletion authority must not become native IPC.
The legacy image pixel limit is64million, with RGBA length checks AFTER backend
read; native capture must bound encoded input and decoded dimensions before layout.
GdkClipboard.read_texture may already allocate before the client checks dimensions.
Use explicit bounded PNG MIME stream acquisition instead of automatic texture
capture; PNG decoding/preview/encoding and memory budgets require a separate source
plan before code. No automatic fallback from failed text paste into image sending.

Actual kernel probe on2026-10-13 succeeded: os.memfd_create with CLOEXEC and
ALLOW_SEALING, bytes written, WRITE/GROW/SHRINK/SEAL applied, existing owned handle
reopened read-only via /proc/self/fd while writer was held, seals/read-only flags
verified. Original writer pwrite/truncate rejected with EPERM. Read-only descriptor
still returned exact fixture bytes after writer close. All descriptors closed,
no named file created. Terminal closed. This proves kernel primitives only, not
PNG validity, GTK image capture, SDK transfer or Google/device compatibility.

## Concrete storage options

| Option | Behavior | Advantages | Costs / risks |
|---|---|---|---|
| 1 / A — anonymous sealed source (recommended) | Capture on explicit user action, validate/bound image, create immutable anonymous Linux file, pass read-only FD through shared preparation. Kernel releases it with final owner. | No private image path/delete API, no files left after crash, original captured bytes stay stable until actual send owner releases them. Fits existing descriptor architecture. | RAM-backed bytes need a separate aggregate budget across preparing/committed/active owners and kernel-derived accounting; large images can be rejected explicitly. Encoding/decoding must be bounded and owned. |
| 2 / B — private disk spool | Create image in a private daemon/GUI-owned spool, retain descriptor through send, remove through private owned cleanup and restart scavenging. | Supports larger captures with lower RAM retention; possible future persistent recovery. | Private image remains on disk until cleanup; requires storage quota/failure/crash cleanup and lifecycle design. Must never accept caller-provided deletion paths or turn ordinary source names into authority. |

Recommend1/A for stable ownership, privacy and crash cleanup, independent of effort.
Do not treat the existing huge selected-file byte quota as a RAM budget. Either
option must preserve one source/recipient flow and separate actual Send. The prior
unified source choice did not decide image storage or memory-backed FD accounting.
No image production code is written before selection.

## Proposed A stages after selection

1. Freeze source/acquisition/validation/budget invariants and rollback. Starting
   proposal: encoded PNG<=32MiB, dimensions<=existing64million pixels before GTK
   decode, aggregate anonymous-source reservation128MiB retained through actual
   reader lifetime. No silent resizing/truncation. Budgets provisional, not Google
   limits. Bound the decoder's own allocation as well as decoded pixel product.
2. Implement anonymous sealed/read-only source creation with explicit ownership,
   no named files or deletion capability. Derive memory-backed resource charge from
   actual FD/kernel properties, not untrusted caller metadata; include every staged
   and actual core-held resource until last reader releases it. Record any impact
   on existing native FD admission/public resource contracts before implementation.
3. Explicit GTK image capture/review uses current unlock/epoch and shared gate,
   bounded input stream, owned cancellable acquisition/worker lifecycle. Source
   preparation and recipient selection reuse existing file controller; no Send on
   capture/Prepare. Close/lock/reconnect dispose private preview/capture references.
4. Actual source immutability/accounting/abort/disconnect/private-broker/UI checks
   once; no live clipboard/peer operation needed. Commit atomic stages and export
   exact source. Physical Google clipboard-image compatibility remains acceptance.

Under B, replace stage2 with bounded private spool/FD acquisition, original-path
independence, owned unlink plus orphan reconciliation, injected storage and error
handling. These extra persistent-resource contracts require documentation first.

## Handoff

Text source plan is complete: native-outbound.md, commits56c0dd6/49a409d/fd4b808/
8bbea57. Current remaining priority: clipboard images, received reveal/open,
autostart/update/package lifecycle, protocol/name/visibility/trust/previews/resume/
streams/multi-recipient/Wi-Fi/app features, Google Account/QR and physical matrices.
No source-code/image tests or installed state changes in this review. Native text
actual verification remains its measured observations; no new suite implied.
