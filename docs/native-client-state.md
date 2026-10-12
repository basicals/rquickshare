# Native client state, invalidations and existing controls

2026-10-12. Mode A. Continuation of native-client.md, native-dbus-state.md and
native-dbus-controls.md. The user requests continued implementation, not another
checkpoint-only stop. Complete independent stages until a real reviewed decision
or external acceptance gate prevents progress. Existing wire/authority stays fixed.

## Design and invariants

Subscribe before the bootstrap snapshot and continuously drain both exact
StateChanged and broker NameOwnerChanged subscriptions. zbus 5.19's socket reader
awaits broadcast capacity; leaving a bounded stream unread can block method replies.
Use one owned cancellable pump and watch coalescing, not a GUI-owned unread stream
or unbounded per-event tasks. Eight messages per subscription, one latest hint,
no timers, monotonic maximum revision and sticky protocol/transport/owner failure.
Verify actual senders/path/interface/member/FD-free bodies. Owner replacement or
epoch mismatch never rebinds pending payloads. Close cancels/joins the pump before
closing the connection; Drop requests cancellation only.

Immutable read_state captures the existing metadata/pages, checks collection caps
before capacity allocation, reads <=16 rows/page with exact offsets/count/end,
canonical epoch/IDs, sorted unique rows, bounded endpoint labels and fixed enum/
optional-field invariants. No incomplete view is published. One retained capture
ID survives cancellation and is replaced by the next capture or released by close;
there is no async Drop cleanup task. EndSnapshot completes the read; old cached
views remain immutable and are explicitly stale during incomplete/newer reads.
Read startup warnings through the existing allowlist, never invent progress,
filenames, identity, size, pairing PIN or terminal outcomes absent from this schema.
Scalar resource usage remains reported data; internal admission budgets are not
new public validation limits. Collection/page bounds are the frozen wire budgets.

A latest hint newer than the captured revision keeps the view stale. next_change
consumes watch versions and ignores already captured revisions; events during
paged retrieval remain available, without periodic polling. Cached view freshness
also reflects sticky failure. Lost UI hints do not mean lost core observations:
only the daemon's synchronized field describes authoritative core confidence.

Expose existing DiscardSelection, Respond and Cancel. Selection discard enforces
origin connection/epoch and refuses to destroy work referenced by a pending
ordered/batch request. Transfer controls require current epoch, not originating
client connection, matching the same-UID service-owned transfer contract. Consent
requires explicit nonzero current round revision. Empty success means admission
only; pending intent/terminal phase come from a fresh snapshot. Same controls may
be retried with exact ID/revision after an interrupted wait; never silently accept
a newer round. The UI supplies explicit user action, not auto-accept.

Use typed Direction/Phase/Lifecycle/ControlAction from the existing domain and
isolated client view types, not debug/JSON protocol representations. Wire enum and
optional-field validation is strict. GTK/controller integration follows this stage.
No public bus method, source authority, settings write, dependency or activation.

## Milestones and rollback

1. [x] Analyze fixed schema, zbus backpressure, source/client lifecycle and GTK
   availability; record plan before code.
2. [ ] Implement typed projections, retained captures, owned coalescing signal pump
   and existing controls; update native client lifecycle.
3. [ ] Add scoped unit/private-broker regression checks and run affected Rust checks
   once. Preserve previous measured results and report any corrections honestly.
4. [ ] Commit the recoverable implementation and proceed to native shell feasibility/
   prototype within the accepted compact tray/notification design.

Rollback this additive client slice before any shell which consumes it. No daemon
wire/storage changes. Risks: blocked socket reader, partial/stale view shown as
current, premature capture cleanup, optional-field ambiguity, forged invalidation,
new-owner replay and mistaken consent admission as completion.

## Scoped inventory before tests

| Artifact | Planned checks | Branches/interactions | Categories |
|---|---|---|---|
| View/page decoding | 4 | 0/1/16/17/128/256 counts; max+1 and malformed pages; enum/flag/revision/direction combinations; sorted unique IDs and UTF8 labels | Positive,N1,N2,N6,N7 |
| Real captures | 2 | actual maximum pages/owned selections; interrupted read/new capture and source lifetime; stable old cached view | N2,N5,N9 |
| Invalidation pump | 3 | subscribe-before-read; thousands of mutations while not reading hints; coalescing latest/stale revisions; replacement/no reconnect; exact sender/epoch/FD rejection | N3,N4,N5,N7 |
| Existing controls | 3 | same-UID other client accepts/rejects/cancels; duplicate/stale rounds; terminal cancel and retired ID; discard foreign/pending selections; interrupted control reply | Positive,N4,N7,N9 |

12 initial test methods with table subcases; add independent cases if needed and
record the final count. Existing 14 operation-client and server/service cases
remain the regression base. Tests use real coordinator, dedicated clients and
owned private brokers; condition-based cancellation barriers only. N8/N11 absent
(no locale/time or billing rules); N10 existing redacted errors plus no invented
private metadata; N12 no new storage path, existing FD/source errors retained.
No ten-run development stability gate or exhaustive repository coverage claim.

## Actual observations / continuation

Implementation pending. GTK4 runtime 4.22 is present, but GTK4 and GTK4 Layer Shell
development pkg-config files are absent. GTK4 Layer Shell upstream documents KDE
Plasma Wayland support; this is feasibility evidence, not physical tray-placement
acceptance. Build dependencies may be extracted privately for verification, without
installing system packages or changing user configuration. A material layout/
fallback or platform contract change still requires the user's choice.

Sources: https://docs.rs/zbus/5.19.0/zbus/struct.MessageStream.html and locally
inspected pinned socket_reader.rs; https://github.com/wmww/gtk4-layer-shell and its
API reference. No user service/settings changed. Terminal closed.
