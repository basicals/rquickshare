# Bounded byte ingress before preview support

2026-10-15. Mode A, direct dev. Guide: /home/morozhenko/.codex/AGENTS.md,
sections3.1/11/12. User selected supported authorization only and continuing
independent sharing work. This is a scoped internal root-cause fix; no new
preview decoder, account authority, wire field, IPC, settings or consent UX.

## Analysis and rationale

Both inbound and outbound handlers use an unbounded map of BYTES buffers. Each
first chunk reserves up to5MiB, but no aggregate byte/count limit exists. Outbound
does not remove completed buffers. Subsequent chunks can change total_size;
completion of non-text payloads does not require the declared byte count. Missing
ids/sizes/offsets silently use protobuf defaults. These are shared reassembly
defects, not a reason to give each future preview its own workaround.

Native admission bounds concurrent sessions separately; legacy handling shares
this path. A per-request budget does not claim a global process-memory ceiling.
Files remain streaming, text still requires explicit native consent and its own
128KiB metadata limit. Complete setup frames retain their existing dispatch.

The modern wire model contains preview ids and attachment/resume details.
Pinned public Google incoming/outgoing session/service and file-attachment sources
were inspected; those selected files do not establish preview image encoding or
attachment-hash/resume integrity semantics. This is a bounded negative finding,
not proof of absent implementation on Android/Windows. Do not invent these
semantics from field names or treat filename/size as content identity.

## Frozen implementation contract

- One request-owned assembler shared by both handlers, no global state or timer.
- At most16 incomplete BYTES payloads and5MiB sum of their declared sizes.
  The count also bounds zero-size payloads. Keep the existing per-payload5MiB
  limit. Preserve valid interleaving instead of allowing only one payload.
- Require explicit id, total_size and chunk offset. Accept the full signed id
  range, including0. Bind size on first admission; no size changes mid-payload.
- Validate offset, body bound and final exact length before retaining bytes.
  A separate empty final chunk at total_size and a combined data/final chunk
  both work. Do not require the data chunk to carry the final flag.
- Reserve fallibly after all admission checks. Account declared reservations,
  not just body lengths, so sparse first chunks cannot evade the memory bound.
- Remove completed ownership before decode/dispatch and release its budget.
  On any assembler error clear pending ownership; handlers propagate failure.
  Request drop/cancellation releases the assembler naturally. No replay-history
  cache or claim of completed-id deduplication is added.
- Debug output exposes only counts/reservations, never payload bytes or ids.
  Errors describe the violated invariant without echoing content.

The fixed limits are defensive admission policy for malformed/excessively
interleaved peer traffic, not an advertised Google preview capability. Peers
with more than16 incomplete or more than5MiB reserved bytes fail this request;
sequential valid maximum-size payloads remain supported. Existing handler
wrappers/session termination continue to own disconnect and recovery.

## Plan, risks and rollback

1. [x] B1 analysis: inspect both real handlers, native text consent, protocol
   limits, source references and agent guide; record user option1 and this plan.
2. [ ] B2 implement the private bounded assembler and replace both duplicated
   maps/loops. Preserve text validation and setup dispatch. No protocol/schema
   or dependency change. This is the one logical production commit.
3. [ ] B3 add the scoped verification inventory below, run core checks once,
   record actual results and limitations, update current state/evidence and
   commit. Do not rebuild/sign/publish a package in this prerequisite slice;
   native7 remains the last measured binary, explicitly behind this source.
4. [ ] B4 freeze the next real preview receive/display contract after establishing
   wire semantics and pre-consent decoder ownership. Persistent resume needs
   content identity, partial-storage/recovery and consent contracts separately.

Risk: newly rejected malformed headers or excessive interleaving may expose peer
quirks. Preserve standard framing fixtures and report real acceptance separately.
Rollback: revert the production commit; no on-disk migration, remote operation or
credential exists. The preceding documentation commit preserves rationale.

## Planned scoped verification inventory

This is Mode A regression protection, not an exhaustive repository Mode B gate.
Rust built-in tests, existing tokio duplex/TCP fixtures and core CI apply. No
network/radio/user files, sleeps, new dependencies or mocked domain assembler.

| Artifact / branches and interactions | Planned methods | Categories |
|---|---:|---|
| assembler complete/split/separate-final/empty/full signed ids/interleaved completion | 1 table-driven | Positive,N2 |
| missing id/size/offset; negative/oversize total; invalid starting/continuing offsets | 1 table-driven | N1,N2,N6 |
| changed size up/down, body overrun and short final; rejected state releases all pending buffers | 1 table-driven | N1,N2,N7 |
| aggregate reservation max-1/max/max+1, sparse admission, completion release and error cleanup | 1 table-driven | N2,N7 |
| count15/16/17 including empty incomplete payloads; completion frees slot | 1 table-driven | N2,N7 |
| completed payload reuse is fresh; no growth after repeated setup-size completions | 1 | N4 |
| Debug/error does not disclose bytes/ids; drop ownership has no external effect | 1 | N10 |
| actual inbound fragmented setup routing and aggregate rejection | 1 | Positive,N7 |
| actual encrypted outbound fragmented setup routing, removal and aggregate rejection | 1 | Positive,N7 |

Expected9 new methods. Existing native text and encrypted authentication/transfer
fixtures protect consent and setup compatibility. N3/N8/N11/N12 do not apply to
the memory-only assembler; allocator exhaustion is handled fallibly but not
provoked with host OOM. N5/N9 ownership is per sequential handler/request; no
new scheduler/UI/storage behavior. Mutation review: removing count/aggregate,
size binding, exact-final, offset or release checks must fail corresponding tests.
No blanket branch/coverage or physical interoperability claim.

## Actual verification

Pending implementation. Research commands exited; no watcher/terminal retained.
Terminal closed.
