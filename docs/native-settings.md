# Daemon-owned native settings and receive-directory authority

2026-10-13. Mode A. User selected 1/A from native-settings-review.md. Guide:
/home/morozhenko/.codex/AGENTS.md. Direct dev commits; no install/live migration/push.
The accepted scope includes transactional native persistence, typed service/client
settings, secondary GTK editor and replacement of process-global receive policy.

## Analysis and staged plan before code

Existing native startup is legacy-read-only; GTK cannot repair saved visibility.
The daemon owns one RQS and serialized coordinator; GUI is a thin authenticated
client. CUSTOM_DOWNLOAD is process-global and looked up independently during
Introduction planning and acceptance. That can redirect a transfer or couple
separate RQS owners. Fix the authority boundary before adding mutable settings.

1. [x] S1: injected per-RQS receive policy, propagated through TCP and BLE admission;
   each request snapshots the chosen directory and pins it before file planning.
   Linux creation uses retained directory descriptors after explicit consent.
   Preserve existing file/folder framing, naming/collisions and non-Linux guards.
2. [x] S2: freeze native schema/path, transactional private store/backup/recovery
   and scoped fault inventory, then implement the injected single-writer store.
3. [x] S3: freeze Settings1 XML, bounded read/edit/receipt contracts, idempotency,
   caller/FD validation and nonblocking owned writer/apply lifecycle before API code.
4. [x] S4: integrate native startup/settings owner and saved/configured/applied/
   restart-required state. Unusable storage must remain explicit, never silent defaults.
   Freeze configuration-only recovery/controlled idle restart details before code.
5. [x] S5: secondary GTK settings editor with explicit save/import/recovery, actual
   acknowledgements and persistent Full/Compact. No false saved/applied label.
6. [ ] S6: affected checks once, actual failures/corrections, commits and handoff.
   Physical migration/KDE/Google acceptance remains separate; no ten-run dev loop.

Rollback consumers before Settings1/store activation; original Tauri settings stay
intact. S1 is independently reversible and adds no settings write endpoint/schema.
Detailed later-stage contracts/inventories are recorded here before their code;
this approval does not authorize unrelated cleanup, account trust or packaging.

## S1 frozen directory authority contract

ReceivePolicy is an explicitly injected owner-local shared value, never a static.
RQS::new/set_download_path signatures stay compatible; new owners never change
another instance. Policy is propagated to TCP SessionContext and BLE admission.
InboundRequest snapshots a directory choice at request admission; later edits only
affect later requests. A configured relative path retains its process-CWD meaning.
Default directory resolution retains existing Downloads/home fallback behavior.

Pin the snapshot before file Introduction planning, without creating directories
or files. On Linux, retain a CLOEXEC directory descriptor (O_PATH); configured
symlink aliases may resolve once when explicitly pinning the root. Subsequent
planning uses that retained descriptor; filesystem replacement/rename of the root
cannot change its authority. Display paths remain normal user-facing paths, not
/proc descriptor paths, and diagnostics contain no directory/filename values.
Directory pin failure is an explicit transfer failure, never fallback to a different
root. Text/Wi-Fi transfers do not create/open a file destination unnecessarily.

After explicit Accept, create peer subdirectories/files descriptor-relatively:
normal relative components only, no following child symlinks, O_EXCL leaf creation,
no truncation of collisions. Preserve existing umask-derived file/directory modes.
Keep the same pinned root for planning, metadata and creation. No filesystem
creation before consent. Non-Linux retains its checked canonical-root materializer
with the selected path pinned per request; no Linux descriptor claim there.

Admission bounds remain the existing TCP/BLE/session budgets; no background
watcher, timer, unbounded worker or protocol/crypto change. Retain roots until their
request lifetime ends. Settings may later supply an already acquired directory
capability through this seam; only its owning daemon/store will persist values.

## S1 scoped verification inventory before code/tests

| Artifact | Branches / interactions | Level / planned methods | Categories |
|---|---|---|---|
| ReceivePolicy/ReceiveRoot | independent owners, old/new snapshot, defaults, root unavailable, rename/replacement, normal/parent/absolute paths, symlink child, collision keeps original bytes | real temporary-directory unit 2 | Positive,N1,N2,N4,N5,N7,N9,N10,N12 |
| Inbound planning/Accept | no pre-consent creation, exact selected root, policy edit/root rename before Accept, original relative metadata and folder names, post-consent descriptor creation | existing real inbound fixture + 1 new table-driven method | Positive,N4,N5,N7,N9,N10,N12 |
| TCP/BLE propagation | actual injected policy on both entry points; shared clones keep owner identity, existing lifecycle/transport tests | affected manager/inbound/GATT suites | Positive,N5,N7,N9 |

