# Preview receive/display: decoder boundary review

2026-10-15. Mode A, dev. Preceding bounded byte reassembly is implemented; this
document is the continuation SSOT, not a Google-preview compatibility claim.
The user selected option1 (sandboxed Glycin decoder) on2026-10-15. The significant
decoder-boundary decision is complete; do not ask for it again.

## Current state and missing contracts

The roadmap calls for previews before manual Accept. Core currently has only
preview_payload_ids in the Introduction model. It does not route them to a
preview owner or expose a preview through native typed IPC. Ordinary completed
non-text BYTES are still sharing setup frames. Transfer metadata/private session
details and the GTK card have no preview field. Files/text/clipboard remain
subject to their existing consent contracts.

Pinned Google wire schema and five selected implementation files are recorded in
[byte ingress evidence](evidence/native-byte-ingress-2026-10-15.json). The schema
does not specify a preview codec, id-to-attachment mapping or image bounds. The
selected C++ session/service/file classes do not establish those missing semantics.
Do not manufacture a custom wire format, assume every id is an image or claim
current Android/Windows behavior from schema presence. A real fixture/specification
is a prerequisite for the compatible receive slice; decoder work can be verified
offline independently, with that distinction explicit.

## Significant alternatives

| Option | Description | Advantages | Costs / risks |
|---|---|---|---|
| 1 — sandboxed decoder, recommended | Daemon retains bounded opaque bytes only. Native client requests admitted preview data through its authenticated typed boundary; an isolated image decoder returns validated bounded pixels to GTK. Prefer existing Glycin sandbox support over building a custom codec sandbox. Require sandbox availability; show ordinary metadata if decoding is unavailable/invalid. | Image parser authority is separated from transfer/credential ownership and UI. Decoder failure need not terminate a transfer. Matches the intended pre-consent card while retaining explicit Accept. | New runtime/package dependency and lifecycle/IPC integration; sandbox availability/ABI must be verified on the supported package target. Pixel dimensions/stride/byte count, cancellation, concurrency, CPU/memory and format admission still require explicit bounded implementation. Isolation reduces impact, not a proof of absence of parser bugs. |
| 2 — decode directly in GTK | Keep daemon opaque, feed admitted image bytes to an in-process native image decoder with input/output limits. | Fewer processes/dependencies; simpler image ownership. The daemon itself does not run codecs. | Peer images are parsed with GUI process authority before consent. Decoder faults can terminate the card/tray; limits do not isolate a vulnerable parser from the UI's user-session capabilities. This is a materially weaker boundary, not the quality recommendation. |

Recommend1 for clear ownership and fault containment, independent of work volume.
This selects the boundary, not an unverified claim that an available Glycin version
already meets every resource/wire constraint. No fallback to unsandboxed decoding
when sandbox startup fails. No helper may receive signing keys, account tokens,
receive-directory authority or arbitrary local paths just to decode a preview.

