# Current implementation and continuation state


## Routine stable 1x / race-sensitive 10x CI — 2026-10-17

Ordinary `Preflight` now partitions `core_lib` using
`scripts/ci-core-test-groups.sh`: **all remaining stable unit tests
and every core integration suite run once**, with eight
full-namespace transport/concurrency/teardown risk probes excluded
by name and rerun exactly once each across **ten consecutive
attempts**. A compiled-test inventory check fails if any selected
test is missing, duplicated, or does not actually execute; no
`#[ignore]`, assertions, no-fail retries or product source changes.
The eight probes are *race-sensitive*, not confirmed intermittent
failures. All other Preflight suites remain 1x, and the separate
GTK verification retains its normal 1x checks on relevant pushes.

The strict **final** MODE B still requires ten **complete**
Core+Tauri+Vue+Python suites plus ten native GTK suites before
coverage: routine CI optimization is **not** a change to the
user's Mode B Stage 11 SSOT. Its first run at `7107339e`
failed on Python run 1 due missing pinned boto3/botocore dependency
setup, while the ordinary single Preflight was 5/5 successful.
This was an environment mismatch, not an identified flaky test.
Commit `f8e5d2d0` installed `scripts/requirements-r2.txt`
into Mode B's isolated Python venv to match Preflight. Exact
split-source Preflight:
https://github.com/morozhenko-apps/rquickshare/actions/runs/39066477112
and corrected strict Mode B:
https://github.com/morozhenko-apps/rquickshare/actions/runs/39066477128
were **in progress** when this entry was recorded. Do not claim
a green full 10x or coverage measurement from a started run.

See [CI stable/stress test classification and execution](ci-stable-race-sensitive-2026-10-17.md)
and [strict final Mode B evidence](mode-b-full-ci-2026-10-17.md).
All public methods/branches and Stage 12 completion remain OPEN,
as do package, physical Android/Windows/KDE, real BLE and live
system filesystem acceptance. Installed native29, native30 package,
`master`, release/signing, external services untouched.



## Mode B whole-suite Linux GTK gate — 2026-10-17

Mode B execution coverage previously omitted the active native GTK
Linux UI from its ten consecutive full-suite iterations. Source
`7107339e` adds a fifth native GTK ten-run gate using the existing
Ubuntu 26.04 + isolated X11/D-Bus harness after all four core/desktop/
frontend/Python ten-run suites pass. Coverage now waits for **both**
ten-run jobs before measuring core/legacy desktop/frontend. The
workflow retains tag/manual invocation and additionally accepts only
an explicit `dev` push that changes its **own** YAML file, never a
normal source push. No product source, protocol, native binary,
packages, permissions or public contracts changed.

The exact-source execution is
https://github.com/morozhenko-apps/rquickshare/actions/runs/39063291246,
running after commit `7107339e`; ordinary Preflight also runs on
that source. Exact ten-of-ten and coverage values are **pending
successful job results** and must not be asserted early. See
[Mode B full-CI plan and evidence](mode-b-full-ci-2026-10-17.md).
Even a green five-suite 10x run cannot establish all public branches,
measured native GTK file coverage or physical Android/Windows/KDE
interoperability. The 2026-10-12 inventory and current later
per-feature ledgers still have open gaps; the global completion gate
remains OPEN. User installed native29, native30 package/update,
`master`, system services, signer and release all remain untouched.



## Offline pre-Accept storage admission and rollback — 2026-10-17

MODE B verified that errors in the Linux receive directory or anonymous
staging **before encrypted Accept** cannot create a final file or claim
received data. Five scenarios: missing root before Introduction,
unlinked pinned root after Introduction, a regular-file or symlink
ancestor for the later file after the first has staged, and unsupported
O_TMPFILE on procfs. New test fixture preserves the actual randomized
per-request member iteration order, proving the first staged member
does not escape when the next stage fails. It also checks that no Accept
frame is written, progress stays zero, manifest is unchanged, zero
receipt authority survives Disconnected, and terminal state is
absorbing. Tests use kernel file descriptors and an actual managed
request. No runtime code changes were necessary.

Exact source `c011f10f`: core Rust **704/0**, 8/0 integration,
Python contract **54/0**, GTK isolated **52/0**, Rust fmt/core strict
Clippy/frontend all passed. Legacy Tauri Clippy was still in progress
when first documented. First test compilation needed explicit Pin for
a nonblocking DuplexStream check; `c011f10f` corrected only that
fixture, without removing the assertion. See
[pre-Accept filesystem admission review](native-preaccept-storage-2026-10-17.md).
Whole-repo MODE B/ten consecutive full passes/measured coverage and
physical permission/mount, KDE and Android/Windows peer gates remain
OPEN. Installed native29 and source-only native30, master, system
services, signing and release were untouched.



## Offline inbound storage fault matrix — 2026-10-17

MODE B scoped native inbound receive hardening now checks six actual
kernel/staged-file failures: ENOSPC (/dev/full), EBADF (read-only file
descriptor) and changed staged inode length at final publication,
each with no completed first member and with the first member already
published. The real FILE handler and Linux O_TMPFILE owners reject
failed writes/publish, preserve only fully published content, retain
correct partial immutable manifest/byte counts, expose a read-only
first-member receipt where applicable, and discard unfinished inodes
after Disconnected. No production code, API, protocol, retry/overwrite
semantics, UI or package was changed.

