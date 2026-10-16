# Native live visibility and quick controls

2026-10-16. Mode A, direct dev. Guide: /home/morozhenko/.codex/AGENTS.md.
User selected option1 in native-completion-audit.md: daemon-owned visibility
facade, existing single settings writer, core-owned expiry. This implements that
selection; no account mode, auto-accept, new duration or radio acknowledgement.

## Analysis and frozen contract before code

Settings2 exposes saved intent, not actual policy after expiry. NativeRuntime
already owns RQS; VisibilityPolicy alone owns its monotonic minute. SettingsOwner
serializes bounded disk operations and live application. GTK/controller/menu
currently have no current-mode view or quick commands. Reuse these owners rather
than letting GTK reconstruct whole settings edits or introducing another timer.

Add authenticated Visibility1 alongside unchanged Manager2/Settings2. Read returns
epoch, availability, core revision, running and current mode. Unavailable/stopped
projects Hidden, never the saved Temporary intent. Changed(epoch, revision) is a
bounded invalidation, not a per-second countdown. No deadline crosses processes;
Full and Compact show the actual mode (Temporary labelled up to one minute).
Tray and card context menu offer Visible, Hidden, Visible for one minute, with
radio selection from actual policy. Existing lock/preview/disconnect/busy gates
apply. No new card-sized settings surface. Controls require a valid settings
snapshot, usable running core and no unresolved settings/visibility mutation.

Set takes epoch, saved base revision/token, request identity and one existing mode.
It returns the unchanged Settings2 receipt shape; actual policy is read separately.
The SDK owns one retained exact pending command, explicit retry and no blind
replay after daemon identity changes. Fresh clicks get fresh random request IDs.
The same base protects a dirty settings editor against quick actions and vice
versa. Conflict refreshes state; it never overwrites another edit automatically.

The single Store performs a visibility-only edit using its current settings;
port/name/card/directory survive unchanged. Its existing private schema2 receipt
stores a domain-separated fingerprint of base and visibility command. No schema
migration or second receipt store. Latest exact replay returns the same durable
revision, changed content under that identity is ReplayConflict, an intervening
save makes the old base Conflict. Disk work remains in the existing owned worker.
Corrupt/unreadable settings require existing Settings recovery, not implicit reset.

Durability precedes live admission, matching Settings2. A fresh durable quick
command explicitly renews Temporary, including same-mode selection or selection
after expiry. Exact replay NEVER reapplies/renews policy. A replay after uncertain
write or missing live admission reports saved-but-unapplied instead of claiming
success; the user can review current state and make a fresh explicit command.
The writer remembers only its last successfully admitted quick identity, bounded
in process. Crash/restart uses the existing saved startup semantics: saved
Temporary establishes a fresh minute. No persisted deadline or process-crossing
command replay. Save failure does not enable discovery; Indeterminate retains
SDK pending state. Existing full-settings saves remain idempotent and unrelated
edits/replays do not reopen or renew expired Temporary. Ordinary expiry is not a
persistence error; current Hidden and saved Temporary can coexist normally.

Quick apply only touches core visibility, not name or root, and does not await
unrelated advertising name acknowledgements. Actual policy changes/expiry wake
the serial bus and invalidate clients. Read views remain coherent snapshots;
policy running is distinct from daemon process/transport acknowledgement. Existing
transfers/consent are untouched. Physical withdrawal and suspend remain acceptance
items. No new logs contain settings tokens, names or payloads.

## Stages, risks and rollback

1. [x] L1: inspect owners and replay/lifecycle paths; freeze this selected contract.
2. [x] L2: visibility-only durable operation, coherent read seam, bounded daemon
   facade/application/change signals; preserve old settings signatures and behavior.
3. [x] L3: exact-retry SDK/controller and thin GTK/card/tray projections/gates.
4. [x] L4: affected checks once, record observed outcomes and corrections here,
   atomic source commits and continuation index update. No unchanged package build.
5. [x] L5: after source checks, prepare a successor native candidate/handoff if
   appropriate. Native8 remains last measured package until a replacement is built.

