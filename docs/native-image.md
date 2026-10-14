# Native clipboard image capture and anonymous source ownership

2026-10-14. Mode A. User selected1/A from native-image-review.md: immutable
anonymous memory-backed file, explicit bounded capture and shared Files preparation.
Guide /home/morozhenko/.codex/AGENTS.md. Direct dev; no user's real clipboard,
config/install/tray registration/physical peer/push/branch. Tests use isolated display.

## Analysis and milestones before implementation

OpenedFile adopts a read-only regular descriptor; coordinator reserves Files/Text
through the actual PreparedSource Arc retained by OutboundRequest. Existing upload
transactions/SDK recovery/CLI gate and explicit recipient Send can carry a PNG file
without new image IPC or deletion API. Native text editor already owns cancellable
GTK stream capture. GDK read_texture decodes before caller dimension validation,
so use bounded image/png stream and png0.18.1 decoder/encoder in an owned worker.

1. [x] I1: anonymous sealed/read-only source factory and kernel-derived memory
   classification; reserve aggregate memory through staged/direct/actual-held file
   sources. Preserve disk descriptors and existing file/CLI/source lifetime checks.
2. [ ] I2: bounded PNG decoder, normalized PNG encoder, small preview, immutable
   source; tracked joined worker/cancellation and explicit GTK review/Prepare.
   Reuse source gate/controller, no automatic image fallback, new Send or path IPC.
3. [ ] I3: actual affected checks once, failures fixed, SSOT/atomic commits/export.
   Physical KDE/Google PNG capture/peer acceptance remains a separate gate.

Rollback UI/worker/decoder before factory/accounting. Existing legacy ephemeral
paths/DTO behavior stays a separate baseline. Keep backward-compatible Files APIs,
selection/manager wire signatures and finite errors; no new transfer owner/service.

## I1 frozen source/accounting contract

memory_source::create(bytes,name) accepts1..32MiB; creates CLOEXEC/ALLOW_SEALING
memfd, writes exact bytes, seals WRITE/GROW/SHRINK/SEAL. Reopen ONLY its held own
/proc/self/fd/<n> reference read-only/CLOEXEC while writer lives; never caller paths.
OpenedFile::adopt independently checks actual kernel properties/size. If GET_SEALS
succeeds the file is seal-capable memory backing: require all four immutable seals,
<=32MiB; charge actual size. EINVAL means ordinary non-sealable disk descriptor,
unchanged existing mutable-file checks. Other inspection errors fail closed.
This deliberately rejects mutable seal-capable memory FDs (including applicable
tmpfs files) rather than pretending their captured size is immutable/accounted.
No caller flag/name can bypass charge. No file deletion capability or named spool.
Anonymous kernel bytes disappear when final descriptors close; no permanent RAM or
crash-proof journal/persistence claim. Ordinary external disk source contents can
still mutate as previously documented; only new memory source is immutable.

128MiB aggregate memory-backed Files reservation, independent of16MiB Text and
received resource budgets. Compute actual memory bytes from adopted OpenedFile,
include preparing uploads+all outer Arc reservations; reserve before batch/direct
ownership commit. Commit moves same files and charge without double count. Abort/
disconnect/queued cancel release, but active/final readers hold reservation until
actual capability drop. One file max32MiB; errors use existing InvalidInput/Capacity.
Public ResourceUsage wire unchanged: total selected bytes/handles already includes
memory sources; private memory budget is additional admission invariant. Standard
file/inbound/text/read/settings/idle restart semantics remain.

## I2 frozen image and UI contract

Capture PNG MIME only on explicit Capture image action. Providers may negotiate PNG
from an image; unsupported/no PNG yields fixed error, no automatic text/image fallback.
Input stream <=32MiB+1 detection, chunks<=64KiB, one capture per shared source gate.
EOF terminates, owned cancellable GLib future/stream close; never read on open.

Decoder uses png0.18.1 with64MiB best-effort internal allocation limit (library's
limit is not an OS-enforced hard RSS bound), ignores text/ICC parsing, checks IHDR
before image allocation. Dimensions nonzero, product<=legacy64million pixels;
transform EXPAND|STRIP_16 to8bit, output<=256million bytes. Decode actual first static
frame and finish/check PNG CRC/deflate/end; reject animation rather than silently
losing frames. Decode/encode errors fixed; no raw image/logs. Reencode pixels to
static normalized PNG with bounded32MiB writer; no resizing/truncation. Strip
ancillary metadata as legacy clipboard image capture did. Original pixel dimensions
preserved (16bit channel source normalized to8bit as legacy RGBA capture).

