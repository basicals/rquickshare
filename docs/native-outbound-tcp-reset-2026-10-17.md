# Native outbound TCP-reset and source-ownership verification

2026-10-17. MODE B: Testing. Direct dev, headless only, essential reliability
priority 2. No user computer/Pixel, service, install, publication, signing,
automatic retry, master promotion, protocol change, production architecture
change or unrequested scope expansion.

## Repository scan and actual interfaces

Relevant SSOT: `docs/native-essential-priorities.md`,
`docs/native-outbound-empty-files-2026-10-17.md`,
`docs/native-transfer-supervision.md`,
`docs/native-managed-dispatch.md`, and the user-maintained
`agent.md` testing mode. The older guide referenced by repository docs at
`/home/morozhenko/.codex/AGENTS.md` is not inside this repo or available
to the remote connector. This ledger supplies the scope-specific inventory.

Existing tests use Rust `#[tokio::test]`, real TCP loopback and
AES/HMAC-encrypted `OfflineFrame` readers; CI uses Preflight (`cargo test`,
format, Clippy, frontend, Python contracts) and independent native GTK
checks. Current starting dev is `4019937e`. Last previous Preflight
run 39057293853 completed all five jobs successfully (699 core tests, 8
integration). GTK run 39057293868 passed 52.

Observed code:
- `OutboundRequest::process_consent(Accept)` retains each original
  read-only opened source, emits FILE Data chunks, increments the local
  byte counter after a successful socket write, sends lastChunk once and
  then removes that member. Any encryption/write error propagates before
  final Finished.
- `OutboundRequest::record_termination` updates a nonterminal request
  to Disconnected and projects the managed session's terminal state;
  it never attempts to resend.
- `manager/session.rs::connect` calls the actual request handler,
  classifies ordinary errors into Disconnected with stable metadata, and
  drops the request before the joined worker completes.
- `ManagedSender` has explicit admission and cancellation and retains
  ownership until its release event; no retry is dispatched on TCP failure.
- Existing tests exercise no-data RST analogues (missing encryption key),
  cancelled native request and prepared source change; they do **not**
  force a real TCP RST during an accepted outgoing FILE body.

One important distinction: a successful local TCP `write_all` does not
certify remote file durability. This test checks absence of *false local
completion after an observed write error*, never remote Google delivery
guarantees.

## Risk and invariant analysis

High: a peer disconnect during an accepted large FILE send must not report
Finished, retire all incomplete working entries or damage/unlink original
opened files. Once the core request is terminated/dropped, it must release
the captured source handles. No fresh connection, new transfer ID,
unexpected retry, credential/PII disclosure or UI success is authorized.

Medium: TCP can buffer writes after remote shutdown; a deterministic
abortive peer close (SO_LINGER zero) and small test-only socket send buffer
constrain the race. We must not assert a particular number of acknowledged
bytes or an exact Linux errno; only require a real I/O failure and
`transferred_bytes < total_bytes`. An outer watchdog only diagnoses a
stalled test; there is no sleep/polling delay as test synchronization.
Kernel TCP reset is not a mock of the handler or the network protocol.

## Exhaustive scoped artifact inventory and branch map

This MODE B slice deliberately inventories the *outgoing accepted-file
RST/ownership flow*; the whole repository's public APIs and unrelated
flows have their own ledgers and are not claimed as covered here.

