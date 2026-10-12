# Native desktop capability ingress

2026-10-13. Mode A. Option 1 selected by delegated architectural judgment.
Quality, correctness, ownership and recovery take priority over work volume/speed.

## Analysis / plan before code

Manager2 selections belong to the exclusive SDK connection. CLI cannot hand its
selection token to GTK. Implement toolkit-independent desktop protocol alongside
PanelController in core_lib::service::client::desktop, allowing real private-bus
tests to reuse the existing coordinator/SDK harness without mocking domain logic
or making the core depend on GTK test sources. Native desktop supplies presentation,
safe local acquisition and process startup only. No daemon adoption API.

Milestones / rollback:
1. [ ] Shared preparation gate, native manifest validation, owned desktop import
   server/client with bounded framing, authentication, replay and disconnect.
   Roll back this SDK addition before its GUI consumer; daemon API is unchanged.
2. [ ] CLI local path/file-URI parser and event-driven GUI cold start; chooser and
   CLI share the gate; present the existing recipient flow. No paths in GUI/daemon
   IPC, invocation never sends. Roll back native bindings independently.
3. [ ] Scoped real private-broker / parser / lifecycle checks, strict affected
   lint/build/format once; record actual observations, commit on dev.
4. [ ] Add package-owned Dolphin assets only after the actual CLI/recipient flow
   works. No live install. Notifications/settings/full parity remain separate gates.

## Frozen Desktop1 contract

Bus service: io.github.morozhenko_apps.RQuickShare.Desktop.
Path: /io/github/morozhenko_apps/RQuickShare/Desktop.
Interface: io.github.morozhenko_apps.RQuickShare.Desktop1.

| Method | Signature | Contract |
|---|---|---|
| Begin | () -> t | One import per dedicated sender connection; replay returns its same nonzero token. Competing preparation returns Busy and presents existing panel. |
| AppendFiles | (t,u,a(hs)) -> u | <=16 read-only regular FDs and relative names; strict contiguous batch sequence, exact object/name/size replay returns original next sequence. Conflicts do not replace admitted sources. |
| Commit | t -> s | Validate complete selection, transfer resources to the GUI-owned worker before awaiting SDK preparation. Repeated Commit awaits/returns that same result; never submits Send. |
| Abort | t -> s | Releases staging only; after commitment returns current result and never cancels a daemon selection/transfer. |
| Show | () -> () | Present recipient view; no source opening, transfer or consent mutation. |

Status strings: ready, recovery, failed, aborted. Commit waits on completion,
without polling. A recovery result is explicit uncertainty; the user recovers in
the panel, and CLI does not start a replacement. One retained import/receipt total,
bounded by the sender lifetime; no timers/TTL. Retired token never starts new work.
Other senders cannot inspect/append/commit/abort the record. Sender loss releases
staging. After Commit admission, GUI owns resources and finishes preparation even
if CLI closes; no automatic cancellation/send. GUI shutdown joins preparation.

Use shared 64KiB/16-FD predecode framing and queue eight. Serial object dispatch,
one continuously drained broker owner-change subscription and one owned preparation
task; no per-request work spawning. Actual sender UID from broker before admission,
same UID required. Client pins actual GUI unique owner and validates reply sender;
owner change fails closed with retained uncertainty, never silently rebinds.

The GUI boot/connection scopes tokens. Exact replay fingerprints include relative
name, size, device and inode from the acquired read handle. This identifies the
selected object without claiming immutable contents. Received handles get atomic
CLOEXEC via the shared socket and pass OpenedFile::adopt; no received name is opened.
Cross-file collisions and byte budgets use existing OpenedSelection validation.

Native preparations must fit complete-view budgets: <=256 files, <=1024 UTF-8
bytes per relative name and <=64KiB aggregate names, plus existing <=1TiB selection
budget. Apply before both local chooser/controller and CLI IPC. This prevents
large 16-FD batches exceeding framing and ensures the native selection can be
reviewed using its Manager2 contract. Legacy/core file-source contract is unchanged;
unsupported large native manifests fail atomically, never silently send a subset.

## GUI / CLI ownership and startup

One shared gate spans file dialog, local acquisition, desktop staging and controller
preparation. No replacement or new preparation while occupied or while a committed
selection/pending mutation already exists. Show the current panel with a fixed
actionable Busy error. User may explicitly abort external staging from the panel.
Existing consent/cancel/recovery continue to use the actual controller contracts.

CLI grammar: rquickshare send -- <local-path-or-file-URI>...; 1..256 roots, bounded
argument bytes, all-or-nothing local acquisition. Decode percent escapes once,
allow empty/localhost URI authority, reject remote schemes/hosts, queries/fragments,
malformed escapes and NUL. Preserve Unicode/spaces/percent signs and raw local OS
paths. Relative paths resolve in the invoking process. No shell evaluation/KIO
downloads and no paths forwarded to a more privileged process.

CLI acquires first, then subscribes to GUI name ownership before lookup/start.
If absent, launch the same executable without paths; await owner event or owned
child exit, not sleeps. A successfully started user GUI intentionally outlives CLI;
verification fixtures are explicitly reaped. No implicit daemon startup. Missing
daemon/unsupported compositor/failed child yield fixed visible errors and no send.

Commit presents Nearby. Reuse last validated tray hint when available; before any
tray hint, use a compact Layer Shell surface at the first monitor's bottom-right
edge. This is explicit initial placement, not proof of actual clock location and
not an ordinary-window fallback. Physical KDE monitor/scale/focus acceptance remains.

## Mode A verification inventory

Expected 7-10 new methods, tables for independent boundary variants:
- Local URI/path grammar: Unicode/spaces/percent/raw paths, remote authority/scheme,
  malformed escapes, empty/excess/count/size, fragments/NUL (Positive,N1,N2,N7).
- Native manifest name/count/aggregate boundaries; no partial SDK admission (N2).
- Actual private broker, controller and coordinator: >=17 FDs, complete import,
  no send, GUI ownership survives CLI disconnect; exact batch/commit replay (Positive,N4).
- Wrong owner, writable/special handles, collisions, gaps/conflicts, aggregate
  limits and gate pressure; rejected batch leaves prior stage intact (N1,N2,N7).
- Interrupted Begin/Append/Commit replies: source retention, same owner recovery,
  no duplicate selection/send; owner replacement rejected (N4,N5,N6).
- Explicit abort, staging disconnect, GUI shutdown/closed gate and hidden-panel
  presentation hints; no originals deleted or unowned workers (N3,N5,N9,N12).
- Startup failure event/child exit, without real user bus/compositor registration.

N10 fixed errors/redacted Debug/no input logging; tests use nonprivate fixtures.
N8 no locale/time rules, physical scaling still external; N11 no monetization.
Do not mock controller/domain logic; reuse actual core private harness. No Mode B
exhaustive coverage/ten-run claim. Preserve previous 511 inventory/observations.
