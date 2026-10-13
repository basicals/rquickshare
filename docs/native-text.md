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

1. [x] R1: allowlisted payload kind plus declared size for native text consent;
   validate actual byte header/body and UTF-8/NUL; carry a separate redacted immutable
   completed resource through live-session observations and retain it in the daemon.
2. [x] R2: typed Text1 captured metadata and explicit bounded read capability,
   same-UID/epoch/ownership/FD checks, checked chunk/read lifecycle in the SDK and
   existing controller. Keep content out of ordinary snapshots/signals/diagnostics.
3. [x] R3: safe sender/PIN text request card; completed text has explicit Review
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

## R1 implementation clarification before correction

Completion must also match the prepared Text/URL type and acknowledged byte count.
A managed text header must explicitly carry ID and total size; its final chunk must
complete exactly the declared length (including zero). This prevents defaulted
protobuf fields and a truncated final body from being acknowledged as successful.
Terminal resource Debug never contains the body.

## R2 lifecycle clarification before implementation

EndRead replays success for the caller's exact last released reader ID. Keep one
bounded tombstone per client, removed on disconnect. This permits recovery after
a lost close reply; foreign, epoch-stale and other unknown IDs still fail. Offsets
are chunk-aligned except offset equal to the length, which returns an empty final
chunk. Metadata adds only fixed-size records within the existing capture base
reservation and never pins bodies. The SDK holds one bounded read ledger, separate
from source/settings actions, and re-reads the same immutable resource on retry.
Closing review drops its own text reference; an explicitly copied GTK provider
continues to own clipboard content until replaced or GUI exit. Lock/close never
silently clears the user's clipboard.

## Actual R1 verification

Core stable all-target check passed (20.58s); terminal closed. After R1 validation
changes: core library suite passed 535 methods (50.66s build +16.03s execution).
Added chunk table subsequently passed 1 method/9 variants (22.69s build +0.01s).
Its first run exposed missing encryption keys in the duplex fixture; the fixture
was corrected, without weakening completion assertions. Metadata table covers
8 size/kind variants and 10 actual introduction variants; session table covers
7 consent/type/length/NUL/ack interactions. Four new methods: Positive, N1, N2,
N4, N7, N10 (overlapping categories, not separate tests). No user clipboard/peer
used. All owned terminal sessions completed/closed; no watcher remains.
R2/R3 not yet activated: native SDK still assumes file metadata until Text1 lands.

## Actual R2 verification and handoff

Text1 XML, captured kind/availability, read cache, checked SDK ledger and controller
are active. Capture base reservation includes fixed metadata row storage; content
never enters StateView/PanelState. Total reader budget remains 32 even if a custom
coordinator raises its client budget. SDK read/retry and EndRead replay are bounded.

Stable all-target check passed (14.40s). Initial expanded library run: 539 passed,
1 failed because reader-capacity fixture hit the separate 32-client budget first.
Fixture now explicitly raises only client budget to 64; assertions still enforce
32 readers. Affected text filter: 22 methods passed (0.90s). Final tests invocation
selected library and integration targets: 543 library/export methods passed (12.70s),
8 binary integration methods passed (0.00s/0.03s); build 13.04s. Strict all-target
Clippy passed (18.66s). Nightly format applied. Seven new R2 methods cover reader
boundaries/history/replay/resource redaction, captured availability, actual UID
rejection, real wire reads/interruptions, foreign/epoch/FD rejection and controller
result privacy (Positive,N1,N2,N3,N4,N5,N6,N7,N9,N10 overlap). No ten-run claim.
All owned terminal sessions completed/closed. Existing bluer future-incompatibility
warning remains; no physical peer or user's clipboard accessed.

