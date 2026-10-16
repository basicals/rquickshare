# Card and tray cleanup

2026-10-16. Mode A, direct dev. Guide:/home/morozhenko/.codex/AGENTS.md,
sections3.1/11/12/13. The user explicitly requested eight presentation changes
after the first user-confirmed Android outgoing transfer. Layout choices within
those concrete requests are delegated; no daemon/protocol/security redesign.

## Analysis and current checkpoint

Installed native13 contains the named tmpfs fix.49 affected source checks and
strict core lint passed; installed eleven-file verification, /tmp append/abort,
repeat launch and one tray passed. User reported the123-byte file received on
Android with matching code. Daemon reports one outbound Finished and released
source resources, but terminal outgoing details are Unsupported with no manifest/
byte counters. Do not claim an independent remote hash or complete interoperability.

Current card repeats application branding, generic readiness and discovery text;
an options button duplicates the tray/context menu. The window has a global
border-radius rule but no separately transparent outer surface/rounded body.
Production code also loads CSS from docs/prototypes, which obscures ownership.
Tray uses network-wireless-symbolic while the launcher uses document-send.
Original application icons remain tracked under app/main/src-tauri/icons.
Existing hide already preserves application/daemon/transfer lifetime. There is
no About surface yet; Settings already has the application name in its title.

## Selected presentation contract and rationale

1. A transparent, undecorated card window contains one themed rounded body with
   padding and a subtle border. Scope CSS to the card; do not globally restyle
   Settings/dialog windows. Clip body content and remove the square window fill.
   Move the production stylesheet into native_desktop/ui; keep prototype assets
   as historical references instead of another live style owner.
2. The top row keeps Full/Compact at the left and a clear close icon at the upper
   right. Close routes to the existing hide action, like Escape/window-close;
   it never quits the GUI/daemon, cancels work or accepts/declines a request.
3. Remove the visible options/MenuButton and card branding/icon/readiness badge.
   Preserve tray/context and keyboard actions, source clearing and recovery.
4. Remove idle Choose files/Ready to share/Ready and the Discovery label/line.
   Visibility remains observable/changeable in the existing tray/Settings owner.
   Keep actionable error, disconnected, preparing, consent/code and progress
   information; hide empty status/source labels rather than leaving blank rows.
5. Restore the existing blue application icon for tray/launcher/dialog identity
   via standard hicolor assets at32/128/256. No new logo, image-generation work,
   network asset dependency or inline arbitrary decoder. GUI/tray use the same
   application icon name; package verification includes the three actual assets.
6. Keep the name in Settings and a standard GTK About dialog accessible from
   the tray and Settings. It is a secondary manual surface, owned/reused/closed
   with the existing GTK lifecycle and hidden on lock. No extra background owner.
7. Full layout follows task order: small header, navigation, exceptional status,
   selected source/actions, nearby/request/transfer content, explicit Send footer.
   Compact keeps the focused request/transfer or a direct send entry. Use consistent
   spacing, natural height and bounded lists instead of the fixed497px height.
   Width/edge-selection policy and existing selection/consent limits stay unchanged.

Navigation/source/recovery controls must remain available after removing the
duplicated menu. No peer identifiers/PINs/personal contents in evidence. Existing
opaque preview handling is unchanged; offline widget rendering is visual QA,
not renewed payload-thumbnail development.

## Milestones before code

- [x] C1: inspect actual builder/CSS/render/actions/tray/legacy icons and document
  user requests, selected layout, invariants, verification and rollback here.
- [x] C2: implement scoped transparent/rounded styling, header close, quiet idle
  labels and ordered layout; remove obsolete MenuButton bindings. Preserve shared
  hide/actions and safety/readiness logic. No daemon/core behavior changes.
- [x] C3: restore standard application icons and add owned About access in the
  tray/Settings, including safety/cleanup. Increment package revision to14.
- [x] C4: extend existing single GTK smoke for hide/no shutdown, absence of the
  removed controls/text, preserved exception/recovery, About reuse/lock cleanup
  and page/source visibility. Adjust existing tray/package contract assertions
  only for the explicitly selected icon/menu/assets. Run appropriate affected
  suites/lint once; core source tests already passed and are not repeated.
