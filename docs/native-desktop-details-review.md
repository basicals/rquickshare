# Native transfer detail and consent presentation review

2026-10-12. Mode A. Client state/control implementation: native-client-state.md.
This is review/prototype preparation, not approval to publish another data contract.

## Verified source findings

- Manager1 pages expose IDs, direction, coarse phase, consent revision and pending
  control. They have no sender, file manifest, verification PIN or byte progress.
- SessionSnapshot likewise contains identity/state/consent only. Both inbound and
  outbound update_state call session.observe(state), independent of the legacy
  ChannelMessage metadata route. Listening to the legacy broadcast separately
  would not establish ordered, recoverable native state and can lose details.
- hdl/info.rs TransferMetadata contains remote device info, PIN, destination path,
  file names, text description/payload, total_bytes and ack_bytes. It is not a
  native IPC DTO: do not dump/serialize it wholesale. utils RemoteDeviceInfo also
  contains information beyond a display label; inspect an explicit allowlist.
- Inbound introduction enters WaitingForUserConsent before preparing and then
  publishes metadata. Native detail readiness must be explicit and tied to the
  current consent round; a coarse waiting phase cannot enable blind acceptance.
  Actual session command processing/terminal observations remain authoritative.
- Normal info logs currently include remote device data, destination/file names,
  full consent metadata and PINs (both directions). Under the user's section 11
  maintenance authorization, replace these identified metadata/PIN log statements
  with fixed diagnostic messages immediately. Preserve protocol/state/channel
  behavior; do not broaden this into an unrelated repository-wide logging rewrite.

## Concrete options requiring user choice

Both keep same-UID authentication, fresh daemon epoch, existing Operations1 receipts
and source FD authority. Neither introduces cloud/Google credentials, private key
exposure, packet contents, clipboard payload or daemon-readable caller paths.
GUI acceptance requires actual validated details/PIN readiness, never placeholders.

| Option | Description | Benefits | Costs / risks |
|---|---|---|---|
| 1 — additive focused detail reads (recommended) | Preserve lightweight Manager1 list pages. Add bounded typed transfer detail header/manifest pages for a selected transfer, produced by an ordered core/session metadata seam and tied to epoch/consent/revision | Small compatible extension, avoids duplicating every manifest in each whole-service view; fits panel/notifications and current controls | Detail/readiness cache, byte/row budgets and cancellation/stale-round rules must be frozen and implemented; separate summary/detail reads need explicit freshness matching |
| 2 — new complete-view interface version | Introduce Manager2 immutable captures with summary plus bounded detail/manifest pages in one atomic view, retaining Manager1 for old clients | One coherent view for phase, details and progress; stronger whole-view consistency | Larger public-contract/client/cache migration and refactor; increased capture/private-metadata budgets; more rollback/compatibility verification |

Recommend 1: list and focused consent detail have different lifetimes and budgets.
The current summary is deliberately small, and an active consent/detail read can
be matched to its explicit round without copying large manifests for every client.
Option 2 is sound if a unified larger snapshot is wanted now. A channel-broadcast
bridge or blind accept UI is not an acceptable shortcut for either option.

After approval, freeze exact types/introspection, metadata allowlist/budgets and
readiness/retirement invariants before code. The proposed allowlist is display
label (untrusted advertisement, not authenticated Google identity), known device
kind only where available, actual four-digit verification PIN only for the live
round, payload kind/count, safe display names/sizes and actual measured byte
progress with documented total/ack semantics. Original paths, remote addresses,
private key/certificate material and text/clipboard contents stay out of this
read projection. Do not silently truncate a manifest or claim an unsupported
large transfer is fully described. Decide bounded admission/read failure against
existing inbound decoder limits in the implementation inventory, not guesses.

## Reviewable native widget prototype plan

Use GTK4 GtkBuilder .ui and CSS as reusable source; render three example views
(nearby, incoming consent, transfers) using the actual installed GTK4 runtime on
an owned Xvfb and private broker. Missing GI typelibs are extracted from Ubuntu
packages under chat work/, no system installation/configuration change. Python GI
is only the preview/test launcher, not the product language or a replacement for
the accepted Rust/gtk-rs architecture. No service connection, notification or real
peer is involved; example content is explicitly marked Preview.