Source `0bdf73bd`: 702/0 core Rust, 8/0 integration, 54/0
Python contracts, Rust format/core strict Clippy/frontend success
and GTK 52/0 success. Legacy Tauri Clippy was still running at
the exact first final ledger checkpoint; overall Preflight must not
be called 5/5 until that independent job finishes. See
[inbound storage-fault audit](native-inbound-storage-faults-2026-10-17.md).
One parameterized method covers six negative interactions; this does
not close whole-repository MODE B/10-consecutive-runs/instrumented
coverage or physical full-disk/mount policy tests.

Installed native29 and the native30 source-only packaging/upgrade
gate remain unchanged. No user filesystem, live peer, host service,
master, signing or publication was touched.



## Offline outgoing TCP reset and source ownership — 2026-10-17

MODE B scoped tests verified the native outbound FILE path under real
Linux TCP abortive close. One new test resets the peer after introduction
but before the empty-file terminal frame; a second receives a real
512-KiB AES/HMAC FILE chunk of a 32-MiB source and resets the peer in
mid-send. Both check a network I/O error, no Finished/early retirement,
a single preserved manifest/progress, non-overridable Disconnected
state, exact original source on disk and release of its owned Arc once
the request drops. The sender performs **no automatic retry**.
All production code remains unchanged in this slice. The test fixture
initially failed compilation with E0716 (temporary consent frame
lifetime inside `tokio::join!`); it was fixed with a named binding
without weakening assertions.

Exact source `ad2adf04`: 701/0 core library + 8/0 integration,
54/0 Python contracts, Rust fmt/core strict Clippy/frontend and
52/0 Native GTK tests passed. Legacy Tauri Clippy was still in
progress at the documented checkpoint. See
[outbound TCP reset and source ownership evidence](native-outbound-tcp-reset-2026-10-17.md).
The repository's full ten-repeat `mode-b.yml`/coverage workflow was
not triggered: scoped test success is not repository-wide Mode B
completion or physical Google interoperability. Installed native29,
unbuilt/uninstalled native30, master and real peer/package gates remain
unchanged.



## Offline outbound empty-file EOF reliability — 2026-10-17

Review of the accepted sender's FILE loop found that a selected zero-byte
file was silently retired without its required Data(lastChunk=1) frame.
Thus sender Finished could coexist with an incomplete receiver. The native
sender now uses one existing-protocol terminal-frame constructor for both
zero-byte and nonzero files and retires working members only after the EOF
write succeeds. Encrypted loopback tests verify a sole empty file,
empty/nonempty/empty multi-file manifest, exact IDs/offsets/EOF counts and
source preservation. A deterministic missing-key failure before terminal
transmission verifies no false Finished/retirement. This is not a kernel
TCP disconnection regression.

Source `289442da`: Core 699/0 and integration 8/0, Python offline 54/0,
GTK isolated 52/0, Rust formatting and core Clippy/frontend passed.
Legacy Tauri Clippy was still running at that evidence checkpoint.
See [outbound empty-file ledger](native-outbound-empty-files-2026-10-17.md).
The preceding incoming EOF/partial-file tests also remain in this suite.
These are headless single-run results, not physical Android/Windows acceptance
or MODE B ten-repeat proof. Installed native29, native30 package/upgrade
gate, release-signing gate and master stay unchanged.



## Offline transport-loss and partial-file verification — 2026-10-17

A focused accepted two-file inbound regression now closes the external peer
socket during a length header at each prefix position 0-3 or in its frame
body. The first member is already published and the second contains only
unpublished bytes. On real request-owned `DuplexStream` EOF, the existing
terminal cleanup preserved the first exact file/receipt and original manifest,
dropped incomplete anonymous storage and did not replay or fabricate success.
The test harness initially retained an extra legacy broadcast subscriber
that does not exist on the managed path; that fixture interference was
corrected without changing production code. See
[transport interruption ledger](native-transport-interruption-2026-10-17.md).

Exact source `7820b155`: core library 697/0 plus 8/0 integration,
native GTK 52/0, offline Python 54/0 and Rust format/core Clippy/frontend
passed. The fifth Preflight job (legacy Tauri Clippy) was still pending
at this written checkpoint. This is a single CI pass, not the full MODE B
10-run/coverage gate or a physical Android/Windows acceptance result.
Installed native29 is unchanged. Native30 revision is still source-only and
requires exact package/idle gate and explicit installation acceptance.
No host process, personal files or actual peer was touched.



## Offline inbound final-chunk reliability — 2026-10-17

After native30 source-only UI work, dev now handles an inbound FILE data frame
whose final flag and final body bytes arrive together; formerly only a separate
empty lastChunk finalized. Early or oversized final chunks fail before writing,
while exact valid chunks follow existing O_TMPFILE publication/receipt/sync
ownership and no-overwrite rules. Both managed and legacy framing regressions
and real staged multi-file/partial cases were added. Exact-source dev CI at
`1d286316` passed 696 core library methods plus eight integration tests,
core Clippy and formatting, with 52/0 isolated native GTK checks. Separate
legacy Tauri Clippy was still running at this checkpoint; do not infer overall
Preflight completion until that job reports. See
[final-chunk ledger](native-file-final-chunk-2026-10-17.md).

