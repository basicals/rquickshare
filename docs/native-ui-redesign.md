# Native GTK panel redesign: Compact, Full and Settings

_Status: UI/UX direction approved in chat on 2026-10-16. Mode A design/contract checkpoint. No production implementation or physical acceptance claimed._

## Design objectives and decision record

The user reviewed the existing native Compact/Full panel and Settings screenshots and two visual prototypes. These requirements supersede the old visual hierarchy in `native-custom-card.md` without changing transport ownership, consent, or security contracts.

- Keep the system tray/context menu as a direct access point.
- Compact is a fast-action surface **without app logo/title or Settings shortcut**. Four visible actions: **Receive**, **Send**, **Hide**, **Expand**.
- Receive: **option A approved**. Temporarily advertise this computer for 60 seconds and then **restore the visibility mode that was in force immediately before the first Receive activation**. A second Receive press within the interval renews the deadline without changing the saved baseline. Expiry affects discovery visibility only, never incoming consent or active transfers.
- Send opens the existing explicit source/recipient selection flow, never sends on its own; Hide dismisses only the presentation; Expand opens Full. These affordances must be keyboard-accessible with text/icon tooltips.
- Preserve priority presentation of pending inbound consent (Accept/Decline with PIN and safe details), active transfer (progress/Cancel), and any current error/recovery over an otherwise idle action-only Compact surface. Do not bury important safety actions behind the context menu.
- Full removes the repeated logo/app title and uses a compact top bar with **Settings**, **Hide**, and **Compact/Full toggle**. Keep **Devices**, **Incoming**, and **Transfers** sections/tabs, with clear states/badges, but do not duplicate a "Nearby" heading with a "Nearby devices" list heading.
- Full device selection is a **scrollable vertical row list**, not a grid. It must handle 0/1/many devices, long names, and constrained height. Each row has a device name/type/state plus an explicit selection/send affordance. Sending is never triggered by discovery alone.
- Once an authoritative persisted **paired/recent** source exists, sort/group known or recently used devices above other discovered devices. Do not infer trusted/paired identity from name, assume an offline device is sendable, or invent history in the UI. Current endpoint state alone supports only discovered devices; the pairing/recents feature remains a separately approved state/persistence/security slice.
- Keep Settings as a separate surface organized by General (device name, visibility, receive folder, appearance) and Advanced/service (port, login start, recovery/reconnect/restart, error information). Preserve exact daemon-owned save/reload, conflicts, indeterminate receipts, backup/recovery and disabled/busy behavior. Hide maintenance instructions and controls until relevant, but do not conceal critical recovery/error state.
- Retain system theme support, accessible contrast, focus order and stable control labels in **English**. The concept image was presentation-only with Russian annotation; production UI labels remain English.

## Current implementation vs target

Native GTK4 Layer Shell surface: `native_desktop/ui/panel.ui`, `native_desktop/src/panel.rs`, `native_desktop/src/panel/menu.rs`, `native_desktop/src/panel/rows.rs`, `native_desktop/src/panel/presentation.rs`, `native_desktop/src/panel/settings.rs`. Tray actions are shared via GAction; D-Bus/controller state and settings remain daemon-owned.

The current source has a temporary visibility enum and 60-second expiry, but `docs/native-visibility-review.md` identifies an mDNS-owned deadline that currently transitions to Hidden, possible stale tick overwrite, and failure paths with Bluetooth visibility continuing. It recommends a **core-owned visibility controller (review option 1)** versus a separate native deadline owner (option 2); **that architectural selection was not made by the user's UI option A**. Do not interpret approval of the Receive button as approval of either visibility-ownership architecture.

Consequently, **exact previous-mode restoration is a target behavior, not an implemented or validated guarantee**. The restoration must be admitted by the authoritative owner with generation/deadline semantics, avoiding shell-only timer authority. Explicit manual visibility changes cancel/replace the temporary command and must not be overwritten by its stale expiry. Persisted visibility and a transient "Receive" override must remain semantically distinct; interruption/reconnect, start/stop and transport failure require a frozen contract before production changes. If the owner-level semantics require a public API or settings schema change, stop for the required architecture/API selection.

## UI projection / safety acceptance criteria

1. Compact idle: visible Receive, Send, Hide, Expand actions; no app identity or settings shortcut; visible temporary reception status and remaining time when active, without claiming radio transport success.
2. Compact incoming: current live consent round and PIN/details/Accept/Decline where available; unsupported/unavailable metadata must never enable Accept; more details through Full. DND/locked/safety-unknown auto-presentation rules remain unchanged.
3. Compact transfer: active progress and explicit Cancel; hiding/switching mode does not cancel, discard prepared sources, or lose state.
4. Full list: 0/1/many endpoints as vertical rows; stable selection and focus, truncation/tooltip for long names, scrolling, device changes while listing, disconnected and busy states. Known/recent sections appear only when justified by real data, not placeholder matching by name.
5. Full header: Settings, Hide and mode switch in a consistent predictable position; no app-name/logo duplication. Existing right-click/tray menu remains available and actions respect current state.
6. Settings: normal configuration cleanly separated from maintenance/advanced; dirty/saved/uncertain/restart-required/recovery conditions remain explicit and truthful; no auto-restart while an active transfer/preparation is in progress.
7. Visibility: first Receive stores current baseline, announces Temporary for 60s; repeat renews without altering baseline; expiry restores baseline only if that exact temporary generation is still authoritative; manual change or newer command wins; existing transfer and consent continue.
8. No invented account-backed "paired" status, trust, remembered recipients or durable history prior to an independently reviewed data source.

## Implementation milestones / rollback

1. [x] Record reviewed UI requirements and approved Receive option A. Baseline design contract only.
2. [ ] Resolve and document the separate visibility-ownership architectural choice in `native-visibility-review.md`; freeze exact previous-mode restoration, renewal, manual override, start/stop, recovery and failed worker semantics before changing production code.
3. [ ] Implement approved UI projection with existing GTK4 actions and daemon facts first; preserve legacy working interactions, consent gating and state. Keep changes within the panel and settings surface; no protocol refactor in a UI commit.
4. [ ] Implement authoritative Receive time-window behavior only after its owner/API contract is approved, and add deterministic exact-boundary, race/renewal and transport-error regressions.
5. [ ] Validate light/dark GTK captures, accessibility, keyboard paths, 0/1/many endpoints, long names, incoming/current-round races, mode/hide/state preservation and settings save failures. Complete scoped tests and measured hardware acceptance separately.

Rollback: GTK/presentation commits are independently reversible; do not combine visual cleanup with authoritative visibility policy changes or settings schema migration. The prior `native-custom-card.md`, `native-settings.md` and `native-visibility-review.md` remain historical/reference contracts until individually superseded.

This document does not authorize implementation of pairing/trust history, ownership/API changes, or shipping/publishing a new binary.