Use actual policy/filesystem/session logic; no domain mocks or sleeps. Important
assertions kill global state, late root lookup, followed symlink, truncation and
pre-consent creation. N3/N6 use existing transport/session contracts; S1 introduces
no time/billing/new persistent schema (N8/N11; N6 migration belongs to S2). N12 is
applicable to real root/open/create failures. No whole-product coverage claim.

## Observations

Plan and S1 authority/inventory recorded before production changes. No user settings
were read/migrated, daemon/tray installed or live peer tests performed. Previous
notification/settings-review checks remain historical, not new S1 observations.

S1 implemented: no CUSTOM_DOWNLOAD static remains. TCP/BLE admissions snapshot
the owner-local policy; file Introduction retains a root descriptor through Accept.
Normal display paths and existing folder protocol remain intact. Real inbound
fixtures now use owned temporary roots instead of the user's Downloads directory.

Actual S1 checks: 26 inbound, 50 manager, 14 GATT and 2 receive methods passed;
strict core all-target Clippy and nightly formatting passed. The unused import
found during development was removed. Existing bluer future-incompatibility notice
remains. Three new methods primarily cover N5 (2) and N7 (1), with the secondary
categories above; these are focused Mode A checks, not whole-product coverage.
All fixture processes exited. Terminal closed. A final fixture lifetime correction
keeps the non-directory file alive during its negative assertion; its affected
method is rechecked below, without repeating unaffected suites.

The corrected non-directory fixture passed its one affected method (4.89s build,
0.00s execution). Commit 9202dbb completes S1. Terminal closed.

## S2 frozen persistence contract before code

The daemon's native store is `$XDG_CONFIG_HOME/io.github.morozhenko_apps.RQuickShare/`
`native-settings.json` (BaseDirs config fallback). The original legacy file remains
read-only. Inject both paths; developer `--settings PATH` stays read-only and must
not activate native writes. Reading never creates files/directories.

Schema 1 has a nonzero u64 revision, validated visibility (0/1/2), optional u16 port
(0 keeps its existing automatic meaning), compact/full card mode, optional receive
directory record (absolute OS path bytes <=4096, device/inode identity), and the
last committed request identity/fingerprint. Unknown schema/fields, malformed or
oversized documents are explicit recovery state, never defaults. Missing native
state reads the coherent legacy snapshot; both absent retains existing defaults.
Invalid legacy visibility retains its hidden/recovered warning until explicit save.

Every save carries the loaded revision and SHA-256 source token, a 32-lowercase-hex
request ID, explicit scalar values, and Retain/Default/Replace-directory intent.
Replace uses one acquired directory FD, not a caller-supplied path. Validate its
type and canonical absolute /proc identity; pin that capability on successful apply.
Retain preserves the persisted record. Restart resolution must verify the recorded
device/inode before applying it. Non-UTF8 OS paths survive persistence; display is
lossy text only. Fingerprint the raw intent plus FD identity before transforming
Retain, so exact retry after commit remains exact. Reusing the ID with changed
intent fails. Only the last commit is an idempotent retry; older/stale edits conflict.

Read/commit use a retained private directory descriptor, nofollow/nonblocking
regular file opens, actual UID ownership, no group/other bits and single link for
native files. The store directory is private 0700 and native files 0600. Legacy
source reads are bounded regular nofollow reads; legacy permissions are preserved.
Cap every source/document at 1 MiB. Recovery requires an explicit flag and a
readable bounded source; inaccessible/oversized source cannot be silently discarded.

Before first import or replacing invalid native JSON, save the exact source bytes
to a content-addressed private backup. Maximum eight distinct backup files, each
<=1 MiB; no automatic deletion/overwriting. Existing identical backup is verified.
Source token is rechecked immediately before replacement; unrelated external edits
conflict. This CAS protects cooperating writers; same-UID hostile writers are not
a new filesystem security boundary. The process LaunchGuard remains sole writer.

Write an exclusive random private temp file, write all bytes, fsync file, atomically
rename and fsync the directory. Before-rename failure preserves old native state
and runtime. After-rename directory-sync failure returns Indeterminate, never a
rollback claim. Exact retry re-reads the receipt and syncs the directory before
acknowledging. Always join admitted writes. Cleanup only this operation's owned
temp file; backups are retained. No watcher, timer or global mutable settings.

