# Native26 flat transfer rows

2026-10-17, Mode A, dev only. Guide: /home/morozhenko/.codex/AGENTS.md.
User authorized continuing the native25 follow-up. Native25 is installed; Android
fast discovery and successful sending were confirmed by the user. Preserve that
receiver implementation and its package rollback.

## Analysis and approved behavior

The Transfers window renders live capabilities through cached_transfer and
metadata-only persisted records through transfers::Window. Live records still
wrap an otherwise compact heading in an Expander; completed received content
requires expanding to reach its button. The user explicitly requests flat history
rows with the result action directly in the row and no disclosure. Persisted
metadata cannot authorize content access and must remain without invented actions.

Only the Transfers page becomes flat. The card retains incoming consent review,
code/manifest and its existing disclosure. Complete inbound files get one
Show in folder action backed by the already approved whole-result capability;
text keeps Review text. Active rows retain Cancel; pending inbound Review opens
the existing card consent review. Incomplete received files retain the existing
per-file checked actions in a Saved files popover; this preserves partial receipt
access without a row expander or selecting an arbitrary whole result. Buttons
retain current synchronization, lock, revision and ownership guards. No new IPC,
filesystem authority or storage format. Keep metadata/status once in each heading.

A reported additional sender confirmation is still unidentified. Inspection shows
contextual files/folder/clipboard call Send once on preparation completion and
prepared sources call QueueRecipient directly. Text requires composition submit.
Do not remove editor submission or receiver consent based on this ambiguous
report. Seek the exact path/screenshot if source inspection cannot identify an
additional sender dialog; keep the independently authorized history work moving.

## Plan, risks and rollback

- [x] S1: inspect live/history row construction, checked result actions and sender paths.
- [x] S2: record flat-row contract, preservation rules and focused verification.
- [x] S3: flatten history rows; retain direct result/review/cancel controls.
- [x] S4: run affected real GTK scope and strict native lint once; record observations.
- [x] S5: commit source, build a distinct package and perform an idle-gated upgrade.
- [x] S6: record exact installed/export receipts and request physical acceptance.

Main risks: losing partial-file/cancel/consent actions while removing the detail
container, duplicate result launch, attaching one GTK child to multiple parents,
and stale capabilities after service changes. Reuse existing checked actions and
explicit click callbacks; do not auto-run anything from row rendering. Roll back
this independent UI commit or install the preserved exact native25 package after
an idle gate. User data, queue and received files are never removed by the UI change.

Before tests: actual widget assertions for no history Expander, inline received
text/result/cancel controls, pending Review directing to existing card, unchanged
card disclosure, and readonly metadata records. Retain existing source/clipboard
checks; no ten-run Mode B expansion and no phone/network operation in tests.

## Source and measured checks

History live rows now attach the heading directly, without an Expander or hidden
detail body. Existing text/cancel controls are reparented, preserving callbacks;
complete file results use one checked whole-result button. Pending Review opens
the existing card; partial receipts use checked per-file actions in Saved files.
No daemon/receiver change, new source admission or receiver-consent bypass.

Affected actual native panel scope passed19/0 in0.54s (32 unrelated filtered).
The widget smoke verifies no history disclosure, detached duplicate body, direct
parenting and all preserved control callbacks; existing card disclosure tests
still pass. Strict all-target native no-dependency Clippy passed1.55s. Existing
core dependency warnings and offscreen Layer Shell warnings remain unchanged.
No full Mode B or repeated-run claim. Both command sessions exited; Terminal closed.

Package revision26 distinguishes these bytes from installed25. Preserve25 exactly;
build/verify the release package, then fresh known-owner/global-idle upgrade only.
Do not restart with prepared content or nonterminal work. Sender-window identity
remains a separate required clarification; history does not depend on it.

Verified release package: 6653320 bytes, SHA-256 5f8baac1fa56479ec7014dfb2bd71eaa5b2b16537fd05d76d23d94fa6d8bf448.
Binary source4436332; package inventory/dependencies/ELF/checksums/offline checks
passed. Upgrade script pins native25 known owners2699373/2699377, exact old/new
package hashes and fresh global idle gate. It preserves login settings and starts
normal units after APT; it adds no trace override (receiver correction unchanged).

## Installed receipt and clarified request

APT exited0; native26 is installed. All14 regular files match, both units run
(PIDs2738171/2738175), one14-entry Active tray and passive Visibility2/Journal1/
AcquireResult checks passed. Login startup remains disabled; no runtime trace
override was added. See evidence/native26-installation-2026-10-17.json.

The user supplied a screenshot clarifying that the reported extra confirmation
is the incoming card's Receive -> expanded consent area, not an outbound chooser.
The new explicit request supersedes the earlier disclosure contract: no row
Expander or lower detail screen; incoming Decline/Accept appear immediately in
the same row beside peer/type/code. Keep explicit receiver consent, exact
revision/readiness validation, before-bytes code and progress-during-bytes.

