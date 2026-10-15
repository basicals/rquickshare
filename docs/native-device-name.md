# Owner-local live device name

2026-10-15. Mode A, direct dev. User continuation after the recommended option1
authorizes the live-update slice in native-device-name-review.md. Read the guide,
native settings/recovery/runtime/transport code before this plan. No install,
host-settings mutation, production signing/publishing, peer operation or push.

## Stages and rollback, before implementation

1. [x] N1: validated/redacted name and injected owner-local revisioned identity
   with immutable snapshots and explicit consumer acknowledgements. Focused domain
   tests/inventory first, atomic independent commit.
2. [x] N2: inject identity into core/TCP admissions, immutable outbound handshake,
   live mDNS endpoint data and GATT slot0 provider. Preserve DNS host/endpoint IDs,
   crypto/PIN, discovery visibility and joined shutdown. Scoped contract checks.
3. [x] N3: schema2 native persistence with exact schema1 backup/migration, Settings2
   service/client contract and live-apply receipt. Preserve write/replay/CAS/recovery.
4. [x] N4: secondary GTK name entry/default reset, typed validation and accurate
   saved/applied/error presentation; update relevant docs/package source identity.
5. [ ] N5: appropriate affected checks once, actual observations and remaining
   hardware acceptance, atomic commits and recoverable handoff.

Rollback UI/client before schema/service, then transport and identity foundations.
Schema2 is not silently readable by an older binary: retain exact private schema1
backup; never let rollback overwrite a newer incompatible document. No external
configuration or release rollout is performed by this source change.

## Frozen owner and name contract

Name is presentation data, never authentication/trust. Optional override None uses
the startup OS hostname; explicit names must be nonempty, no leading/trailing
whitespace, no controls or bidi override/isolate controls, <=237 UTF-8 bytes.237 is
the actual BLE receiver endpoint budget (255 minus18 fixed bytes); mDNS and peer
handshake must use the same string instead of independent truncation. Preserve
ordinary Unicode and internal spaces; no case folding/normalization/silent trim.
Empty GTK entry/empty IPC override means restore automatic name. DNS hostname is
resolved independently once for ServiceInfo and never set to the user display name.

Each RQS owns one identity policy. It publishes monotonic revisions and immutable
name snapshots; consumers cannot write settings or change another owner's name.
Same effective name is idempotent. Updating the override still persists intent
even when its resolved name equals the automatic value. No process-global state,
ambient environment overrides, unbounded queue/history or name-bearing diagnostics.
Debug output redacts names. Counter exhaustion refuses the update, never wraps.

Session identity is snapshotted before outbound connection I/O, then retained for
both ConnectionRequest fields throughout the session. Later updates affect only
new admissions. Existing default/legacy standalone constructors retain OS-name
behavior and resolve it once per handshake, not independently for the two fields.

## Frozen live apply and failure contract

mDNS and, where compiled, GATT name consumers explicitly register and acknowledge
the exact revision installed locally. mDNS acceptance means local service info was
updated and the library accepted registration when visible; no physical-peer claim.
Hidden receiving stays hidden. Name updates must not reset temporary visibility.
GATT slot0 reads use the current immutable name-derived data; BLE FEF3 advertising
contains no human name and needs no unrelated radio restart. An ongoing offset
read must not splice bytes from different revisions. Use a request-local coherent
snapshot or conservatively retain consistent full-read semantics per read request.

A dropped/failed consumer is unavailable, not implicitly acknowledged. Unsupported
compiled-out transports are not required consumers. Apply waits on actual revision
acknowledgement/failure and owned cancellation, without guessed sleeps/timeouts.
The bus owner must retain cancellation and transfer processing while waiting:
admit/track identity-apply work independently rather than blocking its event loop.
Saved settings survive a partial consumer failure; return explicit not-applied
state. No automatic daemon stop/restart, rollback of durable save, replayed transfer
or automatic repeated settings write. Retrying the exact save may reapply policy
without changing its revision/receipt. Startup readiness must not claim successful
name application solely because a transport task was spawned.

