# Preview receive/display: decoder boundary review

2026-10-15. Mode A, dev. Preceding bounded byte reassembly is implemented; this
document is a review, not preview support. Agent guide sections3/3.1 require a
user choice because parsing peer images before consent changes a security boundary.

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
2. [ ] P1: record user selection. Freeze concrete decoder/package ownership,
   bounded byte/pixel/format/concurrency/resource lifecycle and native IPC/Full/
   Compact behavior before dependent code. No implementation while unselected.
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
IPC/settings field, image acceptance or preview UI was enabled. User choice pending.
