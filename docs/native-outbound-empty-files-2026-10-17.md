# Outbound empty-file completion and failure handling

Mode A; 2026-10-17; direct dev. Scope: essential ordinary file transfer,
accepted outbound FILE payloads, current nearby wire representation. No
desktop/Pixel interaction, installs, builds for installation, signing, new
transport/API, automatic retry, source deletion, master promotion or published
release. The device's current installed native29 is unchanged.

## Repository scan / confirmed source defect

`core_lib/src/hdl/outbound.rs::process_consent` obtains the prepared file
IDs after remote Accept, and loops over the retained open file handles.
For each file it checks `bytes_transferred == total_size` before the
read/write path. That branch currently removes the current member from the
working map and continues without sending a FILE Data frame with `flags=1`.
Since an accepted zero-byte file naturally satisfies `0 == 0`, **no file
payload/EOF is transmitted**, yet the sender eventually marks its operation
`Finished`. The receiving FILE handler expects the exact terminal chunk to
retire and publish its file. A normal nonempty source does send a separate
empty-body EOF frame, but only after at least one nonempty chunk.

This is a source-inspected failure; its end-to-end Android impact remains
unverified. Existing outbound tests verify 3-byte files, prepared-source
size mutation, native cancellation, reader error kinds and an **empty transfer
with no files**; none asserts the payload/EOF for a legitimately selected
**zero-byte file**. A zero-size protocol member is valid in current manifest
preparation and receive code. This work does not add auto-retry or alter
wire conventions: send the existing empty-body EOF frame for every accepted
zero-size FILE payload.

## Mode A plan and scope

1. [x] Inspect actual file sending, prepared handles, tests, frame/EOF
   protocol, local failure mapping and active priorities; record plan.
2. [ ] Add actual encrypted native sender/peer regressions for one zero-byte
   file, mixed zero and nonzero members and failure before terminal write.
   Validate introduction IDs/manifest, each EOF once, exact bytes/progress,
   disconnect and source preservation. Tests use real loopback sockets,
   original captured descriptors and current protocol encoding.
3. [ ] Factor the already existing empty-body FILE terminal-frame constructor
   into a private helper used by both the zero-size and nonzero paths.
   Send before retiring the corresponding working entry. A failed frame send
   propagates an error before removing the member or reporting Finished.
   Do not change data offsets/packet type/metadata/sequence numbers.
4. [ ] Run exact-source CI once; examine failures, fix only actual causes,
   preserve regressions and record counts. Core/GTK/legacy UI change-free.
5. [ ] Update SSOT/current-state and explicitly defer device/physical peer
   compatibility, package and installation tests.

Rollback: revert helper + sender branch and its regressions. Invariants:
single explicit consent, no unexpected transmit before Accept, no auto retry,
read-only source descriptors, zero bytes not advertised as a positive body,
one terminal FILE chunk per completed member, source never unlinked, no
Finished before all required last-chunk writes succeed.

## Branch / risk inventory

| Production branch | Existing/new test intent | Classification |
|---|---|---|
| Zero-size open regular file immediately satisfies equality | actual native encrypted EOF and final state | Positive/N1/N2 |
| Nonzero file needs body then EOF | existing 3-byte plus mixed test; no wire regression | Positive/N4 |
| Multiple IDs/manifest order independent of hash-map iteration | actual mixed zero/nonzero accepted transfer, ID matched | N5/N6/N7 |
| EOF network write failure | no retirement or false Finished; error preserved | N3/N9/N12 |
| Changed/unavailable source before read | existing mutated-size and reader tests retained | N2/N3 |
| Cancelled before data send and cleanup | existing native cancel/lifetime tests retained | N9 |
| Receipt retry/idempotency | existing operation ledger and managed source control, no new retry path | N4/N10 |

N8 and N11 are inapplicable (no time, locale or billing). No new schema,
permission, clipboard, UI or persisted data contract. This is focused Mode A,
not full Mode B, branch-percentage coverage or ten-cycle stability evidence.

## Observations

Pending execution.
