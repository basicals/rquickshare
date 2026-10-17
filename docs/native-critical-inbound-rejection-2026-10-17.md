# Critical incoming reject / no-publication protocol regression

Date 2026-10-17. PRIMARY MODE B (Testing), essential release
priority #1 (consent with a current phone) and #2 (safe interrupted
failure). Source-only GitHub `dev`. No installed native29, machine,
Pixel, Google Windows, `master`, packages, signing or actual
external transfer is touched. This is a **critical behavior
contract**, not a coverage-percentage exercise.

## Repository and evidence scan

User SSOT `agent.md`, `docs/native-essential-priorities.md`,
`docs/native-essential-acceptance.md`,
`docs/native-receive-reliability.md`, current-state and
`core_lib/src/hdl/inbound.rs` plus its included
`inbound/receive_reliability.rs` test harness.
Actual native29 two-file Pixel ↔ Linux roundtrip was user-confirmed,
but receiver **rejection and interrupted rejection** remain
UNTES​TED in the physical acceptance checklist.

Incoming Introduction parses two real file entries, pins the
selected root and requests manual consent without creating a
public file. The existing managed consent handler sends
`ConnectionResponseFrame::Reject` as encrypted Nearby
`PayloadTransferFrame`, followed by terminal `State::Rejected`.
The real Linux root stages with O_TMPFILE **only** during Accept.
`finish_received` must not retain file owner/receipt after
reject, including when the peer socket dies while writing reject.

Existing direct tests cover: accepted two-file publication,
mid-file abort, kernel ENOSPC/write errors, unsafe root/ancestor,
invalid payload ids/limits, terminal result preservation. No
single source-confirmed test presently asserts the **managed
user-decline → actual encrypted Reject wire → no disk, no
receipt** path for a two-file request, or decline against a
closed peer. Do not change protocol, UI or storage logic
without a separately scoped MODE A decision.

## Invariants

1. Before explicit consent there are zero externally visible
   final names, zero staged writable file owners and zero
   received-file read capabilities.
2. Managed `respond(token, revision, false)` consumes the live
   consent exactly once; the daemon applies a **negative** answer.
3. A connected peer receives an actual cryptographically
   framed `Reject` response (NOT `Accept`), complete with
   correct terminal frame and no file-data frame.
4. `Rejected` is a terminal local state even if network write
   fails or a late attempt to mark disconnected occurs.
5. No source/destination content, file bytes or user PIN are
   exposed as test logs. The test only reads synthetic bytes
   from a temporary peer.
6. No automatic retry, publication, accidental overwrite
   or pending file-result capability is fabricated.

## Exhaustive scoped artifact inventory, branch map and estimate

| Artifact / branch | Risk and level | New positive/negative evidence |
|---|---|---|
| `SessionHub::open/respond/apply`, managed consent false | P0 Integration | 2 actual consent transactions with false; revision stays stable, receipt await completes |
| `InboundRequest::process_introduction`: 2 files | P0 Integration | existing valid two-member metadata accepted, pinned destination; no staging before consent |
| `InboundRequest::apply_consent(false)` | P0 Integration | 1 healthy peer: terminal Rejected, `AppError::NotAnError`; 1 peer-drop: rejection terminal despite send failure |
| `send_encrypted_frame` two encrypted payload chunks | P0 Protocol integration | use **real sender and real receiver decrypt**; compare actual Nearby Response::Reject, payload count, terminal offset/flag |
| `finish_received` and SessionControl snapshot | P0 Storage / security | both branches: zero writable owners, zero published files/receipts; no active file authority or retry |
| `record_termination` after reject | P0 State | late Disconnected cannot override Rejected |
| File receive writable stage | P0 Storage | Assert no `a.bin`, no `b.bin`, root remains empty |
| Existing positive accept, ENOSPC, partial result | P0 Regression | existing tests remain unchanged and re-run |

**Planned 2 new asynchronous scenario methods**, both on
real duplex sockets (not mocked protocol or domain logic),
real managed session, a temporary actual Linux directory
and production crypto/framing. Zero new product APIs and no
production source modifications. The paired `Accept` path
is already tested by independent prior cases and must remain
green; no artificial UI adapter or physical peer claim.

