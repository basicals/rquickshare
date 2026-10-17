# Native22 feedback: card, transfers window and discovery

2026-10-17, Mode A. The user tested native22 and reported slow Android-to-Linux
discovery (reverse discovery is fast), interleaved device/completed/unlinked
operation rows, redundant outbound Send steps and overly separated device cards.
The requested UX supersedes earlier recipient-first and inline completed-group
presentation. Work only on dev; do not infer new source/code from screenshots.

## Analysis before changes

The card render currently inserts active transfers, device rows with nested
completed groups and unlinked Recent activity in one scrolling container. Send
opens recipients first; send_to opens the content menu if no source exists, and
prepared source/recipient/multi-send controls coexist. Direct row submission
already uses the coordinator-owned send_many path, so source ownership and
explicit receiver consent must be preserved while simplifying the entry flow.
The screenshot is product failure evidence, not proof of a specific GTK reorder
root cause. Separating completed rows removes the unwanted mixed presentation.

Daemon/SDK already publish bounded current-run transfer snapshots (not durable
history). Session endpoint associations are ephemeral metadata; equal advertised
names do not prove a stable device relationship. Persistent journal retention or
cross-restart remembered devices need an explicit architecture/product decision.

BLE receiver advertising and mDNS visibility/resend are separate from outbound
peer scanning. The initial passive90-minute journal scan returned no matching
known advertisement/refresh events. It neither proves that advertising is absent
nor locates the delay. Diagnose the on-air/GATT/mDNS stages before changing retry
intervals or adding periodic refresh workarounds. Do not alter current transfers,
visibility, network configuration or authorization during passive analysis.

## Requested product contract

- Flat device rows, no separate rounded island for each device.
- Card: devices/prepared source/current receive review; no Completed operations
  or Recent activity transfer history between devices.
- File-first send: explicitly choose content, then one Send at the intended
  discovered device starts admission. Do not introduce an extra sender-side
  confirmation; keep code display and mandatory receiver consent. Choosing a
  file or checking recipients alone must not transmit content.
- A separate Transfers window lists current and completed work in one consistent
  table/list and replaces Incoming requests/Active transfers tray entries.
  Keep explicit incoming review, cancellation and received-content actions valid.
- User-visible history lifetime and remembered-device behavior require choices
  below before implementation. Never match devices by display name.

## Pending decisions and recommended options

1. Transfers journal: recommended daemon-owned bounded persistent metadata
   (last30 days, at most1000 completed operations, explicit clear), surviving
   restarts without storing payload bytes or auto-resuming work. This gives
   honest recovery and one owner but requires schema/storage/API/retention work
   and protects potentially sensitive local names/metadata. Alternative: retain
   only existing current-run snapshots; completed rows disappear after daemon
   restart. It preserves the current storage boundary but cannot serve as a
   cross-restart journal. Wait for the user's choice before public contracts.
2. Device recents: recommended current discovered devices/current receive only
   for this slice; transfer records go to Transfers. No unverified persistent
   device association. Alternative: include remembered devices across discovery
   sessions/restarts, requiring a separately designed stable identity lifecycle;
   names or retained ephemeral endpoint IDs cannot safely implement it.

## Ordered plan after decisions

- [x] A1: inspect native22 source/SSOT/screenshots and read passive existing logs;
  record the requested corrections and missing persistence/identity decisions.
- [ ] A2: freeze the chosen journal/device contract, ownership, compatibility,
  retention/error semantics, implementation milestones and rollback before code.
- [ ] A3: implement file-first direct single-recipient Send and flat card device
  presentation, keeping admitted multi-recipient/source ownership unchanged.
- [ ] A4: implement the selected daemon/window/tray separation, preserving live
  review/cancel and content capabilities. Do not invent durable trust.
- [ ] A5: trace receiver discovery stages with bounded privacy-safe evidence;
  fix only the demonstrated cause and record what needs a live phone check.
- [ ] A6: run appropriate affected checks once (no ten-run development loop),
  record measured failures/corrections and commit logical stages independently.
- [ ] A7: separately verified package and known-owner/global-idle installation
  before the user's physical acceptance. Preserve native22 rollback package.

Rollback: independent UI entry/rows/window/journal/discovery commits, preserving
received files and user configuration. If persistent storage is selected, its
migration and downgrade policy must be documented before implementing it.
No production code, tests, package, service or visibility change in this analysis.