S2 inventory: real temporary-directory store tests cover import/default/native
roundtrip and backup bytes (Positive/N6/N10), stale/exact/changed retries and source
edits (N4/N5), schema/field/value/size corruption and explicit recovery (N1/N2/N6),
wrong permissions/symlinks/hardlinks/FIFO/directory capabilities (N7/N12), backup
quota/existing-content validation (N2/N6), and injected write/file-sync/rename/
directory-sync failures with old/new byte and receipt assertions (N12/N4).
External IO is an injected adapter; parsing, CAS, migration and commit logic are
real. Assertions must kill missing CAS, lost unknown backup fields, truncation,
false rollback after rename, and retry fingerprint changes. No network/time/billing
contract changes (N3/N8/N11 not applicable); abandoned requester ownership is S3.

S2 observations: the injected store is implemented without activating any process
writer yet. Seven real store methods passed (0.09s); an additional final-CAS method
passed after adding a source edit during backup (0.00s). Strict core all-target
Clippy passed, then passed its affected rerun after that addition; nightly format
passed. Eight methods primarily cover Positive (1), N4 (1), N6 (1), N7 (2), N2 (1),
N12 (1), N5 (1), with secondary categories from the inventory. Private fixtures
only; no user settings read/migration. All commands exited. Terminal closed.

## S3 frozen service/client contract before code

Settings1 shares the existing authenticated service/path/unique owner and epoch.
Read returns one bounded typed SettingsView from memory: epoch, availability,
source, configured-values validity, revision/token, visibility, port (-1 means
unset), compact/full, directory display, recovered warning, applied live revision,
running/restart-required and finite error category. No raw JSON/path setter is
exposed. Missing settings ownership in read-only developer/old fixtures returns
unavailable. A valid configured directory display is <=12288 UTF-8 bytes; it is
never authority or a diagnostic. Native record bytes stay daemon-private.

Save takes a typed intent (epoch, expected revision/token, request ID, visibility,
port, card mode, directory intent, explicit recovery) and an FD array of exactly
one for Replace, zero otherwise. Actual received FD count must match. Existing
same-UID sender authentication, object checks and 64-KiB/16-FD transport bounds
apply before decoding/admission. Validate finite enums/scalars/token/epoch and
duplicate the owned FD CLOEXEC. No arbitrary executable/config/directory path.
Read and Reload take no FDs; Reload explicitly refreshes the source/CAS view and
does not apply external edits. Save returns a typed acknowledged view; errors use
finite Settings error names, never source/OS diagnostics.

One owned async writer task admits at most eight commands, performs one blocking
store operation at a time in a joined blocking task, and sends one owner-side apply
request. Serial transfer dispatch never awaits filesystem IO or the writer's apply
ack. An admitted save survives caller disappearance. Shutdown closes admission,
joins admitted disk work, cancels apply/reply waits and joins the writer before
closing its connection. A saved-but-unapplied revision remains explicit. A write
with indeterminate durability invalidates the cached success claim until exact
retry; Reload only observes, it does not claim fsync succeeded.

Live apply changes only future receive admissions and visibility policy, then
clears recovery warning. Retain preserves the currently pinned capability; Replace
uses the acquired FD; Default returns to default policy. Port is never live-swapped.
Applied live revision is distinct from restart-required (saved port vs running
port); it proves owner policy adoption, not peer advertisement visibility. A
configuration-only owner may save but cannot claim live apply. SettingsChanged
carries only epoch/revision; clients fetch the complete typed view.

SDK keeps a settings intent and its FD across an uncertain response. An exact
retry uses the same ID/base; it cannot silently replace an intervening edit. This
ledger is separate from selected-source operations. GTK controller serializes
settings reads/save/retry alongside existing actions; cancellation of a completion
wait never cancels its admitted work. Reconnect never replays transfers or saves.

S3 inventory: private-broker integration checks for read/unavailable, actual UID/
epoch/object/type/FD rejection (N1/N2/N7), real save/apply/port status and warnings
(Positive/N6), exact retry/changed intent/source conflict (N4/N5), slow IO with
continued serial dispatch, caller disconnect and joined shutdown (N3/N5/N9/N12).
Use explicit barriers/events, real store and owner logic. Do not mock save/CAS or
sleep. Existing source/consent suites remain the regression boundary. No network
peer, locale or billing protocol changes (N8/N11 not applicable).

S3 observations: typed service/XML/SDK/controller and owned writer are implemented;
normal daemon startup is not yet switched to it. Five new private-broker methods
passed (0.05s). After strengthening actual Replace-FD and redacted projection
checks, all 233 affected service methods passed once (7.06s), strict all-target
core Clippy passed (7.76s), nightly formatting passed. The initial borrowed message
body lifetime compile error was corrected by retaining the body through FD decoding.
No weakened assertion, sleep, domain mock, user file or live broker was used.
Primary new categories: Positive 1, N7 2, N4 1, N5 1; secondary categories above.
The blocked-disk test proves actual serial reads continue and disconnected caller
work is joined on stop. All private brokers/tasks/commands exited. Terminal closed.

