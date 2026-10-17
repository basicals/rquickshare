# Bounded native D-Bus state

2026-10-12. Mode A. The user selected option 1 in
[native-dbus-state-review.md](native-dbus-state-review.md). This is the implementation
SSOT; it extends the nine existing Manager1 methods without changing ownership,
same-UID authorization, consent, retention, settings or protocol behavior.

## Frozen wire contract

| Method | Input signature | Output signature |
|---|---|---|
| BeginSnapshot | empty | sstsbuutuuu |
| GetEndpointPage | su | ua(ss) |
| GetSelectionPage | su | uas |
| GetTransferPage | su | ua(sssbtbst) |
| EndSnapshot | s | empty |
| StateChanged (signal) | st | n/a |

BeginSnapshot's exact output is `sstsbuutuuu`: snapshot ID, epoch (32 lowercase
hex), revision, lifecycle, synchronized, resource selections (u32), reserved
handles (u32), selected bytes (u64), endpoint count, selection count, transfer
count. IDs remain canonical epoch32hex-counter16hex; the snapshot counter is
separate from the coordinator revision and belongs to the read-method namespace.
Creating/releasing a view does not advance transfer state. Input and output XML
is frozen in the existing crate-local Manager1 fixture.

Endpoint rows are `(id, name)`; selection rows are caller-owned committed unsent
IDs; transfer rows are `(id, direction, phase, has_consent, consent_revision,
has_pending_control, pending_action, pending_revision)`. Absent optional revisions
are zero, absent action empty. Enum spellings are frozen in
`core_lib/dbus/native-state-enums.txt`; never expose Debug representations. Pages
are ordered by ID, contain at most 16 records, and return the next absolute offset.
At count an empty page completes reading; above count is InvalidInput.

State calls reject attached FDs and wrong/missing/extra arguments. Application
errors retain fixed redacted messages; add UnknownSnapshot to the error fixture.
Foreign known views fail WrongOwner; absent/replaced views fail UnknownSnapshot
for reads, and EndSnapshot is harmless for absent canonical IDs. Old epochs,
noncanonical IDs and zero counters fail InvalidInput. Same-UID clients can still
control service-owned transfers under the existing rules; cached view ownership
only isolates each client's read capture/unsent selection list.

## Capture ownership and budgets

One immutable capture per authenticated connection. BeginSnapshot replaces only
after complete successful admission; a failed capture leaves its predecessor
usable. Replacement, EndSnapshot, disconnect and host close release only cached
metadata, never transfer records, replay receipts or source handles. No timers,
implicit eviction or source-path opens. Capture IDs are fresh even without a state
revision change. Snapshot counter exhaustion rejects admission before replacement.

Before copying records, check at most 128 endpoints, 128 caller-owned selections,
256 transfers, 512 total records and representable resource counters. Bound each
endpoint ID/name at 256 UTF-8 bytes and reject controls/invalid IDs. The runtime's
opaque routes provide these fields; arbitrary caller routes never enter the view.
After projection, account vector capacities and owned string capacities plus
fixed record/capture overhead against 512 KiB. Reserve a conservative fixed
512 KiB per retained capture, at most 8 MiB globally (16 retained views, within
the existing 32-caller limit). Reject Capacity before copying if no reservation is
available; replacing one view consumes its existing reservation. Temporary work
is bounded to one new capture plus one bounded coordinator snapshot, in the same
serial owner; no unbounded per-call task or cache is introduced.

Every state reply is SDK-encoded then checked for a body of at most 48 KiB and
complete message at most 64 KiB before sending. The fixed 16-record pages and
field limits give a stricter practical bound. StateChanged is similarly small.
These caps do not change the descriptor ingress transport. Custom coordinator
budgets exceeding this read schema must fail explicitly, never silently truncate.

One coordinator revision also tracks discovery name/address changes. Apply
revision validation before route mutation; identical discovery observations do
not advance it. Endpoint membership still uses the validated endpoint set. No
caller gets an IP/address through the read schema. Stable captures remain readable
after newer state, draining or loss of synchronization, and do not claim currency.

## Invalidation and client protocol

