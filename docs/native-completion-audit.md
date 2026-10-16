# Native completion audit and next everyday UI slice

2026-10-16. Mode A, direct dev. Guide: /home/morozhenko/.codex/AGENTS.md,
sections3/3.1/11/12/13. User requested continuing implementation and an honest
assessment of whether basic features and UI are complete. No installation,
physical peer, account/cloud call, package rebuild or runtime change in this audit.

## Plan and actual audit

1. [x] A1: inspect the continuation index/roadmap, current GTK menus/settings,
   typed state/settings projections, runtime visibility application and actual
   native ingress gates. Separate source support from installed/peer acceptance.
2. [x] A2: document readiness and remaining work. Correct the old native7 release
   paragraph and obsolete roadmap continuation markers; retain dated evidence.
3. [x] A3: record the significant UI/API alternatives before code; user selected
   option1, daemon-owned live visibility and quick controls.
4. [x] A4: [contract and measured source checks](native-live-visibility.md)
   cover current policy, intent/retry/reconnect and GTK/tray controls. Source
   implementation, native9 offline verification and exact artifact export are complete.

Rollback of this audit is documentation only. Native8 artifacts remain immutable;
no unaffected Rust/GTK/package checks are repeated for a documentation change.

## Readiness matrix

| Area | Implemented source | Still missing implementation | Separate acceptance |
|---|---|---|---|
| Manual sharing | Native file/folder selection, multiple files, one explicit recipient, PIN/Accept/Reject, progress/cancel; core BLE/mDNS and LAN upgrade paths | Full advanced-payload/peer parity is not achieved | Current native9 with real Android/Google Windows; radio failure/large files/interruptions |
| Text and clipboard | Outbound text/URL and explicit clipboard capture; reviewed immutable PNG image source; incoming text/URL review and explicit Copy | Complete Remote Copy negotiation/peer semantics; native Wi-Fi/app completeness | Current peer/clipboard/portal behavior on installed desktop |
| Native shell | Rust daemon + authenticated typed D-Bus + GTK4 Layer Shell card/tray, Full/Compact, menu/navigation, consent details and terminal states; live visibility/quick actions | Localization; durable user-facing history/polish | KDE/Wayland near-clock positioning, focus/lock/DND/scaling/accessibility and tray/menu rendering |
| Settings and desktop | Daemon-owned Settings2/schema2, migration/recovery, live name, receive-directory capability, opt-in daemon+tray login start and Dolphin Send assets | Broader desktop/share-target integration is not complete | Installed context menu, autostart, restart, upgrade/removal, recovery and live-transfer behavior |
| Preview images | Bounded byte ingress, real isolated Glycin/Bwrap helper, bounded pixels/cancel/deadline/reaping, synthetic Full/Compact fixture | Canonical peer payload encoding/mapping/order, admitted core routing, daemon IPC/production-card binding and outgoing previews | Real peer preview interoperability; fixture is not this evidence |
| Resume/dedup | Modern wire fields only | Content identity, safe partial storage/integrity/recovery and actual negotiation | Interrupted transfers against supported peers |
| Other payloads | Modern proto fields and some core/legacy parsing | Streams, multiple recipients, complete Wi-Fi/app/sensitive-content behavior | Versioned capability matrix and physical peers |
| Trust/account/platform parity | Bounded token/role/certificate verifier; production certificate source explicitly empty | Supported Google certificate acquisition/rotation, remembered trust/auto-accept, Contacts/Your devices, compatible QR and Apple transport work | Supported account authorization and applicable real platform paths |
| Distribution | Native9 uninstalled Ubuntu26.04 amd64 package/unsigned APT/public unapproved request; source export verified; HTTPS/R2 and local signer/recovery tooling | First approved signed publication and scoped publisher access; ARM64/other package formats remain open | Installed updates/uninstall and actual signed public APT consumption |

Basic manual workflows and a substantial native shell exist in source. Neither
"all basic functionality is release accepted" nor "UI is finished" is supported.
Old core/legacy physical observations do not validate the current native package.
Native9 is locally verified and uninstalled, not a replacement already installed
on the user's desktop. No percentage estimate or full Google compatibility claim.

