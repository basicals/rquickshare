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
1. [x] Shared preparation gate, native manifest validation, owned desktop import
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

## Implementation review / remaining binding checks

The shared SDK protocol and controller manifest validation are implemented. The
preparation gate serializes ownership and watch publication under one short mutex;
separate atomic stores could publish a stale idle state during permit replacement.
Begin verifies sender liveness after admission as well as credentials before it:
the disconnect stream may have already consumed an owner loss before admission.
Abort waits for the existing committed result, never exposes internal preparing.

Initial SDK run: six passed, one assertion failed because a 17-FD message is
correctly rejected by predecode transport (connection closed), before method
validation. The offending sender now uses its own fixture connection; the test
asserts Transport and unchanged admitted sources. That affected method passed.
An additional exact 64KiB / limit+1 / 256-file / prefix-collision method passed.
Strict all-feature/all-target core Clippy passed. A release barrier for the existing
test-only reply hook now permits observing successful worker completion after CLI
disconnect as well as joined shutdown; its affected checks still need execution.

Native bindings compile and nine native methods passed once on the isolated broker
and Xvfb (two new CLI methods plus seven existing). This proves parser/startup
failure and widget/source bindings, not a physical Dolphin/compositor invocation.
The missing-GUI child uses no arguments, inherits no file handles, and is reaped
on its failure path. Strengthen fixture teardown with an RAII broker owner before
the final scoped lint; no live-user registration is part of verification.

Before package assets: finish explicit daemon/tray readiness before publishing
Desktop ownership. Otherwise a cold CLI could prepare successfully while required
tray registration is still failing. A losing concurrent panel closes its own tray
connection and SDK ledger. Keep immutable selection ownership and no automatic send.

Dolphin assets will be package-owned under share/kio/servicemenus, for local files
and directories only, 1..256 roots, invoking `rquickshare send -- %U` directly.
No shell wrapper, embedded path substitutions, self-copy to user config, or live
installation. Package binary/menu together; the legacy Tauri binary is not a CLI
consumer. Use a standard themed send icon until packaging integrates the existing
app icon. Validate metadata statically; real Dolphin discovery remains a hardware
acceptance gate. Source: https://develop.kde.org/docs/apps/dolphin/service-menus/

### M1 completion / SDK verification

2026-10-13: eight new desktop methods implemented; the final affected client-layer
run passed **43/43** in 5.28s, including desktop, controller, state/watch and ordered
SDK methods affected by the test-only cancellation seam. Successful preparation
after committed CLI disconnect and interrupted joined shutdown both use the real
coordinator. Original files remain intact; no automatic transfer is created.
Final core all-feature/all-target Clippy with `-D warnings` passed in 3.49s.
The existing bluer future-incompatibility notice remains unchanged. Format passed.
All command sessions exited normally (terminal closed). No repeat/stability gate
or complete-product acceptance is claimed. Core test inventory is now 504
unit/export + 8 integration; this is not a fresh complete 512-test run.

New SDK methods by primary purpose: Positive 1; N2 limits 2; N4 replay/cancellation
1; N5 admitted worker lifetime 1; N7 owner/capability/UID checks 2; N9 abort/close 1.
Tables also exercise N1 malformed names and N3/N6 connection/owner loss. N10
redacted diagnostics and N12 retained originals/handles are assertions within those
methods. N8/N11 remain inapplicable for the reasons above. No percentages or
exhaustive branch-coverage claim; native CLI/widget checks are reported separately.