Retain uses the active directory identity when unchanged. After external Reload,
a different saved root is resolved in the IO worker; it cannot falsely acknowledge
the old live root as the new configuration. Failed resolution after durable save
is explicitly saved-but-unapplied, not falsely reported as persistence rollback.
Settings signals invalidate the SDK view independently of transfer revision.

## S4 frozen startup and controlled idle restart contract before code

Normal native startup takes LaunchGuard before activating any writer. Native
config path comes from BaseDirs; `--settings PATH` remains the existing read-only
developer override and exposes settings unavailable. `--check` is read-only for
the selected source and also verifies persisted custom-directory identity.

After claiming the bus name, normal startup loads the injected store in one owned,
joined blocking operation. A valid configuration and verified custom directory
capability create/start one runtime. Missing files retain existing defaults.
Unreadable/invalid/unknown configuration or unavailable custom root starts an
explicit configuration-only owner: Manager2 is Stopped, Settings1 remains usable,
no network/Bluetooth discovery/receive transports start, and no invented fallback
configuration is applied. Keep a readable source token/values for in-app repair;
unreadable storage reports its finite error and requires explicit Reload after
access is restored. Saving valid replacement does not implicitly enable transports.

Settings1.Restart(epoch) is an explicit action, independent of Save. It is available
only with a native settings owner, valid cached configured values, and no settings
write/read admitted or in flight. Prepared selections/uploads, nonterminal
transfers, unacknowledged controls/dispatches/releases, or unprovable synchronization
reject Busy. The runtime atomically checks/closes core session admission only when
the SessionHub has no live leases, eliminating a new-inbound race between idle
check and cutoff. Rejecting restart leaves admission and transfers unchanged.

After this barrier, acknowledge the request, join the old runtime/writer/bus and
rebuild in the same process with a fresh epoch/connection from persisted settings.
Keep LaunchGuard across generations. Stop signals take priority; no child process,
shell, timer, kill or auto-restart loop. A lost restart response is uncertain;
the pinned old client never replays it into a new owner. GTK offers explicit service
reconnect after generation replacement, preserving the existing no-source-replay
rule. The restart acknowledgement is intent acceptance, not proof the next runtime
started; its new complete view proves that. Epoch-local notices/history follow the
already documented daemon restart lifetime. A still invalid root returns to the
configuration-only owner with a visible finite error.

S4 inventory: private fixture startup/check paths (missing/legacy/native/corrupt/
directory identity), configuration-only repair without transport startup, idle
restart acceptance vs prepared/active/session-admission/settings-busy rejection,
fresh epoch and joined shutdown (Positive/N4/N5/N6/N7/N9/N12). Tests use retained
fake external channels and actual runtime/coordinator/session logic, not radio
hardware. Installed activation, live settings migration and physical radios remain
separate acceptance gates. Rollback: consumers/API then process activation; original
Tauri settings and read-only developer override are preserved.

S4 implementation review also identified two receive-capability integration details:
use the kernel /proc FD readlink for display instead of resolving that display path
again, and represent a non-UTF8 receive destination lossily in transfer metadata
only. Descriptor-relative planning/creation retains original bytes and authority.
This prevents misleading re-resolution and failure of a valid non-UTF8 chosen root;
it changes no peer framing or file-name sanitization. Extend the existing actual
inbound table to cover a non-UTF8 root; run that affected check once after this fix.

S4 observations: normal native startup now activates the store/writer; override
startup remains read-only. Corrupt config and failed transport initialization retain
an editable configuration-only service. A private-broker scenario actually repaired
corrupt JSON, requested restart, observed a different owner/epoch, and verified an
occupied TCP port returns to configuration-only before radio tasks start. No peer
traffic or user configuration was used. Four new methods primarily cover N6 (2),
N5 (2); they also cover Positive/N4/N7/N9/N12.

Actual checks: 8 process methods, 5 affected settings-broker methods, the new
runtime idle-guard method, new session cutoff method and strengthened inbound
non-UTF8-root table passed. Strict core all-target Clippy (7.12s) and nightly fmt
passed. The test's initially incorrect TransferType import was fixed before execution.
Existing bluer future-incompatibility notice remains. Prepared upload and blocked
writer reject Restart in actual bus tests; invalid source blocks transfer admission.
Every private broker/task/fixture and command exited. Terminal closed.

## S5 frozen secondary editor contract before code

