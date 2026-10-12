# Native D-Bus selection contract review

2026-10-12. Mode A. Status: option 1 approved by the user on 2026-10-12 (choose the architecturally preferable option). Implementation is tracked in native-selection-transactions.md. No bus adapter, method fixture, activation file or system policy is implemented by this review. The accepted FD boundary, native runtime, Rust/zbus direction and compact tray UX do not need reapproval.

## Evidence and reason for review

The coordinator/core runtime is implemented in `c7674e8`, with ordered events/admission in `fb316c2`. It accepts owned regular files, never caller-facing paths. NativePair's ADR-0001 chooses Rust/Tokio/zbus and thin GTK clients, but its inspected daemon remains a foundation executable without a reusable public D-Bus implementation. Its repository was read only; no code/license was copied.

D-Bus transports Unix FDs out of band with the corresponding message. dbus-daemon documents separate per-message and queued incoming/outgoing FD limits. Therefore one atomic call carrying the core's provisional 256 files cannot be assumed portable across user bus configurations. This is an inference from those documented configurable limits, not a measured failure on this host. No user-bus connection, policy modification or daemon registration was performed.

Sources checked 2026-10-12: [D-Bus specification, FD transport and credentials](https://dbus.freedesktop.org/doc/dbus-specification.html), [dbus-daemon resource-limit documentation](https://dbus.freedesktop.org/doc/dbus-daemon.1.html). These establish transport constraints, not the application API proposed below.

The user's global `/home/morozhenko/.codex/AGENTS.md` section 3.1 requires a stop for significant public API/data-model/security decisions. Freezing staged resource lifetime and method contracts qualifies. This is a new wire-contract choice; it is not reopening path versus descriptor input. The explicit development verification override still permits one appropriate run, with affected reruns only.

## Options

| Option | Description | Benefits | Costs / risks |
|---|---|---|---|
| 1 — bounded transactional FD batches (recommended) | Begin an owner-bound upload, append small FD/name batches, atomically commit the complete validated selection, or abort | Multi-file/folder selections do not require every FD in one bus message; preserves complete-selection validation and the approved source-handle boundary | Adds staged ownership/accounting and sequence/commit receipts; unfinished uploads must be bounded and cleaned on abort/disconnect/shutdown; decoder limits still require proof |
| 2 — one atomic FD message | Create the whole selection with one bounded FD/name array; reject selections exceeding the deployed bus-compatible message limit | Fewer states/methods and simple all-or-nothing receipt | A bus-sized file-count ceiling becomes a user-facing limitation, potentially below the current 256-file core budget; increasing it later requires another API/migration; not sufficient for broad folder parity by itself |

Recommendation: 1. The target includes multiple files/folders and future parity; the bus's per-message limit should not become the application's total selection limit. This is not unbounded upload: current aggregate selection/file/byte/FD limits still apply, with explicit pre-commit reservations. Neither option guarantees support on arbitrary more restrictive bus configurations or bypasses global quotas. No automatic fallback to daemon path opening is permitted.

## Concrete option-1 shape for implementation planning

Preserve the proposed service namespace from native-desktop-design.md and publish a versioned Manager1 interface only after the selected contract's introspection/error/schema fixtures are committed. Proposed selection methods:

| Method | Proposed arguments / result | Contract |
|---|---|---|
| BeginSelection | request_id string -> upload_id string | Per-connection bounded preparation; no send, no path input |
| AppendFiles | upload_id, batch_seq u32, entries array of (UnixFD, relative_name string) -> next_batch_seq | Atomic validation of each batch, strict sequence; reject stale/conflicting append; no partial batch acceptance |
| GetUpload | upload_id -> next_batch_seq and preparation/commit receipt | Reconcile a lost append/commit reply rather than blindly duplicate resources |
| CommitSelection | upload_id -> selection_id | Validate the complete manifest and limits, atomically admit to coordinator; stable repeated successful commit receipt within a bounded caller ledger |
| AbortSelection | upload_id -> void | Idempotent cleanup; cannot delete original source files or another caller's preparation |

The first implementation milestone must freeze exact signatures, batch/aggregate/predecode limits, error names, replay/sequence receipts and complete fixtures before code. Fixed batch size must respect the supported broker/SDK configuration; it is a transport budget rather than a Google limit. No magic expiry timer is required: bounded preparations are released by abort, caller disconnect or shutdown, and explicit capacity errors remain visible. Lost caller ownership cannot silently convert an unfinished upload into a sendable selection.

Selection creation does not transmit. Explicit SendSelection uses a discovered opaque recipient and bounded request replay; queued/active transfers then survive client disconnect. The native chooser must own the connection that creates/sends its pending selection, as required by the existing coordinator. A CLI-to-other-client authority handoff is not granted by this proposal. UI activation/ingress must preserve this ownership rather than reopen sources in the daemon or transfer a bare token to an unrelated connection.

Authentication uses the actual bus message sender and connection credentials, with connection disappearance tracked before admitting owned resources. Same UID is not application/sandbox or Google-account authentication. The adapter must prove atomic CLOEXEC receipt and bound FDs/messages before unbounded decoding; checking a decoded Vec length alone is insufficient. No visibility/settings/account/auto-accept API is added in this slice.

## Next steps after selection

1. Record the chosen option; freeze the complete scoped wire/introspection/error/resource contracts and verification inventory before coding.
2. Implement isolated transaction ownership and strict decode/manifest validation, using the existing file-source/coordinator invariants. Test batch failure, duplicates/conflicts, lost replies, foreign callers, limits, disconnect and shutdown with owned disposable FDs.
3. Add authenticated private-bus integration against a task-owned temporary bus, then connect it to the retained native runtime owner. No user session registration, package installation or hardware test is implied.
4. Review terminal/replay retention before a continuously running production daemon; continue native GTK/KDE placement and Dolphin only once the actual chooser/binary work. Physical Google compatibility remains required.

Decision: implement option 1; option 2 is not selected. Rollback: retain the legacy UI/core entry points. Option implementation must be isolated from the core runtime/legacy UI and reversible without altering user settings or source files.
