# Native selection transactions

2026-10-12. Mode A. User-approved option 1 from native-dbus-selection-review.md. This document is the implementation SSOT. The existing descriptor boundary and native runtime remain authoritative. No sources are reopened by destination name, no implicit send occurs, and no Google compatibility claim follows from these checks.

## Milestones and rollback

- [x] S1: Add owner-bound Begin/Append/Get/Commit/Abort to the coordinator, include preparations in its existing aggregate reservations, and expose the same operations through NativeRuntime. Extract shared manifest validation without changing acquisition behavior. Add focused domain regression checks.
- [ ] S2: Add a Linux-only zbus transport with atomic CLOEXEC FD receipt, fixed ancillary storage, aggregate frame FD limit and header length check before zbus allocates the frame. Add receive-boundary checks.
- [ ] S3: Implement a serial authenticated selection adapter and retained runtime host, fixed introspection fixture, private-bus integration and cleanup checks. No live session registration, activation files, GTK, Dolphin or account API in this slice.
- [ ] S4: Run appropriate core checks once, record actual outcomes, update the diary and make reversible conventional commits on dev.

S1 can be reverted independently of legacy entry points. S2/S3 add a native-only dependency and adapter; no existing UI is switched. Revert adapters before the coordinator changes. No schema migration or source-file write is needed. Risks: FD inheritance, frame/allocation amplification, partial append, caller-disconnect races, duplicated commit and staged quotas bypassing aggregate limits. Every corresponding rejection must preserve the preceding preparation and close only newly rejected handles.

## Frozen scoped Manager1 contract

Service `io.github.morozhenko_apps.RQuickShare`, path `/io/github/morozhenko_apps/RQuickShare`, interface `io.github.morozhenko_apps.RQuickShare.Manager1`. Exact introspection is `dbus/io.github.morozhenko_apps.RQuickShare.Manager1.xml`. The initial adapter exposes only the five selection methods; snapshot/send/control APIs remain the existing trusted Rust operations until their wire fixtures are implemented. This is a scoped foundation, not the desktop replacement.

| Method | Input signature | Output signature | Meaning |
|---|---|---|---|
| BeginSelection | s | s | ASCII graphic request ID, 1..128 bytes; same caller/request returns the same bounded upload receipt, including aborted/committed receipts |
| AppendFiles | sua(hs) | u | Upload ID, zero-based expected sequence, 1..16 FD/name entries; atomic batch; returns next sequence |
| GetUpload | s | uss | Next sequence, state (`preparing`, `committed`, `aborted`), committed selection ID or empty string |
| CommitSelection | s | s | Nonempty complete manifest becomes an owned selection once; repeated successful commit returns the original ID |
| AbortSelection | s | empty | Drop preparation, retain bounded aborted receipt; repeat/unknown token is harmless; foreign known token is rejected; committed upload cannot be aborted |

Tokens are canonical lowercase `epoch32hex-counter16hex`; parse rejects noncanonical/zero/wrong-epoch tokens. Client identity is never a wire argument. GetUpload reconciles lost append replies: retrying an accepted sequence returns ReplayConflict, never duplicates its files. Commit receipts survive selection send/discard while the caller connection remains. Receipt IDs are not new authority and never move ownership to another connection. Unknown upload and non-preparing upload are distinct `UnknownUpload` / `InvalidState` errors.

Application errors use `io.github.morozhenko_apps.RQuickShare.Error.<ErrorVariant>` with a fixed non-sensitive message. Existing coordinator variants retain their meaning. Invalid signature/name/descriptor/manifest is InvalidInput; missing or mismatched broker UID is AccessDenied. Unknown path/interface/method uses standard org.freedesktop.DBus errors. No source name/path, FD value, credentials detail or raw OS error enters responses/logs. Introspection requires a live authorized sender too. No settings/visibility/auto-accept change is introduced.

## Ownership and resource budgets

