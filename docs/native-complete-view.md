# Native complete immutable view

2026-10-12. Mode A. User-approved option 2 from native-desktop-details-review.md.
The user explicitly confirms there are no old native clients: replace Manager1
with Manager2 in daemon, SDK and XML; no compatibility adapter or dual cache.
Operations1 ordered receipts and FD ingress ownership remain unchanged.

## Analysis and motivation

SessionLease currently publishes phase/consent, suppressing same-state changes.
Inbound enters WaitingForUserConsent before introduction metadata is prepared.
TransferMetadata is a legacy DTO, with destination, text and credentials-adjacent
fields that must never be dumped into IPC. Core handlers share InnerState across
TCP/BLE ownership; their update_state is the ordered publication seam. Source
InternalFileInfo provides actual prepared safe relative names and signed sizes.
Inbound ack_bytes counts successfully written file chunks; outbound ack_bytes
counts bytes sent locally, not remote durable acknowledgement. Preserve those
semantics and name the public field transferred_bytes. Text payload byte totals
are not consistently available inbound, so unknown must never look like zero.
The inbound decoder allows larger manifests than native selection's 256-file cap;
projection bounds must fail explicitly without silently changing the wire protocol.

## Frozen projection

Add a redacted, immutable Arc<TransferDetails> to SessionSnapshot and coordinator
TransferSnapshot. One ordered observation carries phase plus details. A metadata-
only change advances the coordinator revision without creating a new consent
round. Entering another round clears readiness; repeated phase observations keep
the existing round. PIN exists in IPC only during WaitingForUserConsent with a
matching ready projection. Outside that phase strip it, including terminal views.
Terminal snapshots absorb late observations. Historical immutable captures retain
the details that were true at their captured revision; they cannot authorize a
new round. No core text/clipboard/SSID contents, absolute paths, network routes,
keys, certificates or raw errors in read projections or Debug implementations.

Details status: unavailable (not prepared), ready (complete validated file
manifest), unsupported (cannot safely/fully describe this payload). This slice
supports native file transfers; text/URL/Wi-Fi are explicitly unsupported in the
native detail projection until their consent presentation is separately designed.
No invented PIN, device identity or byte progress. Sender label is an untrusted
advertisement; known device kind unknown/phone/tablet/laptop. File display names
are prepared relative paths, not source or destination absolute paths. Sort by
(name,size), preserve duplicate entries (payloads may share a display name).
Validate names as UTF-8 <=1024 bytes, no controls, backslash, absolute or dot path
segments. Sender <=256 bytes/control-free, actual PIN exactly four ASCII digits.
At most 256 files, checked sum of real nonnegative sizes equals total_bytes,
transferred_bytes <= total_bytes, <=64 KiB aggregate manifest name bytes. A
violation gives explicit unsupported with no partial manifest/private fields.
Require sender and actual PIN before ready live incoming consent; reject/cancel
remain possible for unsupported transfers. Ready non-consent progress can omit
PIN. File metadata availability comes from the prepared file map, not paths.

## Manager2 wire and budgets

Keep existing summary methods/signatures, move their namespace to Manager2.
Add GetTransferDetailsPage(su) -> ua(sssssttu): id, status, sender label, device
kind, PIN, total_bytes, transferred_bytes, file_count. Exactly one detail header
per summary row, in identical ID order, all part of the same immutable capture.
Add GetFilePage(ssu) -> ua(st): snapshot ID, transfer ID, offset -> next offset,
(name,size) rows. At most 16 records/page, canonical current epoch tokens,
connection-owned captures, strict unknown/foreign/offset behavior and no FDs.
Absent/unsupported header: empty sender/device/PIN, zero bytes/count. Immutable
Arc manifests are shared by captures; no live-map reads or source handles.
Capture limits: existing 128 endpoints/128 selections/256 transfers, 16 retained
captures, <=17 MiB logical metadata per capture and <=32 MiB globally. Charge
shared detail data conservatively once per capture, minimum 512 KiB per capture;
preflight these charges before any snapshot copy. A failed replacement preserves
its previous capture and counter. All reply bodies <=48 KiB/full frames <=64 KiB.
One 16-file page at 1024-byte names fits. Temporary serial projection is bounded.
SDK validates complete details, count/name/sum/PIN/phase invariants, then publishes
one StateView. No partial detail cache. Startup warnings are fixed at host startup
and cannot change during a production host lifetime, so their existing read is
not a mutable component outside the capture. Capture failure never truncates.

## Milestones and rollback

1. [ ] Ordered core projection: validate/redact immutable metadata, publish same-
   phase changes, atomically observe phase/details in coordinator/runtime. Preserve
   legacy channel and protocol behavior. Commit with focused source checks.
2. [ ] Replace unpublished interface; capture headers/manifests with preflight
   budgets, add typed page dispatch/XML and SDK decoding. Ready matched details
   are required before SDK accepts consent; retain exact admitted retry behavior.
3. [ ] Focused domain/private-broker regression checks, one affected test run,
   strict core/desktop Clippy and format/diff. Record actual results and commits.
4. [ ] Continue native controller/shell integration using this complete view;
   physical placement/Google interoperability remain external acceptance work.

Rollback API+SDK together, then the ordered metadata slice. No settings schema,
dependency update, user installation, activation, source deletion or push. Risks:
metadata-only observation loses revision, stale PIN/round, mixed snapshot pages,
manifest truncation, excessive stale-capture memory and falsely claiming peer ack.

## Scoped Mode A verification inventory

- Projection: ready/unavailable/unsupported; 0/1/256/257 files; negative/overflow
  sizes, sum/progress bounds; 1024/1025-name bytes, 64KiB aggregate; unsafe path,
  sender/PIN controls/format; every device kind; unsupported text/URL/Wi-Fi;
  redacted Debug, no absolute paths/content. Positive, N1,N2,N7,N10.
- Ordered state: same-phase detail update; current round preserved; new round
  readiness reset; terminal/late observation; PIN cleared outside consent;
  core handler actually delivers file metadata. Positive,N4,N5,N9.
- Capture: immutable header/files after progress/terminal/release; unknown/foreign
  transfer/capture/offset; every page boundary; cache exact/over accounting and
  failed replacement; maximum encoded reply. Positive,N1,N2,N5,N7.
- SDK/private broker: complete real view, strict malformed headers/manifests,
  matched consent readiness; cancelled read does not publish partial details;
  existing ordered-operation and owner-loss suites retained. N1,N3,N4,N5,N7,N9.
- Expected 12-18 focused test methods with table subcases; exact actual counts
  recorded after execution. No new N8/N11/N12 domain (clock/billing/storage),
  inherited FD/storage failures remain. No full repository coverage claim, no
  ten-run development repetition. Assertions kill relaxed bounds, missing phase/
  readiness checks, live-map capture and private Debug disclosure mutations.

## Verification observations

Plan recorded before code. Implementation and measured results pending.