- [x] C5: render Full/Compact/incoming/transfer widget fixtures on an isolated
  broker/display, inspect rounded alpha corners/layout and record actual images.
  No synthetic thumbnail development or physical-placement claim from Xvfb.
- [ ] C6: commit code/docs, build/verify the final distinct candidate (native15 after the documented native14 visual failure) with actual private
  Layer Shell shlibs, preserve native13/12, then upgrade only known idle owners
  under existing application repair authorization. Do not interrupt active work.
- [ ] C7: record installed tray icon/menu/header checks and request the user's
  actual KDE corner/close/layout review. Continue remaining reverse Android,
  versions/content/interruption/Windows acceptance separately from cosmetics.

## Risks and rollback

Theme backgrounds/compositor opaque-region behavior can differ; a renderer image
does not establish the actual KDE result. Removing an options button must not
remove any essential source-clear/recovery/appearance action. About must not
outlive its owner or bypass lock safety. New icon files change the allowlisted
package inventory from11 to14; no unsigned package is a published update channel.
Rollback is each scoped source/docs commit and the retained native13 package;
do not downgrade while active transfers/preparations exist. Never kill unknown
owners or enable login start as part of cosmetic repair.

All prior one-shot commands/observers/builders exited: Terminal closed. Installed
native13 remains running; no watcher/test terminal is left behind. No key/Drive,
signing, publication, accounts, preview or advanced protocol work is authorized
by this presentation slice.

## C2-C3 implementation checkpoint (verification pending)

The actual builder now has a small mode/close header, then tabs, exceptional
status, source controls, content and the recipient/Send footer. No brand icon/
name/MenuButton/badge/discovery widget remains on the card. Idle status/selection
labels are empty/hidden; source replacement buttons hide while a selection exists,
leaving its summary/clear action. Disconnected/recovery/progress/consent remain.
Full natural height replaces497px; list bounds and420px width are unchanged.

Only the card window is transparent; its padded body owns themed background,
border and rounded corners with overflow hidden. Production CSS now lives in
native_desktop/ui/panel.css, not docs/prototypes. Close activates the existing
panel.hide action without shutdown or cancellation. A standard owned/reused
About dialog is reachable from tray/context/Settings, participates in manual
surface suppression, hides on lock and is destroyed on owner cleanup.

Tray and launcher use rquickshare, restoring the existing32/128/256 legacy blue
icons via the package allowlist. New About is explicit menu action22; existing
IDs/signatures/bounds/authentication are preserved. Allowlisted inventory becomes
14 regular files. Packaging validates matching launcher icon metadata. Revision14
is reserved; source repair/core/daemon/wire/settings behavior is unchanged.

Focused checks about to run: existing39 native methods with the single GTK smoke
extended for the requested controls/close/About/safety, existing tray projection/
real-bus menu assertions, existing native Debian package methods, strict native
all-target/no-deps lint and formatting/whitespace. No source-suite repetition or
new Mode B coverage project. Later fixture rendering checks geometry/alpha only;
actual KDE corners still require the user's installed visual observation.

## C4 observed checks

The39 native methods passed once (0.99s), including the extended single GTK
smoke, real private-bus icon/About menu behavior, explicit close/hide with an
acquired preparation gate retained, removed widgets/text, preserved visibility
action state, About reuse/lock and owner cleanup. The five native Debian methods
passed once (2.137s), including the14-file/icon inventory and archive tampering
guards. Strict native all-target/no-deps Clippy passed7.32s; the same nine existing
nonexperimental-core dependency warnings remain visible. Formatting/whitespace
passed. No full core-suite or ten-run repetition. Logs:/tmp/rqs-card-cleanup-native.log,
/tmp/rqs-card-cleanup-package.log, /tmp/rqs-card-cleanup-clippy.log.
All owned verification commands exited: Terminal closed. An isolated widget-render
build is a separate currently owned command; no user owner was stopped.

Native packaging owns exact copies of the selected original icons under
native_desktop/assets/icons, so removing the legacy Tauri tree later cannot break
native packaging. Original bytes remain unchanged and the legacy baseline is
preserved. This internal asset-ownership adjustment affects only packaging; rerun
the affected five package checks once, not the unchanged GTK/core/lint suites.

