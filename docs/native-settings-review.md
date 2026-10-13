# Native settings ownership and migration review

2026-10-13. Mode A. Direction review only; awaiting the new persistence/API decision.
Guide: /home/morozhenko/.codex/AGENTS.md, sections 3/3.1/11/12/13. Work on dev.
The preceding option A/1 selected notification ownership, not settings ownership.
No native settings write API or migration is implemented by this review.

## Current code and missing contracts

- core_lib/src/settings.rs reads bounded regular legacy JSON, validates visibility,
  port/download path, starts hidden on invalid visibility and never rewrites it.
  process::settings_path still points at the legacy AppData .settings.json.
- native_daemon.rs reads settings before claiming the owner lock and creating the
  transport. SelectionBus retains visibility_recovered as a startup warning.
  There is no native settings mutation/reload endpoint or persistence owner.
- native_desktop/src/panel/menu.rs offers Appearance Full/Compact only, retained
  for the current graphical process. It cannot save visibility or repair invalid
  saved visibility. The earlier recovery decision explicitly rejects manual JSON
  repair: native-daemon-startup.md requires equivalent in-app repair later.
- Legacy store.rs uses tauri-plugin-store with explicit save/default/bootstrap
  behavior. change_visibility persists before applying/clearing its warning.
  Native GTK must not share that file as a second independent writer.
- RQS::change_visibility updates a watch policy; port is configured at startup.
  RQS::set_download_path writes process-global CUSTOM_DOWNLOAD, and
  utils::get_download_dir reads that global for inbound receive. Settings ownership
  cannot be sound if a new RQS or a settings update can retarget another owner's
  receive root. A scoped per-runtime receive-policy refactor is required in either
  proposed option; exact active-session pinning must be frozen before its code.
- Both the legacy download-setting command and core setter currently format the
  actual directory into logs. That is a concrete local privacy defect and a small
  authorized internal fix, independent of choosing the settings architecture.

## This review's reversible maintenance plan before code

1. [x] Map actual readers/writers, native gaps and receive-policy ownership.
2. [ ] Replace only the two download-setter log messages with fixed categories;
   keep setter signatures, persistence, path semantics and runtime behavior.
3. [ ] Run focused core lint/format and inspect both setter diagnostics once;
   record actual results and commit separately. No new test mirrors log wording.
4. [ ] Record the current handoff and wait for the significant settings decision.
   Do not implement a new disk schema/API/global receive-policy refactor yet.

The small log fix rolls back independently. It does not claim that all repository
logs are redacted or implement the remaining scoped receive-policy refactor.

## Ownership alternatives

| Option | Description | Advantages | Costs / risks |
|---|---|---|---|
| 1 / A — daemon-owned settings (recommended) | The existing daemon is the sole writer of one versioned native settings store, exposing a bounded typed Settings1 contract. GTK submits explicit edits and reads saved/configured/applied status; it never directly patches service config. Toolkit-independent desktop preferences can be stored as a separate domain in that same store. | Configuration and effective service policy have a single accountable owner; conflict/recovery and persistence acknowledgement stay coherent with runtime state. Fits the accepted thin-client architecture without another process/protocol. | A missing daemon prevents durable preference changes; GTK must label its temporary mode as unsaved and offer explicit reconnect/retry. Requires atomic migration, revision checks and scoped runtime receive-policy ownership work. |
| 2 / B — separate per-user settings service | A dedicated settings process owns the versioned store and typed configuration API for GTK and the transport daemon. The daemon separately reports which saved revision it has applied. | Preferences remain editable while the transfer daemon is stopped; configuration lifecycle is independent and can serve additional genuinely independent clients. | Adds another process/IPC/supervision boundary and two revision lifecycles; must reconcile persisted, observed and applied settings without false success or races. Requires the same migration/receive-policy work plus service startup and availability handling. |