N2 GATT long-read refinement before verification: retain at most32 peer-addressed
immutable slot snapshots. Offset0 starts/replaces that peer's snapshot; later
offsets read the retained bytes across renames, never splice generations. A new
33rd peer evicts the oldest slot; its later continuation explicitly fails, while
a new offset0 can retry. End-offset releases a slot. No timers, background tasks,
unbounded peer history or logs of addresses. Legacy static-provider offsets retain
their existing semantics. Four new N2 methods cover bounded eviction/offsets,
generation coherence, real mDNS ServiceInfo data and actual gated TCP handshake.

## Frozen persistence/IPC/editor contract

Native schema2 adds validated optional device_name. Strict schema1 has its original
exact fields; loading it yields automatic name. The first explicit save backs up
its original bytes through the existing private bounded backup mechanism before
writing schema2 with a new receipt/revision. Reads never migrate/write. Unknown,
malformed or invalid names enter explicit recovery, never reset silently. Include
raw name intent in retry fingerprint; a reused request ID with a changed name must
conflict. Keep source-token/CAS, retained directory capability and durability rules.

Settings2 replaces the unpublished Settings1 contract for the coordinated native
client/service source. Version the D-Bus interface/XML rather than changing an
existing struct signature under Settings1. Add saved override/effective name and
name-apply status; empty wire override represents None. Keep prior scalar/root/
port semantics. Exact typed bounds apply on both bus and SDK, redacted Debug.
The editor has one name field with automatic-name placeholder and explicit Save;
reset uses blank override, not an OS hostname mutation. No field in the tray card
or consent flow. Show saved/pending/unavailable accurately; no claim another peer
has already refreshed its device cache. Preserve ongoing sends, clipboard/source
review and exact retry intent. No GTK direct file writes.

N3 wire refinement before code: append saved device_name override(s), current
resolved_name(s) and name_status(y) to SettingsView; status0 has no live identity,
1 is saved/current or consumer-ack pending,2 locally applied,3 consumer unavailable.
Append device_name(s) to SaveIntent. Empty override means None. Interface signatures
are (sbybtsyiysbtbbysssy) and (stssyiyybs); interface name is Settings2. A name is
not presented as applied when its saved resolved value differs from the current
policy. Read/Reload project fresh consumer state; Save acknowledgement waits for
the admitted generation through a bus-owned cancellation-safe future, while the
main bus continues transfer/control processing. No extra task/timer or repeated
write is introduced. Existing error codes0..4 retain their meaning. Schema1
parsing has a separate strict original value shape; schema2 stores typed optional
DeviceName and explicitly saves a backup before replacing schema1 bytes.

## Scoped branch/interaction inventory before tests

| Artifact | Branches and interacting states | Level / planned methods | Categories |
|---|---|---|---|
| DeviceName | valid Unicode/spaces; empty/whitespace/control/bidi;237/238 bytes; multibyte crossing budget; redacted Debug | table-driven domain2 | Positive,N1,N2,N7,N10 |
| Identity policy | independent owners; initial/default/custom/reset; same-name replay; monotonic update/overflow; immutable old snapshots; consumer current/stale/wrong/future ack; dropped/failed consumer; pending/cancel/closed wait | domain4 | Positive,N1,N2,N4,N5,N7,N9,N10 |
| Transport consumers/session | two handshake fields equal; update during connect retains admission name; mDNS DNS identity unchanged/visible/hidden/temporary/error; GATT coherent slot payload/offsets; consumer exit and joined cancellation | existing real protocol/worker fixtures + focused domain/integration4 | Positive,N3,N4,N5,N7,N9,N10 |
| Schema2 store | exact schema1 backup and explicit migration; defaults/read-only; malformed/unknown/name invalid; retry changes name; source CAS and injected write failures | real store tables3 plus affected existing suites | Positive,N1,N2,N4,N5,N6,N7,N10,N12 |
| Settings2 bus/client/runtime | actual isolated broker typed read/edit/retry; validation and wrong epoch; saved vs actual acknowledgement; unavailable consumers, cancellation and service shutdown; no transfer replay | isolated broker2 plus affected existing suites | Positive,N1,N3,N4,N5,N6,N7,N9,N10,N12 |
| GTK editor | default/custom/reset/load/save; byte-invalid Unicode input; dirty/retry preservation; saved/pending/failed name status and closed/locked service | isolated GTK2 plus affected existing editor tests | Positive,N1,N2,N4,N5,N9,N10 |

