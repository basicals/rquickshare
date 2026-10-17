# Native text and clipboard acceptance review

2026-10-13. Mode A. Guide /home/morozhenko/.codex/AGENTS.md sections 3/3.1.
User selected 1/A: explicit Copy after receipt; R1-R4 are implemented in source. Implementation SSOT is
[native-text.md](native-text.md). Settings and separate tray menu source are complete.
The analysis below describes the historical review before R1-R4 implementation.
Use native-text.md for the current implemented receive/read/copy contracts.

## Current evidence and next significant product boundary

Core inbound framing already distinguishes Text/PhoneNumber/Address and URL;
completed UTF-8 data lives in transfer_metadata.text_payload. Native
sessions/details.rs deliberately returns Unsupported for text_payload/text_type,
and valid Ready currently requires at least one file. Native GTK/Manager2 therefore
cannot pretend text compatibility exists. Legacy clipboard-image sending and
open_download_destination do not establish native text/clipboard capability.

Before implementing native text, decide whether accepting text authorizes changing
the user's system clipboard. File acceptance currently authorizes receiving a
payload, not an unrelated clipboard mutation or URL launch. Clipboard overwrite
is a significant UX/security-side-effect decision requiring an explicit selection.
No approval is inferred from generic file-transfer consent or "continue".

## Options for received text / URL

| Option | Behavior | Advantages | Costs / risks |
|---|---|---|---|
| 1 / A — explicit copy (recommended) | Receive after existing sender/PIN consent, display plain text on explicit review, and change clipboard only through Copy. URL remains inert text with a separate explicit Open action if included in later scope. | Clear authority for each side effect; unrelated clipboard content survives receipt; repeated packets/reconnect cannot overwrite it. Fits daemon payload ownership and thin GUI. | Requires a separate copy click and bounded payload lifetime/recovery; native SDK/display contracts need a documented extension. |
| 2 / B — copy on acceptance/completion | Text acceptance also authorizes copying the completed text to system clipboard. UI labels this consequence before consent. | Fewer actions for deliberate clipboard-transfer usage; explicit consent wording can make the effect understandable. | Replaces existing clipboard contents as a transfer side effect; concurrent transfers/completion order and unavailable GTK clipboard provider need explicit conflict/recovery rules. Receive acknowledgement cannot falsely imply clipboard success. |

Recommend 1/A for correct ownership and predictable side effects, independent of
implementation effort. Received data must not launch a URL/application implicitly
under either option; URLs do not become shell commands. Sender receive acknowledgement
proves payload completion, never clipboard persistence or remote durability.

## Common engineering proposal after selection

Keep payload content out of Manager2 display metadata/notifications/logs. Extend
allowlisted metadata with payload kind/declared bounded byte count before enabling
text consent. Preserve exact round/PIN checks and Unsupported fail-closed handling
for Wi-Fi or mixed/unknown attachments until their own scope is accepted.

The daemon remains payload owner; retain a bounded private epoch-local text resource
through GUI restart, publish a separate authenticated explicit-read capability and
finite expiry/release status, never broadcast private text in signals. Freeze exact
budgets/schema/read capability/retention/clipboard-success contracts and scoped
invariant inventory before code. No permanent text journal, auto clipboard watcher,
account trust, URL handler installation or private payload diagnostics are implied.
Outgoing clipboard read/send is explicit; preserve prepared-source and recipient
ownership without unsolicited Send or automatic clipboard capture.

## Next plan and stop factor

1. [x] Inspect current core text storage/native Unsupported boundary and record
   the clipboard-consent choice plus best-quality recommendation before code.
2. [x] User selected 1/A; freeze implementation before text/clipboard code.
3. [x] Freeze domain/API/retention/clipboard semantics and branch inventory in a
   new implementation SSOT, then implement in logical reversible dev stages.
4. [x] Run affected focused checks once and record actual results; real Android/
   Windows peers and installed clipboard/Wayland behavior remain physical gates.

Settings/tray commits can continue independently from this recorded review.
Rollback future GUI/text-read consumers before new payload owner/API activation.
No new branch, push, installation, user clipboard read/write or peer exchange.