Recommend 1/A because this application's service already owns the effective
transfer/visibility policy. Keeping explicit saved/configured/applied state at
that owner makes acknowledgement and recovery coherent. A distinct settings service
is justified by independently available configuration clients/lifecycle; no such
requirement exists here. Work volume and speed do not decide the recommendation.
Direct GTK/CLI writes to the legacy file are not proposed as a durable alternative.

## Common proposed acceptance contract

- Native store has a single writer and a versioned schema with bounded typed
  values. Exact path/schema/XML/budgets/DI and test inventory are frozen after
  direction approval, before implementation. Existing valid meanings and fresh-
  install visibility defaults are preserved; no account/trust policy is invented.
- Reading legacy settings on startup remains read-only. Migration uses explicit
  in-app save/import with a bounded coherent source snapshot, private backup and
  a new native store. Preserve the original Tauri file and unknown fields in the
  source backup; do not silently overwrite, truncate, reset or remove it. Detect
  source/revision changes instead of importing a stale snapshot without warning.
- Back up before committing migration, validate before replacing native state,
  use private descriptor-relative temporary writes, atomic replacement and durable
  synchronization. Failure before commit preserves runtime/warning. Ambiguous
  post-rename durability failure must be reconciled, not reported as proven rollback.
  Corrupted/unreadable/unknown-version storage does not become default settings.
- Bound authenticated same-UID requests and writer queue; use expected revision
  and idempotent intent identities so reconnect/retry cannot overwrite another
  edit or fabricate a successful save. No caller-selected executable/config path,
  shell, raw diagnostics or unsupported preference toggles in IPC.
- Distinguish saved values from current runtime policy and restart-required state.
  Visibility recovery clears only after explicit successful persistence and
  owner-side policy update, including explicit Hidden. Do not claim that enqueueing
  a radio policy proves a peer already sees the advertisement.
- The receive-directory choice must remain explicit, locally acquired and validated.
  Replace CUSTOM_DOWNLOAD with injected owner-local policy; pin each admitted
  incoming transfer's directory authority so a later edit cannot redirect it.
  No pre-consent directory creation or broad path-opening control API is implied.
- Port/configuration requiring transport restart is reported honestly. No implicit
  stop/restart of active transfers. A controlled restart policy requires an idle
  owner boundary and explicit action; final detailed lifecycle is frozen first.
- Full/Compact remains one UI. Settings stay secondary. Missing backend can keep
  an in-process display choice, but cannot present it as durably saved. No silent
  source/transfer replay on reconnect. Legacy-only keys are retained in the backup;
  migration does not pretend autostart/logging/close behavior is implemented natively.
- No user file migration, installation, live bus configuration, autostart enablement
  or destructive repair occurs during development. Fixtures use private directories
  and brokers. Installed migration/rollback and physical KDE/Google acceptance stay
  separate gates. This choice does not retire Tauri by itself.

## Implementation sequence after approval / rollback

1. Freeze owned schema/path, settings read/edit/receipt API, lifecycle/revision
   semantics, backup/commit/recovery invariants and scoped test inventory.
2. Implement injected transactional store with real temporary-directory checks:
   corrupt/oversized/unknown schema, source changes, concurrent/stale/replayed edits,
   permission/disk/rename/sync failures and exact old/new state acknowledgement.
3. Implement typed authenticated service/SDK operations and scoped owner-local
   receive policy. Real private-broker checks preserve consent/source/active-session
   authority and cannot block the serial transport owner on filesystem IO.
4. Build the secondary GTK editor with explicit save/import/recovery and actual
   saved/applied/restart-required feedback, plus persistent Full/Compact preference.
5. Run affected checks once, document failures/corrections, atomic commits/handoff.
   Roll back GUI consumers first, then new API/store activation; original legacy
   settings remain intact. No Mode B ten-run or whole-product parity claim.

## Observations

Analysis/review and the narrow log-fix plan are recorded before production edits.
No user settings file has been read or changed for this review. Settings ownership
and the substantial receive-policy refactor remain awaiting user selection.
