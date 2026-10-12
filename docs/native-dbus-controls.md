# Native D-Bus transfer controls

Current continuation: immutable read pages and directed invalidations are implemented
in [native-dbus-state.md](native-dbus-state.md). The nine-method-only checkpoint
below records this earlier control slice; the current interface has fourteen
methods plus StateChanged. Executable and native UI work remain pending.

2026-10-12. Mode A. Continuation of the accepted native service direction and
existing coordinator command contracts. This slice exposes the already specified
DiscardSelection, SendSelection, Respond and Cancel operations through Manager1.
It does not change the same-UID desktop trust boundary, core protocol, consent
policy or ownership model. The original five selection methods remain compatible.

## Analysis and scope

The coordinator and NativeRuntime already implement these operations. SelectionBus
serializes authenticated calls into that same coordinator and progresses the real
runtime between calls. Reuse this route instead of adding another command queue,
transport owner or callback task. Existing runtime tests cover core delivery and
processing receipts; new tests must cover actual wire decoding, authorization,
error mapping and resulting coordinator state. No service startup, live user bus,
GUI, settings migration, new retention policy or peer compatibility is included.

Snapshot/discovery/events are a separate stage: internal Rust snapshots are not a
frozen wire schema, and the inbound 64-KiB cap cannot be mistaken for a bounded
outbound snapshot. Do not publish arbitrary Debug/serde representations or guess
missing transfer metadata in this slice.

## Wire contract

The authoritative introspection fixture remains
`core_lib/dbus/io.github.morozhenko_apps.RQuickShare.Manager1.xml`.

| Method | Input | Output | Existing invariant |
|---|---|---|---|
| DiscardSelection | s | empty | Caller-owned unsent selection only; absent ID is harmless; foreign known selection is WrongOwner; originals are never deleted |
| SendSelection | sss | s | Selection ID, discovered endpoint ID, request ID; atomic explicit enqueue; identical request/arguments returns original transfer ID; conflicting replay is rejected |
| Respond | stb | empty | Transfer ID, current consent revision, accept; current inbound round only; identical answer replay is harmless; conflicting/old answer is StaleConsent |
| Cancel | s | empty | Transfer ID; queued work cancels locally, active work records a cancellation intent; terminal/repeated cancellation is harmless |

Selection and transfer IDs use the same canonical lowercase epoch32hex-counter16hex
encoding as uploads; zero, malformed and old-epoch IDs are InvalidInput. Unknown
current-epoch IDs retain the coordinator's operation-specific semantics. All four
methods reject attached FDs, missing/extra arguments and wrong signatures before
mutation. Request/endpoint bounds remain 128/256 graphic ASCII bytes. Only the
runtime supplies discovered endpoints; caller addresses never become routes.

Pending selections remain connection-owned. After SendSelection succeeds, the
transfer belongs to the service and survives caller disconnect. Any authenticated
same-UID client can respond to or cancel a service-owned transfer, as already
specified by the shared panel/notification model; transfer IDs are not Google
identity or an additional sandbox authorization mechanism. An empty success reply
acknowledges command admission, never peer delivery, processed consent or terminal
completion. Runtime receipts and state observations remain authoritative.

## Milestones, risks and rollback

- [x] C0: Record this plan before code and commit it independently (`7527abd`).
- [x] C1: Extend the fixed fixture, canonical ID decoding and serial dispatcher;
  add focused regressions against the real coordinator/private broker.
- [x] C2: Run the core suite once, all-target Clippy and format/diff checks;
  record actual results, update the diary and commit implementation.

Rollback C1 removes only the additive native wire methods and their tests; the five
selection methods and legacy Tauri app remain usable. No disk migration exists.
Risks: treating an accepted command as completion, accepting a stale consent,
consuming a foreign/failed selection, duplicate enqueue after lost reply, confusing
token types/epochs, or allowing FD-bearing control calls to mutate state.

## Focused verification inventory

