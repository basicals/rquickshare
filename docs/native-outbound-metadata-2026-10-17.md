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
2. [ ] Add actual managed outbound regressions; demonstrate the missing details
   before the repair. Cover file manifests/progress/retirement, text/URL and
   terminal interruption without mocking the handler or session.
3. [ ] Seed the admitted recipient's display info in InnerState. Preserve the
   last actual metadata when recording nonterminal -> terminal interruption.
   No new schema/API/wire/UI flow or journal rewrite; old zero-metadata history
   stays unchanged because its exact original content is unavailable.
4. [ ] Run affected core verification once and strict lint; record observations,
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

Pending. Real reverse transfer and content hashes, folder/text/decline/cancel and
Windows/platform/lifecycle checks remain open. No new release or compatibility
claim follows from the outbound user report alone.