Counts are scoped Mode A estimates, refined against executable branches before
each stage; do not duplicate unrelated complete suites. Real filesystem/protocol/
identity logic, external IO seam only, no subject mocks or magic waits. Mutations
to byte limits, name binding, generation equality, dropped-consumer checks, backup
condition or receipt fingerprint must fail corresponding assertions. N8 has no
time/calendar contract; N11 no entitlements. No final Mode B/full-parity claim.

## N1 implementation and actual verification

Added identity.rs:237-byte validated/redacted DeviceName with strict serde decoding;
owner-local immutable snapshots and monotonic/idempotent overrides; fixed two-slot
required-consumer acknowledgements; stale/future/duplicate registration rejection;
RAII failure on consumer exit; condition-driven cancellation-safe waits. Old name
snapshots and independent owners remain unchanged by a later rename. Public serde
uses a bounded String without enabling extra dependency features.

Six focused identity methods passed in one invocation (build38.65s, execution0.00s;
578 unrelated library methods filtered, not executed). Primary categories N1=1,
N10=1,N5=1,N4=1,N9=2; positive/boundary/interactions overlap these tables. All control
and bidi override/isolate codepoint ranges are covered. Assertions check exact
state/generation, not only success. Nightly formatting passed. Existing bluer
future-incompatibility notice remains. Terminal exited; Terminal closed.

N1 is an independent foundation only: no hostname reader, settings schema, IPC or
GTK behavior changes yet. Continue N2 transport binding, then N3/N4; do not claim
that core identity tests prove transport updates or Google peer compatibility.

## N2 implementation and actual verification

RQS now resolves one bounded fallback at transport start, owns optional configured
override and its consumer lifecycle, and releases the policy after joined stop.
TCP context snapshots identity before connect I/O; OutboundRequest retains that
snapshot for both wire fields, including standalone fallback resolution once.
mDNS updates endpoint data without changing DNS identity or the visibility timer;
hidden policy stays hidden. GATT installs name-derived slot data atomically after
service registration; bounded long reads preserve their own snapshots. Consumer
failure is explicit and no radio/daemon restart is introduced.

Initial compile found a test type mismatch: endpoint_name is wire bytes rather
than String. Corrected the assertion to exact byte comparison without changing
the expected name. One actual whole library run then passed588 methods in8.01s
(build21.77s), including four new N2 methods and the six N1 methods. Broad core
execution was appropriate because RQS/TCP/GATT integration spans every core owner;
this is not ten repetitions or physical Google/BlueZ acceptance. Primary new N2
categories N5=3,N7=1. Native binaries/GTK have not been checked for this slice yet.
Nightly formatting passed. All subprocesses exited; Terminal closed.

Current handoff: N2 source and domain/protocol checks are complete, but the name
setter is not exposed by persisted settings/GTK until N3/N4. Existing signed
publication and installed acceptance remain separate. No user daemon was started.

## N3 implementation and actual verification

Schema2 adds typed optional DeviceName; a strict separate original schema1 decoder
preserves automatic naming and original bytes. First explicit schema1 save backs
up those bytes before replacement; name intent participates in replay fingerprint.
Startup applies the saved override to its RQS before transport start. Old source
and unrelated settings/root/port/durability behavior remain intact.

Settings2/interface XML/signatures now carry override/resolved name and actual
consumer status. Owner/Writer projections inspect the injected owner-local policy;
startup task creation is not name-application success. A bus-owned bounded apply
future leaves transfer/control processing live until acknowledgement/failure;
close drops it and joins the existing settings writer. Save response preserves
durable saved state on partial failure. GTK temporarily preserves the loaded name
on other setting edits until N4 adds the editable field.

