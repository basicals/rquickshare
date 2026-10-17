# Outbound metadata continuity repair

Mode A, 2026-10-17, dev after fd108b6. Native29 stays installed while this
follow-up is implemented. The user confirmed successful receipt of the prepared
files on the available Android peer. Phone-side hashes remain unverified.

## Observation and analysis

Read-only Journal1 evidence after installation contains two finished outbound
operations and one earlier disconnected attempt. All three have kind=files but
files/total/transferred=0. Do not infer why the disconnected attempt happened or
claim that the user reported multiple independent tests. The actual phone receipt
and the daemon's finished phases are separate pieces of evidence.

`OutboundRequest::new` retains discovered recipient information only in legacy
`transfer_metadata.source`; its `InnerState.remote_device_info` is empty.
The shared validated `TransferDetails::from_state` requires remote display info
to construct Ready metadata. Consequently real prepared files/text remain
Unsupported and the journal correctly copies those empty details. Repair the
missing source state rather than fabricating counts from filenames in the journal.
The recipient remains advertised/unverified; this grants no identity/trust.

## Plan and rollback

1. [x] Record user report, sanitized readback and this plan before code.
2. [x] Add actual managed outbound regressions; demonstrate the missing details
   before the repair. Cover file manifests/progress/retirement, text/URL and
   terminal interruption without mocking the handler or session.
3. [x] Seed the admitted recipient's display info in InnerState. Preserve the
   last actual metadata when recording nonterminal -> terminal interruption.
   No new schema/API/wire/UI flow or journal rewrite; old zero-metadata history
   stays unchanged because its exact original content is unavailable.
4. [x] Run affected core verification once and strict lint; record observations,
   commit the repair and update current-state. Installed29 peer checks may proceed
   independently; a source-only fix is not installed acceptance.

Risk: inadvertently granting trust to an advertised name, leaking absolute paths
or message contents into persisted metadata, losing the immutable manifest after
the working map retires files, or presenting locally written bytes as remote disk
durability. Existing allowlist/limits and remote consent remain authoritative.
Rollback is the repair commit; installed29 and retained original files are intact.
Any subsequent package delivery requires its own fresh idle/preparation gate.

## Scoped verification inventory

Mode A focused regressions; no exhaustive Mode B gate or repeated stability loop.

| Artifact / branch | Planned test cases | Categories |
|---|---|---|
| Managed outbound files | Two opened files, one nested display path; actual Introduction then accept; Ready count/names/total, exact local progress, retained manifest after working files retire | Positive, N2, N10 |
| Managed outbound text | Text and URL; actual Introduction/accept; correct kind, UTF-8 byte totals and completion; no received-text capability on sender | Positive, N8, N10 |
| Interrupted outbound | Rejected, Cancelled and Disconnected after preparation; exact retained count/bytes/phase; terminal state must not be replaced by late termination | N3, N5, N9 |

Mutation expectations: omitting recipient state must fail readiness assertions;
dropping manifest/progress or using character count instead of UTF-8 bytes must
fail exact assertions; overwriting a terminal result must fail phase assertions.
Input/auth/storage rules are unchanged: N1/N4/N6/N7/N11/N12 are outside this repair.
N10 covers display-only basename/relative names, with no plaintext body in details.

## Verification

The initial three regressions failed with Unsupported instead of Ready (14.36s
build,0.00s execution). After seeding recipient info, the complete core library
run passed692 and failed1 in10.00s: actual outbound completion retained completed
working-map entries. The normal nonempty-file final-chunk branch sends EOF and
breaks without retiring its entry, whereas the already-complete/zero-byte branch
does retire it. This is a second small internal lifetime inconsistency, not a
reason to weaken the completed-map assertion. Before further edits, extend step3:
retire the current working entry only after its final encrypted chunk has been
successfully written. Preserve all original sources and immutable display details.
Failure before that write still exits through existing request cleanup. Recheck
only the affected handler group after this correction; the rest of the core suite
already passed. Strict Clippy did not execute because the test command failed.

Separately, installed native29 real two-file roundtrip passed: user confirmed
phone receipt and reverse receipt with automatic file-manager reveal. At14:40:57Z,
two snapshot/revision-bound Files1 read-only descriptors were checked with bounded
pread. Both exact sizes/SHA-256 and basenames match the original local manifest
(65792 and34 bytes). No receive directory traversal, configuration change, deletion
or agent-initiated Send. An initial local probe hit a Python basename-property
typo; its descriptors/snapshot were closed, then the corrected probe passed.
This validates installed29 ordinary two-file roundtrip, not the pending source
metadata repair. Folder/text/decline/cancel and Windows/lifecycle remain open.

After the final-chunk retirement correction, the affected handler group passed
135/0/0 in0.45s (15.47s build), including all3 new functions/6 table cases.
Strict `cargo clippy --all-features --all-targets -- -D warnings` passed in6.03s.
Existing bluer0.17.3 future-compatibility and stable-rustfmt nightly-option
advisories remain distinct from failures. No repeated stability loop or full
693/0 claim: the earlier full run was692/1, and its affected group was rerun once
after the root-cause correction. Original source/received files remain intact.

Current delivery state: repair is source-only on dev. Installed29 and immutable
package bytes are unchanged; any later native package must increment revision
and repeat the fresh idle/prepared-source gate. Existing outbound history cannot
be truthfully backfilled. The ordinary two-file roundtrip is accepted on29;
source metadata/retirement changes still need installed acceptance.
