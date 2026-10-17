# Ordered native core events and admission readiness

Status: R1 implemented and verified offline, 2026-10-12. Mode A. Parent plan: [native-core-runtime.md](native-core-runtime.md), committed before implementation in `9a1bcfb`.

## Contract and rationale

The bounded `SessionHub::active` view contains live sessions only; per-session watch intentionally coalesces updates. Neither can reconstruct a short-lived incoming session or its consent rounds. An additive `SessionEvent` feed records Opened, Changed and Closed in owner order. The hub publishes synchronously into 512 broadcast slots. `subscribe_fresh` rejects a closed or previously used hub; subscribe before transport startup. A lagging owner must mark its coordinator permanently unsynchronized. This feed is not persistent history, authentication or authoritative recovery.

Opened carries a live control capability whose current state may already have advanced. Its immutable identity/direction are useful; ordered Changed/Closed snapshots carry historical phase/consent. An owner disappearing without a terminal phase produces Closed with the actual last state, never a fabricated Disconnected or Finished. Existing absorbing terminal rules remain. Decision acknowledgement itself does not fabricate a phase event. Native event IDs are opaque; the feed has no file metadata, payload, socket address or peer error fields. Public legacy SendInfo IDs remain caller-provided and are not newly claimed private.

ManagedSender internally reserves a queue slot before creating a lease. Full/closed admission therefore creates no phantom lifecycle. No permit is exposed or retained across an await. `ready` waits on bounded queue capacity, hub membership and closure; another concurrent producer can still win after readiness, so callers must handle Full and retain their unsent input. Hub waiting subscribes before checking membership, avoiding a lost wake. The native runtime will serialize admission and shutdown in one owner. Existing arbitrary external ManagedSender clones are not a new process-wide synchronization boundary.

## Scope / rollback / verification

Production: `core_lib/src/sessions.rs`, `core_lib/src/managed.rs`. Five regression methods were planned before code in the parent inventory and implemented in existing test modules. Revert this additive seam after removing its future runtime consumer; legacy Tauri and wire formats are unchanged.

New methods: ordered short-lived lifecycle and absorbing terminal (Positive=1); late subscription/owner loss (N6=1); bounded feed lag, full hub wake/close and queue-full phantom prevention/readiness (N2=3). N1/N3/N4/N5/N7/N9/N10 overlap existing session/managed tests and these assertions rather than adding duplicate methods. N8/N11/N12 have no new clock, billing or storage behavior. No exhaustive Mode B claim.

Actual verification: the completed full all-feature core run passed 350 unit/export and 4 integration tests, including all five new methods and the then-current runtime tests. Final all-target/all-feature Clippy and format checks also compile this seam. The full run first stopped at compilation on two new runtime return-type mismatches; no assertions ran on that attempt. The compiled correction passed; no assertion was weakened. Later runtime-specific changes and checks are recorded in the parent document. No test repetition loop or live BLE/desktop registration was used.

Coverage/mutation reasoning: tests distinguish open/change/close order, terminal absorption, missing terminal versus actual owner loss, late subscription from recovery, exact broadcast overflow, both readiness wake conditions, queue rejection without side effects and release-before-retry. Removing ordering/closure checks, suppressing a wake or opening a lease before queue admission would fail these assertions. Mutex poison/allocation failures and instrumented line/branch percentages are not claimed.