The shell already uses GTK/theme-relative CSS; do not list all dark/system styling
as nonexistent merely because the old roadmap has a polish bullet. Locale strings
are currently fixed English. The daemon retains in-memory transfer rows; do not
call the Transfers page a completed persistent history product. Background consent
notifications exist and should not be listed as wholly missing either.

## Selected visibility scope: pre-implementation analysis and decision

At A1 (before the selected implementation), the core owned expiry and exposed
read-only VisibilityReader. Then-current
native StateView/Manager2 has no live mode/deadline projection. SettingsView has
the saved visibility intent, not the current expired policy. The GTK editor has
a visibility dropdown; the native DBusMenu/GTK menu has no visibility action.
Therefore a user can configure temporary visibility but the shell cannot display
its authoritative expiry state. Do not use saved Temporary as proof of currently
visible transport or add a GTK-owned expiry policy to hide that gap.

Relevant source: core_lib/src/visibility.rs, service/runtime.rs,
service/settings_service.rs, service/client/state.rs, native_desktop/src/panel/
settings.rs, menu.rs, tray.rs and native_desktop/src/tray/menu.rs. This is an
everyday UI gap that can progress without guessing undocumented preview bytes.

| Option | Description | Advantages | Costs / risks |
|---|---|---|---|
| 1 — daemon visibility facade, recommended | Publish actual core policy through an authenticated typed facade; card/tray consume it and send explicit visibility intent commands. The existing daemon settings writer remains sole persistence owner and the core remains sole deadline owner. Add the three existing modes to quick controls. | Same source for all surfaces; expiry/reconnect is observable; UI does not orchestrate saved-versus-current policy or own timers. Explicit command identity can prevent retry from renewing a window. | New IPC/API and coordinated daemon/client/GTK/menu implementation; persistence/live-apply failure and stale command behavior need a frozen contract. Policy acceptance still does not prove on-air withdrawal. |
| 2 — GTK orchestrates existing settings saves | Add quick controls that read/edit/save Settings2 in the GTK client, plus a read-only live status projection. Reuse current settings error/recovery handling. | Reuses the existing save path and fewer daemon command surfaces. Persistent mode intent has the same writer. | Presentation must coordinate current state, saved revisions, retry and explicit Temporary renewal; conflicts/expiry/reconnect logic can drift between settings, card and tray. A plain repeated save currently intentionally does not renew Temporary, so it is insufficient by itself. |

Recommend1 for ownership, correctness and maintainability. Scope uses Visible,
Hidden and the already selected one-minute mode; it does not introduce Google
account visibility, a new duration or automatic transfer acceptance. Quick-mode
intent should use the same saved-mode/startup semantics as explicit Settings
selection, while current expiry stays separate. Exact durable-write/live-apply,
failure, replay and renewal semantics must be frozen at A4 before implementation.
Do not silently replace settings authority or reopen an expired mode on unrelated
saves. No second expiry clock or radio-success claim from the GUI.

The user selected option1. Implementation follows native-live-visibility.md. This is a new slice;
the already selected Glycin decoder boundary must not be asked again.

## Continuation order

1. Live visibility/quick-control source is complete; native9 offline verification/export passed; installed acceptance is next.
2. Complete installed KDE/Wayland and current-package manual-sharing acceptance
   when the required peer/session is available; failures may reveal source defects.
3. Obtain canonical preview fixtures and implement the compatible receive/IPC/card
   slice using the already verified decoder. Do not call synthetic pixels peer support.
4. Resume/dedup, multi-recipient and remaining payload contracts, then their code.
5. Localization/history/accessibility/presentation polish alongside supported
   account/QR/platform research. Supported authorization only remains selected.
6. Exact release approval/scoped publisher credential, signed APT consumption and
   installed update lifecycle remain separate; do not sign or publish on "continue".

Actual checks: repository/source inspection and documentation link/whitespace
verification only. No new tests/builds or external services were needed. Owned
inspection commands exited; Terminal closed.