Risks: same-mode renewal versus replay, disk/live failure, competing editor,
expiry/reconnect invalidation, stale UI action and settings queue shutdown. Revert
client/UI before facade/store extension; existing saved schema and old full-save
fingerprints remain readable. Preserve native8 immutable artifacts. No installation,
signing/publication, Drive or user authorization/configuration changes.

## Focused development verification inventory (before tests)

| Artifact | Branches / interactions | Level / planned new methods | Categories |
|---|---|---|---|
| Visibility-only Store operation | retain all other fields; all modes; exact replay and altered replay; stale base after full save; malformed IDs/modes; corrupt state; write/rename/sync failure | unit, real store and external IO adapter / 3 | Positive,N1,N2,N4,N5,N6,N7,N12 |
| Coherent current view | unavailable/stopped Hidden; running three modes/revision; malformed epoch/mode/running combinations; privacy Debug | unit / 1 | Positive,N1,N2,N7,N10 |
| Daemon/SDK | real private broker; fresh same-mode renew versus replay after expiry; ordinary settings save retains expiry; current-state invalidation; CAS conflict; exact pending retry; invalid types/FD/epoch; unavailable and foreign UID | integration / 3 | Positive,N1,N3,N4,N5,N7,N9,N10 |
| GTK/tray projection | all modes selected; unavailable/preview/locked/disconnected/pending disabled; new menu actions bounded/authenticated; Full/Compact actual-mode label; retry route | extend existing menu/GTK smoke | Positive,N2,N4,N5,N7,N9,N10 |

Use real policy with paused Tokio time for expiry, real temporary store/broker,
existing fault IO and reply-interruption seams. No workstation radios or sleeps.
Existing core timer/store/controller suites protect unchanged lifecycle and storage
branches. Assertions must catch replay renewal, overwritten unrelated settings,
wrong current/saved mode, lost invalidation and enabled unsafe controls. N8 uses
existing monotonic-clock coverage (no new clock); N11 inapplicable. This is Mode A
focused verification, not a whole-product Mode B coverage/stability claim.

## Actual observations

L1 is source/document inspection only. No new tests have run yet. All owned shell
invocations have exited (Terminal closed). Native8 remains the immutable measured
package; no installed desktop or peer compatibility evidence is added here.

L2/L3 implementation inspection found a stop/admission race: checking running in
bus dispatch alone cannot protect a queued command if the core owner stops during
disk work. Before final code, require running inside the same policy write lock as
quick admission. Keep existing public pre-start set behavior unchanged; use a
private running-only seam and add its stopped rejection to the view regression.
The controller expiry integration will also wait for the actual GTK-independent
PanelState update after a policy-only signal, not infer notification from a read.


## L2 measured source verification

Seven new Rust test methods (three Store, one read-view, three private-broker/SDK
scenarios), plus extended existing GTK/menu smoke assertions. Primary categories:
N1=1, N4=2, N7=2, N12=2; positive/privacy/concurrency interactions overlap rather
than add methods. The existing full core library inventory is633 (626 previous).

Initial compilation caught a borrowed temporary in the cancellable signal branch;
fixed with an owned event binding before tests ran. First core execution:632/1,
10.09s. The new minute simulation exceeded its outer30-second test watchdog;
changed only that scenario's watchdog to180 simulated seconds (it now deliberately
advances two minutes), not production timers or assertions. Affected method then
passed1/0 in0.16s. After adding atomic running-only admission and controller-only
expiry invalidation, final affected visibility group passed18/0 in0.47s (615
filtered). Whole633 was not repeated or claimed as a final full green run.
Core all-features/all-target strict Clippy passed16.58s. Its existing bluer future
compatibility warning remains visible. Old public settings signatures/schema2 and
full-save fingerprints are preserved. Policy-only signals drive controller
updates; no GUI polling clock or transport-success promise was introduced.

Initial GTK binary run:38/1 in0.84s. The remaining menu fixture still expected18
group rows after adding four actions. Corrected to ROWS.len()+1 (including root),
then that affected method passed1/0 in0.05s,38 filtered. The existing GTK smoke
passed all newly extended mode/radio/label/pending/disconnect/lock gates. Final
native all-target no-deps strict Clippy passed10.70s. Nine existing core warnings
without experimental features and Xvfb EGL/DRI3 warnings were emitted, not hidden.
Formatting/diff checks passed. No ten repetitions, workstation radios, installed
services, cloud access, signing or publication. Terminal closed after each check.


