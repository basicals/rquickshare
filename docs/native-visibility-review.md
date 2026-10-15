# Temporary visibility: deadline ownership review

2026-10-15. Mode A, direct dev. Analysis complete; significant ownership/source-API
selection pending. No production code, settings, package or live radio changes in
this review. Guide: /home/morozhenko/.codex/AGENTS.md, sections3/3.1/11.

## Current behavior and evidence

The existing UI offers Visible, Hidden and Visible for one minute. Preserve the
60-second duration; this review makes no Google-duration or interoperability claim.
The name slice already prevents unrelated settings saves from renewing a running
window or reopening an expired window. Do not undo those fixes.

Source inspection at f013bdd1934848db03b91057fd8ef52124eee69e:

- `core_lib/src/hdl/mdns.rs`: MDnsServer owns the repeating60-second interval,
  cached visibility and the shared visibility writer. Expiry unregisters mDNS
  before publishing Invisible. Monitor/register/unregister failures can return
  from this worker before the shared expiry publication.
- `core_lib/src/lib.rs`: the mDNS task result is not supervised as the TCP task is.
  A failed mDNS worker does not cancel the core lifetime. Both BLE advertisers
  independently observe the shared watch; neither owns the temporary deadline.
  Consequently an mDNS failure can leave Temporary observable indefinitely and
  allow Bluetooth advertisements to continue. This is a source-level failure
  path, not an observed on-air reproduction.
- The interval branch tests its cached Temporary value and later writes Invisible
  without checking the watch generation. A newer mode/renewal already published
  but not yet consumed by the mDNS task can therefore be overwritten by an old
  tick. Merely checking the current enum would still miss a newer Temporary
  renewal; expiry needs the matching admitted generation/deadline.
- Identity changes use the authoritative watch, but BLE-triggered and slot-refresh
  mDNS resends still read the cached enum. All presentation paths must use the
  authoritative policy when this ownership changes.
- `RQS.visibility_sender` is public mutable authority. NativeRuntime writes it
  directly; `RQS::change_visibility` explicitly notifies even for the same mode.
  Tauri stores a clone to subscribe/read; its production commands use the method.
  Core/native test fixtures also use the writer. Repository production writers
  and readers must be migrated together if write authority is encapsulated.
- Startup/stop already have one per-RQS cancellation token and joined TaskTracker.
  The deadline lifetime can use that ownership without another process, global
  task, settings writer or desktop-owned timer.

Expiry is discovery policy, not transfer authorization or cancellation. Preserve
explicit consent/PIN and existing active transfer behavior. Publishing Hidden
does not prove a successful physical radio withdrawal; transport errors and real
BlueZ/mDNS acceptance must remain distinguishable from policy state.

## Significant alternatives

| Option | Ownership | Advantages | Costs and risks |
|---|---|---|---|
| 1 — one per-RQS core visibility controller (recommended) | Core admits mode commands, assigns a generation and owns one temporary deadline. mDNS/BLE consume read-only state. Native and legacy shells use the same owner. | Policy survives an individual discovery worker failure; stale expiry cannot override a newer command; one implementation and lifecycle for every shell. Explicit write ownership is enforceable. | Substantial scoped refactor: replace public mutable watch authority with command/read seams, coordinate core/native/Tauri source callers and mDNS constructor. Incorrect generation/start/stop ordering could renew or leak a window; requires deterministic boundary/lifecycle regression checks. |
| 2 — native daemon owns expiry, legacy core retains its own policy | Native runtime owns a separate deadline; explicit startup selection disables mDNS expiry for that runtime. Legacy entrypoints retain their timer. Never enable both owners in one runtime. | Daemon owns native commands and expiry directly; native policy can be tested separately from discovery. | Two policy implementations and ownership modes can drift; more startup/restart adapter states. Accidental double ownership or legacy-only fixes remain risks. The library continues exposing mutable authority unless separately refactored. |

Recommend1 for correctness, clear ownership and maintenance. Visibility is a
core discovery policy shared by independently failing transports; neither a
transport worker nor an individual shell should be its authority. Option2 can be
made coherent, but duplicates an invariant that applies to all entrypoints.