Keep the card and its existing Full/Compact menu. Settings opens the existing
secondary window, extended with Visibility (Visible/Hidden/Visible for one minute,
matching current core policy), optional port (empty or zero = automatic), receive
folder display/Choose folder/Use default, and Appearance. Appearance still previews
immediately; mode/menu/navigation changes are temporary until explicit Save. Apply
the daemon's saved card preference on first valid view or a changed saved preference,
without resetting temporary navigation on every progress update or fabricating a
manual presentation request. No automatic persistence from widget notifications.

The editor retains its immutable loaded base token while editing. Explicit Reload
saved values discards unsaved fields; Save creates a fresh exact intent and uses
the loaded base. Conflict requires Reload/review; no silent overwrite. Invalid data
requires the visible explicit backup-and-replace checkbox. Unreadable data cannot
be reset. Missing/read-only backend disables persistence and labels appearance
temporary. Retry pending save reuses the SDK-held identity/FD. No saved success
before receipt; distinguish saved/applied/restart-required/unapplied/uncertain.

Folder choice uses the local chooser, acquires O_PATH directory authority on the
existing tracked acquisition owner, and submits the FD only after explicit Save.
No editable path/URI setter or filesystem IO in GTK callbacks. Keep the capability
through editing/retry; cancellation releases only local unsaved capability. Existing
ingress gate prevents overlapping source/directory acquisitions and joins blocking
IO at shutdown. GTK close cancels its chooser/local completion waits, while already
admitted daemon saves remain owned by the controller/writer. All secondary controls
honor the existing lock/safety/disconnected/busy state.

Start/restart is an explicit secondary action; Busy asks the user to finish/discard
prepared work. Acceptance says restart requested and offers the existing explicit
Reconnect action; only the next complete view proves successful startup. Port zero
and unset have equivalent effective startup meaning and must not cause a spurious
restart-required flag when switching representation. No source/save replay on
reconnect, installation, autostart or unrelated settings options.

S5 inventory: pure port/form boundaries and fixed status/error mappings (N1/N2/N6),
existing GTK smoke extended for temporary appearance, missing backend, explicit
recovery controls and close/lock lifecycle (Positive/N7/N9), plus actual controller
save/retry state from the private-broker contracts. Run native strict Clippy/build
and existing isolated Xvfb suite once after code. Compiled inert settings capture
is a review artifact only; no installed KDE/Wayland/device proof. No ten repetitions.

Final integration review: invalidate Settings1 readers on failed commands that
change cached error state as well as success; keep Restart reply waits cancellable
after the atomic idle cutoff. The stopped card and finite settings errors must
direct the user to Settings rather than disabled send controls or a false rejection
claim. These are scoped contract corrections; no new policy or workflow is added.

Compiled visual review caught GtkDropDown selecting Visible for an unavailable
backend despite INVALID_LIST_POSITION. Use an explicit UI-only "Select visibility"
placeholder, never a wire policy. Disable Save until a real visibility is selected;
saved wire values stay 0/1/2. Form titles should not take initial text-selection
focus. Recheck only the affected GTK/form methods and native lint/build after this
correction; unaffected core checks remain valid.

S5 observations: the secondary editor is implemented with explicit Save, Reload,
Retry, recovery confirmation, acquired directory FD, temporary appearance and
guarded service restart/reconnect. Configuration-only state directs the card to
Settings. Port unset/zero are compared by effective meaning. No GTK callback writes
configuration or opens a filesystem path; the tracked ingress owner joins acquisition.

Actual verification: the full core command passed (532 unit/export methods and
8 integration methods); strict all-target core Clippy passed (8.28s). Native's 18
methods passed in an isolated Xvfb/broker session (0.38s); subsequent checks were
limited to affected widget/form methods after UI/capture fixes. Final four affected
panel methods passed (0.74s), native strict all-target no-deps Clippy passed (4.91s),
native build passed (8.50s). Core/native nightly fmt and diff checks passed.
The initial restart-header select lifetime compile error was fixed before full
execution. Native Clippy found a test module preceding production items; it was
moved to the end without lint suppression. Existing nine core no-default-feature
warnings, bluer future-incompatibility notice and Xvfb DRI3 warnings remain distinct
from new lint failures; no physical renderer/radio claim is made.

Three new native methods primarily cover N1 (1), N6 (1), N9 (1); the existing single
GTK smoke is strengthened instead of adding competing GTK test threads. Compiled
inert settings capture is 420x620, reviewed after the unavailable-visibility fix;
no overlaps/clipping of actions, no live settings/backend state. All private fixture
tasks, brokers, display wrappers and commands exited. Terminal closed.
