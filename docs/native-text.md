# Native received text and explicit clipboard copy

2026-10-13. Mode A. User selected 1/A from native-text-review.md. Guide:
/home/morozhenko/.codex/AGENTS.md. Work directly on dev; no live clipboard/config,
peer connection, install, push or branch creation. Plans/contracts precede code.

## Analysis and stages

Core already parses Text/Address/PhoneNumber and URL as UTF-8 bytes. Introduction
does not currently record declared text size/type; native details deliberately marks
all text Unsupported. Manager2 contains only safe metadata, and the coordinator
retains terminal history independently of GUI lifetime. GTK owns clipboard/UI.
Preserve file consent/source/session authority and payload privacy while adding
explicit received text review/copy. Outgoing clipboard/text/image, Wi-Fi actions,
URL opening, reveal and permanent payload history are separate later capabilities.

1. [ ] R1: allowlisted payload kind plus declared size for native text consent;
   validate actual byte header/body and UTF-8/NUL; carry a separate redacted immutable
   completed resource through live-session observations and retain it in the daemon.
2. [ ] R2: typed Text1 captured metadata and explicit bounded read capability,
   same-UID/epoch/ownership/FD checks, checked chunk/read lifecycle in the SDK and
   existing controller. Keep content out of ordinary snapshots/signals/diagnostics.
3. [ ] R3: safe sender/PIN text request card; completed text has explicit Review
   and Copy in a secondary plain-text window. Copy only after fresh lock/epoch/read
   checks. No automatic copy/URL launch, clipboard read, shell or file creation.
4. [ ] R4: actual affected checks once, fixes, documentation, reversible commits,
   clean handoff/export. Physical KDE clipboard and Google peers remain acceptance.

Rollback GUI/read API before consent/resource activation. File framing/legacy
public signatures remain compatible. Detailed R2/R3 contracts below and any
subsequent corrections must be recorded before their implementation.

## R1 frozen authority and bounds

PayloadKind is Files (default), Text or Url; PhoneNumber/Address map to plain Text.
No content/title in TransferDetails; Ready text has zero file entries, bounded
declared total bytes and ordinary sender/device/PIN. File Ready still requires a
complete nonempty validated manifest. Unsupported/Unavailable stay empty/default.
same_description includes kind and size, so changed requests cannot borrow consent.

For managed native requests, accept only one text attachment, explicit payload ID
and size 0..128KiB, no file/Wi-Fi/mixed attachments. Reject larger/missing/malformed
metadata before enabling consent. Record type/declared bytes at Introduction.
Matching text payload Bytes header must have the exact ID/size and arrives only
after Accept in ReceivingFiles. Reject header/chunk mismatch/overflow before buffer
allocation/appending. Existing setup/BWU byte payloads retain their own bounds and
are not mistaken for text. Completed native text is valid UTF-8 without NUL; URL
is inert text. Validation precedes success acknowledgement. Legacy unmanaged
framing keeps its existing broad limits; shared total/type metadata is clarified.

Only an accepted Finished inbound text creates ReceivedText (redacted Debug,
immutable Arc). Payload is separate from display metadata, notice messages and
Manager2 captures. Failed/cancelled/unsupported or outbound state cannot create it.
The core does not set clipboard or launch URLs. Session observation retains prepared
kind/size while transport working maps retire; completion must match those values.

Daemon retention follows existing epoch-local transfer history: normally 64 recent
eligible terminals, maximum 256 transfer records. Each resource <=128KiB; max
32MiB in transfer records, one read per authenticated client (32 readers max,
4MiB additional pinned resources), no disk journal/timer or unbounded body copies.
Session's existing 512-event/32-live-lease bounds remain; Arc shares content across
events. These are provisional native admission/resource budgets, not Google limits.
Retiring history expires new reads; an already acquired immutable reader remains
valid until explicit close/disconnect/epoch end. Never silently trim text or claim
permanent clipboard persistence. Read metadata reports availability/revision honestly.

## R2 frozen wire / SDK before code

Text1 uses the existing service/path/pinned owner and authenticated clients.
GetMetadataPage(snapshot_id,offset) returns up to 16 (transfer_id,kind,available,
content_revision) from that caller's immutable Manager2 capture. Kind is files/text/
url; content revision is the terminal revision, nonzero only for a retained Finished
inbound resource. No content/title/path in metadata or StateChanged/notice signals.

