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
2. [ ] Implement real protocol + actual on-disk status
   test in existing inbound reliability fixture.
3. [ ] Verify both variants on same-source CI, no
   weakening; correct test fixture only if needed.
4. [ ] Record evidence and update `current-state.md`
   with critical backlog next, keeping hardware
   cancellation/retry UNTESTED.
Rollback removes one test method and this ledger,
without changing runtime/network/OS behavior.

## Execution

Pending.
