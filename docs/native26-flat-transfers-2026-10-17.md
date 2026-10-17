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
- [ ] S5: commit source, build a distinct package and perform an idle-gated upgrade.
- [ ] S6: record exact installed/export receipts and request physical acceptance.

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
