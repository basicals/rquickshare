# Canonical preview wire and routing review

Scope update2026-10-16: the user excluded preview functionality from active work.
See [essential priorities](native-essential-priorities.md). The foundation and
observations below are historical; do not resume codec research, UI/IPC binding
or thumbnail implementation. Existing bounded opaque routing remains input safety.

2026-10-16. Mode A; direct dev. Guide: /home/morozhenko/.codex/AGENTS.md.
Continue P2 in native-preview-review.md after completed native9 live visibility.
The user already selected sandboxed decoding; do not reopen that boundary.

## Analysis and research plan before implementation

The pinned Google Introduction schema defines preview_payload_ids but the selected
files reviewed for byte ingress did not establish encoding, attachment mapping,
arrival order or sender/receiver semantics. Current native decoder is an offline
foundation, not a protocol adapter. Inspect canonical implementation/tests before
choosing a production contract; unknown wire behavior must not be invented.

1. [x] W1: inspect the complete relevant public google/nearby sharing source and
   tests at the existing immutable revision, not just the five earlier selected
   files. Record public source URL, archive/file digests and bounded search scope.
   Identify actual preview producer/consumer and fixture/mapping evidence.
2. [x] W2: inspect our Introduction/byte dispatch, session ownership, native
   details/consent and decoder lifecycle against established upstream behavior.
   Separate demonstrated semantics from unknown platform behavior.
3. [ ] W3: freeze a concrete receive/routing/IPC contract with risks and scoped
   regression inventory before code. Significant new API/security/UX choices
   require user selection under guide3/3.1 unless already explicitly selected.
4. [ ] W4 after selection: implement the approved slice, focused checks once,
   atomic commits and evidence/current-state handoff. Build only a changed complete
   candidate; native9 remains the measured package until a successor is verified.

W1/W2 are read-only public research and source inspection. No workstation radio,
account authorization, upload, browser login, signing, installation or publication.
No product/schema/runtime changes while semantics are unresolved. Preserve native9
and all backup/release artifacts. Rollback is documentation only at this stage.
Research failures are not a reason to fabricate compatibility or weaken decoding.

## Observations

Pending W1. Previous pinned revision:
d128b5fca30b3fa8d4642f8c75dd9e314e090e74. The public schema has field9, but no
codec or mapping comments. No new runtime checks have run for this research.


## W1/W2 actual source evidence

Downloaded the immutable public google/nearby archive once for each revision;
no code from either archive was executed. Prior revision d128b5f contains300
regular sharing files (2313222 bytes); scanned all3310 regular archive files
(139220818 bytes) for preview_payload/thumbnail tokens. The only preview_payload
occurrence outside generated unrelated Windows/analytics material is schema
sharing/proto/wire_format.proto. Broad sharing preview search also found text-title
truncation, Android system share-sheet comments, analytics and unrelated MIME data;
none establishes transmitted image bytes.

The current main observation is1c1bcbe8e7c5d40597014789414a6a48c40294a2,
committed2026-10-15T03:17:00Z. Its complete archive has1611 regular files,
21842518 bytes. The only preview_payload matching file is again the wire schema;
all regular sharing files were extracted and searched. Archive provenance/digests
and exact scope are in evidence/native-preview-wire-2026-10-16.json. This is a
negative search result within these source snapshots, not proof that Android or
Google Windows never sends previews. No canonical codec, per-attachment mapping
or ordering fixture was established. Do not infer these from a list of IDs.

Primary schema:
https://github.com/google/nearby/blob/1c1bcbe8e7c5d40597014789414a6a48c40294a2/sharing/proto/wire_format.proto

Our inbound handler reassembles every non-text BYTES payload, then decodes it as
sharing Frame. It ignores the advertised preview ID set. Therefore optional
preview bytes can fail protobuf decoding; if they happen to encode Cancel they
are misclassified as control. This is an evident internal routing defect within
the user's maintenance authorization (guide11), independently of image codec.
Fixing it introduces no new parser, image acceptance, display/API or credential
boundary. Full preview display/IPC still requires canonical data and its contract.

## W3 frozen defensive routing fix (before code)

1. [x] R0: document source evidence, exact safe behavior and focused inventory.
2. [x] R1: private request-owned PreviewIds. Bound raw announcements to256 entries
   (existing manifest count defensive scale, not a negotiated protocol limit).
   Accept explicit signed64-bit values, including0/negative; normalize duplicates.
   Reject any overlap with file/text/Wi-Fi/app-split/stream content IDs before
   changing state or publishing consent. Use existing getter/default addressing
   when checking content so an absent content ID cannot hide a collision with0.
   No per-file mapping or arbitrary assumption that preview IDs align by position.