R3 internal implementation seam: one owned Review component receives an immutable
text resource plus a fresh GTK-thread permission callback. Production callback
checks current known unlock, epoch/transfer/revision/kind/size and panel availability.
Tests inject external lock/resource observations and use only the isolated display's
clipboard. Disable TextView's built-in copy action so it cannot bypass the guarded
Copy button. An admitted read may finish in the owned controller after GUI dismissal;
a presentation token rejects late private results after lock/close/epoch change.
No source/settings action is cancelled as a side effect of text dismissal.

## R3 widget correction before code

Use a nonselectable plain GtkLabel inside a scroller instead of TextView. TextView
owns built-in clipboard actions and selection clipboard behavior; suppressing a
keyboard signal alone would not cover every context-menu/accessibility path. A
nonselectable label has no copy/selection provider, keeps markup disabled and
preserves the same review/read/copy flow. The explicit Copy button copies the full
immutable body. Text is limited to 128KiB before layout. This is an internal widget
choice, not a new receive/clipboard policy or URL action. Reference APIs:
https://docs.gtk.org/gtk4/signal.TextView.copy-clipboard.html and
https://docs.gtk.org/gtk4/method.TextBuffer.remove_selection_clipboard.html.
Keep copy authority in a fresh injected callback, test actual GTK clipboard ownership
on the isolated display, and independently test all key/gate mismatch branches.

R3 first isolated GTK run exposed an actual unintended PRIMARY clipboard change:
the shared label helper makes labels selectable, and focus selected the generic
review heading on presentation. The body was already nonselectable. Correct the
new review heading as well; retain assertions for BOTH clipboard/PRIMARY preservation
through open/update/lock/epoch failures and explicit Copy. Do not weaken the test.

R3 clipboard acknowledgement uses GdkClipboard.set_content with an immutable string
provider and its checked boolean receipt, rather than the void set_text convenience
API. A closed display or provider rejection yields a fixed failure message and no
success claim/retry. The provider contains a copy of the approved body; review close
only drops its own references and does not revoke that explicit copy.

R3 ownership integration review: propagate text_pending into existing GTK settings
and descriptor-ingress admission readiness. Otherwise the CLI could acquire a new
preparation while an uncertain read owns the serialized controller, only to fail
later. Keep source-recovery classification separate: text_pending never pretends a
source upload was admitted. Add one actual interrupted-read/desktop Begin/recovery
fixture and extend the existing pure panel readiness table before closing the slice.

## Actual R3 verification

Native all-target check passed (11.20s). First native suite: 20 methods passed,
1 GTK method failed on the actual PRIMARY heading-focus defect (20.39s build,
0.53s execution). Heading fix affected GTK method passed (5.19s build,0.43s).
After checked clipboard provider receipt: affected GTK method passed (6.09s build,
0.45s); strict native Clippy/build passed (3.68s/5.28s). Earlier lint/build before
that change: 5.94s/18.32s. No unchanged ten-run/repeated-suite claim.

Final single-action admission fixture initially failed compilation because its
shared test-only text fixture module was private; expose it only under cfg(test),
with no production API. Corrected actual read/CLI/recovery method passed (0.18s);
strict core all-target Clippy passed (7.32s). Extended native pure readiness table
passed (0.00s,7.62s build). Final affected strict native lint/build passed
(4.70s/7.31s). Production changes since the preceding suite are limited to the
checked GTK provider/heading and pending-read readiness, verified by these checks.
All owned terminals completed/closed; private broker/display fixtures joined.

One new native pure method covers epoch/identity/revision/direction/phase/kind/size
and unknown/locked/closed desktop inputs. The existing single GTK smoke now covers
Text/URL Full/Compact consent labels, literal untrusted markup/link body, BOTH
clipboard/PRIMARY preservation on open/update/stale/locked/not-ready clicks, actual
Copy button result and actual panel-lock destruction/body release without clearing
explicitly copied text. One new core integration method covers uncertain-read CLI
admission and exact recovery (N4,N5,N7,N9,N10). No user clipboard or physical peers.
GTK provider refusal/closed-display code handles failure honestly; physical Wayland
provider failure/persistence behavior remains acceptance, not a simulated success.