After each serial runtime/IPC progress turn, emit at most one directed StateChanged
to each authenticated client if its last delivered revision differs. Multiple core
observations in one runtime step coalesce into one hint. No broadcast to unknown
clients, event replay log or per-client worker. Forget notification revision on
disconnect; a newly authenticated connection gets the current hint. Publication
is covered by the host's existing stop/bus-failure cancellation and joined cleanup.
Broker socket write completion is not UI acknowledgement; slow/disconnected peers
remain subject to broker/connection failure, not a promised delivery history.

Clients install the signal subscription before BeginSnapshot. Read the immutable
pages at the returned revision; if a newer hint arrives, refresh after reading or
discard that view. Reconnect/restart requires a fresh capture and epoch. Lost core
events remain unsynchronized; a fresh view never silently recovers that condition.
Only existing coordinator fields are projected: no invented payload progress,
file summaries, PINs, credential metadata or peer completion.

## Milestones and rollback

- [x] V0: Record selected contract/inventory and commit before code (`9546431`).
- [x] V1: Implement pure bounded capture/cache and enum mapping; add domain checks.
- [x] V2: Integrate authenticated state methods/directed invalidations and coherent
  discovery revisions; add private-bus/runtime checks and fixed fixtures.
- [x] V3: Run appropriate core checks once, record outcomes and diary, commit.

Rollback V2 before V1 restores the nine-method adapter. No on-disk migration or
legacy UI change occurs. Risks: mixed live pages, leaked captures, cross-owner
reads, silent truncation, unknown enum strings, oversized replies, lost label
updates, unbounded notification work or false freshness after reconnect.

## Scoped inventory and verification plan

| Artifact | Branches/scenarios | Level |
|---|---|---|
| Cache admission | Empty/full views, exact/over each count, field/counter bounds, global exact/over captures, replacement at capacity, failed replacement, ID exhaustion | Unit with real coordinator |
| Stable paging | All three collections: 0/1/15/16/17/max records, every valid offset/count/+1/u32::MAX, stable ordering/counts, live mutation does not alter previous view | Unit + private bus |
| Ownership/cleanup | Two clients see only own selections; foreign read/end fails; absent end idempotent; replacement invalidates old ID; disconnect/close releases cache; no transfer/source/replay mutation | Unit + private bus |
| Projection | Every lifecycle/direction/phase/control enum, optional present/absent revisions, unsynchronized/draining view; no private source/path/address/error fields | Unit, fixed enum fixture |
| Wire | Exact XML/signatures, wrong bodies, ancillary FDs, malformed/stale/unknown IDs, bounded body/full message, maximum label/capture sizes | Unit + private bus |
| Invalidation | Subscribe before capture, hint revision matches capture, multiple changes coalesce, unchanged state emits none, directed destinations only, disconnect/reconnect/new epoch, retained view at old revision | Private bus |
| Discovery | Name/route change advances shared revision, identical update stable, failed/exhausted revision leaves route unchanged | Actual native runtime |

Interactions: cache-full x replace/end/disconnect; snapshot old/current x live
selection/send/consent changes; owner/foreign x every page/end; epoch x reconnect;
unsynchronized/draining x read versus unchanged command guards; name/address change
x same endpoint ID; slow/failed publication x stop/cleanup. Assert complete rows,
counts, IDs, resources, revisions and intents, not merely successful responses.

Negative applicability: N1 signatures/IDs/fields/offsets; N2 count/size/cache caps;
N3 existing host bus-loss paths plus cache cleanup; N4 idempotent release/repeated
capture; N5 coherent reads across state changes; N6 restart/unsynchronized views;
N7 owner/authentication; N9 disconnect/stop; N10 redacted snapshots/errors. N8 has
Unicode byte boundaries, no clock/locale conversion; N11/N12 add no billing/storage
operation. This Mode A slice does not claim exhaustive repository coverage or a
ten-run final stability gate.

## Verification observations

The first complete all-feature core test invocation stopped at compilation because
test unwrap diagnostics required Debug on the redacted AuthenticationError enum;
the enum now derives Debug without adding sensitive fields. The next invocation
executed **415 unit/export methods: 410 passed, 5 failed**, zero ignored, in 0.90 s.
All five failures were an incorrect test comparison of the SDK Signature display
form `(sstsbuutuuu)` with the actual D-Bus argument-list form `sstsbuutuuu`.
Inspected cached zbus message/fields.rs: it encodes Signature using
`to_string_no_parens`. The test now checks that exact wire representation, and also
checks every page signature against the frozen XML. No contract/assertion was
relaxed and no production signature was changed to fit a test.