3. [x] R2: after successful supported Introduction, install the set on that
   inbound request only. Route its BYTES through the existing exact bounded
   assembler and drop completed opaque ownership immediately. Never try image,
   sharing-frame or text decoding; never copy, persist, publish or count preview
   bytes as transferred content. Validate existing size/offset/final constraints;
   no second assembler or budget. Keep16-incomplete/5MiB shared reservations.
   Before announcement/unknown IDs keep existing strict control behavior. No new
   pre-Introduction buffering or guessed ordering. File payload handling and
   explicit Accept/Reject/PIN are unchanged. Unsupported Introduction does not
   install preview authority. Outbound has no incoming Introduction/preview list
   and is unchanged.
4. [x] R3: focused real parser/handler checks once, strict affected lint/format,
   measured evidence and atomic commit. No complete preview feature/package10
   claim: native9 stays the last measured binary and does not include this fix.

This is an optional-data metadata-only fallback, not implemented peer image
support. Malformed/oversized auxiliary framing still fails the request as existing
BYTES framing does; valid opaque bytes cannot become Cancel/control. Rollback
reverts the private routing/helper and its regressions together; saved schema,
IPC, installed services and release artifacts are untouched. No substantial
refactor or new UI/API selection is needed for this scoped internal bug fix.

| Artifact | Branches / interactions | Level / planned new methods | Categories |
|---|---|---|---|
| PreviewIds |0/1/256/257 announcements, duplicate normalization, signed extrema/0, collisions across all5 families including default0, Debug redaction | unit /1 table-driven | Positive,N1,N2,N4,N7,N10 |
| Actual inbound router | managed/legacy; real file and text Introduction; fragmented opaque PNG-like/junk/valid Cancel bytes; pending state and post-Accept; exact completion and repeat; no file creation/content/progress/control side effects | integration /1 table-driven | Positive,N1,N2,N4,N5,N7,N9,N10,N12 |
| Bad routing/framing | colliding/oversized Introduction rejected before consent; declared versus undeclared bytes; malformed offset/final/size; shared incomplete/reservation bounds; unsupported Introduction and before-announcement behavior | integration /2 table-driven | Positive,N1,N2,N4,N5,N6,N7,N9,N10 |

Assertions kill removed ID routing, content-based control parsing, missing collision
checks, extra pre-consent file creation, ignored framing limits and retained complete
buffers. Real domain/parser/assembler is the subject; filesystem and private duplex
streams only, no user files/radios. Existing text-consent and byte-assembler suites
protect unchanged branches. No new time, billing, network retry or persistence
(N3/N8/N11 not applicable; N12 uses actual temporary destination). No sleeps,
representative token tests or ten-run development loop. Record observed counts.


## R1–R3 source completion and measured checks

Implemented private redacted PreviewIds and request-local inbound routing. Both
unsupported-attachment and unknown legacy-text exits return before installing the
ID set. Announced BYTES are validated by the one existing assembler and discarded;
no new buffer owner, parser, IPC, thumbnail field or UI is introduced. Before
announcement and unknown IDs retain strict setup parsing. Declared IDs cannot
shadow any of the five attachment families. Signed IDs/duplicate normalization
and raw256-ID cap are internal defensive contracts, not peer capability claims.

Four new Rust methods, primarily N7=2,N1=1,N2=1; positive, idempotency, interruption,
privacy, concurrency and temporary-storage interactions overlap those methods.
Full core library initial run:636 passed/1 failed in11.12s (637 total,633 previous).
The strong unsupported-introduction test caught a missed outer rejection branch in
the introduced boolean guard. Replaced that fragile guard with explicit early
returns from both rejection branches before installing PreviewIds. Focused preview
run:3 passed/1 failed in0.02s; the expanded native/legacy unknown-text scenario
still had the old success unwrap. Corrected it to assert the existing contract
exactly: native rejects before consent, legacy sends unsupported response without
installing IDs. Final affected method passed1/0 in0.01s (636 filtered). No assertion
was weakened and no final full637 green run is claimed. The other three new methods
passed in the affected run; the633 old methods passed in the initial whole run.

All-features/all-target strict core Clippy passed11.39s; existing upstream bluer
future-incompatibility notice was emitted. Nightly formatting and diff whitespace
checks passed. No unchanged GTK/helper/package/legacy checks or ten repetitions.
Terminal closed; no radio, account, installed-service or release operations.

A final offline scan of every regular file in both archives also checked snake_case
and camelCase preview identifiers: only sharing/proto/wire_format.proto matched.
That closes a search spelling blind spot, not the missing canonical codec/mapping
fixture. Full production preview receive/display remains open. Native9 package and
its immutable source archive predate this source fix and remain unchanged; do not
claim the fix is already in that binary. Prepare a successor when the next complete
reviewable native feature is ready, with fresh source provenance.
