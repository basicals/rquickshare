# Frame prefix interruption repair

Mode A, 2026-10-17. Active checkout: dev, starting checkpoint56e320d.
This is the first local reliability slice of the essential peer acceptance work.
Native28 remains installed; no peer compatibility or new installation is claimed.

## Analysis and invariants

Both inbound and outbound `handle` race a local four-byte `read_exact` against
control messages. A pending read can already have consumed one to three bytes.
If a control branch wins, the future and its local prefix are discarded; the next
handle starts at the wrong byte. Inbound native consent has the same race as
legacy broadcast handling. Outer manager cancellation terminates the request and
does not resume its frame reader; body reads are not raced against resumable
control handling. Plain BWU reads are outside this repair.

Related ownership references: [supervision](native-transfer-supervision.md),
[managed dispatch](native-managed-dispatch.md),
[receive publication](native-receive-reliability.md),
[physical acceptance](native-essential-acceptance.md).

Preserve framing, size bounds, crypto, consent validation and command ordering.
Keep partial prefix bytes in a private request-owned reader. Use cancellation-safe
single reads and commit the consumed count before polling again. Reset only when
all four bytes have been returned. No new public API, setting or wire format.
This is a small internal root-cause repair under the maintenance authorization.

## Plan and rollback

1. [x] Analyze resumable select boundaries and record this plan before code.
2. [x] Add focused regressions for all three split positions and actual control
   interruptions; record the pre-fix failure once.
3. [x] Introduce one private prefix reader and use it in both request types.
   Verify cancellation, complete/reset, EOF and read errors without sleeps.
4. [x] Run affected core checks once; record actual results and remaining peer
   checks. Prepare disposable ordinary-content fixtures locally without changing
   the installed app, prepared sources or sending to any peer.
5. [x] Commit the repair/evidence atomically and update current-state.

Risks: losing or replaying bytes when resetting the reader, swallowing EOF or
read errors, or changing frame limits. Handler regressions must assert actual
parsed protocol outcomes; helper tests alone are insufficient. Rollback: revert
the repair commit, leaving its preceding plan and installed native28 intact.

## Scoped verification inventory and branches

This is focused Mode A regression verification, not a full Mode B coverage gate.

| Artifact | Planned cases / assertion | Categories |
|---|---|---|
| Private prefix reader | Cancel/resume at0,1,2,3 bytes; exact bytes; two consecutive headers reset; EOF at0,1,2,3 bytes; I/O error preserved | Positive, N2, N3, N5, N9 |
| Inbound handle | Interrupt at1,2,3 bytes using irrelevant legacy events and a queued native command invalidated by current phase; resume a real connection request exactly | Positive, N5, N7, N9 |
| Outbound handle | Interrupt at1,2,3 bytes with irrelevant legacy events; resume a real connection rejection with the exact rejection result | Positive, N5, N9 |

Branches: complete versus incomplete prefix; zero read versus nonzero read versus
error; all partial offsets; reset after completion; both select integrations and
the inbound native mailbox. No new payload validation/storage/time/account rules:
N1/N4/N6/N8/N10/N11/N12 are outside this private byte-retention change. Existing
frame length/crypto/consent suites remain applicable and unchanged. Assertions
must kill resetting the offset early, dropping an increment, failing to reset,
ignoring EOF/error, or leaving either handler on the old local `read_exact`.

## Verification and continuation

Before the repair, the two new actual-handler regressions failed with `early eof`
instead of the intended connection request/rejection outcome (0 passed/2 failed,
26.54s build,0.00s execution). After the repair,
`TMPDIR="$PWD/target/native23-test-tmp" cargo test --all-features --lib hdl:: -- --nocapture`
passed132/0/0 in0.48s (22.85s build). This includes7 new functions/23 table cases:
14 reader cases,6 inbound legacy/native cases and3 outbound cases. Existing real
managed BLE consent, crypto and partial-file publication tests also passed in
that affected group. No real Bluetooth peer was used by this run.

Strict `cargo clippy --all-features --all-targets -- -D warnings` passed in9.23s.
The existing dependency `bluer0.17.3` emits a future-compatibility advisory; no
new warning was accepted. After strengthening the native receipt assertion to
exact `SessionError::Stale`, that affected test passed1/0 in0.00s (13.96s build)
and strict Clippy passed again in4.53s. No repeated stability loop was run.
Stable rustfmt reports unsupported nightly configuration options, as before.

Local source-only fixtures are at
`/home/morozhenko/Documents/ChatGPT/quickshare и nativepair/outputs/rquickshare-peer-check-2026-10-17`.
Five files total65935 bytes: single file, two-file selection including space/UTF-8
names and a two-level folder. `manifest.json` holds independently reread sizes
and SHA-256; `text-to-copy.txt` covers multiline UTF-8 text and a non-opening URL.
Creation used an exclusive new directory; originals/prepared sources are intact.

User reports Pixel6a with Android17; Quick Share version is unknown and Windows
is unavailable. This identifies the available peer, not a new transfer result.
Next live checks: send the two `multiple` files Linux -> Pixel, return them
Pixel -> Linux, compare exact bytes; then folder/text and explicit decline/cancel.
Use the installed28 checkpoint until an explicitly managed upgrade; the new
source repair is not present in its binary. Do not label those physical checks
as validation of the repair. Network-loss/disk/lifecycle matrix stays open.

No automatic retry, new trust, radio/account change, installation, signing or
publishing was performed. No terminal session remains active after checks.