## Native27 scoped continuation plan before code

- [x] C1: remove row disclosure construction/state; attach existing guarded consent
  and current control callbacks directly to the heading on all live row surfaces.
- [x] C2: keep manifest names/sizes as plain tooltip/accessibility description;
  no additional lower screen. Preserve inline result/text/cancel actions and
  partial receipt capabilities from native26. Receive focuses existing Accept
  rather than expanding content. Remove obsolete expansion state/tests.
- [x] C3: verify real widget inline consent, no Expander/Receive indirection,
  callbacks/readiness, code/progress stages and unchanged retained row ordering
  once; strict native lint once. Update synthetic incoming preview to the actual
  same row builder instead of a historical expanded fixture.
- [x] C4: commit, build distinct27, idle-gated upgrade, record measured receipts.

This is the user's specified layout change, not automatic acceptance. No new
protocol/API/security authority or BLE change. Reuse the existing bounded
manifest and exact Respond callback. Retain original content checks. Rollback26
remains immutable; do not interrupt an incoming request/active transfer to update.

Native27 implementation checkpoint: live rows construct a flat heading directly;
there is no old Expander to unwrap, expansion state or Receive disclosure button.
Existing exact consent callbacks move into the heading, preserving readiness and
revision checks. Receive focuses the direct consent action without activating it.
Full received manifest names/sizes remain plain tooltip/accessibility metadata.
The synthetic incoming card uses the same flat row builder.

First affected run:17 passed/1 failed in0.25s. The preview used a typed Widget
iterator over GTK ObserveChildren whose declared item type is Object, triggering
GIO's type assertion. Replace it with actual first_child/next_sibling traversal;
retain strict no-expander/direct-consent assertions. Also found that partial-file
buttons were previously lazy inside an unattached manifest; reconstruct them
through the existing checked file_action callback rather than searching an
unmaterialized child. This restores intended existing partial-receipt access
without new authority. No assertion weakening or protocol/receiver changes.

Final native27 affected panel18/0 in0.51s; strict all-target native no-dependency
Clippy passed1.68s. The obsolete expansion-reset test was removed because the
expansion mechanism is gone, and actual-widget no-Expander/direct-consent/plain
manifest-tooltip assertions replace the old presentation contract. Original
readiness, exact consent callbacks, progress and bounded-content checks remain.
No ten-run/full Mode B claim. Command sessions exited; Terminal closed.

Native27 release package verified:6653458 bytes, SHA-256
f023a2ca0575d4166203e972d0e70040e0e507c6c67578e75dbd7ffc5f36dd6c.
Binary/source e523523. Exact extracted-package incoming preview was visually
inspected: peer/type/code and direct Decline/Accept fit one420px card row; no
arrow/lower detail block. Preview remains inert and does not prove real peer/KDE
acceptance. Upgrade pins native26 owners2738171/2738175 and both package hashes,
requires a fresh global idle gate, and preserves received files/journal/login state.
No new runtime diagnostic override. Stop before mutation if any request is active.

## Native27 installed and continuation receipt

APT exited0; native26 upgraded to27 at2026-10-17T13:38Z after the fresh global
idle gate. Daemon/desktop run as2808363/2808365, all14 installed regular files
match the package, one Active14-entry tray and passive Visibility2/Journal1/
AcquireResult checks pass. Login start remains disabled. No temporary diagnostic
override, protocol/BLE change, file deletion, cloud access or public release.
See evidence/native27-installation-2026-10-17.json. All owned command sessions
exited with Terminal closed; no watcher/test process remains.

The screenshot resolved the ambiguous extra Send report as the incoming card
Receive/disclosure step. That indirection is removed. No unrelated outbound
editor/input contract was changed. Fresh incoming consent, progress/completion
and flat Transfers action still need physical user acceptance on this package.
The user's fast discovery/successful transfer observation remains native25 evidence.

Local handoff export after this receipt uses git archive of the final docs commit,
separate immutable25/26/27 package/source names and neighboring SHA256 sidecars.
The external export JSON records binary-source versus archive-documentation
commits, archive hashes and entry counts. Generic handoff source/current-state
copies refresh to27, preserving older exact rollback files. No signing/publication.

Local export completed: immutable native27 source archive at documentation commit
2bd733d5f5e2009e6d271e5748ba514e9ef534b6,646 entries, SHA-256
d8335182a943be71a2bbe11f81e190471b8ea2819271cb007e88a947fb4d2fbf.
Native25/26 source archives and exact rollback packages were also retained.
External per-version export manifests distinguish binary and documentation commits.
Source/evidence pushed to origin/dev without force. This docs-only follow-up does
not rebuild/reinstall or claim another test run. Physical acceptance remains open.
