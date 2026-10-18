# MODE B P0 TCP CLIENT_INTRODUCTION parsing and routing identity

Date: 2026-10-18. Primary mode: **MODE B (Testing)**. Scope: **test-only**
`core_lib/src/hdl/inbound.rs`, the pure BWU TCP introduction validator,
four-byte non-consuming peek, and expected endpoint comparison. Working
branch `dev` only. No production behavior, parser contracts, authentication
policy, desktop install, release pipeline, or `master` changes authorized.

## Stage 1 — scan and exact baseline

Read the project universal `agent.md`, `docs/mode-b-test-plan.md`,
`docs/mode-b-completion-audit-2026-10-18.md`,
`docs/mode-b-p0-bwu-route-2026-10-18.md` and source
`core_lib/src/hdl/inbound.rs` (`validate_client_introduction`,
`peek_client_introduction`, `BwuClientIntroductionIdentity::matches_expected`,
`InboundRequest::do_bandwidth_upgrade`) alongside its existing
`client_introduction_validation_accepts_only_bwu_intro` test.

Rust `#[test]` unit cases run under Core CI. Normal Preflight splits stable
tests from race-sensitive tests; full MODE B separately repeats the entire
suite 10 times. Baseline has valid intro, valid last ID, one partial prefix,
wrong event and empty current ID. The missing independently testable branches
are malformed/missing nested protobuf fields, every prefix size 0–3, truncated
complete frames, malformed payload, explicit 0/over-limit length, trailing
TCP bytes, endpoint matching/mismatching and empty previous ID normalization.

## Stage 2 — risk and invariants

P0, because an unauthenticated network peer controls the plaintext TCP
initial frame used to select a pending BLE session. The parser must not
publish any route identity for corrupt protobuf, wrong frame kind, missing
negotiation/event/identity, or absent current ID. `peek` is deliberately
non-consuming and returns `None` for partial/invalid/ordinary frames.
It must identify the **first** complete valid frame despite trailing
unconsumed bytes. Only exact string equality with current or the explicitly
present last endpoint ID may match the expected session ID.

**Important security limitation:** wire-provided `last_endpoint_id` is
NOT cryptographic proof of peer ownership. The separate upstream
authentication and encrypted BLE-to-TCP handoff security gap documented
in `mode-b-p0-bwu-route-2026-10-18.md` remains OPEN. Tests must not
silently treat an attacker-claimed alias as authenticated.

## Stage 3 — exhaustive scoped artifact inventory

| Artifact / rule | Level, risk | Planned distinct methods |
|---|---|---|
| `validate_client_introduction`: protobuf decode, v1/type/negotiation/event/identity/current ID guards | P0 unit, untrusted input | 1 table-driven negative suite, 7 field variants + malformed bytes |
| `peek_client_introduction`: header 0..3 bytes, zero/huge length, incomplete body, corrupt body, exact complete first frame, trailing data | P0 unit, parser boundary | 1 table-driven boundary suite |
| `BwuClientIntroductionIdentity::matches_expected`: current/last matching, missing/empty/case-wrong/prefix-wrong | P0 unit, route owner string logic | 1 table-driven unit suite with wire empty-alias normalization |
| `BwuRouter` and `do_bandwidth_upgrade`: actual socket routing and encrypted channel | Existing scoped tests + unverified hardware | NO new tests in this pure-parser stage; authentication OPEN |

Three new `Given_When_Then` test methods; table rows each have a
separately labeled error/edge contract. Fixtures construct real generated
protobufs, use actual protobuf decoder, no mock subject, no clock/network,
no sleeps and no additional implementation seam.

## Stage 4 — branch map

- `OfflineFrame::decode` error; missing `v1`; wrong frame type;
  missing negotiation; non-ClientIntroduction event; missing
  `client_introduction`; missing/empty `endpoint_id`:
  `given_invalid_bwu_introduction_fields_when_validated_then_no_identity_is_admitted`.
- `peek_client_introduction`: fewer than four bytes; zero and
  above-SANE length; valid declared length but insufficient body;
  complete valid body; malformed or unrelated complete body; complete
  first frame with extra unread TCP bytes:
  `given_partial_or_malformed_bwu_tcp_prefix_when_peeked_then_only_complete_valid_identity_is_returned`.
