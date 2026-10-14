# Native received-file reveal ownership

2026-10-14. Mode A. User selected option1 from native-reveal-review.md: explicit
Show in folder bound to the actual completed inbound file and original directory.
Guide /home/morozhenko/.codex/AGENTS.md. Direct dev; no subagent/branch/push/install,
real user clipboard/config/portal/file-manager or physical peer operations.

## Analysis and milestones before code

Receive Root pins a directory FD before consent. InternalFileInfo stores actual
collision-resolved destination and owned write FD, discarded on exact file EOF.
SessionLease already freezes accepted final details/text; runtime/coordinator retain
bounded terminal history. StateCache captures metadata without file IO. Native SDK
currently rejects all FD replies; Files1 must permit exactly one only for acquisition.
GTK controller serializes requests and keeps private results out of PanelState.

1. [x] F1: bounded private completed file receipts, original directory authority,
   actual EOF metadata/fingerprint capture, manifest order, session/runtime/history
   ownership. Preserve declared-size success, legacy DTO/transport and text behavior.
2. [x] F2: Files1 captured availability plus typed read-only FD acquisition, bounded
   joined daemon filesystem worker, SDK/controller checked private result. Preserve
   Manager2/Text1/Operations signatures, no caller paths or persistent file leases.
3. [ ] F3: explicit per-file Show in folder controls and owned portal OpenDirectory
   adapter; current epoch/revision/unlock checks and cancellation. No OpenFile/default
   application/URL/automatic reveal. Reuse one controller and existing UI ownership.
4. [ ] F4: affected domain/kernel/private-broker/GTK/portal checks once, strict lint/
   build/format/XML, actual results/failures, atomic commits and exact export/handoff.
   Physical KDE file-manager selection remains separate acceptance.

Rollback GUI/portal, then Files1/client, then receipt propagation. Tauri remains
baseline; no existing saved-data migration or wire framing change.

## Frozen F1 resource contract

Linux private ReceivedFiles contains one original Root authority, ordered records
(actual protocol-relative collision-resolved name, size, dev/ino, mtime/ctime incl
nanoseconds). Capture fstat from each actual opened destination only on exact EOF;
require regular file and actual length equal declared size. Collect records by introduction payload ID, independent of completion order;
then sort by(relative name,size) to match the existing public details manifest. No re-resolve by current settings.
No complete received-file FD is retained in history. Root-only authority does not
pin each deleted file's disk contents. Root rename survives; moving a file outside
its recorded relative position or detectable replacement/edit makes it unavailable.
Fingerprint checks are not cryptographic content attestation or atomic selection in
an external file manager; same-UID malicious races are outside sandbox-consent claims.

At most256 records/64KiB relative-name bytes per receipt (existing detail limits).
SessionHub owns an explicit shared128-receipt budget. A reservation lives in the
actual ReceivedFiles Arc through every session/event/runtime/history/worker reference,
releasing only on actual final drop. No global counter. Coordinator additionally
retains at most128 receipt-bearing transfers; existing64 recent/256 hard history
semantics remain. Slot/capture/fstat failure disables reveal for that transfer,
without converting correctly received bytes into failure or forging availability.
Incomplete/rejected/cancelled/outbound/non-Files/unaccepted/mismatched manifests
never publish a receipt. Debug/log/state contain no absolute receipt paths or FDs.
History/epoch shutdown drops its ownership; old external state captures do not pin it.

## Frozen F2 contract

New interface io.github.morozhenko_apps.RQuickShare.Files1:
- GetMetadataPage(snapshot_id:s,offset:u)->(next:u,rows:a(stu)); exactly one row per
  captured transfer: transfer ID, nonzero terminal revision iff receipt available,
  receipt file count (zero otherwise). Captured metadata only; no filesystem IO.
- AcquireFile(transfer_id:s,terminal_revision:t,index:u)->file:h. Same authenticated
  unique same-UID client and existing frame/queue limits. Reject all input FDs and
  unknown/malformed arguments. Validate live completed inbound Files receipt/revision
  and index; no caller-supplied path/name/directory/command. Acquisition is read-only,
  repeatable, not a desktop side effect and never enters the operation receipt ledger.

Resolve with openat2 beneath retained root, NO_SYMLINKS/NO_MAGICLINKS, RDONLY|
NONBLOCK|CLOEXEC; nonblocking prevents replaced FIFO opens from hanging. Require
regular file and exact captured fingerprint on the opened FD before returning it.
Detectable moved/replaced/modified/permission failures map to finite InvalidState;
no fallback to another file/current configured folder. Existing same-UID authority
checks are not a sandbox permission grant.

One tracked spawn_blocking acquisition per serial SelectionBus, storing its handle
before await. It owns the receipt Arc; cancelled dispatch/shutdown joins that handle
before bus cleanup. No detached task, magic timeout, unbounded request task or
long-lived per-client lease. At most one opened result in the server acquisition/
reply path; successful FD reply transfers ownership, dropped/rejected reply closes.
SDK permits exactly1 FD only on this known successful method, still checking sender
and epoch. Errors/other replies must have zero FDs. Client validates read-only regular
CLOEXEC descriptor; ReceivedFile holds Arc<File> with redacted Debug and private
controller completion, never watched state. No pending/retry desktop operation:
repeated Acquire only duplicates checked read authority, never launches anything.

TransferView gains separate reveal_revision; Text1 content_revision retains its old
meaning. Files1 rows checked against complete capture, count/details/direction/phase
and revision. The controller refreshes state before acquisition. Existing command
serialization/pending-text/source/settings checks remain in force.

## Frozen F3 desktop contract

