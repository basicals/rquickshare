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

## Significant product/publication choice — pending

| Option | Behavior | Advantages | Costs / risks |
|---|---|---|---|
| 1 — atomic publication per file, recommended | Stage each file under request-owned authority; publish its intended final name without overwrite only after complete validated content and successful storage completion. On cancellation/failure, discard unpublished content automatically and retain already published complete files. Expose partial outcome and retained files explicitly. | Correct file-level ownership; preserves useful completed work; ordinary destination layout and folder hierarchy remain; no false promise of atomic rename across many paths. | Requires staging/publication/cleanup ownership and partial-result projection. A group can retain three complete files out of five; UI must distinguish that from full success. Late name/storage failures must be explicit, with no automatic resend or overwrite. |
| 2 — atomic publication of one transfer directory | Stage the entire request in a private directory and expose a new containing directory only after every file succeeds; failed requests publish none. | One visible group commit and simple all-or-nothing result; completed members of a failed group are not exposed. | Every receive gains a containing folder and changes the ordinary destination layout; failure discards fully received members too. Folder publication, collision policy and crash cleanup still require ownership design. Atomic multi-path publication into existing user folders is not promised. |

Recommend1 for architectural fit and recovery quality, not implementation speed.
The payload file is the natural independently completed object; validated completed
work should survive an unrelated member's failure, with an honest partial result.
Option2 is appropriate only if the user wants a new all-or-nothing transfer-folder
product flow. Neither option changes peer consent or invents wire resume/dedup.
The choice changes filesystem publication and visible partial outcomes; wait for
the user before freezing its implementation contract or modifying production code.

## Staged plan

1. [x] R1: inspect receive creation/writes/length checks/request drop/session receipts;
   document the defect, alternatives and scope. Prepare a short current-candidate
   ordinary peer acceptance procedure independently of implementation.
2. [ ] R2: user chooses publication/partial-outcome behavior. Freeze exact directory
   and file authority, completion/flush semantics, collision/crash behavior,
   cancellation and receipt/UI contracts before code. Check target filesystem
   support before selecting mechanisms; unsupported storage fails explicitly or
   uses a documented safe strategy, never direct final-name streaming.
3. [ ] R3: implement only the selected receive ownership/projection slice with
   reversible atomic commits; preserve single consent and path/descriptor safety.
4. [ ] R4: inventory affected branches and add focused real filesystem/handler
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