First compilation identified that SelectionBus's existing borrowed async methods
need Sync; its new erased future initially specified only Send. Added the actual
Sync future bound/type alias, without an extra task, global lock or unsafe marker.
Then270 affected service methods passed in6.47s (build25.48s,323 filtered), including
five new methods: N6=2,N4=1,N5=1,N3=1 primary categories. Existing injected write/
sync/rename failure cases now also assert exact Before/After name preservation.
Actual isolated broker proves another client can read during pending name apply,
only the exact two consumer acknowledgements finish it, reset returns automatic
name, and a failed consumer preserves saved/error state. Name/serde bounds and
strict old-shape migration reject malformed data. Signature assertions/XML match.
Nightly format passed; subprocesses exited (Terminal closed). No whole593 run,
new real desktop service, physical Bluetooth or Google acceptance claim.

Continue N4 editor integration and N5 affected native/binary/lint/package checks.

## N4 editor refinement before code

Add Device name entry with Automatic computer name placeholder and a plain-text
resolved-name hint. Empty input resets the override on explicit Save; no OS rename.
Do not set a truncating character limit on Entry: validate the shared UTF-8 byte
contract and reject invalid input rather than saving its prefix. The existing
dirty/load/retry/sensitivity lifecycle also owns this field. Name edits are disabled
while a save is pending or the editor lacks write authority. Readiness0/1/2/3 and
error4 must not all render Saved and applied. Retain the existing bounded-height
scrollable secondary settings body so all controls remain reachable as the extra
field increases content height. No tray-card/window sizing or flow changes.

Verification includes one new pure status table and the existing isolated GTK
editor method extended with actual Entry values/default/custom/long Unicode/dirty
binding checks. Together with N3 real broker Save/Reset tests, avoid duplicating
daemon validation through a mock settings owner or adding a per-field E2E suite.

## Final verification and package plan before execution

N4 GTK run initially stopped at pkg-config: host development GTK4 is not installed.
Recovered the previous task-private sysroot location from successful Cargo GTK
fingerprints, rather than installing host packages or inventing another SDK.
Existing root: /home/morozhenko/Documents/Codex/2026-10-12/new-chat/work/gtk-native/root.
Set PKG_CONFIG_SYSROOT_DIR to that root, PKG_CONFIG_PATH to its
usr/lib/x86_64-linux-gnu/pkgconfig, and LD_LIBRARY_PATH to its library directory.
Observed private GTK4 version4.22.4 and Layer Shell1.3.0. These are local verification
inputs, not committed runtime search paths. All39 native methods then passed in
one isolated broker/Xvfb run, including the new name-status table and extended
actual editor method. Captured /tmp/rqs-device-name-settings.png and inspected it;
the existing620px scrolling body fits the new entry. Removed unwanted selection
from the informational name hint; recheck only that affected GTK method once.

Final source review also keeps the identity read guard across acknowledgement
inspection so concurrent publication cannot produce a mixed-generation applied
answer, and shares the receiver byte budget with BLE encoding instead of copying
237 in unrelated modules. Recheck the affected identity methods and lint once.

Next: strict core/native lint, existing native binary integrations, affected GTK
method and format/diff checks. Increment native package revision to3 before a real
release build because the GTK/daemon wire contract changes together. Build/verify
the uninstalled Debian package with the existing explicit private library metadata,
prepare a fresh unsigned APT snapshot and export the exact committed source plus
new package/checksum. Preserve older artifacts and their provenance; do not replace
published version bytes, sign/publish, enroll APT or run user services. Record the
binary source commit separately from later documentation-only export commits.

## N4 and final source checks observed

N4 adds the actual secondary name entry, automatic reset via blank+Save, shared
validation without GTK truncation, plain-text resolved hint, dirty/load/retry and
write-readiness binding. Pending/unavailable name application cannot render Saved
and applied. Existing bounded scrolling/window layout remains. Native run39/0
passed0.54s after restoring the private dependency environment (build41.80s).
The informational-hint selection adjustment was rechecked only through the affected
actual GTK method:1/0 in0.45s. Screenshot inspected; no real clipboard/portal/tray.

