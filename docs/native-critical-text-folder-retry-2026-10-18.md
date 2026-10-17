# P0 ordinary text, nested folders, and explicit retry — 2026-10-18

Primary MODE B (Testing); final acceptance after these critical offline
scenarios must run the existing **strict full Mode B 10x** on the final
source, not merely the ordinary stable 1x / risk-sensitive 10x
Preflight. `dev` source only. `master`, installed native29, daemon,
user KDE/Wayland, Pixel, Google Quick Share Windows, publication and
signing remain untouched. Physical peer acceptance explicitly remains
UNTESTED until the user operates those devices.

## Repository scan / risk / existing coverage

The project's `agent.md` (universal SSOT), active
`native-essential-priorities.md`, `native-essential-acceptance.md`,
`native-receive-reliability.md`, full-mode-B workflow, inbound handler
and managed-session publication, outbound text/file preparation and
the 2026-10-17 critical decline/Cancel ledgers were scanned first.
Accepted two-file Pixel ↔ Linux native29 roundtrip is partial physical
evidence, but real folder/text/link roundtrips and interrupted
manual retry remain untested.

Existing Rust tests validate outbound UTF-8/URL classification,
filename/tree metadata, reject unsafe parent folders, inbound text
size/UTF-8/NUL/offset failures, inbound folder manifest remapping,
multi-file atomic commits, network truncation and managed Cancel.
**Missing cross-stage P0 regression:** genuine managed Introduction
→ explicit positive consent → byte-payload text/link or nested-folder
frames → immutable terminal receipt, with a separate explicit fresh
consent after incomplete attempt and no accidental overwrite.
These are scoped additional *critical user flows*, not cosmetic
coverage percentage work.

## Invariants, branch map, test inventory (all planned scenarios)

| Artifact and branch | Risk / level | New observable checks |
|---|---|---|
| `inbound.rs::process_introduction` for Text and Url | P0 integration | two distinct kinds, correct size, consent token, **no early received_text**, no file destination |
| `inbound.rs::apply_consent(true)` for managed text | P0 integration | actual consent command, encrypted Accepted two-chunk response; no auto-open |
| `inbound.rs::process_offline_frame` fragmented UTF-8 bytes | P0 protocol | correct offset / completed UTF-8; Finished and encrypted Disconnect, immutable `received_text` only terminal |
| `inbound.rs::process_introduction` nested two-member folder | P0 integration | exact relative parents, initial directory untouched before consent, non-overwrite of sentinel |
| `inbound.rs` exact file frames and O_TMPFILE publication | P0 filesystem | member-by-member publication, final bytes/names and readonly descriptor capability |
| `inbound.rs` interrupted member and new independent attempt | P0 N3/N4 integration | pre-existing first completed member intact; incomplete member absent; fresh token/consent required for retry; no auto-retry or silent overwrite |
| `received_files.rs` terminal receipt subset/validity | P0 security | interrupted receipt proper subset, final completed receipt whole; bytes match, only explicit files |
| Existing source/receiver protocol and failure tests | P0 regression | original assertions and suites unchanged |

Planned **3 new async test methods**, with Text/Url table 2 variants
and retry/transfer variants preserving actual managed sessions,
real temporary Linux filesystem, real Nearby payload parser,
crypto frames and immutable capability objects. No mocks of the
domain subject, no product code or interface changes. Each method
fails on a different P0 user-facing invariant.

## Scoped Positive/N1–N12 and interactions

Legend: P = new assertions, E = already existing test evidence,
X = not applicable to this domain (rationale below).

| Flow | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Managed text/link | P | E | P | E | E | E | E | P | X | P | P | X | E |
| Nested folder successful | P | E | P | E | E | E | E | P | X | E | P | X | P |
| Interrupted + explicit new folder attempt | P | E | P | P | P | E | E | P | X | P | P | X | P |

N1 invalid text/manifest and N2 large/zero/excessive lengths already
have named tests; this slice adds multibyte UTF-8 fragmented content.
N3/4 interrupted member and **new** fresh authorized attempt;
N5 existing task/supervision races; N6 malformed wire/metadata
prior suites; N7 fresh consent tokens and readonly receipt; N8
no clock/date logic; N9 in-flight aborted user flow; N10 no
clipboard, auto-link navigation, user secrets or data logging;
N11 no entitlements; N12 staging and name collisions with
actual temporary filesystem, earlier ENOSPC/read-only matrices.
No category implies all possible methods/branches of the repo.

Interaction cross-product: content `Text|Url` × UTF-8
fragments × consent true; nested directory and existing collision
× staged first publication × second interrupted × new explicit
consent × full second-round terminal publication. No automatic
restart; content never obtained before explicit consent.

## Risk, mutation review, file-level production map

- `core_lib/src/hdl/inbound.rs`: branches for text/link
  introduce/accept/receive, nested files, Cancel/Disconnected,
  on-disk stage/publish, terminal receipt.
- `core_lib/src/sessions.rs` and `sessions/text.rs`: admission
  token, immutable terminal `received_text`.
- `core_lib/src/receive.rs`, `received_files.rs`: pinned roots,
  collision-safe publication, file receipt identity/capability.