### Positive/N1–N12 matrix

P = new planned assertions, E = existing adjacent scoped
tests re-run; X = inapplicable and why.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Negative consent + wire response | P | E | E | P | P | E | E | P | X | P | P | X | P |
| No-publication + terminal receipt | P | E | E | P | P | E | E | P | X | P | P | X | P |

N1/2: prior malformed and bounded introduction/payload tests;
N3: dropped real peer duplex; N4: no replay or implicit
accept/retry, late terminal state; N5: pre-existing
concurrent session/admission tests; N6: prior damaged
frame/metadata fixtures; N7: actual managed consent token
required, no unauthorized accept; N8: no date/time logic;
N9: real user declines/cuts connection; N10: byte/PIN
privacy check; N11: no billing; N12: actual empty Linux
directory plus existing staged filesystem fault cases.
No category is silently treated as achieved global coverage.

### Interactions and mutation review

Interactions: consent false × {healthy duplex, dropped duplex};
two-file manifest × never Accepted; wire failure × absorbing
Rejected; pending file metadata × terminal snapshot.
A mutant sending `Accept`, setting `ReceivingFiles`,
pre-staging a file, persisting a receipt on rejection or
changing terminal outcome to Disconnected **must** fail
at least one explicit assertion. Use the real production
frame decryptor to ensure the wire status is genuine.

Scoped per-production-file coverage map:
`core_lib/src/hdl/inbound.rs`: specifically the negative
branch of `apply_consent` and existing
`reject_transfer`/send framing, before/after terminal
`finish_received`. Adjacent `sessions.rs`/session
control, `receive.rs`, `received_files.rs` behavior is
exercised through real owners but not exhaustively
remapped here. Stage 12 **overall NO** remains.

## Milestones / risks / rollback

1. [x] Inspect real priority and code; record negative
   consent branch and invariant inventory first.
2. [x] Implement 2 exact, single-scenario regression tests
   under the established `receive_reliability.rs` fixture.
   If failure shows production defect, stop Mode B and
   separately plan Mode A fix. Never weaken assertions.
3. [ ] Observe headless Preflight (Core stable 1x +
   race-sensitive 10x) and separate Native GTK if needed
   (no GTK production changes). Record exact outcomes;
   do not mark running jobs successful.
4. [ ] Update this ledger and `docs/current-state.md`;
   preserve physical Pixel refusal and Windows checks
   as UNTESTED unless the user personally runs them.

Rollback removes only two test methods and ledger;
no product behavior, filesystem API, network protocol,
packaging or release changes anticipated.

## Source-only execution checkpoint

Test-only source `da71cdcd4c79c214d78b6080291710867141fb8f`
implements both preregistered managed negative-consent scenarios
in `core_lib/src/hdl/inbound/receive_reliability.rs`.
The original accept/reliability assertions and all product code
are unchanged. Test 1 uses the real two-chunk encrypted Nearby
frame and a peer-side `decrypt_secure_message` to assert exact
`Response::Reject` and terminal offset/flag, with no extra
frame. Test 2 destroys the duplex peer before Reject is sent and
asserts sticky `Rejected` and zero visible file/result authority.

Exact-source CI:
[Preflight run 39075819730](https://github.com/morozhenko-apps/rquickshare/actions/runs/39075819730),
[Native GTK UI verification 39075819773](https://github.com/morozhenko-apps/rquickshare/actions/runs/39075819773).
At the initial checkpoint the **stable Core Rust suite**, Rust
format, frontend, core Clippy and Native GTK were successful;
the Core stress repetitions and legacy Tauri Clippy were
still active. Those unfinished jobs must not be marked green
until their final statuses/logs are checked.

Hardware status remains **UNTESTED** for actual Pixel rejection,
peer app close during Reject and Google Windows compatibility.
The existing installed native29 was not changed; the newly
tested source is not an installed device-acceptance result.
