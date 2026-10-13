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
3. [ ] S3: freeze Settings1 XML, bounded read/edit/receipt contracts, idempotency,
   caller/FD validation and nonblocking owned writer/apply lifecycle before API code.
4. [ ] S4: integrate native startup/settings owner and saved/configured/applied/
   restart-required state. Unusable storage must remain explicit, never silent defaults.
   Freeze configuration-only recovery/controlled idle restart details before code.
5. [ ] S5: secondary GTK settings editor with explicit save/import/recovery, actual
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
