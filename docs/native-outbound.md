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

1. [x] O1: typed immutable PreparedSource (Files/Text), redacted native text;
   coordinator reservations and managed runtime/request retain its exact Arc until
   reader/request release. Preserve existing file input and physical-free fixtures.
2. [x] O2: typed staged text preparation wire and selection metadata in immutable
   captures, bounded receipt/replay/domain rules, checked SDK preparation/recovery
   within existing source owner; add actual broker/domain checks.
3. [x] O3: explicit native clipboard/text capture, private review and preparation,
   recipient selection and separate Send, single source gate, lock/epoch cleanup;
   isolated GTK checks, no real clipboard capture during development.
4. [x] O4: actual affected checks once, docs, atomic commits and export/handoff.

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

O3 capture clarification before code: use GdkClipboard.read_async text MIME stream
and Gio.InputStream.read_bytes_async, <=16KiB chunks and <=128KiB+1 detection,
rather than the unbounded read_text convenience API. Cancel owned GTK future on
close/lock/epoch. Editor insert handler rejects insertion that would exceed128KiB
or introduce NUL before applying it; editing existing bounded text remains possible.
Keyboard/context paste routes to the SAME bounded explicit read instead of GTK's
unbounded default paste. There is no image fallback in this text slice. Stream EOF,
not sleep, terminates capture. Close the owned stream after completion/rejection;
provider/read failures preserve the editor's existing text and fixed diagnostics.
References: https://docs.gtk.org/gdk4/method.Clipboard.read_async.html and
https://docs.gtk.org/gio/method.InputStream.read_bytes_async.html.

Text preparation action checks source occupancy again in the serialized controller
after fresh state. A stale enabled GTK button cannot replace another preparation.
An already admitted Prepare continues under the controller if the editor is closed;
it has no authority to Send. Clear sensitive editor text on close and disable undo
so GTK does not retain prior buffers. Source summary stays generic/metadata-only.

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

O1 actual first check: all-target compilation passed (16.81s). First library run
543passed/3failed: two runtime fixtures still asserted the old native payload
variant; update them to require NativeSource and retain the SAME outer-Weak drop
assertions. A 2000-operation private-bus fixture exceeded its existing 30s event
watchdog under full-suite host load (no timeout change). Rerun failed checks only.
NativeSource text must share legacy Text's existing introduction/response byte path;
retain the outer capability rather than unwrapping into a bare legacy Text value.

O1 affected results after correction: runtime28passed(0.03s), long client1passed
(16.05s), outbound34passed(0.73s); strict all-target Clippy passed(19.84s). New
NativeText boundary method and actual Files/Text request Arc lifetime method passed
in the initial suite. Existing opened-file request fixtures now exercise NativeSource
and preserve bytes/descriptor/unlink/cancel checks. Terminal closed. O1 source is
independently reversible; text ingestion/GUI are not yet exposed.

O2 first compilation identified a missing BeginText arm in the SDK's pre-mutation
handle validation; fixed explicitly. All-target check then passed(19.25s). First
service filter247passed/2failed: BeginText was implemented by the executor but
omitted from the outer operation dispatcher allowlist, yielding Incompatible.
Add the allowlist entry and rerun the two affected outgoing cases plus real native
transport integration; preserve assertions and existing finite allowlist.

O2 actual affected results: outgoing broker/controller2passed(2.36s) after allowlist
fix; strengthened two-real-managers transport method passed(0.39s), including a
native typed text request and the legacy text path. Strict all-target Clippy caught
one test bool-assert style issue; equivalent assert fixed, lint passed(20.61s).
Initial service filter247 other methods passed(19.71s run), including 3 new domain
tables for append/replay/UTF8/quotas and existing file/SDK/runtime regressions.
No repeated whole-suite claim. Terminal closed. Text summary only in state; body
owned by actual source. First all-target SDK compile omission fixed without changing
contracts. Five new O2 methods + strengthened real transport fixture, not full Mode B.

O3 menu addition before code: Send text is exposed in existing header/context menu
and standard tray DBusMenu. Preserve IDs1..15, append action ID16 and fixed row
before request navigation; 16children/17totalnodes. u16 readiness bit15, existing
16event group/mailbox bounds stay. The action is disabled when source occupied,
busy, locked/unknown or private editor already holds the preparation gate. No private
text in menu labels. Keyboard/context paste uses guarded explicit capture.

O3 first native all-target check passed(12.99s); all22 native methods passed(1.09s,
24.82s build), strict native Clippy passed(7.67s). Review found the Compact empty-row
still used the historical "Files prepared" label and keyed only on selection presence.
Update both Full/Compact source summaries from captured kind/count/bytes and include
summary in empty-row cache key so replacing one preparation cannot retain stale UI.
Add pure summary assertions and strengthen actual ID16 enabled/disabled broker event,
tray readiness and clipboard/PRIMARY preservation assertions before final checks.

O3 final affected results: panel7passed(0.45s), real/pure tray menu2passed(0.06s),
strict native all-target no-deps lint passed(3.67s), native build passed(15.41s).
New summary method plus strengthened single GTK smoke prove explicit bounded paste,
no capture/preparation on open, read-only source capture, failed insertion preserving
body, cancellation clearing body/gate, literal text and generic source summaries.
Clipboard/PRIMARY unchanged by editor open/paste, checked only on isolated display.
Actual ID16 broker enabled/disabled event and lock/unlock tray projection asserted.
No physical user's clipboard or installed tray registration. Terminal closed.

## O4 completion / actual verification and recovery

O4 explicit integration targets passed8methods: modern protocol4(0.00s), native
daemon4(0.05s); both nightly format checks, diff and outgoing XML checks passed.
Core inventory551 (baseline544 +2O1 +5O2). Observations are initial543passed/3failed
O1 then affected runtime28/long1/outbound34 passed; O2 service247passed/2failed then
affected outgoing2/native+legacy transport1 passed. Do not claim551 freshly passed
in one suite or whole-product Mode B completion. Native inventory23: all22 passed
before summary addition, then affected panel7/menu2passed. All final strict lint
and native build passed. No ten-run/physical peer/coverage percentage claim.

New methods9 total: core7 (source bounds/lifetime2, staged text/validation/quota3,
wire ownership1, recovery controller1), native2 (insert bounds/source summary).
Primary categories N1=2,N2=3,N4=1,N5=1,N7=1,N10=1; Positive,N3,N6,N9 interactions
are embedded in actual tables/transport/private-broker/GTK assertions, not extra
methods. N8/N11/N12 not applicable. Existing single GTK smoke strengthened rather
than multiplying toolkit tests. All owned terminal/fixture processes exited.
Terminal closed. No real user's clipboard/config/install/registration/push/branch.

Checkpoints: 53f9fee plan,56c0dd6 unified source,49a409d staged text/SDK,fd4b808 GUI.
Rollback consumers before wire/coordinator/runtime/source. Local/CI tests use
core_lib's locked stable command; GTK uses the isolated display command in
native_desktop/README.md. Earlier exact results remain above, including failures.
Artifacts/export should record actual committed HEAD and exclude build/private data.

Source text flow is implemented; remaining image capture, received reveal/open,
autostart/update/package migration and Google Account/QR/parity are separate stages.
Retain upstream refs and Tauri baseline until installed/physical equivalence gates.
