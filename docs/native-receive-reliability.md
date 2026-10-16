# Receive reliability: incomplete-file publication review

2026-10-16. Mode A, direct dev. Guide: /home/morozhenko/.codex/AGENTS.md,
sections3/3.1/11/12/13. Active scope: essential ordinary sharing, no previews,
resume, account or advanced-payload expansion.

## Analysis: observed source behavior

- receive::Root::create_file resolves under pinned directory authority on Linux,
  rejects symlink traversal and creates the final leaf with CREATE/EXCL/NOFOLLOW.
  This prevents intentional overwrite but publishes an ordinary final filename
  before its contents are complete.
- InboundRequest::accept_transfer creates every destination before sending Accept.
  A later creation failure can leave earlier empty destinations even though the
  peer never received acceptance. A socket-write failure after creation has the
  same ownership problem.
- The FILE handler writes chunks directly to those files using write_all_at and
  validates offsets/declared length. A truncated EOF does not mark Finished; this
  existing correct validation does not remove already published partial content.
- InternalFileInfo owns Option<File> and a display path, with no incomplete-file
  publication/cleanup capability. Request/session drop closes descriptors, not
  directory entries. No inbound unlink/drop cleanup was found in the inspected
  receive/handler/session paths.
- Completed-file receipts are assembled only after all files finish; native
  sessions expose them only for Finished. They currently do not express a set of
  fully received files preserved after another file fails/cancels.

Evidence is source inspection, not a new executed failure/physical-peer test.
Relevant modules: core_lib/src/receive.rs, hdl/inbound.rs, hdl/info.rs,
received_files.rs, sessions.rs, service/runtime.rs and existing receive/inbound/
receipt tests. Existing direct terminal-chunk regressions protect length/state,
not request-drop filesystem cleanup. No new runtime check was run at this review.

This is a necessary correctness defect: a failed transfer can leave an ordinary
filename containing empty or truncated content. Do not patch it by blindly
deleting display paths; deletion needs verified owned-directory/file authority.

## Significant product/publication choice — option1 selected

| Option | Behavior | Advantages | Costs / risks |
|---|---|---|---|
| 1 — atomic publication per file, recommended | Stage each file under request-owned authority; publish its intended final name without overwrite only after complete validated content and successful storage completion. On cancellation/failure, discard unpublished content automatically and retain already published complete files. Expose partial outcome and retained files explicitly. | Correct file-level ownership; preserves useful completed work; ordinary destination layout and folder hierarchy remain; no false promise of atomic rename across many paths. | Requires staging/publication/cleanup ownership and partial-result projection. A group can retain three complete files out of five; UI must distinguish that from full success. Late name/storage failures must be explicit, with no automatic resend or overwrite. |
| 2 — atomic publication of one transfer directory | Stage the entire request in a private directory and expose a new containing directory only after every file succeeds; failed requests publish none. | One visible group commit and simple all-or-nothing result; completed members of a failed group are not exposed. | Every receive gains a containing folder and changes the ordinary destination layout; failure discards fully received members too. Folder publication, collision policy and crash cleanup still require ownership design. Atomic multi-path publication into existing user folders is not promised. |

Recommend1 for architectural fit and recovery quality, not implementation speed.
The payload file is the natural independently completed object; validated completed
work should survive an unrelated member's failure, with an honest partial result.
Option2 is appropriate only if the user wants a new all-or-nothing transfer-folder
product flow. Neither option changes peer consent or invents wire resume/dedup.
The user selected option1. The following contract implements that delegated
per-file publication and explicit partial-outcome scope.

## Staged plan

1. [x] R1: inspect receive creation/writes/length checks/request drop/session receipts;
   document the defect, alternatives and scope. Prepare a short current-candidate
   ordinary peer acceptance procedure independently of implementation.
2. [x] R2: user chooses publication/partial-outcome behavior. Freeze exact directory
   and file authority, completion/flush semantics, collision/crash behavior,
   cancellation and receipt/UI contracts before code. Check target filesystem
   support before selecting mechanisms; unsupported storage fails explicitly or
   uses a documented safe strategy, never direct final-name streaming.
3. [x] R3: implement only the selected receive ownership/projection slice with
   reversible atomic commits; preserve single consent and path/descriptor safety.
4. [x] R4: inventory affected branches and add focused real filesystem/handler
   regressions for incomplete drop/cancel, preparation failure after an earlier
   member, write/completion failure, complete success and collision/replacement.
   No subject mocks, no sleep-based waits, no full unrelated suite repetitions.
5. [ ] R5: run affected checks once, record observations, update SSOT and prepare
   a changed candidate only after final source is ready. Preserve native11.