- `matches_expected` current=true, last=true, neither=false,
  case-distinct/substring-distinct=false; empty last ID normalizes
  to `None` in wire decode:
  `given_bwu_current_and_last_ids_when_compared_then_only_exact_expected_id_matches`.

## Stage 5 — Positive/N1–N12 matrix

C = existing covered, P = planned here, X = not applicable to this
pure-parser scope, OPEN = separate unresolved integration/security.
Every applicability cell is populated.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Validator | C+P | P | P | X(no I/O) | X(no retries) | X(pure) | P | P | X(no clock/locale) | X(no UI) | P(no unverified ID) | X(no billing) | X(no disk) |
| TCP peek | C+P | P | P | P(truncated TCP bytes) | X(no retries) | X(pure) | P | P | X | P(partial arrival) | P(no route on bad bytes) | X | X |
| ID comparison | C+P | P | X(no length limit here) | X | X | X | P(empty last ID) | P(exact owner) | X | X | P(no similar owner confusion) | X | X |
| Authenticated BLE→TCP migration | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | OPEN | X | OPEN | OPEN | X | OPEN |

## Stage 6 — interaction matrix

Wire kind {BWU / wrong / malformed} × field completeness
{all / each required absent / empty ID} × framing {0..3 header,
0 / oversize / header+partial body / exact / extra next frame}
× route strings {current / old / absent / unrelated / case mismatch}.
Cover independent parser classes without claiming exhaustive
`do_bandwidth_upgrade` hardware/network interaction coverage.

## Stage 7 — expected counts

Exactly 3 new test methods, numerous parameterized assertions.
Overlapping category allocations: Positive 2, N1 3, N2 2,
N3 1, N4 0 X, N5 0 X, N6 3, N7 3, N8 0 X, N9 1,
N10 3, N11 0 X, N12 0 X. Report verified actual counts
only after same-source CI.

## Stage 8 — per-production-file map

`core_lib/src/hdl/inbound.rs`: three pure functions covered
here (`validate_client_introduction`, `peek_client_introduction`,
`BwuClientIntroductionIdentity::matches_expected`).
Remaining: `do_bandwidth_upgrade` actual notifier/session/crypto,
its timeout/drain/swap and every other inbound production branch outside
this exact scope. Existing `core_lib/src/hdl/bwu.rs` real-socket
owner tests remain intact; not a substitute for caller authentication.
Global per-file/function/branch map and Stage 12 remain incomplete.

## Stage 10 — mutation review (expected)

Deleting any required protobuf guard, accepting an empty current ID,
changing the 4-byte threshold, allowing a zero/over-limit length,
returning identity for truncated body, losing first-frame handling,
or replacing exact string comparison with substring/ignore-case
must fail assertions. Executed mutation engine is NOT claimed.

## Stages and rollback

1. [x] Scan source, risks and existing tests; record Stages 1–8 **before** code.
2. [x] Add all three scoped table-driven methods (no production edits).
3. [ ] Validate CI on exact source; debug failing **tests**, never soften
   a correct security assertion to accommodate a defect.
4. [ ] Update this ledger, `docs/current-state.md`, global MODE B gaps
   and confirmed results; maintain global Stage 12 = NO.
Rollback: revert only test and documentation commits.

## Implementation handoff (unverified CI checkpoint)

- Documentation-before-code commit: `c192a608`.
- Three test-only functions added: `ea84a83e`.
- Original GitHub Rust format job reported two cosmetic differences; fixed
  without weakening assertions at `c6484ec8`.
- Independent exact-name non-cancelling 10-run workflow:
  `.github/workflows/p0-client-introduction.yml`, commit `216d0fac`.
  [Run 39085557977](https://github.com/morozhenko-apps/rquickshare/actions/runs/39085557977).
  **Not yet verified here:** compiled inventory, ten attempts, overall result.
- Other simultaneous `dev` commits may cancel ordinary Preflight; do not
  treat a cancellation as a test failure or count it as a passing attempt.

Rollback remains documentation + test-only source + isolated CI workflow.
