# Native GTK shell integration

2026-10-12. Mode A. Accepted compact GTK4 tray panel, complete Manager2 view and
bounded controller. Existing preview is docs/prototypes/native-panel.ui/.css.
This implementation remains uninstalled until compositor/package acceptance.

## Analysis and plan before code

The host has GTK4 4.22.4 runtime; GTK/Graphene development metadata and GTK4 Layer
Shell 1.3 development/runtime were downloaded/extracted under chat work/, without
apt installation. Use a separate native_desktop Rust crate to isolate GTK4/GLib
0.22 dependencies from existing GTK3/Tauri. Pin compatible GTK4 0.11 and Layer
Shell 0.8; preserve existing core/desktop lockfiles. Seed the new lockfile from
core to avoid unrelated core dependency upgrades. New shell depends on the SDK,
never creates RQS, opens network-peer sockets or owns transfer lifetime.

Milestones:
1. [x] Add native Rust crate/lockfile, pure tray-coordinate placement, GtkBuilder
   production widget source derived from the review and actual state rendering.
2. [x] Connect injected controller on owned Tokio runtime, GLib-only widgets,
   explicit file/folder chooser acquisition and send/consent/cancel/discard.
3. [x] StatusNotifierItem activation on KDE session bus; bounded activation hints,
   owned registration, no implicit installed daemon launch or fallback window.
4. [x] Focused pure/native widget/private-broker checks, one affected check run,
   strict lint/format/build, isolated actual rendering; record results and commit.

## Invariants and boundaries

One owned runtime/controller for the GTK application's lifetime. GLib receives
coalesced watch state asynchronously, no timer polling or blocking GTK callbacks.
Close/Escape hides the surface; explicit Quit joins local clients/acquisition and
tray tasks, not daemon stop. All mutations use the bounded controller and explicit
user actions. Complete sender/manifest/PIN is shown before acceptance. Treat sender
as advertised/unverified, labels as plain text, no markup evaluation. View exposes
unsupported payload truthfully and allows decline/cancel. No percentages implying
remote durable acknowledgement. All file rows are available in bounded scrolling.

Chooser acquires selected local roots through OpenedSelection::acquire on one
owned blocking ingress task; source paths never enter daemon IPC. Only one chooser/
acquisition is active, joined on exit, and acquiring/choosing does not send.
Existing committed selection must be sent/discarded before selecting another.
No forwarded CLI paths, new desktop selection handoff endpoint, settings writer,
Google identity/clipboard/credentials or notifications implementation in this slice.
Those still require their documented ownership/contracts and follow next.

Normal mode requires supported GTK4 Layer Shell and registers the KDE
org.kde.StatusNotifierItem. Activate coordinates are screen-coordinate hints,
not trusted file/transfer authority. Validate containing monitor geometry, choose
nearest monitor edge and clamp 420px panel margins within monitor; use logical
monitor coordinates (no additional integer scaling). Layer surface has zero
exclusive zone and on-demand keyboard interaction. Window remains hidden until
tray activation. Never claim physical clock anchoring from an Xvfb capture.
Preview is explicit --preview/--preview-output, never automatic platform fallback;
it does not connect a service or register a tray item, all controls disabled.

Status notifier registration uses a separate connection from NativeClient's
exclusive ordered ledger. Bounded latest activation hint, owned registration/
shutdown. Same-user session bus only. Sender/credentials on activation are checked;
no methods open paths or mutate transfers. Register/handle actual watcher ownership
changes without unbounded streams/tasks, or fail explicitly until restarted.
No live registration is performed by verification: private broker fixture only.

## Verification inventory / risks / rollback

Pure placement tables: all edges, monitor origin/negative origin, point outside,
small/invalid monitors, integer overflow, panel/margin clamping and scale semantics.
Native widget smoke: actual GtkBuilder parse/mapping, truthful ready/unavailable/
unsupported incoming state, complete scrolling rows, disabled stale/busy controls,
plain text and close/hide. Private tray fixture: activation coalescing, ownership/
UID checks, fixed interface/properties, registration/loss/close; no user tray.
Controller already has real private-broker action/lifetime checks; do not mirror
all business rules in GTK E2E. N1,N2,N3,N5,N7,N9,N10 applicable; N8 no locale/time,
N11 no billing, N12 source IO uses existing acquisition tests. Expected 5-8 pure/
widget/tray methods, record actual counts. No full Google/KDE hardware acceptance.

Rollback standalone native_desktop before the controller/core API. Risks: blocking
GTK, orphan source acquisition, false placement/scale, stale accept button, hidden
service failure and accidentally registering on user's real bus. No install,
settings modification, source deletion, Tauri retirement or push.

Sources:
https://docs.gtk.org/gtk4/section-threading.html
https://gtk-rs.org/gtk4-rs/stable/latest/book/main_event_loop.html
https://docs.rs/gtk4-layer-shell/0.8.0/gtk4_layer_shell/trait.LayerShell.html
https://specifications.freedesktop.org/status-notifier-item/latest/status-notifier-item.html
https://github.com/KDE/plasma-workspace/blob/master/applets/systemtray/statusnotifieritemsource.cpp
KDE uses org.kde interfaces although the freedesktop specification uses org.freedesktop.

## Actual observations

Plan recorded before code. Physical placement/keyboard/scale acceptance pending.

### Implementation review before final hardening

The standalone shell, controller binding, chooser ingress and KDE tray fixture
are implemented. Native crate check/build and strict Clippy passed after a tooltip
type alias; seven new methods passed across the initial six-success/one-failure
run and corrected two-widget run. The widget expectation initially omitted the
advertised-device suffix; the assertion now checks the complete literal label.
One later affected widget rerun passed after explicit GLib task ownership.
Three compiled executable preview captures are 420x498, not physical KDE proof.