The fix is NOT installed and is not current Android interoperability evidence.
Native29 stays installed. Native30 source package is not built or installed;
the existing pinned idle/source-owner, release-artifact and real-peer checks
are still pending. No physical device, running process or signing/publication
state was touched during this headless continuation.



## Offline-only continuation — 2026-10-17

Development proceeds remotely against `dev` without access to the installed
Ubuntu desktop, real Pixel, user systemd units, local files or signing keys.
Installed native29 remains untouched. Native30 card/History source changes and
the tracked package revision30 are in dev, but no native30 package, upgrade or
installed acceptance has occurred.

An offline reliability follow-up fixed the sole nightly rustfmt check failure:
test-only imports in `core_lib/src/hdl/frame_header.rs` were reordered with no
behavior change (`3c606db`). Exact-source dev Preflight run
https://github.com/morozhenko-apps/rquickshare/actions/runs/39048444812
passed Rust formatting, 54 offline Python contract tests, 693 core library
tests plus 8 integration tests, frontend checks and core Clippy; the separate
legacy Tauri Clippy subsequently passed. Preflight completed all five jobs
successfully on that exact source commit. Frontend Vitest passed 46 tests.
Native GTK run
https://github.com/morozhenko-apps/rquickshare/actions/runs/39048444825
passed its strict source checks and 52 isolated widget/controller tests.
Both source receipts are single runs, not ten-repeat Mode B evidence.
See [frame follow-up](native-frame-interruption-2026-10-17.md) and
[native30 ledger](native30-card-history-polish-2026-10-17.md).

Next strictly non-hardware work may cover scoped static/reliability review and
deterministic tests from the active essential priorities. Native30 locked
release packaging, owner/prepared-source checks, installation, screenshot
acceptance and current Android/Windows physical tests remain distinct OPEN gates.
Do not auto-promote to master or sign/publish on the strength of headless CI.

2026-10-17 native29 passed a real two-file Linux -> Android -> Linux roundtrip.
The user confirmed both receipts and automatic file-manager reveal. Bounded,
snapshot/revision-bound Files1 descriptor reads verified exact sizes/SHA-256 and
basenames against the original manifest (65792+34 bytes). Available peer is
user-reported Pixel6a/Android17; Quick Share version unknown, Windows unavailable.
Folder/text/decline/cancel and lifecycle matrix remain open.

Readback also exposed outgoing journal metadata as zero despite finished sends.
A follow-up source-only repair on dev seeds advertised display info and retires
completed working-file entries after successful final-chunk write. Three new
regressions and the affected135-test group pass; strict core Clippy passes.
The preceding full core run was692/1 before the retirement correction. See
[metadata repair](native-outbound-metadata-2026-10-17.md). This follow-up is not
installed or merged, and cannot backfill old metadata. Native29 stays running.

Native29 binary source
c05e776 includes repair e31b3d5: both request handlers retain partial frame prefixes
across control interruptions. Wire/consent/size-limit contracts and UI are unchanged.
Affected handlers132/0 and strict core Clippy passed; the exact package's14 files,
both services, one14-entry tray and passive APIs passed. Login startup stays off.
See [repair and continuation](native-frame-interruption-2026-10-17.md) and
[native29 evidence](evidence/native29-installation-2026-10-17.json). Native28 rollback
is preserved. Dev contains the new source; master remains at the accepted native28
integration. No public release or full compatibility claim.