Compact 420px content, no dominant empty drop zone, explicit Choose files and
recipient controls; persistent transfers and consent pages; hide/close remains
separate from transfer cancellation. Consent preview shows required-detail/PIN
slots with disabled actions, not an invented verification code or a usable blind
accept path. Source is isolated under docs/prototypes; it does not alter/install
legacy or native launchers. GtkBuilder parse/render and native pixels are visual
review evidence, not KDE/Wayland tray placement, live consent or feature parity.

GTK4 Layer Shell upstream supports KDE Plasma Wayland and edge/corner placement;
X11/GNOME Wayland lack that protocol. On the target, investigate StatusNotifier
Activate coordinates, output/scale selection and margins with the real compositor.
Do not silently ship a large ordinary-window fallback or claim clock anchoring
from the Xvfb review capture. A material fallback/platform decision needs approval.
Sources: https://github.com/wmww/gtk4-layer-shell and
https://wmww.github.io/gtk4-layer-shell/gtk4-layer-shell-GTK4-Layer-Shell.html.

## Preparation checklist and rollback

- [x] Identify missing ordered metadata and explicit consent readiness.
- [x] Remove the identified normal-log disclosure without changing transfer logic.
- [x] Build/inspect native widget prototype screenshots and record actual results.
- [x] Commit the recoverable review/prototype and prepare option 1/2 request under AGENTS 3/3.1.
- [ ] Implement selected new contract only after user choice.

Rollback diagnostic-only changes independently; prototype/review are independent
of SDK/daemon behavior. No user settings/download/source deletion, dependency
version update, live display/service modification or push. Terminal closed.

## Actual preparation observations

The seven identified normal metadata/PIN statements were replaced with fixed
messages. Strict core all-feature library Clippy passed after renaming the now
unused display-name loop binding; the existing zip/transfer logic is unchanged.
No protocol/functional tests were redundantly rerun for diagnostic-only changes.
This is a targeted disclosure fix, not a claim that all repository logging is
privacy-audited. Rust format/diff checks passed.

GtkBuilder parsed the committed source against actual GTK 4.22.4. Three mapped
widget trees were captured after their frame-clock after-paint event, at 420x497:
nearby, incoming and transfers. Native render-node/texture PNG export succeeded;
all three images were visually inspected. The Python preview launcher compiles.
No fixed sleep or periodic polling is used to wait for layout. The launcher runs
inside owned dbus-run-session/Xvfb children with a private XDG cache/config/data/
runtime and GI typelibs extracted in chat work/. All children exited normally;
terminal closed. Nothing was installed or activated on the user's desktop.

The widget-only prototype is docs/prototypes/native-panel.ui and .css; rendering
utility scripts/render_native_panel_preview.py. Reproduce with GTK4 GI available:
`dbus-run-session --config-file docs/prototypes/preview-bus.conf -- xvfb-run -a /usr/bin/python3 scripts/render_native_panel_preview.py --page nearby --output /absolute/path/preview.png`
Set GI_TYPELIB_PATH to the private extracted typelib directory if using the same
host preparation, GDK_BACKEND=x11, GSK_RENDERER=cairo and GTK_A11Y=none. Other page
values: incoming, transfers. Native pixels do not establish tray coordinates,
Wayland scale/focus/keyboard behavior, notification actions or Google compatibility.
Incoming actions are disabled and the PIN is absent until the selected contract
supplies real matched details; example peers/transfers are explicitly Preview.

Await user's option 1/2 before changing public metadata/readiness/security-facing
presentation under global AGENTS.md sections 3 and 3.1. This is the actual stopping
condition, not an arbitrary milestone boundary. The next agent can resume from
this review without guessing a schema or replaying old source selections.

Preview isolation refinement: the default private session broker initially
activated portal/accessibility helpers. Their logged PIDs were checked afterward:
none remained alive. The committed preview-bus.conf now has no service directories,
so GTK cannot activate those helpers. Rerendered all three affected previews using
that fixture: all succeeded at 420x497, no service activation in logs. This rerun
was justified by the changed isolation configuration, not a repeated stability
loop. Remaining Xvfb DRI3 warnings are non-fatal; texture capture uses Cairo.