BeginRead(transfer_id,content_revision) returns (reader_id,kind,byte_count). Exact
same client/target/revision retry returns the same capability; another target while
it is held returns Busy. No implicit consent: only a matching already Finished
resource may be read. Reader IDs are epoch-bound; one per client, total <=32.
GetChunk(reader_id,offset) returns next byte offset plus <=16KiB bytes; require
chunk-aligned offsets <=length, exact final/empty chunk semantics, no FDs. EndRead
releases the caller's reader; foreign/disconnected/stale/unknown reads fail with
existing finite domain errors. Every route rejects any received descriptor before
body decode; transport 64KiB frame/16FD bounds remain. No arbitrary file/path read.

SDK validates metadata kind/Ready relation, counts, exact offsets/lengths, valid
UTF-8/NUL, pinned epoch/owner and phase/revision. Keep target/capability ledger on
interrupted begin/chunk/end; exact explicit retry resumes/re-reads that same resource.
Dropping a GUI wait does not leak independent bus workers or auto replay a save/send.
Client close/disconnect releases reader authority. Controller serializes explicit
read/retry with existing actions; complete ordinary state remains payload-free.

## R3 frozen presentation / copy before code

Text requests show ordinary advertised sender and verification code, Text/Link and
declared byte count; no manufactured file name or clickable text title. Accept and
Decline preserve exact existing round/readiness checks. Compact still permits review
of the request identity; completed events use generic outcome notices without body.
Full transfer rows offer Review text only for available Finished inbound resources.

Explicit Review obtains the read through the controller, then opens one owned
secondary plain TextView (no markup/automatic hyperlink/executable interpretation).
Opening/reading never modifies clipboard. Copy uses GTK's actual local clipboard
provider only on button activation, after current known unlock and matching daemon
epoch. It reports "Copied to this clipboard" after GTK takes content ownership;
it never claims persistence after GUI exit or cross-device delivery. Missing/stale/
locked/unavailable content yields a fixed message and no clipboard mutation.

Do not auto-copy after receipt/reconnect/view updates or subscribe to clipboard.
Close/lock/epoch change destroys the text surface and drops GUI body references;
pending read waits are owned/cancelled, SDK reader authority remains recoverable
or released through the dedicated client close. GTK shutdown never leaks callbacks.
Existing settings/menu/source readiness, history retention and no-source-replay
rules remain. No physical user's clipboard is read or written during development.

## Scoped verification inventory before tests

| Artifact | Branches / interactions | Checks | Categories |
|---|---|---|---|
| Metadata/session text | Files vs Text/URL/Wi-Fi/mixed; sizes 0/1/max/max+1/missing/negative; sender/PIN; description mutation; terminal body type/length; redaction | table-driven domain and real inbound fixtures | Positive,N1,N2,N5,N7,N10 |
| Core bytes completion | before consent/wrong size/ID, partial/final offsets, UTF-8/NUL, unchanged file/setup behavior | actual managed duplex request | N1,N2,N4,N7,N9 |
| Daemon/read resource | Finished inbound only; repeat/different body; retained/history expiry; snapshot vs live revision | coordinator/retention tests | Positive,N4,N5,N6,N10 |
| Text1 and SDK | actual UID, wrong owner/epoch/FD, chunk boundaries/order, interrupted begin/end, read release/disconnect, immutable captured metadata | isolated private broker with real resource | N1,N2,N3,N4,N5,N7,N9,N10 |
| GTK copy/review | plain-text rendering, text consent summaries, inert preview, no implicit copy, lock/epoch close, local button effect via isolated test clipboard only | strengthen single GTK smoke; pure projections | Positive,N5,N7,N9,N10 |

Use actual core/resource/read logic, no domain mocks/sleeps. Kill weakened consent,
late/body substitution, unbounded allocation, clipboard side effects without click,
private diagnostics and stale-capability acceptance. N8/N11/N12 not applicable:
no locale/time/billing/new persistent storage. No whole-product exhaustive coverage
or ten-run development claim; track actual method counts/results at each milestone.
