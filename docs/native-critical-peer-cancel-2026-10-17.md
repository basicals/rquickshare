# Critical peer Cancel after file consent: completed-member safety

2026-10-17. Primary MODE B (Testing). User priority is critical
release behavior, not achieving an arbitrary coverage number.
Direct `dev` source-only GitHub work. No host process,
native29 installed package, Android/Windows peer, release,
signing or `master` changes.

## Scan, reason, observed gap

Sources: `agent.md` SSOT, essential native priorities/
acceptance, `core_lib/src/hdl/inbound.rs`,
`core_lib/src/hdl/inbound/receive_reliability.rs`,
`core_lib/src/received_files.rs` and existing source
tests. Original handler detects the protocol's
`sharing_nearby::v1_frame::FrameType::Cancel` during
`process_transfer_setup`, observes terminal `Cancelled`,
sends protocol `Disconnection`, then exits.
`finish_received` discards anonymous in-progress
staged members while preserving published completed
member receipts. Existing offline tests prove
mid-file **TCP EOF**, invalid last chunks and storage
failure, plus `Cancel` before file consent in a bytes
fixture. No source-confirmed test combines real
managed accepted two-file transfer, published first
member, untrusted remote Cancel and the terminal
file receipt/disconnection in that order.

## Branches, level, invariants and planned count

| Scoped artifact / branch | Risk and level | Verification |
|---|---|---|
| `admitted` managed session with two-file manifest and actual Accept | P0 integration | accepted consent, exactly two encrypted response chunks |
| `process_offline_frame` with real byte-payload sharing Cancel | P0 protocol integration | parse untrusted Cancel through production bytes and sharing-frame handler; no unrelated file result |
| `process_transfer_setup` cancel path | P0 protocol state | emit encrypted Disconnection and stick `Cancelled` |
| `finish_received` and `PendingFiles::finish_partial` | P0 storage/security | zero published member → no receipt; one complete member → exact immutable read receipt |
| `record_termination` post cancel | P0 state | cannot overwrite with Disconnected or Finished |
| Existing inbound EOF and accepted completion | P0 regression | remain unchanged and included in normal suites |

One table-driven async test method with **two distinct
critical variants**: (a) partial first member with none
completed, (b) first durable member and second partially
staged. Data are disposable 1/3-byte local temp fixtures,
real cryptographic frames and session owners. No fake
domain logic, new API, product/refactor, or changed
test assertion. Real peer test remains separately untested.

### Positive/N1–N12 matrix

| Scope | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Managed accepted then peer Cancel | E | E | E | E | P | E | E | P | X | P | P | X | P |
| Atomic partial outcome and receipt | P | E | P | E | P | E | E | P | X | P | P | X | P |

N1/2 existing malformed and size-bound metadata and zero/one
files; N3 existing real EOF/socket reset; N4 no implicit
retry or late terminal override; N5 existing manager
race/cancellation; N6 existing invalid framing; N7
actual managed consent and scoped completed-file
read descriptor; N8 no clock math; N9 user/peer
cancel; N10 no printed private bytes; N11 no billing;
N12 anonymous staged real filesystem and earlier ENOSPC
matrix. Other categories are adjacent *existing*
coverage, not new proof of all product branches.

### Interaction and mutation review

Two-file payload + authenticated Accept × real
Cancel before/after first member publication ×
incomplete second inode × encrypted Disconnection
× terminal state × no automatic restart.
Changing Cancel to Finished, unlinking a completed
member, publishing partial second, fabricating
receipt for zero completed, or losing the expected
Disconnection must fail an assertion.

Per-production-file risk map:
`inbound.rs` shares active Cancel branch, file transfer
state machine and cleanup; `received_files.rs`
retains immutable per-member capabilities; `receive.rs`
stages O_TMPFILE. Untested hardware BlueZ/Google peer
and full Stage 12 mapping remain OPEN.

## Plan and rollback

1. [x] Scan critical priority, code, previous regressions;
   record specific missing branch and two-variant matrix.
2. [x] Implement real protocol + actual on-disk status
   test in existing inbound reliability fixture.
3. [x] Verify both variants on same-source CI, no
   weakening; correct test fixture only if needed.
4. [ ] Record evidence and update `current-state.md`
   with critical backlog next, keeping hardware
   cancellation/retry UNTESTED.