After that correction and explicit sender headers on the new encoded replies and
signals, all **46 affected bus methods passed**, zero failures/ignored, in 0.34 s.
The four previously unexecuted `modern_protocol` integration methods then passed.
Current inventory is **415 unit/export + 4 integration = 419**, including **18 new
methods** (9 pure cache/projection, 7 wire/private-bus, 2 real-runtime discovery).
The 369 unaffected unit/export methods passed in the preceding complete run;
there was no redundant final 419-method run and no ten-run stability gate.
Schema round trips are not Google device compatibility evidence.

Final source review strengthened the no-extra-signal assertion with an ordered
broker fence on the sending connection and round trips on both receiving
connections; this prevents relying on receiver-task scheduling or a guessed wait.
Only that affected notification method was rerun, passing in 0.02 s. Final
all-feature/all-target `cargo +stable clippy ... --locked -- -D warnings`, nightly
format check and `git diff --check` passed after this test-only change. Existing
bluer future-incompatibility notice remains. No disabled-feature, instrumented
coverage, remote CI, package build or live bootstrap check was run in this slice.

All scenarios use real coordinator/runtime logic, SDK messages and disposable
private brokers/socket ownership. Maximum real wire capture includes 128 endpoints
with 256-byte UTF-8 labels, 128 descriptor-backed selections and 256 transfer
records; all pages satisfy the encoded body/frame guards and expose complete rows.
One real authenticated connection owns the prepared fixture state. Cross-owner
read/end, disconnect cleanup, new epoch, stable old pages, replacement at full
cache and explicit Capacity are exercised. Releasing views does not delete source
files, transfer history or upload/send receipts. The tests do not start BlueZ/mDNS
or an installed daemon and do not fabricate processing/completion.

Primary category counts, one category per new method: Positive 3; N1 2; N2 7;
N5 3; N7 2; N9 1. N4/N6/N8/N10 are secondary assertions for repeated release,
reconnect/epoch/unsynchronized state, UTF-8 byte limits and metadata redaction.
N3 retains existing bus-loss/host cleanup tests in the affected suite; no new
network-peer fault scenario was added. N11/N12 introduce no billing/storage write.
No exhaustive Mode B branch/line percentage is claimed.

Assertion/mutation review: removed owner guards fail foreign-page/end checks;
live-map paging fails immutable old-view checks; relaxed offsets/count/cache/size
caps fail exact/over tables; truncation fails complete row/count comparisons;
unversioned discovery changes fail shared revision assertions; mutation before
exhaustion fails route/sequence preservation; broadcast/uncoalesced signals fail
destination/ordered-empty-stream checks; fake consent completion fails explicit
phase/presence/revision rows. This is assertion review, not executed mutation tooling.

## Handoff and remaining work

Manager1 now has 14 methods and a directed StateChanged signal. The serial host
passes a borrowed bounded discovery iterator to read projection; no eager global
endpoint copy, extra owner or per-call task was introduced. New source is
`service/bus/state.rs`; regressions are in its tests and bus/tests/state.rs. Exact
XML/error/enum fixtures live inside core_lib/dbus. The native runtime now advances
the same revision for label/address changes and validates it before route mutation.

Closing the bus clears read captures and delivered revisions alongside existing
caller cleanup. Publication is inside the host's stop/failure select; externally
abandoning run still requires retained-owner close/shutdown as previously specified.
Core faults retain the preceding fail-closed runtime behavior; a read capture is
not a recovery mechanism. No transfer progress/file-summary fields, executable,
settings migration, GTK UI, installed registration or terminal/replay eviction
policy has been added.

Next: native executable/user-bus address discovery and owned signal-driven process
shutdown; continuous-service terminal/replay retention review; native client and
GTK tray/notifications; Dolphin; physical KDE/BlueZ and Google peer matrix. Existing
Tauri remains the current GUI. All owned terminal commands completed; terminal
closed. No user-session bus service, package install, external policy, push or
master promotion occurred.