| Production file/artifact and branch | Test level | Existing/this stage | Risk |
|---|---|---|---|
| `hdl/outbound.rs`: Accept -> SendingFiles -> body write/error vs full EOF -> Finished | encrypted TCP integration | completed body/EOF existing; RST before first body / after first body planned | critical |
| `hdl/outbound.rs`: nonempty vs zero-byte FILE, exact ID, bytes, EOF, no early retirement | encrypted TCP integration | existing 699-source tests; RST members planned | high |
| `hdl/outbound.rs`: error -> record_termination(Disconnected), repeated terminal absorb | integration + managed session | source error existing; RST and final snapshot planned | high |
| `file_reader.rs`: exact advertised size/read offset/EOF/permission failure | unit | existing tests retained | high |
| `file_source.rs` / `prepared_source.rs`: retained original FD, no display-path reopen, user-source preserved on request drop | filesystem integration | existing native tests; RST weak Arc + source bytes planned | critical |
| `manager/session.rs`: nonterminal network error -> Disconnected, final suppression, request drop | loopback manager integration | existing supervision tests; direct request contract cross-check planned | high |
| `managed.rs`: release acknowledgment distinct from Finished; no auto retry | managed integration | existing tests; new RST must not call dispatch | high |
| `service/operations.rs`: command receipt replay vs repeated send | unit | existing operation tests; not modified | high |
| Native UI/IPC and package | unchanged | GTK/contract CI only, no new scenario | medium |

Every relevant branch above is either previously covered or mapped to the
real RST scenarios. Complete repository-wide coverage and mutation
instrumentation are outside this focused slice, and the MODE B global
completion gate remains open.

## Positive and N1-N12 scoped matrix

Entries: E = previously tested; P = planned new cases; X = inapplicable.
The scope is one accepted outgoing FILE send, not an input parser rewrite.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Actual encrypted send handler | E | E | E | P | E | P | E | E | X | P | E | X | E |
| Source FD/transfer state | E | E | E | P | E | P | E | E | X | P | E | X | E |
| Manager/managed error projection | E | E | E | P | E | E | E | E | X | P | E | X | X |

N8 X: no clock, DST, locale or pricing transformation. N11 X:
no monetization. Manager N12 X: manager does not own filesystem writes.
N1/N2/N6/N7/N10 are existing data/identity/privacy validations, not
new RST parser cases. N4 existing same-ID/retry controls: the new
interrupted request never replays itself. N5 covers RST raced against
active output, one-file body in flight; no new parallel transfer.
N9 covers external peer application exit, local cancellation remains
in existing tests. N12 in sender/source means preserving disk content
and FD lifetime, not simulated disk-full.

## Interaction matrix, expected count, coverage estimate

Two direct encrypted loopback cases are planned: (A) remote abort after
valid introduction, before FILE bytes; (B) remote abort after observing
a first encrypted FILE data chunk of a large regular source. In both,
cross request state [SendingFiles, post-Disconnected] with retained
source authority [request still owns, after request drop].
Positive baseline nonempty and zero-byte full sends remain separate
existing tests. Expected two new scenario functions and no new
production dependency/ABI.

For each verify manifest, unchanged owner/source bytes, real I/O error
not a timeout, non-Finished state, strict progress below total,
nonterminal map before error cleanup and Disconnected once terminated,
no source deletion, strong Arc lifetime while handler exists and weak
release after request drop. For B additionally verify the first actual
encrypted FILE frame and exact source payload byte prefix.

Mutations detected: swallowing socket error; marking Finished on write
error; retiring all files prematurely; ignoring transfer metadata
progress; releasing source authority while request still needs it;
retaining source indefinitely after request drop; reusing an old final
state. No production mutation testing has been run. If a test is
nondeterministic under kernel scheduling, constrain only the peer's
socket buffers rather than weakening the invariant.

## Milestones and rollback

1. [x] Scan code/docs, CI framework, existing fixtures and failures,
   record every scoped testable branch, risk and interaction before edits.
2. [ ] Implement two actual encrypted loopback RST/owned-source tests,
   with strict assertions and no mock of the request/manager domain.
3. [ ] Run real CI on exact source; investigate failures. If a
   production defect is confirmed, stop MODE B and document an MODE A
   repair before altering behavior; do not weaken assertions.
4. [ ] Capture verified result counts, per-production-file coverage and
   mutation review, update the ledger and `docs/current-state.md`.
   Leave MODE B 10-repeat/global repository gate OPEN unless fully
   executed. Preserve native29, native30, master, all physical gates.

Rollback: revert the isolated test commit and ledger. No user data or
installed binary changes; no automatic resend, timeout policy, protocol
or public API decision is authorized by this test stage.

## Results

Execution pending.
