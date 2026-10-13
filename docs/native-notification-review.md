# Native notification lifecycle and consent review

2026-10-13. Mode A. Analysis/review only; awaiting incoming-action UX selection.
The quality-first recommendation is option 1. No notification production code is
implemented by this document. Previously accepted daemon ownership remains fixed.

## Current implementation and accepted boundary

- NativeRuntime and SelectionBus own real transfer/consent state and bounded
  Manager2/Operations1 views. process::run creates one RQS and joins shutdown.
  Native process has no notification adapter or worker today.
- PanelController already validates current daemon epoch, inbound transfer,
  consent round, complete Ready details and pending intent before accepting.
- GTK panel shows sender, PIN and all bounded manifest entries. Hide preserves
  its connection. Quit releases client preparation but accepted transfers and
  headless inbound requests belong to the daemon.
- Legacy Tauri notification.rs uses notification-per-call blocking action waits;
  the request action does not carry the new consent round. It is not a reusable
  lifecycle adapter. Reuse native state/controls, not its stale action routing.
- Native CLI now imports capabilities and reports fixed stderr errors. Dolphin
  does not provide a terminal for those errors; visible error presentation is a
  release prerequisite, not a reason to install a shell/dialog wrapper.
- product-direction.md/native-desktop-design.md already require daemon-owned
  notifications and one lifecycle per transfer. GUI ownership is not an alternative
  proposed here: it would lose background requests on GTK Quit.

Applicable guide: /home/morozhenko/.codex/AGENTS.md, sections 3/3.1 and 13. The user
explicitly delegated the preceding ingress ownership choice, now implemented.
This next material consent UX choice is outside that concrete delegation. Present
both options with the quality recommendation, then wait before notification code.

## Incoming action alternatives

| Option | Behavior | Advantages | Costs and risks |
|---|---|---|---|
| 1 — Review and Decline (recommended) | Notification opens the exact current incoming request in the compact panel; Decline is available directly. Accept stays in the panel beside PIN and complete manifest. | Clear user consent, complete review remains available, consistent with existing panel readiness; no acceptance from a truncated preview. | One extra interaction; Review requires a working graphical session. Suppressed notifications retain the request in the panel. |
| 2 — direct Accept and Decline | Both answers act directly on the exact daemon epoch/transfer/round. Review remains available separately. | Fewer interactions, resembles the legacy quick-action flow. | Large manifests cannot be fully represented in a notification; accidental acceptance is easier. Fresh state and Ready validation prevent stale actions but do not prove the user reviewed the manifest. |

Choose option 1 for quality of consent and recovery. Implementing safe exact-round
routing is required for either option; work volume/speed do not justify option 2.
Authenticated account-based auto-accept is a separate Google parity feature and
is not granted by either choice. No display-name trust or automatic acceptance.

## Common implementation proposal after selection

This is a reviewable outline, not frozen introspection or permission for code:

1. Add a toolkit-independent bounded notification policy/worker in the daemon.
   Read an immutable coalesced projection of authoritative state, with explicit
   injected provider/control/presentation adapters. No notification IO in the
   coordinator critical path, duplicate RQS, per-request blocking wait or global
   mutable state. Own and join every task/connection on service shutdown.
2. Use org.freedesktop.Notifications through the shared bounded socket. Subscribe
   before publication; pin the actual same-UID provider owner and validate action
   signal sender. GetCapabilities determines available actions; missing provider,
   DND, expiry or dismissal never become a consent answer. Mark degradation for
   the panel; keep real request state available.
3. Bound lifecycle records to retained native transfers, not arbitrary progress
   events. Keep notification ID plus provider generation, daemon epoch, transfer
   ID and consent round. Close/retire on phase/round change, terminal state, owner
   loss or shutdown. Delayed/duplicate/conflicting signals cannot apply to a newer
   round. Recover provider loss from current state, never replay a stale action.
4. Send fixed or escaped bounded plain metadata. No raw paths, text payloads,
   addresses, keys, PINs or manifests in logs. Do not create notification IDs per
   progress tick or falsely report success while an action merely awaits the core
   receipt. The compact panel remains the durable detail/control surface.
5. Define narrow authenticated presentation/error routes before coding their XML:
   an exact transfer/round display hint for Review; a fixed allowlisted CLI failure
   code for nonterminal error presentation. Neither route accepts paths, arbitrary
   text, commands, recipients, payloads or authority to send. GUI revalidates display
   hints against current Manager2 state. No bare cross-client selection token.
6. Keep CLI errors visible when the daemon cannot start/connect as well: the adapter
   needs a local native notification path independent of the daemon, sharing the
   same fixed-code formatter/provider contract. If the notification provider is
   also unavailable, retain a nonzero exit/fixed stderr and fail release acceptance
   for that environment rather than silently claiming visible recovery.
7. Review cold GUI activation/package integration explicitly. Current source GUI
   is intentionally uninstalled and CLI starts its own executable. A daemon must
   not infer an executable from received data or open a transfer path. Installed
   activation, migration and provider-error fallback remain release gates.

Freeze concrete signatures/budgets and scoped inventory in an implementation SSOT
before production code after the selected direction is confirmed.

## Planned verification / rollback

Pure policy tables: phase/round/provider-generation/expiry/duplicate combinations,
manifest-readiness, unsupported payload, escaped metadata, fixed ingress errors.
Private provider/broker contract: real notifications/signals, sender/UID/owner loss,
capabilities absent, CloseNotification, bounded coalescing, transport/connection
failure and joined shutdown. Real coordinator integration for exact-round decline
(and accept only if option 2 is selected), stale/terminal actions and no implicit
send/accept. Minimal GTK Review projection; no per-detail E2E suite. N1/N2/N3/N4/N5/
N6/N7/N9/N10 applicable; N8 no wall-clock consent decisions, N11 no monetization;
N12 failure routing and no private source opening. Physical KDE DND/expiry/focus/
lock-screen and actual Google transfers remain external acceptance.

Rollback notification adapters/UI binding before their daemon integration. Keep
Manager2/Operations1 and file descriptor ownership unchanged. No live notification,
service installation, Tauri retirement or package publication during offline checks.

Primary reference checked 2026-10-13:
https://specifications.freedesktop.org/notification/latest-single/
Capabilities, actions and expiry are provider-dependent; retaining authoritative
consent independently of the notification is a design requirement derived from
those limitations, not proof of physical provider behavior.
