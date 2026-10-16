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
   Width/placement policy and existing selection/consent limits stay unchanged.

Navigation/source/recovery controls must remain available after removing the
duplicated menu. No peer identifiers/PINs/personal contents in evidence. Existing
opaque preview handling is unchanged; offline widget rendering is visual QA,
not renewed payload-thumbnail development.

## Milestones before code

- [x] C1: inspect actual builder/CSS/render/actions/tray/legacy icons and document
  user requests, selected layout, invariants, verification and rollback here.
- [ ] C2: implement scoped transparent/rounded styling, header close, quiet idle
  labels and ordered layout; remove obsolete MenuButton bindings. Preserve shared
  hide/actions and safety/readiness logic. No daemon/core behavior changes.
- [ ] C3: restore standard application icons and add owned About access in the
  tray/Settings, including safety/cleanup. Increment package revision to14.
- [ ] C4: extend existing single GTK smoke for hide/no shutdown, absence of the
  removed controls/text, preserved exception/recovery, About reuse/lock cleanup
  and page/source visibility. Adjust existing tray/package contract assertions
  only for the explicitly selected icon/menu/assets. Run appropriate affected
  suites/lint once; core source tests already passed and are not repeated.
- [ ] C5: render Full/Compact/incoming/transfer widget fixtures on an isolated
  broker/display, inspect rounded alpha corners/layout and record actual images.
  No synthetic thumbnail development or physical-placement claim from Xvfb.
- [ ] C6: commit code/docs, build/verify distinct native14 with actual private
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