L3/L4 source complete: Full/Compact display current discovery policy independently
of transfer status, GTK/context and DBusMenu share actual mode radio selection,
three explicit modes and an exact pending-command retry route. Existing recovery
button also routes a retained visibility command to that retry. Busy, pending
settings/visibility, stopped, preview, locked and disconnected gates stay inert.
Uncertain/save-but-unapplied outcomes are visible without claiming discovery
success. No persistent-history/localization/account/remote-preview feature is
claimed. Prepare changed native9 from the next source commit; native8 remains
last measured candidate until build/verification completes. No package overwrite.


## L5 native9 candidate verification

Built from b1ce2e21ea9e5d2f437d65206882e48d166e3491 (core implementation
15dbf08 plus native UI/package revision9). Core release177s, GTK55.15s,
helper0.94s. Package rquickshare_0.11.5+native9_amd64.deb:6366716 bytes,
SHA256990251e7e89d483d1a282a782e6682ceb1a779e93a6fcacf83b8abfdc1c18b94.
Exact11-file inventory, root ownership/modes/assets, generated ELF dependencies,
extracted GUI/daemon/helper help and private offline settings smoke passed.
Ubuntu26.04 amd64 only; no installed desktop, ARM or physical-peer claim.
The existing nine core and usrmerge dpkg-shlibdeps warnings remained visible.

Extracted Full/Compact incoming fixtures rendered and were visually inspected.
Both show the new discovery row without clipping; Full shows the existing synthetic
thumbnail, Compact retains PIN/details/actions without it. These are offline
preview-mode images (Discovery unavailable), not live discovery or peer preview
proof. Live three-mode labels, radio state and safety gates passed GTK smoke.

Unsigned APT native9 has7 exact files, no InRelease/Release.gpg; Date
2026-10-16T12:08:22Z, Valid-Until2026-10-23T12:08:22Z. Public first-release request
native_desktop/target/signing-request-native9/request.json SHA256
9d0732164d550ac4d1ccf4c6ab1b4ad960eff5bdd22fef8f20cc3425f7af2ab7 is
unapproved, without independent source attestation/live empty-baseline proof.
No signing/publication/installation. Native8 package hash was checked unchanged.
[Machine evidence](evidence/native-live-visibility-2026-10-16.json) records exact
package/request/images and actual source checks. Terminal closed.

Finish L5 by exporting the next documentation commit as an immutable source ZIP,
exact native9 package/checksum, unsigned7-file APT archive and public request to
existing outputs. Verify CRC, tracked inventory, selected exact source bytes and
all package/request/APT digests. Update mutable current handoff docs/readiness,
preserve native8 and all older named artifacts. Record export facts in a later
documentation-only commit; do not rebuild or mutate that immutable source archive
just to include its own later completion receipt.


L5 completed. Exported 466 tracked files from
ba314b6e759b8ef44a32b84907da4b1e89e87b81 to outputs/rquickshare-native9-source.zip;
SHA256cdd08ec8db17d09386db2036e5b62fad0bb85a526099ba780635752813ef4130.
CRC/inventory/selected exact source bytes, package/request digests and every one
of the7 unsigned APT entries passed. Unsigned APT ZIP SHA256
533d16d47cd042d4c29713e8758f47f0cdb545be01cdad7072841f8e2c14497f.
The generic source ZIP was refreshed after confirming the prior native8 copy is
preserved. Exact package/checksum, public unapproved request, receipts and current
handoff docs/readiness are in the existing workspace outputs. Remote observations
remain dated2026-10-14, not rechecked. Backup follow-up remains deleted.
This final documentation-only completion record postdates the immutable source
archive; binary source remains b1ce2e2. No rebuild, install, signing or publication.
All owned commands/processes exited; Terminal closed.

Next: installed KDE/Wayland/current native9 physical manual-sharing acceptance
when the desktop/peers are available, and canonical preview wire/mapping review
before receive IPC/production card binding. Resume/dedup, advanced payloads,
account/trust, localization/history/accessibility and signed distribution remain
open as listed in native-completion-audit.md. No full Google parity/UI signoff.
