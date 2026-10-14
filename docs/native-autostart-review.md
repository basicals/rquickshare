# Native login-start behavior review

2026-10-14. Mode A analysis only, following selected systemd user ownership in
native-package.md. No login-start implementation or live user manager operation.
Guide /home/morozhenko/.codex/AGENTS.md sections3/3.1 requires a choice for material
product UX and significant new lifecycle ownership; section11 covers small fixes,
not silently introducing this broader startup refactor.

## Inspected source and existing invariants

Legacy app/main/src/composables/SettingsModal.vue exposes Start on boot. Its
app/main/src/vue_lib/utils.ts applyAutoStart reads the saved flag or false, then
uses the Tauri plugin. Thus preserve explicit opt-in/default-off; no reason to
invent a new enabled-by-default policy. Native settings Values currently own
visibility/port/card_mode/directory only; no login-start boolean exists.

The new process-based daemon unit has no Install section and package installation
does not enable/start it. Explicit GUI launch activates a missing daemon. Native
GUI initially hides its card and owns a tray icon while running. GUI Quit does
not stop transfers; daemon notices can activate a missing GUI on a new eligible
event. Existing notification lock/DND rules and per-generation wake limits remain.

GUI process ownership currently uses DesktopServer's unique D-Bus name and the CLI
ensure_gui path, with a direct GUI activation Exec asset. Login-start of both GUI
and daemon must reconcile those start paths and desktop prerequisites; merely
adding a second launcher/unit would create a brittle competing-owner path. Avoid
guessed delays or starting GTK before actual compositor/tray readiness. This is a
larger lifecycle change to freeze only after the product choice.

## Options requiring selection

1. Recommended: when the user enables Start at sign-in, start the daemon and the
   native tray client, with the card hidden. Benefits: visible access/control near
   the clock, discoverable receive state/settings and a consistent native desktop
   identity immediately after login. Existing explicit Accept/Send remains required.
   Costs: maintain both owners and event-driven desktop readiness; reconcile GUI
   menu/CLI/D-Bus/login starts rather than append an independent launcher. Risks:
   duplicate GUI ownership or startup races if those boundaries are not unified.
   This is more work but the best fit for the accepted tray-first product.
2. When enabled, start only the daemon; create the tray client on explicit launch
   or a permitted incoming notice. Benefits: fewer always-running desktop resources,
   existing daemon independence and lazy GUI wake can be retained. Costs: no tray
   icon/access until GUI launch, less discoverable background state. Risks: users
   may not realize sharing is running or find its controls; missing graphical wake
   must never cause auto-consent or silent payload handling.

Both retain default-off, explicit Settings control, existing visibility/consent
and source ownership. Disabling login-start changes future sessions and does not
cancel current transfers. Quit is not silently redefined as daemon shutdown.
No package install enables anything; no updater or old-settings deletion included.
Prefer1 for discoverability, clear user control and product correctness, not speed.

## Proposed work after selection

1. Freeze selected login components, actual OS enablement as authority (avoid a
   contradictory JSON flag), unit/app/activation ownership and explicit controls.
   Record supported graphical-session readiness and rollback. Do not overwrite
   unrelated user units/autostart entries or silently import legacy startup grants.
2. Add bounded typed adapter/controller/settings presentation for actual state and
   explicit change; preserve error/unknown state rather than display false success.
   Define uncertain-write recovery/readback before code; no blind retry/replay.
3. For option1 unify all installed GUI startup paths and wait on actual readiness
   events with retained cancellation/cleanup. Keep source-only development and
   offline preview behavior separate; never detach GUI/daemon children.
4. Focused private-broker/external-manager/GTK/package contract checks once: off/on/
   unknown/denied, double activation, cancellation, failed change/readback, lock,
   epoch, GUI Quit, unchanged live transfers and fixed enablement identity. Fake
   only external manager/desktop contracts. No real user enablement or login test.
5. Update direct package/manual CI/assets/docs/export; physical KDE sign-in/logout,
   tray readiness/scale/focus and installed upgrade/removal remain acceptance.

## Handoff

Await option1/2. Native package/activation implementation and measured verification
remain in native-package.md. This document adds no production code, tests, service
enablement, host install or Google compatibility claim. All analysis is source-only.
