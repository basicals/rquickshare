# Native D-Bus state read contract review

2026-10-12. Mode A. Status: the user selected option 1. The concrete implementation
contract and progress are in [native-dbus-state.md](native-dbus-state.md). This
review records the decision, not implementation evidence. The preceding transfer
control slice is complete in `1b3b84c`, with actual measurements in
[native-dbus-controls.md](native-dbus-controls.md).

## Why a decision is needed

Manager1 now exposes nine selection/control methods. The future native chooser
still cannot retrieve discovery or transfer state through the wire. Existing
coordinator snapshots are private Rust types; the old desktop proposal's JSON
table was explicitly not a frozen descriptor-based wire schema.

The service transport admits at most 64 KiB per incoming message. The default
read model can contain 128 endpoint IDs/labels, 128 selection IDs and 256 transfer
records. Labels alone can contain 32 KiB of raw UTF-8; JSON escaping can increase
that, before IDs, transfers or framing are included. A complete response cannot
be assumed to fit a symmetric 64-KiB client. Silently truncating records would
hide transfers/consent. Globally raising request/FD ingress limits would weaken a
different resource boundary without solving consistency or client allocation.

Global AGENTS sections 3 and 3.1 require the user's choice for a significant
public API/data-model decision. Existing tray, descriptor, ownership and control
approvals remain valid; this review does not reopen them.

## Options

| Option | Description | Benefits | Costs and risks |
|---|---|---|---|
| 1 — stable bounded snapshot pages (recommended) | Capture one immutable connection-owned view; read typed endpoint, selection and transfer pages from that view; release explicitly | Every message fits the existing small bound; all pages share one revision; one large state cannot force one large allocation/message; limits can grow without changing one-frame assumptions | More read methods and a bounded cache; disconnect/replacement/release must free it; UI must reconcile invalidations arriving during reading |
| 2 — complete bounded JSON snapshot | One GetSnapshot returns strict versioned JSON plus epoch/revision; introduce a separate bounded reply/client-receive budget, provisionally 512 KiB | Fewer calls, simpler capture lifetime; complete response is naturally consistent | Larger transient serialization/client allocation and schema parsing; custom future limits can exceed the cap and reject the entire read; every client must enforce the larger receive budget correctly |

Recommend option 1 because the already accepted daemon is long-lived and bounded,
and every native client needs truthful complete state under concurrent discovery
and transfers. Paging is over a retained immutable capture, not a walk over a
changing live map. This avoids mixed revisions and retry starvation during updates.
It is independent of terminal/replay retention: releasing a read capture never
forgets an accepted send or frees its source reservation.

## Concrete option 1 proposal

- BeginSnapshot creates/replaces one capture for the actual authenticated sender.
  It returns a canonical snapshot ID, epoch, revision, lifecycle, synchronization,
  aggregate resource usage and exact endpoint/selection/transfer counts.
- GetEndpointPage(snapshot_id, offset) returns next offset and typed `(id, name)`
  records. GetSelectionPage returns caller-owned committed unsent IDs only.
  GetTransferPage returns service-owned `(id, direction, phase, has_consent,
  consent_revision, has_pending_control, pending_action, pending_revision)` records.
  Optional values use presence booleans; absent revisions are zero and absent
  action is empty. Fixed enum string allowlists belong in fixtures, not Debug text.
- A page contains at most 16 records and at most 48 KiB encoded body; the complete
  message must fit 64 KiB. Offset equals count means an empty completed page;
  offset greater than count is InvalidInput. Stable ordering is by opaque ID.
  The interface never accepts an arbitrary page size from the caller.
- EndSnapshot releases the caller-owned capture idempotently. A newer successful
  BeginSnapshot invalidates the preceding capture. Foreign known IDs are WrongOwner;
  unknown/replaced IDs are UnknownSnapshot; old epoch/malformed IDs are InvalidInput.
  Failed capture admission preserves the previous view. Disconnect/host shutdown
  clears captures. No guessed expiry timer and no transfer/resource mutation.
- At most one capture per authenticated connection, at most 32 connections; per
  capture at most 512 records (128 endpoints, 128 selections, 256 transfers) and a
  conservative 512-KiB reservation; global reservation at most 8 MiB. Validate
  counts before materializing captures; actual bounded fields and container
  overhead must be checked during implementation. Cache Capacity is explicit.
  These are initial implementation budgets, not Google limits; no silent eviction
  of another caller. Replacement must not require retaining two full old/new views
  per caller indefinitely.
- StateChanged(epoch, revision) is a coalesced invalidation hint to authenticated
  clients. Subscribe before BeginSnapshot. If a newer hint arrives while reading,
  finish/discard that view and obtain a new one; a capture never becomes current
  merely because every page was read. Disconnect/restart requires fresh capture
  and epoch reconciliation. No event history or persisted recovery is implied.
- One revision covers endpoint label changes as well as endpoint membership and
  coordinator state. The current runtime changes names without changing the
  coordinator endpoint set, so implementation must add a serial revision update
  for that change; otherwise clients could miss a visible discovery update.
- Preserve snapshot redaction: no source paths, names, file contents, addresses,
  peer errors, credentials or PIN secrets. Endpoint display labels are explicitly
  user-visible metadata. Existing core snapshots lack payload progress/file
  summaries; those fields are not fabricated. Their actual core-to-service
  projection is a later native presentation stage with its own documented schema.

The snapshot token is read authority for this caller's cached view, never a
transfer-control capability or a handoff between CLI and chooser. Same UID remains
the desktop trust boundary. No settings/visibility/Google identity is introduced.

## Implementation plan after selection

1. Freeze exact method signatures, state enums, snapshot errors and schema fixtures
   for the chosen option before code. Record scoped branch/negative interactions.
2. Implement a bounded pure read projection/cache, shared revision semantics and
   dispatcher integration with the real runtime. Preserve the nine existing methods.
3. Add private-broker reads and invalidation/reconnect tests. Check consistent
   pages during changing state, owner isolation, all offsets, quotas, malformed
   bodies/FDs, endpoint-label changes, cleanup and epoch replacement. Use real
   domain logic and deterministic external fixtures; no sleeps or fake completion.
4. Run appropriate core verification once, record actual measurements, update the
   diary and commit additive changes independently. No user-session registration
   or GTK prototype before this read path works.

Rollback removes only new read methods/cache/events and restores the nine-method
fixture. No disk migration or protocol framing/crypto change is involved. Native
executable, continuous-service retention, GTK/notifications, Dolphin and physical
Google compatibility remain separate milestones.

## Handoff

The user selected option 1; no further approval is pending for this contract.
All previously authorized transfer-control work is committed and verified. No
test session, watcher or private broker remains running. Terminal closed.