This exceeds the authorized small internal-fix category: the public Rust write
seam and shared transport lifecycle change. Obtain the user's selection before
implementation. The prepared review is not selection or implementation evidence.

## Proposed option1 contract to freeze after selection

- Keep the existing enum values, 60-second mode and settings schema2/Settings2.
  No new duration selector, account mode, tray UX, persisted deadline or IPC
  method is part of this slice.
- Admit explicit commands synchronously through one owner-local controller;
  publish coherent state with generation/deadline. No direct external state
  writes and no unbounded command queue. Use read-only subscriptions for shells
  and transports. Ordinary settings saves retain the accepted-policy versus
  ephemeral-current-state distinction from the name slice.
- An explicit Temporary selection starts/renews its window; unrelated edits and
  replayed saves do not. Startup must establish one initial window before
  transport initialization can delay it. Construction without a running core
  must not spawn an orphan task. Freeze the exact pre-start/start seam in V1.
- Expire only the exact still-current temporary generation at its deadline.
  Hidden/Visible/new Temporary commands supersede an earlier deadline. All
  advertising/resend paths consult current policy rather than an old local enum.
- The owned deadline worker starts and joins with the existing core lifetime;
  mDNS failure must not stop policy expiry. No transport/register/unregister await
  precedes authoritative expiry. Failure of policy ownership itself must not
  silently leave the runtime claiming a functioning privacy deadline.
- Preserve current fresh-window behavior when a persisted Temporary mode starts
  a new runtime. No cross-process remaining-time persistence is introduced.
  Do not promise new suspend/resume guarantees without freezing and verifying
  the actual platform clock behavior. Wall-clock/time-zone edits must not renew
  a running monotonic window.
- Do not report policy acceptance as physical transport acknowledgement. Keep
  live-name saved/applied receipts and admitted-session identity intact.

## Staged plan, risks and rollback

1. [x] V0: inspect writers, consumers, failures and lifecycle; record alternatives
   and update the continuation index before code. Selection remains pending.
2. [ ] V1 after selection: freeze exact controller commands/read views, startup,
   cancellation and expiry-generation contracts. Inventory affected branches and
   test interactions before writing regressions. Record any source-API break
   explicitly; do not preserve a second writable bypass as a compatibility shim.
3. [ ] V2: implement owner-local controller and deterministic clock/deadline seam;
   wire the joined core lifetime. Verify independent instances, all three modes,
   deadline-before/at/after, renewal and superseding-command races, cancellation,
   failed initialization and restart. Real policy is the subject, not a mock.
4. [ ] V3: remove expiry/write authority from mDNS; migrate mDNS/BLE policy reads,
   NativeRuntime and legacy read subscriptions/commands as one coherent source
   change. Preserve settings replay and name-only-save behavior. Check expiry with
   an absent/failed transport through a controlled external adapter, without
   using the workstation's radios.
5. [ ] V4: run affected core/native/legacy checks once, plus formatting/lint as
   applicable. Record actual results, failures and corrections in this SSOT.
   Make atomic commits; do not rebuild an unchanged package just for this review.
6. [ ] V5 after source verification: prepare the next uninstalled native candidate
   and current handoff if binaries changed. Native4 remains the last measured
   artifact until a replacement is actually built and verified. No installation,
   signing, publication, Drive requests or physical-peer claim.

Main regression risks are same-mode renewal, settings retry versus explicit
intent, startup delay, stale deadline ordering and drop/join leaks. Deadline
tests use a deterministic clock/event seam, not real minute-long sleeps. Storage
migration, trust, account identity, new UI flows and unrelated radio retry cleanup
are outside this slice. Physical expiry under BlueZ/mDNS errors and suspend/resume
stays an explicit installed acceptance item until separately measured.

Rollback: revert coherent consumer/controller changes together; do not leave
mixed old writer and new read-view callsites. Documentation-only V0 is independently
reversible. No saved schema or user data changes are planned, so no migration
rollback is required. Preserve all original package/signing/recovery artifacts.

## Actual verification for this review

Source inspection only; no Rust tests, build or radio operations were run for V0.
Local documentation links in the three changed documents were checked for existing
targets; whitespace checked with `git diff --check`. No continuously running
terminal, watcher or service is required. All owned command invocations completed
and their sessions closed (Terminal closed).