Risks: accidental removal of user files, premature final names, incorrect partial
success, descriptor leaks, symlink/race escapes or unsupported staging semantics.
Rollback must keep file authority and handler changes together and remove any new
UI/IPC projection first. No persisted resume schema or directory history cleanup
is authorized by this review. No source test, binary, dependency, installed service,
account, signer or publication changed. All owned commands exited: Terminal closed.

Review verification:29 local documentation links resolve and staged whitespace
check passed. This documents an inspected defect and pending decision, not an
executed failure reproduction or a completed repair.

## R2 frozen implementation contract before code

Scope is native/Linux receive ownership, coordinated Files1/SDK/GTK partial receipt
projection and focused regressions. No preview/resume work, schema migration,
account, auto-retry, overwrite or group directory. Non-Linux legacy creation is
unchanged and explicitly not covered by this Linux candidate.

Linux prepares an O_TMPFILE inode on the actual destination filesystem beneath
the pinned receive root. Keep the destination parent FD/leaf as publication
authority, the unlinked file FD as write authority; never clean up by a display
path. A cancelled/dropped process closes unnamed content automatically, including
SIGKILL; created empty hierarchy directories may remain, with no partial file.
No hidden named-file fallback or startup scavenger. Unsupported filesystem or
missing safe publication capability returns a receive error before Accept; do
not silently stream into final paths. This is a documented target-filesystem
requirement, not a claim of all-filesystem support.

Prepare all file resources in local owners before sending Accept; preparation
failure drops all unlinked files. Retain the existing planned collision names;
a later occupied/symlink/directory leaf causes atomic no-overwrite link failure,
not overwrite or guessed renaming. For a valid final chunk, check the actual
regular-file size, sync_data, then link the owned inode through /proc/self/fd and
linkat(SYMLINK_FOLLOW) into the pinned parent. That specific proc link is internal
FD authority, not a peer-controlled symlink. Do not require elevated capability.
Sync the publication directory; a post-link sync error means the complete file
already exists and is retained/reported, but the group is not reported successful.
Do not unlink published content to conceal an uncertain durability outcome. No
absolute power-loss guarantee beyond kernel/filesystem sync semantics.

Reference: Linux man-pages open(2) O_TMPFILE and link(2) document unnamed-inode
lifetime, filesystem support and capability-free procfs publication:
https://man7.org/linux/man-pages/man2/open.2.html and
https://man7.org/linux/man-pages/man2/link.2.html (read2026-10-16).

The request owns staged publication objects separately from shared outbound
InternalFileInfo; do not add receive deletion capability to outbound sources.
At terminal transition/request termination, discard remaining staged files and
freeze a receipt of published complete members, including cancellation or loss.
Reserve one existing receipt-budget slot at accepted manifest preparation so
completion does not fail because a later receipt slot is unavailable. Keep256
files/64KiB manifest/128 receipts and current session limits. Staging adds at most
one parent FD per admitted file, no payload copy or unbounded historical owner.
Full success requires every file published; partial failure keeps its actual
terminal phase and original immutable requested manifest/progress.

A receipt exposes only its sorted completed file names/sizes and original rooted
identity checks. Add bounded Files1.GetCompletedPage(snapshot,transfer,offset)
returning existing (next:u, rows:a(st)) page format inside the same immutable
capture. Existing tuple signatures stay. GetMetadataPage count now means completed
receipt count for any inbound terminal file transfer; zero means no file authority.
SDK TransferView gains a redacted completed_files list, validates sorted unique
subset against requested manifest and terminal phase; Finished requires the whole
manifest and exact total bytes. AcquireFile indices refer to the completed list,
never arbitrary original failed members. Existing receipt-index/revision/FD checks
remain. This coordinated local extension is part of selected partial-outcome scope;
old native binaries are not run against this changed candidate as acceptance.

GTK retains original manifest and terminal failure/cancellation label. If at least
one file was published, display Saved N of M files and an explicit list of saved
members with Show in folder using completed-list indices. Full success keeps the
existing reveal affordance; failed/partial members never gain file capability.
No automatic opening/resend or clipboard action. Normal card/lock/stale gates apply.

## R4 focused branch inventory before tests

