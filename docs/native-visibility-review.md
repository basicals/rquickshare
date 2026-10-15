# Temporary visibility: deadline ownership review

2026-10-15. Mode A, direct dev. User selected option1 after the review. One core
visibility owner, corrected native6 package and public handoff are complete.
No new duration/UX/storage contract. Guide:
/home/morozhenko/.codex/AGENTS.md, sections3/3.1/11.

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

The user selected option1, authorizing the scoped public Rust write seam and
shared transport lifecycle refactor. Alternative2 is retained as decision history.

## Selected option1 contract

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
   and update the continuation index before code. User selected option1.
2. [x] V1: freeze exact controller commands/read views, startup,
   cancellation and expiry-generation contracts. Inventory affected branches and
   test interactions before writing regressions. Record any source-API break
   explicitly; do not preserve a second writable bypass as a compatibility shim.
3. [x] V2: implement owner-local controller and deterministic clock/deadline seam;
   wire the joined core lifetime. Verify independent instances, all three modes,
   deadline-before/at/after, renewal and superseding-command races, cancellation,
   failed initialization and restart. Real policy is the subject, not a mock.
4. [x] V3: remove expiry/write authority from mDNS; migrate mDNS/BLE policy reads,
   NativeRuntime and legacy read subscriptions/commands as one coherent source
   change. Preserve settings replay and name-only-save behavior. Check expiry with
   an absent/failed transport through a controlled external adapter, without
   using the workstation's radios.
5. [x] V4: run affected core/native/legacy checks once, plus formatting/lint as
   applicable. Record actual results, failures and corrections in this SSOT.
   Make atomic commits; do not rebuild an unchanged package just for this review.
6. [x] V5 after source verification: prepare the next uninstalled native candidate
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

## V1 implementation contract and regression inventory (before code)

Use one private watch of immutable snapshots (mode, revision, optional monotonic
deadline, running flag) per policy. Its sender is the sole synchronized mutation
point; no separate writable mode watch or process-global state. Public
VisibilityReader wraps only a receiver and exposes current/current-and-update,
changed and has-changed; callers cannot publish. Revision/deadline stay internal.
This intentionally removes RQS.visibility_sender. RQS.subscribe_visibility returns
a reader; change_visibility returns an explicit error on revision exhaustion.
Native and Tauri callers migrate together. Wire enum/schema2/Settings2 stay fixed.
Move the unchanged Visibility enum/serde/TS/raw-value mapping into the policy
module, preserving the public rqs_lib::Visibility path and serialized values.
This removes the policy module's dependency on a transport-owned type definition.

Construction and pre-start commands record mode without starting a timer/task.
begin_startup synchronously activates the policy and establishes its deadline
before transport initialization, then spawns its future in the existing tracker.
The worker captures its failure guard before first poll: abandonment/panic hides
policy and cancels the shared core lifetime. Normal cancellation clears running/
deadline and retains current mode, so a later startup of a stopped Temporary
owner gets a fresh minute. Failed startup joins the worker through existing stop.
RQS Drop requests cancellation (including discovery) to prevent a dropped owner
from orphaning its deadline/transports; explicit stop remains the awaited join
barrier. Add one actual owner-drop lifecycle regression to the V1 inventory.
Remove synchronous unregister acknowledgement waits from the async mDNS worker:
use its existing channel's recv_async. A blocking recv on a single-thread runtime
could otherwise stall the independent deadline task. No new timeout/retry policy.
An unrelated transport exit does not own or cancel the timer. Wall-clock is not
read. No claim about counting Linux suspend time is added.

Explicit commands renew same-mode Temporary; accepted native config changes use
an idempotent mode command and retain the existing apply_visibility gate. Same
mode has no event in that path. Settings exact retries are not new commands. All
updates and expiry compare/update under the same watch write lock. Expiry checks
running, Temporary, revision, deadline and current monotonic time. Revision
exhaustion rejects the command and publishes Hidden with no deadline; it cannot
wrap to a stale generation. Existing transfers are untouched.

| Artifact | Planned branches and interactions | Level / new methods | Categories |
|---|---|---|---|
| Policy admission/read view | all3 modes before start, independent owners, explicit same-mode renewal versus idempotent config, closed reader, revision exhaustion | unit / 3 | Positive,N1,N2,N4,N5,N7 |
| Deadline transition | before/at/after deadline; stale Temporary renewal/Visible/Hidden generation; inactive/no-deadline guards | deterministic unit / 2 | Positive,N2,N4,N5,N8,N10 |
| Owned worker | activation before first poll, expiry with no discovery worker, failed external task, normal cancellation, unpolled/polled abandonment, restart, duplicate start | paused-clock unit / 3 | Positive,N3,N5,N8,N9,N10 |
| RQS lifecycle | actual begin_startup/stop, partial-startup failure, current Temporary restart, duplicate run guard, joined cancellation | existing lifecycle suite + 1 | Positive,N3,N4,N5,N9 |
| Consumers and settings | latest state before refresh/rename, native name-only-save and exact replay preserve expiry, legacy persistence-before-command/read-only subscriptions | existing mDNS/BLE/runtime/private-broker/Tauri checks | Positive,N4,N5,N6,N7,N9,N10,N12 |

