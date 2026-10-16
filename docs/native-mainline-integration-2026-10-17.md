# Native21 mainline integration checkpoint

2026-10-17, Mode A. The user asked what remains and authorized moving the current
work to the mainline. The actual default branch is master; all implementation
and conflict resolution stay on dev. No new branch or worktree is needed.

## Analysis and scope

Clean local dev8e96466 has35 commits not on origin/dev, while origin/dev has three
repository-configuration commits, including the two new master commits. Incoming
changes are FUNDING.yml and the bug-report form/config. Preserve them by merging
origin/dev, never force-push or reset remote history. origin/master is an ancestor
of origin/dev. There is one checkout and master is not used by another worktree.

Latest remote Preflight38983256169 failed only its Python checks. Core tests,
both Clippy jobs and frontend checks passed at that historical remote revision;
those results do not validate the new native21 revision. Failure roots:
1. The ARM metadata/index fixture rewrites only two ELF headers, but packaging now
   contains three binaries, leaving preview-helper labeled as AMD64 in an ARM64
   package. Fix the fixture to derive binaries from the existing package inventory.
   Keep strict verifier architecture assertions; no real ARM execution is claimed.
2. Preflight invokes the runner's ambient boto3/botocore, which predates required
   checksum options. The repository already pins the supported publication-test
   runtime in scripts/requirements-r2.txt and the native package workflow uses it.
   Reuse that isolated venv in Preflight; do not weaken the adapter or use cloud
   credentials/network publication as a test.

## Ordered plan

- [ ] M1: merge origin/dev into dev, preserving repository configuration. Capture
  any actual conflicts and resolve only with source/contract evidence.
- [ ] M2: correct the binary inventory fixture and isolated Python CI runtime.
  Run the Python suite once with the existing matching pinned venv. No unaffected
  native21 rebuild or repeated Rust/GTK tests for metadata-only incoming changes.
- [ ] M3: update the remaining essential acceptance list to distinguish completed
  native21 UI from physical peer/lifecycle/release gates. Commit scoped changes.
- [ ] M4: push dev normally; inspect exact-head Preflight. Prepare a dev-to-master
  PR and attach it to this task. Merge only once the substantive exact dev-head
  checks are green; same-repository dev PR checks intentionally skip duplicates.
- [ ] M5: synchronize local master, return to dev and confirm both remote branches
  contain the verified source. Record merge/check URLs and remaining work.

No release tag, signing, APT publication, Cloudflare/Drive access, service restart
or new installation is authorized by this integration checkpoint. Existing native21
remains installed. A merge is a source checkpoint; ordinary sharing acceptance
and release publication have their own recorded gates. Preserve native20/21
immutable local packages/source archives. Rollback is a normal revert, not a
history rewrite; retain incoming repository configuration.

## Still required for essential release acceptance

1. Current native21 Android and Google Quick Share for Windows: both directions,
   multiple files, nested folder and text; verify bytes/destination/code, reject,
   cancel and independent multi-recipient results. Native13 one-file Android
   outbound success is historical evidence, not full current compatibility.
2. Failure/desktop checks: interruption/network loss/sleep, no incomplete final
   files or overwrite, received-file location, lock/DND, file-manager Send and
   opt-in login/restart/removal/settings recovery. Natural visibility expiry and
   changed monitor topologies also remain separate physical checks.
3. Exact release approval, publication provenance/scoped publisher access and the
   selected signed APT channel. Infrastructure and protected signer/backup exist;
   the production channel has not been published.

Previews, accounts/Contacts/Your devices, remembered trust/auto-accept, QR, resume/
dedup, Apple/advanced payload parity and localization remain outside this scoped
mainline integration. No full Google Quick Share parity claim.