Primary reference: [Glycin image-loading API](https://gnome.pages.gitlab.gnome.org/glycin/libglycin/index.html)
documents sandboxed loading and GTK texture conversion. Its
[sandbox selector](https://gnome.pages.gitlab.gnome.org/glycin/libglycin/enum.SandboxSelector.html)
supports explicit bwrap selection; AUTO has a development-environment exception
which can disable the sandbox. Therefore relying on the word AUTO is insufficient
for an enforced boundary. Package/API version and executable/dependency ownership
must be resolved from the actual supported target before implementing the adapter.

## Plan after user selection

1. [x] P0: inspect existing ingress, native metadata/consent boundaries and primary
   references; fix the independently authorized unbounded reassembly prerequisite.
2. [x] P1: option1 selected. Freeze concrete decoder/package ownership,
   bounded byte/pixel/format/concurrency/resource lifecycle and native IPC/Full/
   Compact behavior before dependent code. See the frozen first slice below.
3. [ ] P2: establish canonical preview payload encoding/mapping/order using a
   specification or public/physical fixture; document provenance and unknowns.
   Do not block useful offline sandbox/IPC work on unavailable hardware, but
   distinguish it from Google preview compatibility throughout.
4. [ ] P3: implement the selected contract in reversible stages, with focused
   malformed/decompression-bomb/cancel/late/stale-session/sandbox-unavailable tests.
   Explicit Accept/Reject, PIN, file materialization and clipboard stay unchanged.
5. [ ] P4: run affected checks once, record results, then build/verify a new native
   candidate when there is a complete reviewable native feature. Preserve native7
   artifacts and do not sign/publish/install without the existing separate gates.

Resume remains separate: content identity/hash semantics, partial-file capability,
integrity, lifetime, interruption/recovery and peer response negotiation are not
established. No seeking to a peer-supplied offset or filename/size equality shortcut.

Rollback of this review is documentation only. No decoder, process, dependency,
IPC/settings field or peer image acceptance was enabled by the review itself.

## P1 frozen first implementation slice: offline native decoder boundary

Analysis: the native Rust/GTK stack uses glib/gio0.22; Glycin4.0.0 uses matching
0.22 and Rust>=1.93 (local stable1.99). The host has Glycin2.1.0, loaders in the
2+ protocol directory and Bubblewrap. These observations do not yet prove that
our sandboxed decoder starts or that a Google peer sends compatible image bytes.
Pinned crate source was inspected, not executed during analysis.

Glycin's default pool retains loaders for30s and its address-space limit is a
host-memory heuristic, up to roughly16GiB. Neither is our intended per-preview
budget. Therefore use a separately owned private helper process rather than run
the Glycin coordinator in GTK. A helper hard resource ceiling also applies to
descendants; Glycin's attempted raise cannot exceed the inherited hard ceiling.
Keep explicit Bwrap, no base-directory exposure and no unsandboxed fallback.

Scope and stages before any code:

1. [x] D1: add a private rquickshare-preview-helper binary and shared bounded
   binary pixel protocol. It reads encoded stdin only, writes pixels only on
   success, receives no path/account/session/receive-directory authority, and
   never initializes GTK. The helper uses a fresh Glycin pool, zero retention,
   max1 parallel operation and explicit Bwrap. No global pool.
2. [x] D2: add the native async subprocess adapter with explicit executable
   dependency, one-operation admission/no queue, cancellation and deadline,
   bounded concurrent stdin/stdout, kill-on-drop fallback and explicit kill/reap
   on failed/cancelled/timed-out operations. Never manage unrelated processes.
3. [x] D3: exercise the actual adapter in existing developer --preview incoming
   mode with a tracked harmless synthetic PNG. Full mode can show the decoded
   thumbnail; Compact retains its current metadata/actions. This is an offline
   review fixture, not peer ingress, consent or Google image mapping. Stale/closed
   panels discard results and cancel their operation. Decoder failure leaves the
   existing metadata card usable without a picture.
4. [x] D4: packaging source includes the private helper under /usr/libexec; resolve it relative to
   the installed executable's known prefix (never peer/PATH input), include its
   ELF/runtime dependencies and require Bubblewrap/Glycin image loaders. Update
   exact offline package inventory/help checks and documentation. Preserve all
   native7 immutable artifacts; increment revision only for a new candidate.
5. [ ] D5: run focused checks once, record real sandbox success/failure separately
   from deterministic policy/transport tests, update evidence/current state and
   atomic commits. If real sandbox cannot start, never disable it to green a test.
   No installation, credentials, cloud/peer request, signing or publication.

Frozen bounds (internal defensive policy; not wire negotiation): encoded input
1..=256KiB; admit PNG/JPEG magic only; source dimensions1..=2048 each; decode only
one frame, never an animation loop; preview longest edge<=256 with preserved aspect
ratio; tightly packed straight RGBA8,<=262144 output bytes plus16-byte header.
Validate dimensions, format, stride, body size and exact EOF independently in the
parent. No encoded image reaches GTK texture constructors. Ignore/reject unsupported
content without opening URLs, materializing received files or touching clipboard.

Helper hard ceilings:512MiB address space per process,5 CPU seconds per process,
128 descriptors and core dumps disabled. Parent operation deadline10s includes
startup/IO/exit; this is a resource deadline, not a synchronization sleep. Glycin
decode limits add a5s stage timeout and2048 dimensions. Parent helper environment
is explicit/clean with system PATH/data directories, no inherited secrets or
user loader/config overrides. Stdout protocol only; stderr not surfaced/logged.
Budgets are per operation/process, not a global memory/cumulative CPU guarantee.
The helper fixes RAYON_NUM_THREADS=1 during single-threaded startup. Actual JPEG
testing exposed Glycin's format converter creating a host-CPU-sized pool under
the address-space cap; configure the documented pool limit instead of raising
the resource ceiling or removing JPEG verification. This is an internal resource
fix within the selected sandbox boundary. No upstream code is patched.

Pixel protocol is private local helper IPC, never a Quick Share wire extension:
8-byte magic/version RQSP/1, big-endian u32 width/height, then exact width*height*4
bytes. Invalid/truncated/extra data, unsuccessful helper exit or unavailable
sandbox yields no image. The native adapter uses only validated pixels.

Expected scoped verification (Mode A, before tests):7 domain methods for input
admission/signatures/boundaries, dimensions/aspect scaling, frame stride/format/
length validation, protocol magic/version/truncation/extra bytes, exact pixels,
redacted Debug and invalid pixel-object serialization;3 adapter methods for real helper PNG/JPEG pixels,
pre-cancel/busy/missing helper, and process cancellation/deadline/oversized/truncated output
using only external process fixtures. Actual decoder is never mocked.
Deadline verification uses Tokio's paused clock after an actual readiness event,
never a10-second sleep. The added test-util feature is development-only.
Existing GTK preview/safety/close tests protect UI lifecycle; add one fixture UI assertion
if needed. Categories Positive,N1,N2,N3,N4,N5,N7,N9,N10,N12 (spawn/pipe failures).
N6 applies to malformed/versioned helper output; N8/N11 not applicable (no clock
dates/billing). No ten repetitions or blanket repository coverage claim.

Risks: sandbox/package environment availability, helper lifecycle and untrusted
pixel metadata. Rollback each native implementation commit together with its
dependency/packaging changes; no stored schema, protocol or remote state exists.
Wire ingestion, native daemon preview IPC, production card binding and outgoing
previews remain a subsequent compatible slice gated on concrete payload semantics.

## Actual source verification2026-10-16

Implementation uses a package-local non-published Rust library shared by GUI/helper
to keep the local pixel protocol in one place. Glycin4.0.0 resolves via the locked
external/core4.0.3 and utils5.0.1; glib/gio0.22 match GTK. Use Glycin's Tokio feature
with default features disabled to match the existing zbus runtime. An initial
async-io/Tokio feature mismatch was corrected before executing the helper.

- Actual helper PNG smoke: exit0, exact24-byte header/body with2x1 known RGBA.
  Bubblewrap0.11.1 and installed Glycin loaders2.1.5 were used; no host installation
  or sandbox policy change. No Google/physical peer was involved.
- New library suite initially9/10 passed in0.17s; the real JPEG fixture exposed
  a Glycin Rayon ThreadPoolBuildError under the hard address-space ceiling.
  The documented one-thread configuration fixes the cause. Affected actual
  PNG/JPEG method passed in0.14s with the same ceilings and strict pixel checks.
  JPEG uses a documented small color-error bound because its source is lossy;
  pixel ordering, dimensions, length and alpha remain exact.
- Transport method subsequently gained a fake-clock deadline case and strict
  per-fixture error assertions. Final affected method passed in0.08s; child PIDs
  were absent after awaited cancellation/deadline completion. No sleeps or
  arbitrary watcher/process termination. Other passed domain methods were not rerun.
- Existing native GTK binary suite39/0 passed in1.84s on a private display/bus,
  including actual builder/close/preview safety. No additional near-duplicate
  GTK test was added; the final package visual fixture will check Full/Compact.
- Package Python suite5/0 passed in3.227s. The subsequent minimum-loader-floor
  change was checked with only its affected metadata method (result in evidence).
- Final native all-targets --no-deps Clippy -D warnings passed in15.37s;
  format/diff checks passed. Existing9 no-experimental core dependency warnings
  remain visible; none were suppressed. No unchanged core/Tauri test rerun.

Ten unique new Rust methods by primary category: Positive2,N1=1,N2=2,N6=1,N7=1,
N9=2,N10=1. Shared interactions also exercise N3/N4/N5/N12 as described above;
no full coverage/ten-run stability or mutation-runner claim. Assertions protect
admission/count/size/layout/version/EOF/cancel/deadline/reap/redaction invariants.
The manual native-package workflow now installs required build/runtime dependencies
and builds the helper before tests; a runner unable to enforce Bwrap fails the real
smoke. CI itself was not dispatched; namespace/AppArmor availability remains a
runner acceptance requirement, never a reason for an unsandboxed fallback.

Source changes are ready for the native8 uninstalled candidate (revision8 avoids
overwriting native7). Candidate build, extracted helper/GUI and Full/Compact
rendering, fresh unsigned APT/public review request and export are still pending.
Do not mark native8 verified or claim it can display remote Google previews yet.
Owned build/test/lint processes exited. Terminal closed.
