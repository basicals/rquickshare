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
3. [x] F3: explicit per-file Show in folder controls and owned portal OpenDirectory
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

One tracked spawn_blocking acquisition per SelectionBus, storing its handle and
request before returning to the shared event loop. Control/read/runtime processing
continues while it waits; additional acquisitions return Busy. Completion revalidates
the live client/terminal receipt before replying. It owns the receipt Arc; shutdown
joins the handle before bus cleanup and drops the pending reply. No detached task, magic timeout, unbounded request task or
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

F3 cold-start review: an installed portal may be D-Bus activatable without an
existing owner. Explicit Show in folder must use the broker's fixed
StartServiceByName(SERVICE,0) once if GetNameOwner reports NameHasNoOwner, then resolve
and validate the resulting unique owner. This is standard activation of the known
desktop portal on the user's explicit action, not application install, shell launch
or a new auto-start policy. Other lookup errors fail; no retry loop or activation
from watch/receipt events. Verification uses a private non-activating broker and
owned test service; no real portal activation occurs. Add before final adapter checks.

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


F4 first combined observations: full core570/0 passed7.44s/build10.43s, binary
modern_protocol4/0 and native_daemon4/0 passed0.00/0.02s; strict core lint3.83s.
Native built13.30s and observed27 methods pass,1 portal method fail,1 portal wait
remain pending. Interrupted only this owned test session (exit130); no own brokers
or test processes remained. Isolated failing method then reproduced Unavailable
instead of expected success (0.01s/build3.46s). Root cause: the external test portal's
zbus property defaulted to Version while the real portal contract is lowercase
version. Correct the fake's property annotation; keep production lookup unchanged.
The pending test will select both admission and early completion, so contract
failure cannot hang waiting for an event. No sleeps/weak assertions added. This
corrects the external dependency contract, not a domain-logic mock.


Portal fixture follow-up: lowercase property correction reached OpenDirectory;
then an incorrect hard-coded token prefix length in the fake panicked its method
handler, leaving both callers without replies. Correct suffix validation to32 exact
hexadecimal characters after the fixed prefix, and return a D-Bus contract error
instead of panicking inside an external mock handler. Both owned hung test sessions
were interrupted (exit130); no production timeout or assertion relaxation. Also
verify actual private FD Arc drop after cancelled/lost-owner worker shutdown.


F3 completed: per-file lazy full-manifest controls, explicit cancel, latest
controller/epoch/revision/unlock guards and joined descriptor-only portal operation.
Portal fake repairs retained strict contracts: final3 portal methods passed0.07s
(build4.40s); strict native lint4.03s and build11.28s passed. Existing isolated GTK
and identity guard passed in the initial native run. No fresh29 full-suite pass is
claimed after the portal-only repairs. Cold activation follows fixed standard
StartServiceByName; private tests prove missing-service failure/owned-service paths,
not an installed successful activation or physical file-manager focus/selection.
One new guard and three new portal methods, plus strengthened existing GTK smoke.
Terminal closed.

F4 responsiveness repair plan before code: awaiting filesystem acquisition inside
serial dispatch can block runtime.step/control/read processing while an external
filesystem is slow. Move that one already-owned worker result/reply into the existing
bus select loop, as settings already do. Keep one outstanding file request and Busy
for additional acquisitions, revalidate client/terminal receipt before reply, and
join worker/drop pending reply on close. No API/schema/source behavior change. Add
an explicit resolver seam to delay the actual filesystem call in a private-broker
scenario: second client snapshot/cancel must complete while first read waits; a
second acquisition must return Busy. Release by condition, then verify exact FD.
This is a scoped internal availability fix in the approved owner, not another
service or significant public-contract choice. Repeat only affected core/bus checks
and rebuild the native consumer after this shared implementation change.


F4 responsiveness implemented: shared bus loop polls the retained worker without
blocking runtime/control/state. Restart remains Busy while acquisition owns work.
The private harness uses the same completion path. A resolver dependency delays
only the filesystem boundary; the actual receipt resolver still runs. The new
scenario passed1/0 (0.09s/build21.69s), proving second-client snapshot/terminal Cancel
reply and Busy on concurrent acquisition before release, then exact received bytes.
Removed one unused test-local mut warning; affected service258/0 passed7.19s
(build12.16s), including that scenario and shutdown ownership. Receipt kernel3/0
passed0.00s/build5.47s, including newly activated same-size edit plus deterministic
mtime change; this table branch was not in the earlier full570 run. Strict core
lint6.08s and native consumer rebuild6.11s passed. Nightly format checks for both
crates, GTK UI/two D-Bus XML parses and git diff whitespace passed. Inventory571
core/29 native. Earlier full570+8 and native partial/repaired observations remain
separate; no newly rerun571/29 whole-suite or ten-run/coverage claim.
Terminal closed. No owned command/server/broker/worker remains running.

Current handoff: approved reveal implementation is complete; export verification
will close F4. Physical installed KDE portal activation/focus/selection and changed
receive settings/real-peer results remain acceptance, not automated proof. Root
rename/settings independence and collision ordering are checked at the authority
boundary. Same-size edit checks detectable metadata change, not cryptographic
attestation. No live user portal/clipboard/config/install or peer operation occurred.