Historical mainline checkpoint: native28 was installed and user-accepted.
Its source is merged into the actual master default branch by
[PR #7](https://github.com/morozhenko-apps/rquickshare/pull/7), merge4eaf99ada7c4232d34f3b74272d3e93c32c5e392.
Exact candidate3d048082 passed all five Preflight jobs and one native GTK suite
(52 passed/0 failed). The merge tree is identical; subsequent README/evidence
changes are docs-only. See [integration evidence](evidence/native28-mainline-integration-2026-10-17.json).
Existing master/dev refs are synchronized without force or branch deletion and
checkout stays dev. Installed binary source2c14d26/package28 stays unchanged;
[installed evidence](evidence/native28-installation-2026-10-17.json) includes14-file/
tray/API checks and general user acceptance, not an exhaustive physical matrix.

Root/native READMEs now lead with the actual native build/distribution state and
retain legacy/macOS attribution/reference. Signing keys and encrypted cloud backup
are already verified. Next priorities: current Android/Google Windows ordinary
content and cancellation, interruption/partial/collision and desktop lifecycle,
then exact approved signed distribution/provenance/scoped publishing and APT
consumption. See [acceptance](native-essential-acceptance.md) and
[priority order](native-essential-priorities.md). Previews/new account/trust/QR/
resume expansion remain deferred. No new public release or full parity claim.

Historical native27 installed checkpoint:

2026-10-17 installed checkpoint: native27 is installed and running. Binary source
e523523; exact package and installed/passive receipts are in
[native27 evidence](evidence/native27-installation-2026-10-17.json). Incoming
requests now show guarded Decline/Accept immediately beside peer/type/code;
there is no row Expander, Receive indirection or lower detail body. Accept remains
explicit and revision-bound; bytes replace the code with progress. Manifest names
remain plain tooltip/accessibility metadata. Flat history retains direct checked
result/text/cancel actions; metadata-only historical entries gain no authority.

Affected panel18/0 and strict native lint passed; the exact extracted package's
420px incoming preview was visually inspected. All14 installed files, both units,
one14-entry tray and passive Visibility2/Journal1/AcquireResult passed. Login start
is off. The user screenshot resolved the extra confirmation report as incoming
consent disclosure; no unrelated outbound editor change was made. Real incoming
inline actions/progress/results and history still need physical acceptance on27.
Fast Android discovery/successful sending were user-confirmed on25; BLE is unchanged.
Native25/26 rollback packages are retained. No new master merge or signed/public
release. See [active continuation ledger](native26-flat-transfers-2026-10-17.md).

Historical native26 checkpoint: installed successfully with matching14-file/tray/
API checks; [evidence](evidence/native26-installation-2026-10-17.json). Its history
rows were flat, but the card still had Receive -> expanded consent. Native27
supersedes that card behavior on the user's explicit screenshot request.

Historical native25 checkpoint:

2026-10-17 installed checkpoint: native25 is installed and running. Binary source
4b6d98cdacb267335efa79fc32d3c2f8525acc02; exact package, installed14-file/tray/API
receipts and scoped development checks are in
[native25 evidence](evidence/native25-installation-2026-10-17.json). The user
confirmed very fast Android discovery and successful sending after this upgrade;
no numeric latency was measured. The receiver now publishes the coherent Nearby
V2 BLE header after real GATT registration.

New UI/result flows and all approved contracts are recorded in
[native25 interaction polish](native25-interaction-polish-2026-10-17.md). The user
now requests removing a remaining sender confirmation and flattening history rows
with received-result action inline. The exact reported extra confirmation window
needs identification before changing the input flow: current contextual file and
clipboard completion already submit Send directly, prepared rows QueueRecipient
directly, while text input requires editor submission. No follow-up code yet.
Other native25 physical flows are not individually accepted. Login start remains
off; native24 rollback is retained. No new master integration or signed/public
release.

Historical installed checkpoint before native25:

2026-10-17 installed checkpoint: native24 is installed and running. Release
source d3cd9d3; exact package and installed receipts are in
[evidence](evidence/native24-installation-2026-10-17.json). All14 installed files,
one14-entry tray, Visibility2/Journal1/Queue passed passive verification; login
start remains disabled. The card has flat current device rows with progress;
completed operations live in a separate daemon-backed Transfers window. Prepared
sources stay available and each per-device Send joins one sequential daemon
queue in click order; no recipient checkboxes or second sender confirmation.
Metadata retention is30 days/1000 completed entries, without payloads/name-based
trust. Receiver recovery timers no longer restart on unrelated events; phone
discovery speed and physical UI/queue acceptance remain pending. The task-owned runtime
trace override was removed; the current daemon retains privacy-safe stage
tracing until its next normal restart for the pending phone reproduction. See [active plan and measured checks](native23-card-transfer-separation-2026-10-17.md).
Native22 rollback and uninstalled native23 internal package are retained. No
new master integration, signed release or cloud publication.

Historical installed checkpoint before native24:


2026-10-17 current installed checkpoint: native22 is installed and running for
user testing. Binary/package source0f34808537a2b40a22dc8f56a227b5801dc74b61,
6502732-byte package SHA-256
a3fcdc0118782de16e50b276f2260b39cce9c8fd5d1cdd23704ea703f3803194.
All14 installed files and one Active15-row tray passed, along with passive
Visibility2 and the new Batch1 endpoint-association read. Login start stays off.
The exact real Bubblewrap/Glycin PNG/JPEG fixture passed once on this host; the
previous CI container failure remains a separate unresolved environment gate.
The newer recipient-first Send, retained upward expansion and session-endpoint
grouping are now installed, with physical user acceptance pending. Native21
package/source rollback artifacts remain unchanged. See
[installation record](native22-installation-2026-10-17.md) and
[evidence](evidence/native22-installation-2026-10-17.json). No signed/public release.

Historical source integration checkpoint before the native22 installation:

2026-10-17 current source checkpoint: the additional card/endpoint-association
work is merged into master by [PR #6](https://github.com/morozhenko-apps/rquickshare/pull/6),
merge b93ac02d8d923f5cf5a4d83be41c2aff9510e217. Exact candidate fa28d183 passed
all five Preflight jobs (39023412094); native GTK strict lint and 42 tests ten
times passed at source-equivalent 29322530 (subsequent changes are docs only).
At that checkpoint local master/dev were synchronized; subsequent native22
packaging and install records continue on dev. Native21 was then installed,
while the merged source added
recipient-first Send, retained upward expansion and session-endpoint grouping.
Full native package preview-sandbox validation, broader Mode B coverage and
physical KDE/Android/Windows acceptance remain open. A docs-only receipt skips
unchanged CI; it does not claim a new test run. No install or public release.
See the [completed integration record](native-mainline-integration-2026-10-17.md#renewed-integration-receipt).

Historical dev checkpoint before PR #6:

2026-10-17 later dev-only checkpoint: the user approved optional
session-scoped outbound endpoint association (B) with a clean path toward a
future independently verified durable identity (C). The immutable association
survives dispatch/terminal status, is exposed through an additive authenticated
Batch1 snapshot method and validated by the SDK. GTK nests completed outgoing
operations under an exactly matching currently discovered endpoint; inbound and
unmatched operations remain separate. This is not pairing or stable trust.
Contract/UI regressions passed on code commit 29322530: GTK 42 tests ten
consecutive times (420 successful executions) and complete Preflight green.
Whole-repository MODE B coverage and physical acceptance remain open. The broader MODE B completion gate, real peer
testing and release publication remain open. See
[native card operation contract](native-card-operations-2026-10-17.md).


Historical pre-association checkpoint (2026-10-17; superseded by the later selected B implementation above): user selected B for
completed operation grouping, with only provable device association. The current
D-Bus recipient projection supplies names but not endpoint IDs, so no safe
same-device merger is possible yet. New scoped UI changes keep active transfers,
discovered endpoints and collapsed unlinked completed activity separate, retain
transfer capabilities, repair transfer expansion/bottom anchoring and terminal
unsupported-content labels, and open recipients before source choice on Send.
See [card operations plan](native-card-operations-2026-10-17.md) for explicit
API STOP FACTOR, MODE B inventory and change/rollback ledger. New native GTK
test workflow is running; pre-change tests do not validate this candidate.
No native22 installation, release signing or public APT publication is implied.


Mainline integration (2026-10-17): native21 source is merged into the actual
default branch master by PR #4, merge SHA 25cf7dd17ea28e98b2765efd09e0474d0028334a;
remote dev was fast-forwarded to that merge checkpoint without force. The exact
pre-merge dev HEAD passed all five Preflight jobs (run 38984114755), including
the fixed Python fixture/runtime contract checks. The separate post-merge master
run is 38984886879. A documentation-only follow-up records these results; local
workstation synchronization was pending at that earlier checkpoint and is now complete in the PR #6 receipt. See
[integration checkpoint](native-mainline-integration-2026-10-17.md). Installed
native21 and its accepted UI stay unchanged. Current physical Android/Windows
peer, KDE lifecycle, full Mode B ten-run and approved signed-release gates
remain open.

Latest work (2026-10-17): native21 is installed and running. Primary tray Activate
now toggles the card, and valid same-session monitor/bottom margins are restored
after hiding instead of reapplying tray coordinates. Expanded temporary Receive
shows a spinner, visible-device message and countdown. Visibility2 `(sbtbyt)`
returns one core snapshot including remaining_ms; SDK stamps a conservative
monotonic sample and GTK ticks only the display while visible/expanded. The daemon
remains the sole expiry owner. At sampled zero the display waits for authority;
no per-second IPC, optimistic visibility or fresh minute on reopen.
Native42/0 and final affected GTK1/0 passed. The service run292/297 exposed five
old tmpfs fixture-lifetime failures; all five passed after retaining their named
sources, with production adoption/security gates unchanged. Strict core/native
lint passed. Offline package verification,14 installed file comparisons, one
15-row tray, actual SNI Activate hide/show roundtrip and Visibility2 read passed.
Login start is off; current owned daemon/desktop PIDs2201915/2201916. The user confirmed native21 same-session position retention and working
countdown on2026-10-17 after the requested tray hide/reopen check. The native20
panel appearance was also accepted. Natural expiry, changed monitor topologies
and current-package peer interoperability are not claimed by that confirmation. Native20 rollback artifacts remain immutable. See
[card usability](native-card-usability.md) and
[native21 evidence](evidence/native-tray-countdown-2026-10-17.json).
The user selected Receive=request review or one-minute
visibility, view switching only on the card and atomic daemon-owned Add files.
[Source extension](native-source-extension.md) is committed with core/SDK checks.
The single toolbar has four actions; its upper drawer uses the shared device/
request/transfer list with status overlays and inline consent details. There are
no main tabs or separate Full header.
Settings has branded sidebar sections and deferred OK/Cancel. Tray duplicates
and routine Reconnect are removed. Source checks and renderer observations are
recorded in the follow-up. The requested native21 UI checks are user-confirmed;
broader desktop/peer acceptance remains open.
No paired identity or durable history is inferred from the mockup or device names.

Updated2026-10-17. Mode A; implementation on dev. The separately authorized
mainline integration uses existing dev/master only, with no new branch/worktree. This is the current
status index. Dated entries in other documents remain historical evidence; they
must not override this index or the linked feature contracts. The active goal is reliable ordinary nearby sharing with native
tray/card UX. [Essential priorities](native-essential-priorities.md) supersede
the broader execution order: previews are excluded and full feature parity is a
long-term inventory, not an essential-release gate.

[Completion audit](native-completion-audit.md) distinguishes implemented and
user-accepted scoped UI from broader installed/peer release acceptance. Completed
foundations include [live visibility](native-live-visibility.md),
[multi-recipient sharing](native-multi-recipient-review.md) and
[atomic receive publication](native-receive-reliability.md). Their native9/11/12
artifacts and dated observations are historical; native21 is the installed
candidate. The selected one-minute policy expires to Hidden. No production
remembered trust, Google account integration, auto-accept or full parity is claimed.

## Documentation maintenance plan and rationale

Six copied top-level distribution checkpoints drifted: they still required an
isolated signer and unknown fingerprint after the user selected this workstation
and initialized production keys. Product-direction also opened on an obsolete
domain-choice checkpoint. Consolidate current facts here and link the entry points
instead of copying another mutable status paragraph. Preserve dated observations.

1. [x] D1: replace duplicated checkpoints with links to this index, add an explicit
   historical-log notice, and correct the current capability summary.
2. [x] D2: prepare the next device-name ownership/lifecycle review from actual
   core/settings/transport readers. Significant UX/API implementation waits for
   user selection; no protocol/runtime changes in this documentation slice.
3. [x] D3: validate links/removed stale checkpoint claims and whitespace once;
   commit each logical documentation change. No unaffected Rust/package rebuild.

Risk: readers may mistake old dated checkpoints for current state. Mitigation:
entry links and explicit historical labels. Rollback is a documentation-only
commit; no runtime, saved schema, key custody, remote object or package changes.

Measured documentation check: seven entry points and21 relevant local links passed;
at that checkpoint archive verification passed and sidecars were still pending.
All four cloud artifacts are now verified as recorded below and in JSON evidence.
The device-name review explicitly gates implementation on selection.
Whitespace check passed. No production code changed or unaffected checks repeated.
The user subsequently requested continuing the recommended live-update option1.
[Live device name](native-device-name.md) N1–N5 are complete in source, scoped checks
and a verified uninstalled native4 package. The subsequent completed slice is
[temporary visibility ownership](native-visibility-review.md): mDNS previously
owned expiry, so its failure could leave Bluetooth temporarily visible indefinitely;
a stale tick could also overwrite a newer mode command. Core-owned policy is
selected by the user and implemented: core-owned deadline, read-only transport/
shell state, generation-safe expiry and owned mDNS SDK shutdown. Source checks
and the corrected native6 uninstalled package passed. V5 public source/package/
unsigned-APT/request export is verified; exact provenance and hashes are in the
visibility contract/evidence. Continue installed/physical acceptance
separately; do not restart the already completed name slice.

Next completed foundation: [authenticated peer/trust contract](native-trust-review.md).
User selected option1. Bounded role/token-bound P-256 signature/HKDF/certificate
verification and inbound/outbound PairedKey result gates are implemented; explicit
Fail stops before Introduction. Twelve new methods, scoped checks and native7
offline verification passed. Production certificate source is explicitly empty:
operational acquisition/authorization/rotation and remembered trust remain open.
No trusted-device UI, account access or auto-accept is implemented. Native7 public
source/artifact export is verified; exact receipt/hashes are in the contract and
evidence. Do not claim Google Account
or physical peer acceptance from the offline fixtures.

[Certificate-provider review](native-certificate-provider-review.md) researched
the next dependency. No supported third-party scope/client path was established.
An independent optional Linux provider uses an explicitly unofficial master-token/
Google Play services exchange, not normal desktop OAuth. The user selected option1:
supported authorization only; continue independent parity. No sign-in, credentials
or account API calls. Native12 is the latest measured package (see current artifact below). The next scoped
prerequisite [bounded byte ingress](native-byte-ingress.md) is implemented: both
handlers now share request-owned16-payload/5MiB reservation limits, stable headers,
exact completion and release-before-dispatch. Nine new methods and scoped checks
passed as documented; native8 includes this fix, while native7 remains immutable.
[Preview decoder boundary](native-preview-review.md) option1 is selected and its
offline native foundation is implemented: private Glycin/Bwrap helper, hard
resource limits, validated local pixel IPC, cancellation/reaping and Full-card
fixture. Ten new Rust methods/scoped native checks and the native8 uninstalled
Ubuntu26.04 amd64 candidate passed; extracted PNG/JPEG and Full/Compact rendering
were verified. D6 source/package/unsigned-APT/request public export is verified;
exact archive/binary provenance and hashes are in the decoder contract/evidence. Remote
preview encoding/mapping, daemon IPC/production binding and persistent resume
contracts remain unimplemented. Preview development is now excluded by the user;
retain this as historical foundation evidence. Do not ask for the decoder boundary again or claim full
preview compatibility from local fixtures.

[Preview wire review](native-preview-wire.md) now records complete public Google
source inspection at both the earlier pin and current main. No codec/mapping
fixture was established. The scoped internal inbound routing fix is implemented and its focused checks
passed: declared previews are bounded opaque data, isolated from sharing control
parsing and discarded. Production preview IPC/display is unimplemented and excluded from the active
scope. Bounded opaque routing remains input-safety behavior. Native9 remains immutable and does not include this source fix.

## Implemented source and remaining acceptance

| Area | Current evidence | Remaining work |
|---|---|---|
| Native ownership | Rust daemon, authenticated typed D-Bus, GTK4 tray/card, Full/Compact, separate menu/notifications, live visibility status/quick modes, core-owned temporary visibility and owned SDK cleanup implemented | Installed KDE/Wayland lifecycle, placement, lock/DND/focus/scale and physical visibility acceptance |
| Sharing | Daemon-owned1..10-recipient admission/results, file/folder preparation, consent/PIN, progress/cancel, atomic per-file receive publication/explicit partial results, received text explicit Copy, outgoing text/clipboard/PNG, received-file reveal and PairedKey authentication foundation implemented | Current native package with physical Android/Windows peers; operational certificate provider, no full compatibility signoff |
| Settings | Daemon-owned schema2/Settings2 store, migration/backup/recovery, live device name and GTK editor implemented | Real installed migration/recovery and Google/KDE name refresh |
| Desktop integration | Dolphin Send assets, descriptor handoff, systemd activation and opt-in daemon+hidden-card tray login start implemented | Installed context-menu/session/autostart, upgrade/removal/live-transfer acceptance |
| Distribution | Verified uninstalled Ubuntu26.04 amd64 native12 package with live visibility controls and private sandbox helper; public source/package/unsigned-APT/unapproved-request handoff verified; publisher and narrow HTTPS Worker route implemented | Exact release approval, bucket-scoped publisher credential, real signed public APT acquisition and installed updates |
| Signing recovery | Production primary/subkey initialized on existing Linux PC; non-release proof, encrypted local restore and all four independent cloud artifact downloads passed | Recovery checkpoint complete; hourly follow-up deleted; preserve archive password and backup custody |
| Further parity | Modern wire fields, folder support, native live name and core temporary deadline exist | Installed visibility controls, physical discovery withdrawal and suspend acceptance, Ordinary transfer/desktop acceptance first; account identity, resume, streams, Wi-Fi/app and QR/Apple capabilities are deferred; previews are excluded |

Feature contracts and measured checks: [product history](product-direction.md),
[settings](native-settings.md), [desktop ingress](native-desktop-ingress.md),
[autostart](native-autostart.md), [package](native-package.md),
[APT](native-apt.md), [publisher](native-r2.md), [download route](native-download-route.md),
[local signing](native-local-signing.md), [recovery](native-signing-backup.md).

## Distribution binding already selected

Production base is https://download.morozhenko.com/dist/rQuickShare/ (case-sensitive).
The deployed rquickshare-download Worker reads only the dedicated private
rquickshare-native-apt bucket through that prefix. Existing app-dist/Picturix
routes were preserved. Domain and signer-host selection are complete; do not ask
for another host, browser login or replacement domain to resume independent work.

Signing uses protected authority and subkey-only profiles on the existing Linux
workstation, not independent-host isolation or offline primary custody. Public
primary6D8854762CF0C86881138A56695A0888D09FA616 and signing subkey
1F5150D98D94C43109FBA2503E7DE744C1014357 are initialized and verified.
This is not approval to sign a particular release request. The public keyring and
non-release evidence are committed; private profiles/passwords/ciphertext are not.

Recovery archive direct download matched20559 bytes and SHA256
a9de8d32972119de7ec747849c76c10e292543d0b984bb28ba0b948f0220f9bf.
It is the same ciphertext whose local restoration/signature/revocation checks
passed. The .sha256/receipt/instructions were independently downloaded and verified
on2026-10-15 at13:25:25–13:25:52Z, completing four-file cloud acceptance. The app
confirmed deletion of the hourly follow-up. Exact evidence and historical quota/
repair observations remain in native-signing-backup.md and its JSON.

No production APT release has been signed/published. The retained native12 APT staging package SHA256
b4ef947385b46a2b16f2dc625f1cfc216205672b1df75478f821bb4a857962f0 was built
from88c1f94003fc35c665568024273d446b5ced35ed and verified offline. It includes daemon-owned
multi-recipient sharing and the defensive announced-preview routing fix.
Unsigned APT metadata expires2026-10-23T18:08:35Z; canonical
first-release request 34d1ca73c10956a269196ee42005996abcfbb09012a9dafbe3406bc3fd7c7e05
is unapproved. Exact source/check/artifact evidence is in
native-receive-reliability.md and evidence/native-receive-reliability-2026-10-16.json.
Native11 remains the immutable multi-recipient checkpoint.
Native10 is a preserved intermediate package whose visual review exposed the
Nearby-footer navigation defect corrected in native11; it was not handed off.
Native9 remains the previous live-visibility checkpoint.
Native8 is the isolated decoder checkpoint and remains immutable.
Native7 is the verifier checkpoint, native6 the core visibility checkpoint, native4 the name checkpoint;
intermediate native5 was superseded by explicit SDK shutdown. Prior artifacts
remain preserved. Do not confuse a source-commit assertion with proof
of build provenance or report later documentation-only commits as binary rebuilds.

## Latest implemented parity slice

[Multi-recipient sharing](native-multi-recipient-review.md) selected option1:
the daemon admits1..10 recipients atomically using one prepared source and bounded
per-peer descriptor reservations. Existing ordered receipts recover exact results;
GTK never loops over sends or silently drops a selected vanished peer. Separate
transfer rows retain recipient names and independent Cancel. Core8 affected methods
and native39 passed; final navigation-specific GTK1/0 and strict lints passed.
Native11 includes these changes and the defensive preview routing foundation.
Remote preview codec/mapping, production preview IPC/card binding and full Google
interoperability remain unestablished. Installed/radio/current-peer acceptance is
separate from these offline source/package observations.

## Current essential work order

The authoritative ordered queue and completion criteria are in
[native-essential-priorities.md](native-essential-priorities.md):

1. Current Android/Google Windows discovery and ordinary two-way sharing acceptance.
2. Safe interruptions, storage failures, partial-file handling and explicit retry.
3. Installed KDE/Wayland tray/card, consent, progress/cancel and recovery.
4. Installed file-manager Send, optional login start and single-process ownership.
5. Straightforward package lifecycle and the selected signed APT update path.

Without physical peers, inspect existing failure/cleanup/retry paths, fix concrete
scoped defects, prepare targeted acceptance steps and check package lifecycle
requirements. Do not substitute new accounts/QR/preview work for missing physical
evidence. Previews are excluded; resume and advanced parity are deferred.
Development checks run once per affected change/failure. No installation or release
approval is implied by this ordering.

## Active reliability finding — 2026-10-16

Source review found that incoming files are created under final names before
content arrives and request drop has no incomplete-file cleanup authority.
Exclusive creation and short-EOF validation already exist, but do not prevent
visible empty/truncated files after failure. The initial finding was source-only. The selected repair and offline regressions
are now implemented; this is not physical-peer acceptance.
[Receive reliability](native-receive-reliability.md) records the significant
publication/partial-outcome choice: recommend atomic publication per file, preserve
complete members and expose partial outcome; alternative is one all-or-nothing
containing transfer directory. The user selected option1; implementation now stages unnamed files, publishes
complete members without overwrite and exposes terminal partial receipts through
the existing native file capability. Core focused checks, native39/0, strict lints and the native12 offline package
passed. Exact public source/artifact handoff follows the receive contract; native11
and earlier artifacts remain immutable.
[Essential acceptance](native-essential-acceptance.md) provides the short current
Android/Windows/KDE check procedure; physical transfer checks remain unexecuted. Native11 and
its immutable public artifacts are unchanged.

## Installed acceptance checkpoint — 2026-10-16

[Installed acceptance](native-installed-acceptance.md): after explicit user approval,
native12 replaced legacy0.11.5 on Ubuntu26.04.1/KDE Wayland. APT installed only
rquickshare0.11.5+native12 and libgtk4-layer-shell0 1.3.0-1;11 installed regular
files match candidate bytes/modes/root ownership. The exact private five-file
legacy recovery package remains available; downgrade was not executed.

One systemd daemon/client and one real KDE tray item are running. Repeat normal
launch retained the same PIDs; login start remains disabled. User confirmed the
icon/card and available Android. Owned disk descriptor Begin/Append/Abort passed
with zero remaining source/handle/byte/transfer resources; no Commit/Send.
The /tmp fixture exposed the documented broad seal-capable-source rejection:
[tmpfs source review](native-tmpfs-source-review.md) records the subsequently
selected and implemented accounting repair in native13, detailed below. Actual Android/Windows transfers, remaining visual/
lock/DND/file-manager/login-start/stop-restart/removal acceptance and signed APT
publication remain open. Exact installed evidence is in
evidence/native-installed-acceptance-2026-10-16.json. Backup/keys and immutable
native12 binaries/source/release artifacts are unchanged.

## Installed tmpfs repair checkpoint — 2026-10-16

The user selected option1 in [tmpfs source review](native-tmpfs-source-review.md).
Ordinary linked read-only tmpfs files now follow existing filesystem-source rules;
anonymous sources still require full immutable seals,1..32MiB and128MiB aggregate
accounting. No wire/settings schema change or spool. Binary source
1607c8f implements the guard and focused regressions:49 source checks passed,
strict all-feature/all-target core lint passed. Native13 package SHA256
3d52d836d5b2a4a4832b1ea00b8964e7b3c6a16d1bb2e19926d94b8c22414d06,
6,416,616 bytes, is now installed and all11 installed files match. One daemon/
client/tray, repeated launch and the formerly failing installed /tmp append/abort
passed. Native12/legacy recovery packages remain available; login start disabled.

A123-byte nonpersonal Android fixture is prepared through the real installed CLI,
with explicit recipient selection/Send still required. The agent has selected no
peer or sent content. User-confirmed Android availability/icon/card is distinct
from pending physical transfer/code/content/version results. Keep the prepared
source and application owners intact while awaiting the user's check.
Native13 is not signed or staged into APT; the earlier native12 unsigned repository
and unapproved request are retained historical artifacts, not this repaired release.
Exact repair/build/install/runtime observations: evidence/native-tmpfs-source-2026-10-16.json.

First Android outgoing smoke passed by user report: file received, matching code.
The daemon shows one outbound Finished and zero source resources; outgoing terminal
metadata is Unsupported with no manifest/byte counters, so independent byte/hash
verification is not claimed. Exact versions, reverse sharing and interruption/
Windows checks remain open. Next selected work: the user's eight-point card/tray
cleanup in native-card-cleanup.md, while preserving the essential acceptance queue.

## Requested card/tray cleanup installed — 2026-10-16

[Card cleanup](native-card-cleanup.md) implements the user's eight presentation
requests: transparent outer window/rounded themed body; upper-right hide cross;
no duplicated options button, card branding, idle Ready/Choose files/Discovery
labels; original blue app icon; Settings/About identity and consistent ordered
Full/Compact content with measured bounded height. Existing consent, error/recovery,
visibility owner, source clearing and transfer lifetime are preserved. No core/
wire/settings changes; native13 tmpfs repair remains included.

Native15 is installed: SHA256 f32e3f35021c358e3357bab066ce695c9d8a2272616f030d9a29e9b9c0340099.
Fourteen installed files match; one daemon/client/active KDE tray item, restored
icon name and About menu verified. Native14 is an uninstalled diagnostic package
whose visual review found premature scrolling; final sizing repair is native15.
Native13 remains rollback and first user-confirmed Android outgoing checkpoint.

Native39 and package5 passed initially; affected GTK1/lint passed after each
visual sizing correction, and final package5 after icon ownership relocation
passed. Final release widgets show transparent corners and full short lists.
Physical KDE cosmetic review is pending user response. Broader Android reverse/
version/hash/interruption, Windows, lock/DND/focus/lifecycle and signed APT gates
remain; do not equate this eight-point slice with full product/UI/parity completion.
Exact source/build/install/visual observations: evidence/native-card-cleanup-2026-10-16.json.