| Artifact | Planned checks | Level |
|---|---|---|
| SendSelection | Real FD upload/commit, discovered endpoint, identical/conflicting replay, no duplicate queue item, caller disconnect preserves queued resources | Private bus + actual coordinator |
| Selection ownership | Foreign send/discard denied; owner discard releases reservation and is idempotent; commit receipt remains stable after discard | Private bus + actual coordinator |
| Respond | Accept/reject, duplicate/conflicting/stale revisions, outbound/terminal refusal, correct pending intent; cancellation supersedes answer | Private bus + actual coordinator |
| Cancel | Queued versus active versus terminal; repeated call; another same-UID client can cancel; lost observation still permits cancellation | Private bus + actual coordinator |
| Validation | All four exact signatures, forbidden ancillary FDs, malformed/zero/old epoch IDs, unknown valid IDs; request and endpoint bounds; no mutation on rejection | Unit wire decoding + actual coordinator |
| Admission | Missing endpoint, queue capacity, unsynchronized service; failed send retains selection and request ID for a corrected retry | Unit wire decoding + actual coordinator |
| Fixture | Exact extended XML compared through authenticated Introspect; fixed errors unchanged | Private bus |

Interactions: owner/foreign connection x unsent/queued; replay x conflicting
arguments/disconnect; accept/reject x duplicate/conflicting revision/cancellation;
queued/active/terminal x cancel; malformed body/FD/token x unchanged revision;
failed admission x successful corrected retry. Reuse existing domain/runtime
coverage instead of duplicating every coordinator branch in wrapper tests.

Primary categories planned: Positive, N1, N2, N4, N6, N7 and N9. N5 is covered by
serialized ordering and existing runtime receipt races; no new concurrent worker
is introduced. N3 bus failure cleanup retains the preceding private-bus coverage.
N10 fixed error bodies/no source metadata remain inherited and asserted by shared
rejection helpers. N8/N11/N12 introduce no clock, billing or storage-write contract.
No broad Mode B branch/coverage or ten-run stability claim is planned.

## Verification observations

One `cargo +stable test --manifest-path core_lib/Cargo.toml --all-features --locked`
passed **397 unit/export + 4 integration = 401 checks**, zero failures/ignored;
unit execution 0.97 s after compilation. This includes all **8 new methods** with
table-driven inputs. No repeated development loop was run. Final all-feature,
all-target Clippy with `--locked -- -D warnings`, nightly formatting and
`git diff --check` passed. Existing bluer future-incompatibility notice remains.
Previous 393-test observations remain in the selection SSOT.

The four new private-bus methods use real broker connections, descriptor
upload/commit and the actual coordinator. Their harness deliberately does not
start mDNS/BlueZ or simulate consent processing; it inspects the retained intent
and ownership after serial dispatch. Existing runtime/real-worker checks ran in
the same complete suite. This is not full native-executable or Google-peer smoke.

Four unit wire-decoding methods additionally exercise all new signatures,
otherwise valid bodies with forbidden ancillary descriptors, canonical epoch
tokens, unknown IDs, exact/over identifier bounds, queue capacity, desynchronization,
failed admission/retry and cancellation versus held resources. No coordinator,
use case or protocol state machine is mocked. The test-only raw FD fixture uses
SDK-encoded body/signature bytes with an extra unused owned descriptor; it does
not add unsafe production code.

Primary counts (each new method counted once): Positive 1; N1 1; N2 1; N4 1;
N6 1; N7 2; N9 1. N3/N5/N10 retain shared bus/runtime coverage described above;
N8/N11/N12 introduce no applicable new operation. No instrumented coverage or
executed mutation-tool percentage is claimed. Assertion review: duplicate sends
fail exact queue/transfer counts; removed owner/replay/epoch/FD guards fail rejection
and unchanged-state checks; premature active release fails resource accounting;
fake command completion fails phase/intent assertions; stale acknowledgements
cannot remove a newer cancellation.

Changed production files: `service/bus.rs` and the Manager1 XML. Test harness
preparation was extracted without changing its serial lifecycle, allowing a real
coordinator to supply discovered endpoints/inbound sessions. New regressions are
in `service/bus/tests/controls.rs`. No dependencies, lockfiles, settings or legacy
desktop code changed. All owned commands exited; terminal closed. No user bus
registration, installation, push or release promotion occurred.

Next: freeze the bounded snapshot/discovery/invalidation read contract before
implementing it, then executable/lifecycle and retention work. The panel still
cannot obtain a native wire state view. Nine Manager1 methods are implemented;
there is no installed native daemon or replacement UI yet.