| Artifact | Branches/invariants | Planned methods / categories |
|---|---|---|
| Root/staging | invisible zero/nonzero file; drop; successful publication; actual size mismatch; occupied regular/symlink/directory leaf; root move; unsafe parent; prepare failure | 3 table-driven filesystem / Positive,N1,N2,N5,N7,N9,N12 |
| Handler/request | real consent; all resources prepared before Accept; short terminal chunk; complete first + truncated second; cancel/disconnect; directory-sync/link errors retain prior members; full out-of-order receipt | 2 new plus affected existing / Positive,N2,N3,N5,N7,N9,N12 |
| Receipt/session/coordinator | partial sorted subset; budget reserved/released; terminal-only exposure; no outbound/unauthorized/stale index; actual manifest identity; no false Finished | 2 new plus existing affected / Positive,N2,N4,N5,N7,N9,N10 |
| Bus/SDK/GTK | actual private broker partial snapshot/completed-page/acquire; count/subset/terminal validation; saved list matches reveal index and stale/lock guards | 1 integration + extend GTK/reveal checks / Positive,N1,N2,N6,N7,N9,N10 |

No new wall clock/locale/billing rules (N8/N11 inapplicable). N4 reuses immutable
receipts and stale-revision checks; no new retry protocol. N6 covers malformed IPC,
not a stored schema. Use actual anonymous files and real handlers/broker, explicit
fault seams only at syscall boundary if real deterministic storage failure cannot
be constructed; no mocked domain logic or sleeps. Assertions must detect premature
name publication, overwriting, false whole-group success and leaked failed-member
file authority. Run affected checks once; preserve failures and rerun only fixes.

R3 source checkpoint: anonymous creation and capability-free no-overwrite link
were actually exercised in both /tmp and the task test filesystem before coding;
both supported the mechanism. These probes used only owned temporary directories
and closed their files. This does not establish support for every user-selected
filesystem. Initial development compilation caught a cfg expression and nested
test-module path error; corrected before tests executed. Final source also rejects
duplicate completed names independently of advertised size.

R3 partial-card clarification: cancelled/disconnected/failed inbound file rows
show Saved N of M even when N=0, and explicitly direct an intentional sender
retry of remaining files. A zero-member result has no reveal list or capability.
Rejected and outbound transfers do not get this receive-only guidance. No automatic
retry or false Completed label is added. The extracted saved-list renderer is
covered in the existing GTK smoke thread, including completed-list index0 for a
member whose requested-manifest position differs.

## Development observations before final candidate

Full core run:653 methods,652 passed/1 failed (27.12s). The one failure was
an existing assertion that Accept immediately creates an ordinary empty final
file, which contradicts selected atomic publication. Updated that regression to
require absence before EOF, then exact abc bytes/Finished under the original
pinned receive root after rename or policy replacement, including non-UTF8 root.
Affected method passed1/0 (0.06s). The SDK private-broker partial receipt/acquire
method passed1/0 (0.12s) after the final duplicate-name validation refinement.
All8 new methods passed in the full run; the other644 prior methods also passed.
There is no final whole653-green or ten-repetition claim.

Initial module/cfg compilation errors were corrected before any core tests ran.
Strict core lint then found a now-unused Phase import; removed it from production
and imported it explicitly in the affected test module, with no lint suppression.
Logs: /tmp/rqs-receive-core.log, /tmp/rqs-receive-core-final.log,
/tmp/rqs-receive-root.log, /tmp/rqs-receive-sdk.log,
/tmp/rqs-receive-core-clippy.log. Owned sessions exited: Terminal closed.

R3/R4 implementation is complete. Core strict all-feature/all-target Clippy
passed32.68s. Native39/0 passed1.70s on the isolated broker/display, including
existing GTK/reveal smoke extended with Saved1of2/Saved0of2 and safe file indices.
Directory-sync failure is exercised through an actual O_PATH directory FD after
publication (fsync rejects it); the complete file must remain intact. This is a
filesystem boundary regression, not a physical power-loss simulation or whole
handler storage-fault exhaustiveness claim. Logs and final affected result are
recorded at R5. Current candidate revision will become native12; native11 and
previous exported source/checksums remain immutable. No thumbnail work is added.

## R5 source verification checkpoint

Final affected directory-sync boundary regression passed1/0 (0.03s),652 filtered;
log /tmp/rqs-receive-sync.log. After that final test extension, strict core
all-feature/all-target Clippy passed31.21s. Native strict all-target/no-deps Clippy
passed20.58s; nine pre-existing nonexperimental-core dependency warnings remain
visible. Both nightly format checks and whitespace verification passed. No ten-run
or full-product coverage claim. Logs: /tmp/rqs-receive-clippy-final.log,
/tmp/rqs-receive-native.log, /tmp/rqs-receive-native-clippy.log.
All owned sessions exited: Terminal closed.

Native12 is the changed next candidate. Build and verify its committed code with
the existing eleven-file Debian inventory/dependency/ELF/help/private-settings
smoke, then export exact public Git source/package/unsigned APT/unapproved request.
Existing helper/dependencies are preserved; no new thumbnail feature/fixture work
is part of this receive fix. Do not install, sign or publish. Preserve native11
and earlier immutable source/package checkpoints.
