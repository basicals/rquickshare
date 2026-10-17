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
2. [ ] Add focused regressions for all three split positions and actual control
   interruptions; record the pre-fix failure once.
3. [ ] Introduce one private prefix reader and use it in both request types.
   Verify cancellation, complete/reset, EOF and read errors without sleeps.
4. [ ] Run affected core checks once; record actual results and remaining peer
   checks. Prepare disposable ordinary-content fixtures locally without changing
   the installed app, prepared sources or sending to any peer.
5. [ ] Commit the repair/evidence atomically and update current-state.

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

Pending. Physical Android/Google Windows versions and content/cancel matrix are
still unverified. User was asked for available peer details; local work does not
depend on them. No automatic retries, radio changes, release signing or publishing.