Review identified ordinary zbus framing on the new tray connection while the SDK
already uses the hardened 64KiB/16-FD predecode socket. Before committing the shell,
expose a narrow shared connection builder around that existing socket and use it
for the SDK and tray. This is transport reuse, no new wire/auth/selection ownership
contract. Preserve queue eight and existing credential checks. Run affected SDK
and private tray checks once; do not repeat unchanged widget/business suites.
GLib subscriptions and the single completion waiter are explicitly aborted on
exit; tracked filesystem acquisition, controller and tray IO tasks are joined.

### Implemented state and measured verification

native_desktop is an independent pinned GTK4 0.11.3 / Layer Shell 0.8 crate with
its own committed lockfile. It consumes the real PanelController complete view,
with explicit chooser/send/respond/cancel/discard/recovery bindings. The chooser
rejects nonlocal roots and more than 256 selected roots before acquisition; folder
enumeration uses existing bounded OpenedSelection. No paths cross daemon IPC.
One tracked blocking ingress worker is joined; a previous finished worker is joined
before reuse. Preview controls cannot admit operations, even programmatically.
Preview never creates NativeClient or Tray. Initial state, pending operation,
unsynchronized/draining/disconnected/busy and local chooser work disable actions.

The tray uses the same predecode 64KiB/16-FD socket and eight-message queue as the
SDK through service::bus::connection_builder. Actual sender UID is resolved by
the broker for activation; watcher UID and unique owner are pinned before
registration. Owner change is sticky and closes the GUI client with an explicit
restart diagnostic, leaving daemon transfers alone. Normal mode requires Layer
Shell; an ordinary preview window is never used as a silent fallback. Validated
logical hints place/clamp against the containing monitor and measured panel height.
Same UID remains a desktop boundary, not sandbox isolation or proof of user intent.

Observed checks (development, no ten-run/stability claim):
- Initial native test link failed because privately extracted development symlinks
  lacked installed runtime targets. Added symlinks only under chat work/; /usr is
  unchanged. No apt installation.
- Initial seven-method run: six passed, one literal-label fixture assertion failed.
  Corrected the expected advertised-device suffix; both affected panel methods
  passed. After explicit GLib task ownership, the affected widget method passed
  again (0.17s). No weakening of markup, full 256-file or hide assertions.
- Both private-tray methods passed again after shared bounded transport (0.06s).
  Fourteen existing SDK methods passed after the shared builder extraction (5.25s).
- Native binary build passed. Final core all-feature/all-target and native
  all-target strict Clippy passed after shared transport extraction. The dependency
  core build without default features still emits nine existing unused/cfg warnings;
  these are not native shell lint errors. Core all-feature strict lint is clean.
  Existing bluer future-incompatibility notice remains. Format/diff checks passed.
- Compiled binary captured Nearby/Incoming/Transfers previews on a private broker
  with no activation directories and Xvfb; all three are 420x498, visually reviewed.
  They contain explicitly marked example data and disabled controls. The later
  transport-only extraction does not enter preview mode and did not warrant
  repeating unchanged screenshots.

Final source review also found that preview write failure and a lost tray watcher
printed an error but returned process success. Corrected by preserving a fixed
local exit failure through joined cleanup. Final native strict lint/build passed.
On 2026-10-13, one isolated negative executable check wrote to a directory target:
exit 1, fixed "Cannot save preview image" diagnostic, no file created; owned display
and broker exited. This is one observed executable N12 check in addition to seven
automated methods, not an eighth unit test or a repeated stability run. The lost-
watcher GUI exit code is source-checked; private tray loss itself is exercised.
This corrects diagnostic exit status, not product flow/API. Nightly format and
diff checks passed after the final correction. Terminal closed.

Seven new methods, primary categories: Positive 1, N1 1, N2 1, N5 1, N7 1,
N9 1, N10 1. Secondary N3 covers missing/lost watcher. N4/N6 existing SDK/controller
recovery and strict Manager2 checks are reused, no new ledger/schema in GTK.
N8 no locale/time rules; logical geometry is tested but physical DPI is pending.
N11 no monetization; N12 uses existing file-source validation/IO coverage rather
than a new storage backend. This is scoped Mode A verification, not exhaustive
GUI branch coverage or measured full-product coverage. Core inventory remains
496 unit/export + 8 integration; native crate adds 7 = 511 across both crates.
There was no fresh full-511 run. Prior measured suite results remain authoritative.

Remaining: real KDE scale/focus/monitor/clock placement and chooser interaction;
daemon-owned notifications; settings migration/editor, clipboard/reveal/autostart/
updates/package equivalence; context-menu ingress ownership; actual Google peers.
Tauri remains installed/current until those gates pass. No live daemon, real tray,
user settings, Dolphin entry, installation, push or source deletion by this work.
Work remains on dev; upstream remote-tracking refs are retained as requested.
All owned verification processes exited. Terminal closed.

The next context-menu ownership decision is reviewable in
[native-desktop-ingress-review.md](native-desktop-ingress-review.md). No new ingress
endpoint or daemon ownership-transfer method is implemented before that choice.

## 2026-10-13 CLI integration checkpoint

Context-menu ownership option 1 is implemented in native-desktop-ingress.md.
Normal panel presentation can also be requested by CLI import, reusing a validated
tray hint or the explicit compact Layer Shell initial placement. Chooser dialogs
reserve the shared preparation gate; CLI staging cannot replace them. Five bounded
GLib subscriptions (controller, local ingress, shared gate, desktop, tray) and the
completion waiter are owned and aborted on exit. CLI import/server, source worker,
controller and tray are joined. Nine native methods now exist and passed across
the scoped runs recorded in the ingress SSOT. Native notifications remain pending.