Completed inbound Files with receipt availability offer Show in folder for each
manifest entry (bounded list inside existing full details). Relative names are plain
labels and never opener authority. Compact offers existing navigation to full details.
An explicit click captures epoch/transfer/revision/index. Recheck current safe state,
unknown/locked/disconnected/closing, freshness and shared source gate before submit,
after FD completion and immediately before portal admission. No implicit replay or
launch from watch updates/reconnect/selection/receive completion. Pending reveal owns
one cancellable UI wait and one tracked portal owner; lock/epoch/hide/close cancels
wait/token and drops private FD. Already accepted external file-manager actions cannot
be retracted and selection is best effort. Nothing executes received file contents.

Use portal OpenDirectory on the held file FD; monitor the returned Request Response,
not merely method return, and close owned pending Request/connection on cancellation.
Validate portal version>=3 and expected owned sender; missing/unsupported/failed
portal shows fixed actionable error. Portal's own documented folder fallback remains;
no custom shell/URI/default-application fallback. Use activation/window identifiers
only if supplied by actual supported GTK APIs; no guessed focus token or coordinates.
No registration/install or live file-manager action during verification.

## Scoped supporting verification inventory before tests

| Artifact | Planned branches/interactions | Check / categories |
|---|---|---|
| Receipt builder/budget/resolver | actual EOF, order/collision/nested names; original root rename/settings independence; zero/exact bytes; missing/edit/replace/symlink/FIFO; 128 cap/release/old Arc; redaction | domain+kernel table methods Positive,N1,N2,N5,N6,N7,N10,N12 |
| Session/runtime/history | accepted Finished Files only; manifest/count/size mismatch, rejected/incomplete/outbound; propagation/terminal absorption/history/actual release | actual request/session/runtime tests Positive,N2,N4,N5,N9 |
| Files1/state/SDK/controller | capture ownership, stale epoch/revision/index, FD abuse, unknown method, exact output FD/read-only/CLOEXEC, repeated read no launch, dropped reply/worker join, metadata privacy | private broker+real SDK methods N1,N2,N4,N5,N7,N9,N10 |
| Portal adapter | version/missing owner, expected Response, success/cancel/error/malformed, subscribe-before-call, close Request/connection and joined cancellation, FD ownership | private fake external portal by contract N3,N5,N7,N9,N12 |
| GTK | explicit per-file action, plain names, no receive/reconnect auto launch, stale lock/epoch/source-gate checks, late completion drop, failure notice and pending control state | extend existing single isolated GTK smoke N1,N5,N7,N9,N10 |

Assert final resources/state, not only return values. No domain mocks/sleeps or
parallel GTK tests. N8/N11 not applicable (no time/billing). No exhaustive Mode B or
ten-run gate; failures trigger focused repair/recheck. Record actual observations.
Primary desktop API reference inspected in review; recheck adapter ownership details
against official portal Request/Response documentation before implementing F3.

F3 adapter detail before code: reuse the existing joined Ingress task owner for
the portal operation, retaining the shared permit and received FD through actual
completion even if the GUI wait is dropped. Use a dedicated bounded connection,
resolve portal unique owner and verify same UID, subscribe to Response and owner
changes before OpenDirectory. Random per-request handle_token uses the existing
rand0.9 dependency graph. Verify the expected request path, not only method success.
Send owned Request.Close without waiting for a Close reply on cancellation, then
close the dedicated connection. Do not Close arbitrary mismatched returned paths.
No suitable backend-specific GTK surface handle is currently exposed by this crate;
use the officially permitted empty parent_window, without guessed focus identity.
Physical portal focus/selection remains acceptance, not an automated claim.

Official Request and Window Identifiers pages inspected2026-10-14:
https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Request.html
https://flatpak.github.io/xdg-desktop-portal/docs/window-identifiers.html
Response codes0/1/2 represent success/user cancellation/other termination. A
caller-supplied handle_token permits subscription before method call; Close ends
interaction without a Response signal. An empty parent is permitted without a
suitable actual window handle. These details avoid reply/signal and focus guesses.


F1 inspection correction before further code: TransferDetails already sorts Files
by(relative name,size). Receipt indices must follow that established display order,
not introduce wire introduction order into the existing UI. Freeze this alignment
without changing display behavior. The first actual-request fixture also lacked the
real handshake sender/PIN context; supply it and assert Ready before consent rather
than weakening the required accepted/valid-manifest gate. Preliminary13 received-
filtered methods passed0.05s/build35.58s; the separate accepted request initially
failed because receipt was correctly withheld for unavailable fixture details.


F1 implemented: actual EOF records align with canonical display sorting; accepted
Finished session resource propagates through runtime to bounded coordinator/history.
Receipt reservation follows actual Arc lifetime, not history labels. After sender/PIN
fixture correction and preserving established sorted manifest, actual accepted
out-of-order/collision request passed1/0 (0.00s, build20.66s). Final received filter
passed14/0 (0.07s, build10.32s), including kernel replacement/root/budget and session
eligibility tables. Five new methods so far. Temporary unused resolver methods in
library builds remain until F2 uses them; no warning suppression. No portal invoked.
Terminal closed.


F2 implemented: Files1 captured metadata and read-only acquisition, exactly-one-FD
SDK success exception, private controller File completion and tracked joined daemon
worker. Initial compile fixes bound the reply body before borrowed Fd decode and
qualified the test broker path; no contract relaxation. All26 files-filtered methods
passed0.10s/build25.34s; strict core all-target clippy passed11.29s. Five new F2
methods cover broker/SDK, stale revision/index/FD abuse, history metadata nonownership,
actual worker join and coordinator128 cap. Controller completion assertions added
before final overall core verification; do not claim that later addition was already
run here. Library inventory570. GUI TransferView fixtures receive reveal_revision
None; actual reveal controls are F3. Terminal closed.
