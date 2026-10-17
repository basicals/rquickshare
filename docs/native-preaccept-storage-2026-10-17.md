# Linux inbound preparation before consent: storage failure contract

2026-10-17. MODE B — Testing. Source-only development on dev, no user machine,
Pixel, service, package/installation, master merge, signing, automatic
accept/retry or protocol extension. User's installed native29 unchanged.

## Repository scan, risk and invariants

SSOT: `docs/native-essential-priorities.md`,
`docs/native-receive-reliability.md`,
`docs/native-inbound-storage-faults-2026-10-17.md`, and the uploaded
universal agent prompt (MODE B). Project guide outside GitHub checkout is
not available to connected GitHub. Framework: Rust async Tokio, Linux
rustix openat O_TMPFILE, actual inbound request with DuplexStream and
SessionControl, existing core tests/Clippy/format, GTK and Python
contracts under GitHub Actions.

`core_lib/src/hdl/inbound.rs::process_introduction` pins the selected
receive root and announces immutable consent/manifest. Once the user
authorizes, `accept_transfer` takes the current HashMap IDs, calls
`Root::stage_file` for **every** member into a local Vec, and only
then transfers the opened file/publication owners into the request. It
sends encrypted Accept **after** all staging succeeds and moves to
ReceivingFiles **only after** a successful send. A failure before this
barrier must leave no on-disk final names or active staged owners; the
manager later maps failure to Disconnected and freezes zero receipts.
`Directory::pin` and `Root::destination_parent` operate on pinned
directory FDs, refusing unsafe path ancestors. The partial-receipt
feature applies only to completed files, not prepared ones.

Existing tests cover one mutated parent path, accepting-socket failure,
staged-file drop, symlink ancestor and collision separately. Missing
combined evidence: invalid selected root **before introduction**;
removed pinned directory **after introduction**; second-member
failure after a first successful anonymous stage, with two unsafe
parent variants; and actual O_TMPFILE refusal on a filesystem without
this capability. These are evidence gaps, not preclaimed production
defects.

Critical invariants: no Accept frame, ReceivingFiles, final file name,
successful receipt, progress or unexpected overwrite if preparation
fails. An already staged first member must be dropped atomically when
a later member fails. No stale resource owner, no escape through symlink,
no user-owned sentinel changes. No fake success by subsequent terminal
notification. Error remains generic and does not include untrusted
paths. A root that becomes unavailable after being pinned is allowed
to fail safely; selecting an unsupported filesystem must fail before
remote consent.

## Scoped exhaustive artifact/branch inventory

| Production artifact / branch | Risk | Evidence and planned method |
|---|---|---|
| `Directory::pin`: existing/missing directory at introduction | critical | existing valid; new missing root before Introduction (1) |
| `Root::destination_parent`: existing normal/symlink parent | critical | new later-index regular-file / symlink replacements (2) |
| `Root::stage_file`: O_TMPFILE supported/unsupported/unlinked root | critical | existing positive; new removed root and procfs refusal (2) |
| `InboundRequest::accept_transfer`: first or later staging failure; no Accept; no partial owner | critical | new removed root (first) and later member (2), plus procfs (1) |
| `InboundRequest::record_termination` and `finish_received`: discard all pending resources, zero receipts | high | new four accepted-consent failure variants |
| `SessionControl` and immutable manifest: WaitingForUserConsent, then Disconnected with exact zero progress | high | new four variants |
| `ReceivedFiles`: no member can be acquired if no files were published | high | new four variants |
| `OutboundRequest`, existing success and TCP errors, GTK/IPC | medium | unchanged existing suites |

Tests invoke the **real** domain handler and Linux syscalls, without
mocking the handler/receiver or changing system permissions. The
unsupported-fs probe uses a procfs directory only to try O_TMPFILE,
which the Linux kernel rejects; it must never create a named file.
No files owned by users or other processes are opened for writing.

## Branch/interaction map and test count

Five deterministic required cases, across two methods:

A. Missing configured root **before** Introduction: pin rejects, no
manifest/files, no acceptance/receipt.
B. Root unlinked **after** Introduction: root FD remains pinned but
O_TMPFILE creation fails; no Accept, no staged owner.
C. Later member parent is a regular file: earlier member stages, later
fails ENOTDIR; original sentinel untouched; no Accept.
D. Later member parent is a symlink to an outside directory: earlier
member stages, later fails under NOFOLLOW; outside untouched.
E. Selected procfs root: Introduction pins a real root, staging rejects
unsupported O_TMPFILE before Accept; zero receipt/progress.

For C/D, choose the last ID from the actual HashMap iteration order
**before** writing the test fixture. This forces first-stage success
then second-stage failure regardless of randomized hash seeds.
Tests assert pending session consent before the attempt; no outbound
bytes using a one-shot nonblocking `poll!` of the real duplex peer
after the failed call; retained exact metadata and zero progress;
no staged/pub owners both before and after the terminal transition;
file-system sentinel invariants and absorbing Disconnected.

Positive acceptance on supported filesystem and sent Accept, valid
EOF/receipt and socket failure are already covered by existing
`receive_reliability` tests and remain in the full suite.

Estimated new methods=2, subcases=5. No production changes, DI or
new dependencies planned. Testability refactor not required.

## Positive/N1-N12 test matrix

E = covered by existing suite, P = planned new scenario, X = not
applicable to this path.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Root pin/staging | E | E | E | X | E | P | P | P | X | P | E | X | P |
| Multi-member Accept barrier | E | E | E | E | E | P | P | P | X | P | E | X | P |
| Terminal/receipt projection | E | E | E | E | E | P | P | E | X | P | E | X | P |