## C5 first visual review and scoped height correction

Four actual isolated widget PNGs rendered successfully: Compact Nearby420x137,
Full Nearby420x475, Compact Incoming420x330, Full Transfers420x367. All four
corner alpha values are0 and center alpha255. No CSS/parser/critical error was
reported; software Xvfb emitted only DRI3 acceleration warnings. This establishes
renderer transparency, not an actual KDE compositor signoff.

Visual inspection found surplus space below a short Nearby list. Further source
inspection found a second fixed497px minimum inside place_at, despite removal of
window default height. Before this correction, document the final layout choice:
use a120px minimum,300px maximum natural-height list, nonhomogeneous/nonexpanding
pages, and the actual widget natural measurement for monitor-fit/edge placement.
The nearest tray edge, width420, monitor guards and consent/source lifecycle are
unchanged. This removes a brittle presentation constant rather than masking its
symptom. Incoming consent text and a long list remain bounded and scrollable.

Only the affected single GTK smoke and native lint are rerun after this height
change. Package checks after icon ownership relocation passed5/0 in1.960s; their
code/assets are unchanged by the height correction. Build/verify native14 once
from the committed final source, then render the final four cases using that
release binary, avoiding another unrelated debug build/full source suite.
All owned initial render/build processes exited: Terminal closed.

After the documented height correction, the affected single GTK smoke passed1/0
(38 filtered) and strict native all-target/no-deps Clippy passed. The prior39
methods and final five package checks remain measured above; no claim that the
whole39 were repeated after this small presentation change. Final release rendering
and installed KDE observation remain pending. Runtime/core source is unchanged.
All owned affected-check processes exited: Terminal closed.

## C5 release visual finding: natural measurement was not allocated

Native14 built/verified with14 regular files, SHA256
7e47b89b5e3ee98002e816a7cf4765bf12d500023a85d696599fd23ca68b3515.
Final release PNG corners remain transparent, but Full Nearby420x375 and Full
Transfers420x267 prematurely scroll after about one row. The actual GtkWindow
default height still requested1(minimum), while natural height was used only
for monitor-fit. This visual failure was found before installation; native13
remains installed. Preserve native14 as an intermediate diagnostic artifact.

Before correcting code: centralize content sizing after row/fixture population,
reset the old default height before measurement, then request the real natural
height. Scrolled lists still cap at300 and close/consent/placement ownership is
unchanged. Call this sizing step for complete, unavailable and fixture renders.
Increment final candidate to native15; do not overwrite native14 bytes. Rerun only
affected GTK smoke/lint after this correction, then build and visually inspect
the final release. No premature installation or successful layout claim.

The post-population sizing correction passed the affected GTK smoke1/0 and
strict native all-target/no-deps lint. The failed native14 geometry remains in
evidence alongside its checksum; final candidate is native15. No core source
change or package-check repetition was needed. All owned checks exited: Terminal closed.

## C5 final release visual result and C6 installation gate

Native15 built/verified14 files from344909f; daemon0.26s(cache), GUI24.71s,
helper0.22s(cache). SHA256 f32e3f35021c358e3357bab066ce695c9d8a2272616f030d9a29e9b9c0340099.
Final release PNGs are Compact Nearby420x137, Full Nearby420x421, Compact
Incoming420x330 and Full Transfers420x377. Both example devices/transfers fit
fully; the premature scrolling is corrected. All four corner alpha values are0.
Fixtures are explicitly marked inert previews; production idle text is hidden.
No actual KDE compositor observation is implied by these isolated frames.
The trailing search returned1 because no network-wireless reference remained,
not because rendering/build failed. All owned render processes exited.

Next, use existing repair authorization to check installed native13 hash, known
owned daemon/client PIDs1909694/1909695, no source/preparation resources and no
nonterminal transfers. Completed history is not active work and may exist. Close
only the owned idle services gracefully, install the exact native15 local .deb
with pkexec apt-get, reload without enablement and launch normally. Require14
installed files to match, one tray/client/daemon and actual restored icon/menu.
Never stop an unknown owner or active transfer. Native13 remains the rollback
package; native14 is diagnostic and has never been installed.
