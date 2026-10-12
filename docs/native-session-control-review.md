# Native session control readiness review

2026-10-12, Mode A. Status: option 1 approved by the user; implementation follows [native-session-controls.md](native-session-controls.md). Completed checkpoints: TCP supervision `ee2139a`; owned native outgoing dispatch/startup rollback `2354c1d`.

## Current source evidence

- `RQS` uses a broadcast channel of capacity 50 for both protocol events and legacy frontend commands. `InboundRequest::new` and `OutboundRequest::new` subscribe when each handler is constructed.
- `InboundRequest::handle` filters direction and ID, then executes Accept/Reject/Cancel. A broadcast receive error is logged and handling continues. The sender's successful broadcast send reports subscribers, not acceptance of a particular command by a particular live session. Outbound handling likewise cannot provide an application receipt; its send-loop polling logs Lagged and continues.
- The native coordinator's `acknowledge_control` clears the exact pending revision. Calling it merely because a broadcast send succeeded would invent a delivery acknowledgement. The managed outgoing token now solves early outgoing cancellation and separately reports local ownership release; it does not solve inbound consent processing.
- TCP inbound identities are remote socket address strings. GATT creates inbound requests using the constant `ble-weave`; its existing session mutex serializes BLE sessions, but subsequent sessions reuse that identity. A native adapter needs a fresh identity for each session incarnation, retaining that identity through BLE-to-Wi-Fi upgrade. Neither a network address nor a peer endpoint is a session capability.

These are source-level findings, not a measured Google-device failure, executed exploit, or proof that a given normal notification click was lost. Existing successful loopback consent tests do not establish loss-free delivery under saturation/reincarnation.

## Why the next change requires a decision

The user approved bounded TCP task supervision and continuing native integration; those foundations are now committed and verified. Replacing shared control delivery in the generic protocol handlers and their GATT/TCP owners is a further substantial control-plane refactor. The user's AGENTS sections 3/3.1 and cross-project maintenance preference require asking before such a change. Small corrective work, documentation and verification are already complete; no additional approval is needed for the completed fixes.

## Concrete options

| Option | Change | Benefits | Costs / risks |
|---|---|---|---|
| 1 — common managed sessions for TCP and BLE now (recommended) | Add fresh private session identities, owned bounded per-session Accept/Reject commands and processing receipts, owned inbound cancellation, and a legacy bridge. Use the same generic protocol handler contract for both transport owners; preserve session identity through BWU. | A coherent native control contract across required transports; avoids publishing an adapter that treats broadcast send as application acknowledgement; prepares truthful consent/state mapping. | Substantial refactor across handlers and GATT/TCP ownership; consent/cancel races and migration require a new scoped inventory, deterministic tests and later physical peer checks. BLE lifetime supervision still needs explicit verification rather than assumption. |
| 2 — managed TCP sessions first, BLE migration later | Implement the same durable session contract for TCP first. Keep the existing Tauri BLE route as rollback, and do not connect legacy BLE controls to the new daemon until their own migration. | Smaller separately reviewable change with real TCP adapter progress; existing legacy app retains BLE during development. | Native daemon remains incomplete for BLE until a second stage; extra mixed-mode boundaries and later integration work. It cannot be presented as feature-equivalent or ready to replace Tauri. |

Recommendation: option 1 because full Google parity and one service owner are the accepted target, and both transport paths already use the generic inbound handler. This does not add a new crypto scheme, Google account policy, visibility fallback or UX choice.

## Required invariants after approval

1. Allocate an opaque live-session identity separately from peer endpoints, socket addresses and coordinator transfer IDs; never alias an old session and never change the live identity during BWU.
2. Owned command queues are bounded; exact command/consent-round tokens reject stale/conflicting delivery. A receipt means acceptance by the current live handler, not peer approval, delivered bytes or Finished. The adapter clears only its matching coordinator control revision.
3. Owned cancellation reaches queued/blocked inbound operations without relying on a broadcast subscriber; terminal states absorb late commands. Closing a UI client does not cancel service-owned transfers.
4. Legacy Tauri construction/control remains an explicit reversible bridge. Keep framing/crypto and original-file ownership. Do not mask native command loss with fabricated coordinator events or a feature flag in tests.
5. Keep event loss explicit and fail closed; no authoritative recovery or persistent snapshot is invented by this control refactor. Live bus authentication, descriptor decoding budgets, retention policy and GTK positioning remain separate gates.

The user selected option 1. No further confirmation is needed for the common TCP/BLE session-control refactor. The detailed inventory and implementation milestones are recorded before code in [native-session-controls.md](native-session-controls.md).