Generate RGBA8 nearest-neighbor preview<=512x512,<=1MiB from decoded bytes. GTK
creates MemoryTexture only from that validated small pixel buffer; never hand the
original PNG to an unbounded GTK decoder. Source name fixed Clipboard.png. Decoder
and memfd creation run in a tracked spawn_blocking job, serialized/joined by existing
Ingress owner. A CancellationToken checks boundaries before decode/encode/source
and preview rows; underlying single decoder/encoder calls may finish before cancel
observes. Worker owns shared gate permit across CPU work and joins on shutdown,
even after GUI wait is dropped. No second capture until that owner releases it.
No claim that cancellation preempts third-party CPU instructions instantly.

Secondary image surface: Capture clipboard image, plain dimension/byte information
and bounded preview, Prepare image, Cancel. Known unlock and current daemon epoch/
source-empty state rechecked at explicit capture/Prepare and after asynchronous
results. Close/lock/epoch cancels waits/token, clears preview/source; already admitted
controller preparation keeps existing explicit recovery/abort rules and never Sends.
Preparation uses existing file controller, then Nearby recipient choice and Send.
Cannot replace existing file/text preparation; no clipboard writes.

Expose Send image in header/context/tray without displacing existing action IDs.
Tray actionID17 requires widen private enabled bitset u16->u32;16-existing IDs and
16-event/mailbox limits stay,17children/18nodes, no private content in labels.
This is an internal projection bitset extension, not a new external DBusMenu method.

## Scoped supporting verification inventory before tests

| Artifact | Planned branches / interactions | Level / categories |
|---|---|---|
| Memory source/adoption | empty/1/max/max+1,exact bytes/seals/read-only/CLOEXEC,mutable/grow/rename independence,redaction,descriptor drop | domain+kernel Positive,N1,N2,N7,N10,N12 |
| Coordinator/upload | direct/staged quota boundaries,mixed disk+memory,text independent,abort/disconnect/queued cancel,active actual reader retained,failed batch atomic | domain+actual SDK broker N2,N4,N5,N7,N9 |
| PNG capture | invalid/truncated/header-size/product/CRC/deflate/animation,8/16bit and alpha/color transforms,output bound,exact preview dimensions/bytes,cancel | domain real PNG fixtures Positive,N1,N2,N9,N10 |
| Worker ownership | dropped GUI reply/cancel/close,gate retained until real worker completes,no late source,shutdown join | actual Ingress integration N5,N9,N12 |
| GTK/image/menus | open never capture/Send,explicit bounded clipboard,unsupported preserves source,stale lock/epoch rejection,prepare callback,generic source summary,ID17 enable/event | existing single isolated GTK smoke+actual menu broker N1,N5,N7,N9,N10 |

Tests exercise actual source/accounting/PNG/worker domain; external UI controller
adapter is separately exercised through existing real broker. No mocked subject.
N3 covered existing independent clipboard/bus failure handling plus fixed image
failure; N6 invalid/corrupt PNG; N8/N11 not applicable(no clock/billing). N12 resource
creation failure must reject honestly, no disk cleanup/storage migration added.
No full-product Mode B/ten-run/Google signoff claim. Record actual observations here.

## Primary API references inspected

I1 environment observation: this host's /tmp is seal-capable tmpfs. The initial
`memory` test filter passed4/failed2 because existing disk fixtures used mutable
tmpfs descriptors. This is the deliberately frozen rejection above, not a relaxed
assertion. Disk compatibility checks use TMPDIR=$PWD/core_lib/target/image-test-tmp
on the repository filesystem; tmpfs rejection stays independently tested. Test
commands must retain this setting when they create ordinary mutable disk fixtures.

https://docs.rs/png/0.18.1/png/struct.Decoder.html (limits are best effort, caller
buffers separately bounded); https://docs.rs/png/0.18.1/png/struct.Reader.html;
https://man7.org/linux/man-pages/man2/memfd_create.2.html. Local png/rustix source
inspected alongside these APIs. New dependency pinned, lock changes checked.

I1 actual verification: memory filter6passed/0failed0.07s with disk TMPDIR; includes
existing text quota/retention plus4 new factory/accounting scenarios. Strict core
all-target clippy passed6.54s. Kernel-source size/read-only/seals and atomic staged/
direct admission/lifetime checks passed. Terminal closed.