- `core_lib/src/hdl/outbound.rs`: existing text classification
  and folder parent metadata tests remain the complementary
  sender-side contract; no new sender source branch claimed.

Mutation review: bypassing consent, changing Text↔Url kind,
wrong UTF-8 byte length/offset, unexpected second member,
non-atomic publication, overwriting existing user-owned file,
fabricating completed receipt after incomplete attempt,
reusing a consumed consent token, auto-resending without a
new user action, or reporting Finished too early must fail
these or adjacent checked assertions. This is review of
assertion sensitivity, **not** executed mutation testing.
Per-file exhaustive public branch/N1–N12 Stage 12 and
physical Android/Google Windows remain OPEN.

## Milestones and rollback

1. [x] Repo scan and per-flow risks; documentation-first inventory.
2. [x] Implement the three scoped P0 managed-session integration
   methods, no product behavior/UI/policy edits.
3. [ ] Run and inspect exact-source ordinary Preflight 5/5 plus
   existing GTK workflow; repair only test-fixture mistakes,
   preserve first red failure evidence.
4. [ ] Trigger the strict **full Mode B** on final source:
   **all Core/Tauri/Vue/Python × 10**, **GTK × 10**, and
   dependent coverage job. Verify exact attempt counts/logs.
5. [ ] Update this ledger and `docs/current-state.md`, report
   full-mode-B gate independently from mandatory physical
   Android/Windows/KDE acceptance. No false global Stage 12 pass.

A CI-only minimal path-scoped trigger extension to
`.github/workflows/mode-b.yml` may be necessary to launch the
strict run through available GitHub connector access without
the user operating their computer. It must retain the
full ten-run cadence, read-only token, and all existing
assertions; never broaden to all dev pushes. Document the
run SHA and verification. Rollback removes the scoped tests,
docs and optional workflow trigger; leaves production,
master and installed binary unaffected.

## Verified critical-source test results and strict-run start

The exact P0 source `940a3a9f54f0b965df54555190b353ea7ef845c4`
completed
[Core stable test job 117528745242](https://github.com/morozhenko-apps/rquickshare/actions/runs/39077876927)
with **702/0** passed once, **8/0** additional integration
tests (4+4), and exactly ten cycles of eight
race-sensitive tests (**80/0** single-test results).
All three **new** managed cases appear individually
`ok` in the full Rust log. Rust format, frontend,
Core strict Clippy for that source passed, and the
independent [Native GTK 39077876891](https://github.com/morozhenko-apps/rquickshare/actions/runs/39077876891)
was **SUCCESS**; the prior Preflight's legacy Tauri
job was cancelled by the next `dev` workflow-only
commit, not marked 5/5.

Workflow-only commit `5776e0ea65b0240cdbf3057b5d5ab7d84d4f9935`
adds a **fail-closed inventory gate** at the start of
the full Mode B job to ensure all six critical
incoming P0 tests (decline, peer disconnect,
remote Cancel, text/link, folder, explicit retry)
are present **exactly once** in the compiled
Rust test registry. It leaves the existing
ten full suite loops, independent GTK ten loops
and final dependency-gated coverage measurements
unchanged. That same `dev` workflow-file push
automatically started the strict
[full Mode B run 39078162262](https://github.com/morozhenko-apps/rquickshare/actions/runs/39078162262)
on **exact HEAD `5776e0ea`**, with
[Preflight 39078162267](https://github.com/morozhenko-apps/rquickshare/actions/runs/39078162267)
also invoked. Both were still active when this
entry was written. Strict Stage 11 and overall
Stage 12 cannot be claimed from a running job.

All user-operated Google Android/Windows,
installed native29 upgrade and real KDE/Wayland
peer cases remain UNTESTED for the new P0
scenarios, regardless of final CI result.

## Source-only implementation checkpoint

The three requested high-risk methods and scoped managed-session
helper were added only to
`core_lib/src/hdl/inbound/receive_reliability.rs`
at source `d1a662ec` and corrected for deliberate UTF-8
fragmentation and path-sorted immutable receipt records in
`940a3a9f`. No production source was edited.
These exercise actual `process_introduction`, managed
`respond`/`consent_command`/`apply_consent`,
Nearby `PayloadTransferFrame` byte/file processing,
immutable session snapshots and the real temporary
Linux filesystem rather than mocking the domain.

The text/link method verifies both `Text` and `Url`
after positive consent; the first packet intentionally
ends within a multibyte UTF-8 scalar.
The nested-folder method verifies non-overwrite of an
existing user-owned tree and both file-capability bytes.
The retry method interrupts a real duplex frame header,
confirms only the first completed file is retained,
then creates an independent session and waits for a
new user consent before accepting any bytes under new
collision-safe filenames.

Exact-source ordinary
[Preflight 39077876927](https://github.com/morozhenko-apps/rquickshare/actions/runs/39077876927)
and isolated
[Native GTK 39077876891](https://github.com/morozhenko-apps/rquickshare/actions/runs/39077876891)
were pending/running when recorded. No green result
is asserted until completed GitHub job logs are read.
Next: strict full Mode B from the finally validated
source with ten consecutive suite/GTK attempts and
conditional final coverage.
