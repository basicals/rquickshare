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
4. [ ] N4: secondary GTK name entry/default reset, typed validation and accurate
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
