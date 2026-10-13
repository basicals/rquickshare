# Native outgoing prepared sources

2026-10-13. Mode A. User replied to the concrete outgoing review with permission
to check as needed and continue. Proceed with recommended A: shared Files/Text
source lifecycle. Work on dev; no live clipboard/config/install or physical peer
needed for deterministic checks. Guide /home/morozhenko/.codex/AGENTS.md.

## Analysis, milestones and rollback

native-outbound-review.md maps current file-only coordinator/SDK/runtime ownership.
An outer Arc capability must survive the actual managed outbound request, not just
queue dispatch. Keep legacy OpenedFiles/Files/Text DTO contracts available; add a
native-only serde/TS-skipped source variant. Same transport owner, ordered ledger,
prepared selection ID, explicit recipient Send, quotas and discard/cancel semantics.

1. [ ] O1: typed immutable PreparedSource (Files/Text), redacted native text;
   coordinator reservations and managed runtime/request retain its exact Arc until
   reader/request release. Preserve existing file input and physical-free fixtures.
2. [ ] O2: typed staged text preparation wire and selection metadata in immutable
   captures, bounded receipt/replay/domain rules, checked SDK preparation/recovery
   within existing source owner; add actual broker/domain checks.
3. [ ] O3: explicit native clipboard/text capture, private review and preparation,
   recipient selection and separate Send, single source gate, lock/epoch cleanup;
   isolated GTK checks, no real clipboard capture during development.
4. [ ] O4: actual affected checks once, docs, atomic commits and export/handoff.

Rollback GUI/SDK before typed wire/ledger, coordinator/runtime before source type.
No unrelated cleanup, trust/account/QR policy or Tauri retirement. Clipboard image
encoding/FD lifetime follows in a separate plan; this slice completes outgoing text.

## Frozen O1 source contract

PreparedSource wraps Files(Arc<OpenedSelection>) or Text(NativeText). NativeText
owns immutable UTF-8 String, 1..128KiB, no NUL. Text kind is derived using existing
core classification; URLs are inert. Redact Debug; public native constructors
validate. The selection's outer Arc is owned by coordinator/dispatch/NativeSource
payload through actual OutboundRequest drop. Nested file Arc clones cannot release
the outer reservation early. No synthetic files or deletion permission for text.
Legacy API signatures that accept OpenedSelection remain and wrap Files. Native
SendDispatch.selection becomes the typed source; record this internal Rust API
change. Files helpers retain safe names/sizes/descriptor clones; no path reopening.

Aggregate text source reservation is 16MiB across preparing + committed + actual
core-held outer source capabilities; reserve declared bytes before allocating at
BeginText. Existing selections128/uploads4 perclient/32clients/queue10/transfers256
remain. Text<=128KiB; runtime32 owners may additionally hold bounded protocol working
copies. Resource usage selected_bytes includes reserved declared text bytes during
preparation, and actual same bytes thereafter. Received resource32MiB/read4MiB budgets
are independent. No body stored in ordered receipts, captures, notices or signals.

## Frozen O2 staged wire / recovery contract before code

Add Operations1.BeginText(byte_count u, operation_seq t)->upload_id s, ordered
kind begin_text. Existing CommitSelection/AbortSelection/SendSelection/receipt ack
apply to both kinds. Files BeginSelection unchanged; no mixed upload kind. Text1
AppendText(upload_id s, sequence u, bytes ay)->next_sequence u, no FDs,1..16KiB per
append. Begin declares1..128KiB. Upload counts/resources/owner/epoch/lifecycle and
ordered origin checked identically. Sequential append advances once; exact retry
of LAST accepted sequence+length+SHA256 replays receipt without body duplication.
Different bytes/same seq, future/older sequence reject. Unknown committed/aborted
append fails. Commit requires exact declared count, valid UTF8/noNUL, then moves
body into immutable native source. No trimming/normalization or capture recopy.

Text1.GetSelectionPage(snapshot_id s,offset u)->next u, a(ssut): ID,kind,file_count,
byte_count for each caller-owned prepared selection in its immutable capture,
<=16rows. Kind files/text/url; no body/title/name. Files count1..256, text count0.
Capture fixed reservation charged. SDK verifies count/order/IDs/types/bytes and
exposes metadata only in StateView, preserving existing selections handle vector.

SDK extends existing Command/OperationKind with BeginText(count), retains pending
append fingerprint AND owned immutable local text source through uncertainty;
retry reuses same upload/seq/bytes, then ordered commit/ack. File AppendFiles recovery
remains its current GetUpload sequence reconciliation. Controller Preparation is
Files/Text, one shared upload/cursor/recovery/abort flow. Prepared source does not
send itself. Client/reconnect releases unsent caller resources; daemon queued/active
transfer retains ownership independently. No automatic recapture/send on retry.

## O3 proposed exact flow before implementation

Header/right-click offers Send text. A secondary plain editor has Paste clipboard
(explicit read only), Prepare, Cancel; empty/oversize/NUL rejected with fixed errors.
Paste captures text once with owned cancellable GTK future, current known unlock
and same owner generation required after callback. No automatic format fallback,
clipboard subscription, read on panel open or automatic Send. Existing prepared
file/text source must be discarded explicitly before another capture. Captured text
can be reviewed/edited before Prepare; no body in ordinary panel model/state.
Prepare uses the existing gate/controller, then closes private editor and presents
Nearby recipient choice. Source summary says Text/Link and bytes; Send is separate.
Unknown lock/close/epoch cancels capture/destroys editor/clears private buffer and
rejects late completion presentation; already admitted preparation is not silently
sent/cancelled. Uncertain preparation uses existing Recover/Discard controls.

## Scoped supporting verification inventory before tests

| Artifact | Invariants / branches | Level / categories |
|---|---|---|
| NativeText/PreparedSource | empty/1/max/max+1,NUL,Unicode,files parity,Debug | domain Positive,N1,N2,N10 |
| Coordinator/upload | reserve declared count,kind/owner,append sequence/hash boundaries,UTF8/count,commit/abort/disconnect,cap/late actual source release | domain Positive,N1,N2,N4,N5,N7,N9,N10 |
| Runtime/managed core | exact source Arc to actual request,files descriptor lifetime,text classification/framing,terminal/cancel/no second transport | existing+actual integration N4,N5,N9 |
| Wire/SDK/controller | pinned UID/epoch,FD/type/chunk bounds,capture immutable summary,interrupted Begin/Append/Commit/Send,shared recovery/CLI gates | private broker integration N1,N2,N3,N4,N5,N6,N7,N9,N10 |
| GTK | explicit capture only,editor/summary/recipient path,lock/epoch/cancel,source preservation,inert preview | existing single isolated GTK smoke+pure projections N1,N5,N7,N9,N10 |

N8/N11/N12 not applicable to no-time/no-billing/no-new-storage text slice. Tests
must exercise real domain/queue/read ownership, not mocked subject. No Mode B
exhaustive coverage/ten-run/Google acceptance claim. Record actual results here.