Tests use real policy with explicit instants for pure expiry and Tokio's paused
clock for owned worker integration (dev-only test-util feature). No real sleeps,
workstation radios, mutable domain mocks or new remote calls. Assertions must
kill missing generation/at-boundary checks, same-mode renewals on config edits,
transport-owned expiry and orphan workers. N1 enum parsing stays existing coverage;
no new strings/input format. N6/N12 reuse settings store/replay/failure coverage;
N11 is inapplicable (no billing). Complete product Mode B coverage is not claimed.
Actual method counts/results will replace these planned estimates after execution.

## V2/V3 source completion and measured development checks

Implemented the private snapshot watch, read-only VisibilityReader, synchronous
command admission and revision/deadline-checked expiry. begin_startup owns the
guarded deadline worker before transport initialization. RQS Drop requests
cancellation; stop remains the joined barrier. mDNS no longer holds a writer or
timer; both BLE advertisers and legacy/native callers use the new seams. mDNS
unregister acknowledgements now await asynchronously, so they cannot block a
Tokio worker thread and starve the policy. Native admission happens before root
capability adoption if the visibility command can fail. No writable compatibility
shim, process-global timer, new wire method, schema or UI control was added.

Ten new methods:8 policy and2 actual RQS lifecycle tests (the additional Drop
regression is justified above). Primary non-overlapping risk attribution is N2=1,
N3=1,N4=1,N5=2,N7=1,N8=1,N9=3. Positive, privacy and further boundary interactions
overlap these counts; they are not extra methods or whole-product coverage.

The first core library run passed603 methods (48.81s compile,7.56s execution).
After the owner-drop regression and fallible-admission ordering change, the
affected full library run passed604/0 in8.68s. Strict all-features/all-target core
Clippy passed27.91s. Native GTK/private-broker/Xvfb run passed39/0 in0.57s with the
same private GTK4/Layer Shell dependency root recorded in native-device-name.md.
After replacing the two blocking mDNS acknowledgements, real binary integration
tests passed: modern_protocol4/0 and native_daemon4/0 (0.00s/0.03s).

Legacy Tauri bin tests passed10/0 in0.00s, preserving persistence-before-command
and recovery behavior. Native all-target no-deps strict Clippy passed7.97s;
its no-experimental core dependency reports9 existing unused/dead-code warnings,
outside that no-deps lint scope. They were not suppressed or broadly cleaned up.
Final source moves the unchanged wire enum into the core policy module and
retains the public root re-export; affected visibility/export/settings/runtime
filter passed11/0 in0.01s (593 filtered). Final all-features/all-target core strict
Clippy passed13.18s. Nightly formatting checks passed for core/native/Tauri.
The full604 run preceded the enum relocation and async unregister refinement;
it is not claimed as a repeated final604 run. No ten-run loop or final Mode B gate.
V5 package verification remains separate from these source checks.

## V5 intermediate native5 candidate provenance (superseded)

Release candidate source is fb5c83dfd265215e079ed01baac63a173fd8a915,
package revision5. The uninstalled release build uses the existing explicit
private GTK library root and /tmp/rqs-layer-metadata/shlibs, as native4 did.
Native4 remains the last verified package until this build/verification completes.
Do not label later documentation-only commits as binary rebuilds. The source
assertion in a signing request remains distinct from build attestation and exact
release approval. No signing, publication, installation or remote request is part
of this build. V5 artifact hashes/actual verification will follow after completion.

## SDK lifetime correction before final candidate

Native5 built and passed offline verification, but final dependency inspection
found another owned-lifetime defect: mdns-sd ServiceDaemon has no Drop shutdown;
its run loop exits only on Command::Exit, not disconnected command senders.
Returning from an mDNS worker can therefore leave the SDK thread announcing
registered services. This is an evident scoped resource-lifetime fix within V3,
not a new visibility mode or unrelated transport redesign.

Before code: wrap run_inner completion with an awaited SDK shutdown admission/
acknowledgement on both success and error. Preserve the primary worker error,
report shutdown failure without private service details, and add a Drop fallback
for abandoned/unpolled owners. Never retry opaque failures or claim that local
cleanup proves remote cache withdrawal. Do not cancel active transfers on an
ordinary discovery error. SDK refusal/failure remains explicit, not a successful
physical acknowledgement. No new timeout or background cleanup task.