Final affected identity6/0 passed after the atomic-read/central-budget correction;
core all-features/all-target strict Clippy passed; native all-target no-deps strict
Clippy passed with the private sysroot. Native-daemon4 plus modern-protocol4 binary
integrations passed. Package Python5/0 in1.076s, core/native nightly format and
whitespace checks passed. These are development observations, no ten-run/final
Mode B or physical acceptance claim. Exact stage timings remain above. All owned
test processes/brokers/displays exited; Terminal closed.

Native package revision is3 so coupled Settings2 GUI/daemon replace native2 through
one package. Next build that release from the committed source, verify offline,
stage unsigned APT and a public canonical signing request for review only. Request
preparation does not approve its digest, sign a release or prove build provenance.

## Final privacy regression found during package review

The in-progress native3 build is an intermediate candidate, not approved for
promotion. Code review found existing apply_live_settings sends a visibility watch
notification even when visibility is unchanged. Therefore a name-only Save while
Temporarily visible could reset mDNS's60-second interval, despite the identity
consumer itself correctly preserving it. Fix the root notification contract:
publish visibility only on an actual value transition. Initial/changed visibility
still notifies; a same-value settings/name save must not extend temporary exposure.

After the owned in-progress build exits, change only that publication and add one
deterministic runtime/watch regression: initial temporary transition notifies,
rename with unchanged visibility does not, a subsequent hidden transition does.
No clock sleeps or radio test is needed to prove the reset trigger is absent.
Recheck the affected regression and core strict lint once, advance the final
package revision to4, and build a distinct corrected artifact. Keep the measured
native3 intermediate provenance; do not silently replace its bytes/request.

The same review also covers an already expired window: saved visibility can still
be Temporarily while the live watch has expired to Hidden. Comparing only the
watch value is insufficient. The settings writer must track the last accepted
visibility configuration separately from that ephemeral live state. Reapply it
only when the saved configuration changes (or has not been accepted). A name-only
save/retry must neither extend an active window nor reopen an expired one.

Use an internal two-part apply receipt: core receive-root/visibility policy accepted,
and identity consumers acknowledged. This avoids losing accepted root/visibility
bookkeeping merely because an independent name consumer failed. The existing
global saved/applied/error projection remains truthful; no Settings2 wire/schema
change. Extend the actual broker test to expire the owned watch, save only a new
name, acknowledge consumers and assert Hidden stays unchanged. No new user choice,
public transient command, timeout, storage field or radio operation is introduced.

The expanded runtime regression passed, then six settings broker cases exposed a
test-fixture lifetime error: moving fixture creation outside the spawned closure
for visibility observation captured only its runtime field; its retained outbound
receiver/discovery owners dropped early, correctly reporting DaemonChanged. Move
the whole fixture into the closure before borrowing runtime. Do not weaken client
identity/liveness assertions or add retries. Recheck the affected seven-case
settings module and strict core lint; unchanged runtime regression need not repeat.

After that fixture fix the seven settings cases and core lint passed. A final
transport guard now reads the authoritative visibility watch for name publication,
not the mDNS loop's possibly stale cached value while an expiry event is pending.
The existing mDNS identity method checks unconsumed Hidden/Visible watch changes.
The failed-consumer broker method additionally renames an already adopted receive
root and changes visibility through Retain: partial name failure must preserve
the live directory capability, not try reopening its obsolete display path.
Recheck those two strengthened methods and affected lint before the final build.

Corrected privacy/policy checks passed: expanded runtime1/0; settings broker7/0
after the fixture capture fix; final strengthened mDNS1/0 and failed-consumer/root
retention1/0, plus strict core all-target lint. No assertions were weakened.
The partial-policy receipt preserves retained receive authority even after its
display path is renamed, and name-only saves cannot reopen expired visibility.

Intermediate native3 release build and offline package verification succeeded
(core optimized100s, native28.72s); unsigned APT staging succeeded. Public request
preparation correctly refused the missing explicit first-release/previous flag
before creating a request. No signature or publication occurred. Keep native3
as superseded local evidence only. Final native4 request preparation explicitly
asserts first-release for operator review; authenticated publisher state must still
confirm the empty baseline, and the request digest is not automatically approved.
