# Native client: serialized operation and descriptor reconciliation

2026-10-12. Mode A. Approved direction: native-desktop-design.md,
native-retention.md. This slice consumes existing interfaces; it introduces no
wire method, automatic reconnect/resend, daemon activation, persisted data or UX.

## Analysis and implementation choices

The daemon is implemented, but no reusable client performs ordered receipt ack.
NativePair confirms the Rust/zbus/GTK direction; its CLI is a foundation stub and
has no production protocol client/GTK to reuse. Host pkg-config still cannot find
gtk4. The compact tray/notification interaction remains accepted; compositor
placement/prototype and installation remain separate reviewed gates.

Use an isolated core_lib::service::client module now, sharing the pinned zbus,
bounded Unix transport and validated OpenedFile ingress without another lockfile,
workspace migration or copies of security-sensitive receive code. This module
owns only a dedicated client connection and bounded reconciliation state, never
RQS or transport workers. Crate packaging can be separated with the GTK/package
stage; this is not a second core owner. No new dependencies or unsafe code.

Connect through an injected UnixStream. Reuse 64KiB/16FD framing before decoding.
Resolve the service through the actual broker, validate daemon UID, pin its unique
bus owner, and read the current epoch from a bounded snapshot then end that view.
Require Operations1/GetOperationState=(0,0) on the dedicated new connection. No
Manager1 fallback, activation, imported shared connection or client Clone.
Service calls target the unique owner; broker and successful/error replies must
have the expected actual sender. Owner replacement/loss is a distinct failure;
no pending operation moves to another connection/epoch. All diagnostics are fixed
redacted codes, never remote bodies/source names/paths.

## Client invariants

One &mut owner serializes calls. A mutation stores its exact command and next
sequence BEFORE its first await. A cancelled/failed wait leaves it pending; a
new conflicting command cannot discard it. Exact mutation retry automatically
reconciles the same pending command. recover_pending is the controller seam for
resuming work without recreating the command. No periodic timer or implicit retry
loop. The future GUI controller uses this seam; no user cleanup action is required.

For an uncertain mutation, inspect last-accepted/acknowledged progress on the same
owner: not accepted => submit the retained command; accepted => obtain its typed
receipt; any unexpected gap/ack/kind/result is a protocol failure, never new work.
Record the validated result in bounded local state before AcknowledgeThrough.
A cancelled/lost ack wait retains the result and replays only acknowledgement.
After successful ack, retain the most recent outcome for cancellation at caller
boundaries. Accepted work survives client closure; a pending/failed ack cannot be
reported as a rejected send. Callers can inspect the recorded outcome separately
from an ack transport error. No automatic resend after daemon or client restart.

FD append also needs reconciliation: retain one <=16-file owned batch until the
reply is validated. A cancelled append checks GetUpload: preparing at the same
batch sequence => resubmit the retained handles; preparing at sequence+1 =>
accepted; other state/sequence => fail closed. Never send a later batch or commit
while this batch is unresolved. Batch source handles are closed after confirmation
or client disposal; original files are not unlinked. Client batches clone existing
validated handles, not paths; shared-offset safe daemon readers remain unchanged.

Upload/selection handles carry the originating client connection/epoch. Local
foreign/stale handles cannot create a pending request. Receipt kinds/result types
must match the retained command, and IDs are canonical current-epoch nonzero tokens.
The transfer result is service-owned metadata, not a selection handoff.

Explicit asynchronous close closes the dedicated connection and its state; Drop
only drops ownership. No client shutdown method calls daemon stop/cancel. Client
errors distinguish fixed daemon rejection, pending work, protocol mismatch,
source clone failure, transport uncertainty and daemon replacement. Remaining
native state projection/invalidation subscription and controls are a separate
slice before GTK; bootstrap snapshot is only epoch acquisition, not UI state.

## Stages and rollback

1. [x] Analyze existing wire/transport/file ownership, NativePair and GTK status;
   record this SSOT and scoped verification inventory before production edits.
2. [ ] Implement owner/UID/epoch pinning, fixed error decoding and one bounded
   pending mutation/batch state with cancellation-safe automatic acknowledgement.
3. [ ] Verify actual client/daemon logic using owned private brokers and injected
   reply-observation barriers; add no mocks of coordinator/client state logic.
4. [ ] Run affected checks once, record actual results, update diary and commit.

Plan and implementation are separate conventional commits on dev. Rollback is
module removal (and internal receive visibility), with no daemon behavior/storage
change. Risks: losing a command at await, ack before result storage, blind FD retry,
following well-known name replacement, accepting forged reply/error sender and
logging remote error bodies. Explicit retained state and sender checks address them.

## Scoped inventory and branch/interaction plan

| Artifact | Planned tests | Cases / assertions | Categories |
|---|---|---|---|
| Client connect and validation | 3 | dedicated empty ledger; current epoch/UID; missing service; broker/service replacement; fixed domain errors vs malformed IDs/receipt kinds/senders | Positive,N6,N7,N10 |
| Ordered mutation owner | 4 | all four methods; automatic ack; known rejection reuses sequence; conflicting pending; foreign handles; sequence overflow; cancellation before submit/after acceptance/while ack | Positive,N1,N2,N4,N9 |
| Descriptor batch owner | 3 | real files/offsets; 0/16/17; original file lifetime; lost append reply; before-admission cancellation; next sequence mismatch/closed upload; release clones | N1,N2,N4,N5,N9,N12 |
| Continuous owner/disconnect | 2 | thousands of abort cycles without receipt quota; pending/accepted work and source cleanup on explicit close; no replacement replay | N4,N5,N9 |

12 planned tests with parameterized subcases; refine count with reasons below.
Use existing private-bus harness plus actual coordinator. Observation barriers
wait for a real method result then notify and suspend before the client observes
it; cancellation is event-driven, not a sleep/timeout. This injects external reply
loss only and leaves the subject/state machine unmocked. Lost ack and append replies
must not cause another operation/batch. Existing kernel/framing/descriptor rejection
and daemon shutdown tests stay the regression base. N3 transport failures covered
at actual bus boundary; N6 old/replaced epochs; N7 ownership/sender; N10 fixed
redacted errors. N8 no locale/clock policy, N11 no billing. N12 only handle clone/
source lifetime (no database). No ten-run development gate or coverage percentage.

## Actual verification and handoff

Implementation pending. Previous measured retention results remain in
native-retention.md. Terminal closed.