N3: no external network fault in Root; existing socket-send failure
tests cover handler. N8: no date/locale. N11: no entitlements. N4:
existing explicit retry/idempotency; no resend added. N10 existing
redacted errors; new cases assert no untrusted path in user-facing
failure. N5: preparation across multiple members; N6: directory
removed/replaced between introduction and consent. N7: symlink,
directory access, pinned-authority guard. N9: terminal cleanup after
a failed user-approved operation. N12: actual filesystem rejects.

## Mutation review and completion gate

Mutations to catch: committing staged owner inside the loop; sending
Accept before the loop ends; publishing a leaf during preparation;
following parent symlinks; turning failed staging into ReceivingFiles;
inventing successful completion/receipts; leaking staged FDs into
state; incrementing acknowledged bytes; losing original manifest;
accepting missing/unsupported root. This is an assertion-level
mutation review, not an executed mutation tool.

Full repo MODE B branch/negative matrix and instrumented coverage,
plus the `mode-b.yml` ten-complete-suite repetitions, remain
separate explicit **OPEN** gates. Physical Linux permission/mount
behavior beyond procfs and Android/Windows compatibility cannot be
proven by these tests. Rollback is one isolated test commit and
documentation commits; no runtime behavior modified unless a
confirmed defect requires a separately documented MODE A repair.

## Milestones

1. [x] Scan actual root/staging/consent/error code; inventory risks,
   branches, five cases, matrix, mutation review and rollback here.
2. [x] Implement all five cases using real request/session and kernel
   file descriptors; source comments/code English.
3. [x] Run source-exact CI, triage any failure without assertion
   weakening, record exact results.
4. [x] Update this ledger and `docs/current-state.md`; retain all
   physical/coverage gates and source-only dev handoff.

## Execution and exact-source CI evidence

Plan commit `017ccae2` preceded all code. Test source `db19ca8f`
added two parameterized tests and one optional receive-root fixture
parameter. Its first Preflight failed during test compilation, not
domain execution: `futures::poll!` needed the duplex-read future to
be pinned (`E0277: PhantomPinned`). Changed the assertion to use
`tokio::pin!(read); futures::poll!(read.as_mut())` in
`c011f10f` without weakening the no-Accept contract. No production
code, dependency, schema, protocol or host permissions changed.

**Verified source commit: `c011f10f`.**

- GitHub [Preflight](https://github.com/morozhenko-apps/rquickshare/actions/runs/39062398167):
  core library **704 passed / 0 failed**, modern protocol **4/0**,
  native daemon **4/0**. Both added test methods are explicitly
  reported `ok` by the job log; they execute all five cases.
  Rust format, core strict Clippy, offline Python contract
  **54 passed / 0 failed**, and frontend lint/type/unit passed.
- [Native GTK](https://github.com/morozhenko-apps/rquickshare/actions/runs/39062398170):
  strict GTK build/lint and **52/0** isolated tests passed.
- Legacy Tauri Clippy was **still in progress** at this write.
  Do not report full 5/5 Preflight until this job completes.

### Coverage map by production file, primary categories

| Source and method | New coverage | Existing/remaining |
|---|---|---|
| `receive.rs::Directory::pin` | Missing root before Introduction, safe generic error | Positive configured/adopted and moved root in prior tests |
| `receive.rs::Root::stage_file` / `destination_parent` | Unlinked root; regular and symlink later parent; unsupported procfs `O_TMPFILE` | Real selected full/read-only mounts and permission ACL variation remain physical |
| `hdl/inbound.rs::process_introduction` | Pin failure prevents metadata/files | Existing valid Introduction and consent |
| `hdl/inbound.rs::accept_transfer` | First-stage failure and first-success/later-failure; no outbound Accept; no active owners | Existing complete Accept, network send error and cancellation |
| `hdl/inbound.rs::record_termination` / `finish_received` | Zero completed members, zero receipt and absorbing Disconnected for all post-Introduction failures | Existing saved partial first member on later receive failure |
| `sessions.rs`, `received_files.rs` | Original manifest unchanged, no file capabilities, exact zero progress | Existing complete/partial and stale capture contracts |
| Other unchanged core/UI files | All existing tests reexecuted | Not counted as new targeted branch coverage |

Five subcases across two methods by **primary N-category**:
N6=1 (deleted root after Introduction), N7=2 (regular-file
and symlink parent replacing expected directory), N12=2
(missing configured root; unsupported-fs anonymous staging).
Positive, N1–N5, N8–N11 have **zero newly added subcases**,
but their applicable existing regressions were reexecuted.
N8 (clock/locale) and N11 (billing) are inapplicable. N3 is
not applicable to the Root alone; existing handler socket-failure
tests remain green. N5 interaction (successful first preparation
then failure of second), N9 cleanup and N10 generic error redaction
are checked **across the relevant new cases**, not counted as
additional independent scenarios.

Assertion mutation review: treating an absent root as pinning success,
following an unsafe parent symlink, committing a first staged file
before all are prepared, sending encrypted Accept before all stage
operations succeed, leaking open staged owners, incrementing bytes
on failure, fabricating a completed-file receipt or overwriting
Disconnected with Finished each conflicts with one or more direct
assertions. No mutation-testing engine was run.

**Scoped pre-Accept stage complete; repository-wide MODE B remains
OPEN.** The `mode-b.yml` ten-run/coverage workflow was not executed,
and no whole-repo statement/branch percentage is asserted. The
procfs case models actual unsupported O_TMPFILE, not an exhaustive
survey of mounted filesystems. Physical ACL/suspend/full disk,
installed KDE/Wayland, and real Android/Google Windows interoperability
remain external acceptance gates. No native29 install, native30
package refresh, `master` change, background service, signing
or release took place. All GitHub changes stayed on dev.