- Up to 4 preparing uploads and 128 total upload receipts per caller; up to 32 callers. Receipts are not silently evicted; Capacity requires an explicit reconnect/new lifecycle. This limitation is visible and must be reviewed together with transfer replay retention before a continuously running product daemon.
- Preparing uploads count as selections from Begin, including empty ones. Files reserve twice their handle count and their declared bytes in the **same** existing coordinator budget as committed/queued/active resources (128 selections, 1024 reserved handles, 4 TiB). Commit replaces a preparation reservation without a release/reacquire gap. Direct trusted create_selection must also respect staged reservations.
- Each complete selection retains existing 256-file, 64-directory-depth, 1-TiB limits and strict UTF-8 relative names (16 KiB total name, 255 bytes/component; reject controls, traversal, absolute/empty components, backslash, colon, duplicate and file/descendant collisions). Names are destination metadata only. Batches validate the complete candidate manifest before mutation, without duplicating earlier FDs.
- Append accepts already owned read-only regular files only. Received FDs are first received atomically CLOEXEC; owned decoding duplicates at most 16 CLOEXEC handles after bounded borrowed decoding and unique/exact FD-reference checks. A source can still be changed by its owner; this grants read authority, not immutable content snapshots.
- Transport cap: 64 KiB/frame, 16 FDs/frame (aggregate across fragmented recvmsg), fixed ancillary capacity 16, reject control truncation. Lengths are inspected from the 16-byte primary header before zbus frame resize; both endiannesses and checked length arithmetic are supported. A valid batch must also fit the byte cap, so callers split long-name batches further. Invalid frames close the bus connection; no path fallback.
- zbus receive queue 8 plus adapter inbox 8, one processing message and one transport frame: at most 18 retained inbound frames / 288 ancillary handles, independently bounded from admitted resources. Borrowed FD decoding avoids repeated-index FD amplification. No object-server task is spawned per method. A single pump drains zbus into a try_send inbox; inbox overflow fails closed and shuts down the host, rather than deadlocking broker credential replies behind a full application queue. Flood availability is not promised.
- Caller credentials come from the broker for the actual unique message sender, checked against service UID, with NameHasOwner before admission. NameOwnerChanged is subscribed before claiming the service and processed in the same ordered inbox as calls. Disconnect releases unfinished/unsent resources and caller receipts; queued/active transfers remain service-owned. Bus loss, name loss, inbox overflow and requested stop join the runtime and pump and close the owned bus connection. No guessed expiry timeout or detached worker is needed.
- Acquire the well-known name with DoNotQueue and no replacement before core.start; a second host must fail before starting transports. Same UID is not sandbox isolation or Google-account identity.

## Focused verification plan (Mode A)

Domain: full multi-batch admission/commit/send; Begin and Commit replay; append stale/future sequence and closed-state rejection; invalid batch with valid prefix leaves previous state/quotas intact; empty commit; manifest conflicts/depth/file/byte limits; combined staged/direct/active quotas; per-caller preparation/receipt caps; foreign/unknown/stale tokens; abort/disconnect/shutdown close handles without unlink; revision exhaustion cannot consume preparation. Use owned temporary files and existing real coordinator/runtime; no mocks of domain logic.

Transport: little/big endian and fragmented primary headers; max/max+1 frame and FD counts; multiple SCM_RIGHTS fragments; truncation/EOF; CLOEXEC observed on receipt; invalid frames do not allocate advertised payload size. Socket pairs are local I/O fixtures, not external devices.

Private bus: a test-owned foreground dbus-daemon with child kill+wait RAII; exercise exact introspection, FD calls, malformed calls, foreign client, lost-reply reconciliation, actual unique-name disconnect cleanup, denied UID configuration, duplicate-name rejection and runtime-host requested stop. No user-session bus is registered. No sleeps for readiness; use daemon's printed address and actual protocol replies/events. Real BlueZ/mDNS startup and Google devices remain separate gates.

## Verification observations

S1: `cargo test --manifest-path core_lib/Cargo.toml service::uploads --lib` passed all 13 new domain methods (table-driven boundaries included), 0 failures/ignored, in 0.04 s. A temporary unused adapter accessor warning was removed before commit; it will be added with its actual consumer in S3. The owned command exited; terminal closed. S2/S3/final checks not run yet. Earlier runtime results remain in native-core-runtime.md and are not replaced by plans here.

## Dependency review

Inspected cached zbus 5.19.0 source and official documentation: [ReadHalf](https://docs.rs/zbus/5.19.0/zbus/connection/socket/trait.ReadHalf.html), [Builder](https://docs.rs/zbus/5.19.0/zbus/connection/struct.Builder.html), [broker limits](https://dbus.freedesktop.org/doc/dbus-daemon.1.html). The inspected default Unix recvmsg uses empty receive flags and does not supply this application's aggregate frame bound. Therefore the adapter supplies a safe rustix transport while retaining zbus's authentication and message parser. No SDK internals are copied or patched.
