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

## Delivery follow-up plan

After source repair commit e31b3d5, prepare a distinct native29 local package so
the available Pixel can exercise the repaired code. This is the next existing
installed-test workflow, not a new distribution channel or UI decision.

1. [x] Increment the tracked package revision to29 and commit it before building.
2. [x] Build/verify the locked release artifact with the established private GTK
   sysroot and actual shlibs metadata. Preserve native28 for rollback.
3. [x] Read fresh installed version, service owners and global idle/source/tray
   gate. If preparation or nonterminal work exists, preserve it and ask the user;
   do not restart. Pin the exact observed owners before any stop.
4. [x] If idle, join only the two rQuickShare units, request system package
   authorization through Pinentry/Polkit, upgrade through APT and restore normal
   launch. Read back all14 files, units, startup preference and passive tray/API.
   Never ask for a password in chat. Failure/cancel restores normal launch.
5. [x] Record installed evidence and source preparation.
6. [x] Record the first user-operated two-file Pixel check.

Rollback is the exact preserved native28 package after the same idle gate. No
received content/history/settings are removed; login startup remains unchanged.

Native29 package was built from c05e776 and verified offline by the existing
builder:6660010 bytes, SHA-256
f22a8c1eb7726c9fba3c983d4ab75bfd1435ed64ef988ca3ad8811bae1a79b88.
All three locked release binaries and14-file inventory passed. The native client's
existing no-experimental core dependency emits9 unused-code warnings; the core
all-features strict check above is separate. `dpkg-shlibdeps` reports the existing
libc usr-merge diversion warning; all dependencies were resolved and verified.

At14:31:01Z, native28 had zero reservations/handles/bytes/owned selections and
zero nonterminal transfers; one retained terminal entry and4 nearby devices.
Observed daemon/desktop owners were2885495/2885499, startup disabled. The actual
upgrade repeats the full idle/tray gate and checks unchanged unit owners before
joining the two units. Native28 rollback bytes were rehashed. Its task-local
upgrade script restores normal launch even on installation error/cancellation.
APT then upgraded28 ->29 successfully. Both units are active with new owners
2972173/2972174; all14 files match exact package bytes/modes/ownership. Passive
checks at14:32:03Z found one Active14-entry tray, valid Visibility2/Journal1/
Batch1/AcquireResult interfaces and11 retained journal records. Login startup
remains disabled; no runtime trace override. See
[installation evidence](evidence/native29-installation-2026-10-17.json).
The exact package and SHA-256 sidecar are copied under distinct output names.
Physical Pixel results remain pending; next prepare only the known two-file
fixture through the existing installed Send entry point after a fresh empty-source
and no-active-transfer check. The user selects the recipient and explicitly sends.

The installed `rquickshare send -- <two exact fixture paths>` returned
`Files prepared. Choose a recipient in the panel to send.` No recipient or Send
was selected by the agent. Subsequent snapshot: synchronized/running, one native
source reservation,4 reserved handles,65826 selected bytes, zero transfers.
The fixture manifest independently totals65826 bytes for these two members.
A first local diagnostic incorrectly expected legacy `owned_selections` to be1;
it is0 for this native prepared-source path. This diagnostic assumption is not a
failed transfer and did not trigger a source replacement/retry. Preparation and
physical transfer are distinct. The user subsequently confirmed successful
receipt of the prepared files on the available Android peer. Phone hashes are
not independently verified. Journal readback shows two finished outbound entries
and one disconnected attempt, with erroneous zero content counts in each; do not
infer the cause of that attempt. The concrete metadata defect is tracked in
[outbound metadata repair](native-outbound-metadata-2026-10-17.md).


## Offline CI follow-up — 2026-10-17

Mode A, source-format maintenance; no host, peer or installed-package actions.
The dev Preflight run 39048046221 failed in Rust format before later Python
contract checks could run. The exact nightly rustfmt diff at
`core_lib/src/hdl/frame_header.rs` places `tokio::io::AsyncWriteExt` before
`super::*` with one blank line between them. Core tests and core Clippy passed
on the unformatted candidate. The observed failure is import formatting, not
a protocol regression or evidence of peer interoperability.

Plan:
1. [x] Inspect the failed CI job log and current file; identify the only emitted
   diff instead of changing formatter configuration or unrelated code.
2. [x] Apply the exact import-order/blank-line correction in the existing test
   module. Preserve all test assertions and production logic.
3. [ ] Verify the next dev Preflight format, Python contract and remaining jobs;
   record observed results without claiming a local nightly build.
4. [ ] Keep the release/installed native29 state and the native30 package/user
   acceptance gates unchanged.

Risk is limited to incidental source-test formatting. Rollback is an ordinary
revert of the standalone formatting commit. No wire, security, privacy, stored
data, UI, packaging, daemon state, or platform behavior changes.

### Verified headless follow-up

The standalone test-only import change is commit `3c606db`. At that exact
commit, dev Preflight run 39048444812 passed the nightly Rust format job,
including 54 offline Python contract tests. Its core Rust job passed 693
library tests and 8 integration tests; core Clippy and frontend jobs passed.
Native GTK run 39048444825 passed formatting, strict lint and all 52 isolated
widget/controller tests once. Those are CI observations, not local machine
or physical peer evidence, and do not constitute the 10-run Mode B gate.
Native30 remains source-only; installed native29 is unchanged.
The separate legacy Tauri Clippy job was still in progress when these results
were first recorded.