Rollback removes one test method and this ledger,
without changing runtime/network/OS behavior.

## Execution — source committed, CI pending

Test-only source commit `65f63e7232514a41410ee8fc296fe1847eda432d`
implements one table-driven two-variant async test in the actual
inbound receive-reliability fixture. No production files,
contract or public behavior changed. The test:
(1) checks actual encrypted two-chunk Accept response;
(2) routes a real Nearby Cancel body through the byte
payload parser after file admission; (3) checks real encrypted
Disconnection response and absence of subsequent frames;
(4) verifies zero-file vs preserved-first-file outcomes,
exact durable file bytes and read-only received-file
capability, no partially written second file, zero
open stage owners, and an absorbing Cancelled state.

Actions automatically started
[Preflight 39076370296](https://github.com/morozhenko-apps/rquickshare/actions/runs/39076370296)
and
[Native GTK 39076370294](https://github.com/morozhenko-apps/rquickshare/actions/runs/39076370294).
**Both were still executing at the source checkpoint.**
Do not count this new cancel case as verified until the
exact-source CI results are inspected; no physical
Pixel cancellation or installed release is implied.

## First CI outcome and test-contract correction

First source `65f63e72`, Preflight
https://github.com/morozhenko-apps/rquickshare/actions/runs/39076370296
had Core **698 passed / 1 failed** in the newly added
`given_remote_cancel_after_accept_before_or_after_first_file_when_handled_then_only_durable_members_survive`.
Actual failure: test at line 324 expected transferred_bytes 1,
actual 2, after first complete 1-byte file and an additional
1 byte written into the second anonymous staged file.
This was a **faulty test expectation**. The existing
`TransferDetails::transferred_bytes` documented contract
is bytes actually written, not bytes durably published.
The actual terminal `Cancelled`, encrypted protocol
Disconnection, cleanup and unique first-file preservation
had already passed earlier assertions in that test.
The original assertion failed before later receipt checks,
so those were not yet validated by that run.

Independent `ReceivedFiles` API definitions confirm a
partial receipt must satisfy `subset_of(details)` but **not**
`valid_for(details)`, which requires all announced members.
The repair changes test expectations to exactly
**2 written bytes for one complete + one incomplete member**
(and zero when no chunks have arrived), and now asserts
both `subset_of` and `!valid_for` for partial results.
It never weakens no-publication, stale-terminal, wire-code,
received-file integrity or empty stage-owner checks.
No production change or ambiguous business rule choice
is made. Test-only correction `b69a6581`.
The final headless run for this corrected source is
required before claiming a pass.

This is distinct from a tested protocol defect. The
red first run remains in the evidence record; no test
was skipped, made permissive or quarantined.

## Verified corrected-source P0 Cancel behavior

After the contract-level fixture repair, source
`b69a658115330d53e2e7f46ed575505f16078e38`
passed the real
[Core Rust job 117525018812](https://github.com/morozhenko-apps/rquickshare/actions/runs/39076613994).
The new test appears individually `ok` in the Rust
runner, and its table loop covers **both** preregistered
variants. Aggregate: **699/0** stable Rust unit cases,
**4/0 + 4/0** integration binaries, and ten sequential
race-sensitive group iterations with **80/0** exact
single-test runs. Formatter, frontend and strict
Core Clippy passed. Separate
[Native GTK run 39076614010](https://github.com/morozhenko-apps/rquickshare/actions/runs/39076614010)
was successful, too. Legacy Tauri Clippy remained
in-progress on the first correction checkpoint, so
do not infer complete 5/5 Preflight from the Core
and GTK evidence alone.

This closes the **critical source-level** Cancel
expectations: real encrypted Accept, actual byte
payload Cancel parsing, encrypted Disconnection,
no false Finished, zero published files and no
fabricated receipt before the first completion, or
exactly one retained immutable file capability
with a partially written second member discarded.
Progress correctly counts both already written
bytes while the partial receipt is only a subset
of the announced full manifest. No domain changes
or policy decisions were introduced.

The actual Android/Google Windows remote Cancel,
lost peer and subsequent **explicit** retry are
still UNTESTED in physical installed-device
acceptance. The mandatory final full MODE B 10×
on the final post-hardware source and global
Stage 12 branch inventory also remain OPEN.