Add one table-driven real completion-orchestration regression: primary success/
failure crossed with external shutdown success/failure, held acknowledgement
must keep completion pending, primary error wins when both fail. The external SDK
acknowledgement is injected; the domain completion logic is real. No live socket
or discovery initialization. Recheck this affected mDNS suite and strict core
lint once; unchanged GTK/legacy flows need no repeated test run.

Keep native5 as a verified but superseded intermediate (its local receipt is
/tmp/rqs-visibility-intermediate5.json). Advance final package revision to6; never
overwrite native5 bytes. Final package will be built only after this source fix.
Native5 size4639060, SHA256
8644f7b9daf26755522b268958c0f4cd07c02d34d627d0bf477ce7d3e9eb30ea;
source fb5c83dfd265215e079ed01baac63a173fd8a915. It was neither exported as the
current candidate nor signed, installed or published.

SDK correction implemented: run_inner always flows through awaited SDK shutdown;
owner Drop requests shutdown if the run was abandoned before that admission.
The core logs an mDNS worker/shutdown failure without raw SDK/service metadata.
The timer remains independent; discovery failure does not cancel active transfers.
The5 affected mDNS methods passed in0.01s (605 total compiled,600 filtered),
including the new four-outcome gated-ack method. Final strict core all-features/
all-target Clippy passed17.40s; affected nightly formatting passed. The final
slice adds11 methods, including this extra N3-attributed lifetime method. The605
library inventory is not a claimed whole605 execution. Rebuild/verify native6
from this corrected source; preserve all measured native5 bytes as intermediate.

## Corrected native6 package and unsigned review artifacts

Built/verified from32fc64f99bfd37b382cd732eaa97ff31c863fe05:
`native_desktop/target/packages/rquickshare_0.11.5+native6_amd64.deb`,4643346 bytes,
SHA2565b4f07fc3546ac0dc493ed1d69740baae7c7b8c188d30eb4d61c30480a930692.
Core release compilation took3m49s, GTK2m02s. Exact allowed paths, ownership/modes,
assets, dependencies, extracted GUI/daemon help and private settings fixture
passed. Ubuntu26.04 amd64 only; no installed/24.04/ARM or physical-peer claim.
Existing no-experimental warnings and dpkg-shlibdeps usr-merge/diversion warnings
were emitted; unresolved dependencies were not ignored or suppressed.

Unsigned snapshot `native_desktop/target/apt-unsigned-native6` passed its package/
index checks,7 exact files, suite rquickshare-native-ubuntu26.04, created
2026-10-15T20:09:55Z, valid until2026-10-22T20:09:55Z. No InRelease/Release.gpg.
Public canonical request `native_desktop/target/signing-request-native6/request.json`
SHA256551a9499ab5ee893cdc3cdd4ed6a58009cdaf0762428f3f3f1ebe6bca2f96821,
source assertion32fc64f99bfd37b382cd732eaa97ff31c863fe05, explicit first-release
baseline assertion. Request preparation does not independently establish an empty
production baseline or prove build provenance. Exact request approval and scoped
publisher credential are still missing; no production release was signed/published.

[Machine evidence](evidence/native-visibility-2026-10-15.json) retains the measured
scope, exact hashes and superseded native5. Recovery remains complete and untouched;
the deleted Drive follow-up is not recreated. No private profile/ciphertext is
part of this feature's Git or public artifacts.

Final V5 step after committing this evidence: export exact tracked source of that
documentation commit, native6 package/checksum, unsigned snapshot and public
unapproved request to the existing workspace outputs directory. Verify ZIP CRC,
tracked inventory, selected source bytes, package/request digests and unchanged
binary inputs since32fc64f. Preserve the old generic source ZIP before refreshing
it. Record the exported source commit/ZIP hashes in the output receipt, then in a
documentation-only completion commit; do not rebuild/re-export the immutable source
ZIP just to include its own later verification record.

V5 completed: exported440 tracked files from8d4af939c6d85cba19509601262390976d86651e
to `outputs/rquickshare-native6-source.zip` and refreshed the generic source ZIP
after preserving its previous bytes. CRC/inventory/selected exact source bytes,
package/request digests and unchanged binary inputs since32fc64f passed. Source
ZIP SHA256bf1c88e896aa8990b47d7b117edb38690e1259d8916d2a25ac8901dd42bed1d7;
unsigned7-file APT ZIP SHA256
161cb49eea01da7fb452ddc6874cf665bca67c3eba4e542d1129258a539c2e07.
Exact public receipts, package/checksum, request, current docs and progress/readiness
are in `/home/morozhenko/Documents/ChatGPT/quickshare и nativepair/outputs`.
The final documentation-only completion commit is intentionally later than that
immutable source ZIP; binary provenance remains32fc64f. All owned build/export
commands finished; Terminal closed. No user services/authorization changed.

No physical discovery, suspend, installed services or release publication was
tested. The existing bluer future-incompatibility warning remains an upstream
dependency observation, not a failed strict lint or a new waiver.
