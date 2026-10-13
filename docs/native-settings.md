# Daemon-owned native settings and receive-directory authority

2026-10-13. Mode A. User selected 1/A from native-settings-review.md. Guide:
/home/morozhenko/.codex/AGENTS.md. Direct dev commits; no install/live migration/push.
The accepted scope includes transactional native persistence, typed service/client
settings, secondary GTK editor and replacement of process-global receive policy.

## Analysis and staged plan before code

Existing native startup is legacy-read-only; GTK cannot repair saved visibility.
The daemon owns one RQS and serialized coordinator; GUI is a thin authenticated
client. CUSTOM_DOWNLOAD is process-global and looked up independently during
Introduction planning and acceptance. That can redirect a transfer or couple
separate RQS owners. Fix the authority boundary before adding mutable settings.

1. [x] S1: injected per-RQS receive policy, propagated through TCP and BLE admission;
   each request snapshots the chosen directory and pins it before file planning.
   Linux creation uses retained directory descriptors after explicit consent.
   Preserve existing file/folder framing, naming/collisions and non-Linux guards.
2. [ ] S2: freeze native schema/path, transactional private store/backup/recovery
   and scoped fault inventory, then implement the injected single-writer store.
3. [ ] S3: freeze Settings1 XML, bounded read/edit/receipt contracts, idempotency,
   caller/FD validation and nonblocking owned writer/apply lifecycle before API code.
4. [ ] S4: integrate native startup/settings owner and saved/configured/applied/
   restart-required state. Unusable storage must remain explicit, never silent defaults.
   Freeze configuration-only recovery/controlled idle restart details before code.
5. [ ] S5: secondary GTK settings editor with explicit save/import/recovery, actual
   acknowledgements and persistent Full/Compact. No false saved/applied label.
6. [ ] S6: affected checks once, actual failures/corrections, commits and handoff.
   Physical migration/KDE/Google acceptance remains separate; no ten-run dev loop.

Rollback consumers before Settings1/store activation; original Tauri settings stay
intact. S1 is independently reversible and adds no settings write endpoint/schema.
Detailed later-stage contracts/inventories are recorded here before their code;
this approval does not authorize unrelated cleanup, account trust or packaging.

## S1 frozen directory authority contract

ReceivePolicy is an explicitly injected owner-local shared value, never a static.
RQS::new/set_download_path signatures stay compatible; new owners never change
another instance. Policy is propagated to TCP SessionContext and BLE admission.
InboundRequest snapshots a directory choice at request admission; later edits only
affect later requests. A configured relative path retains its process-CWD meaning.
Default directory resolution retains existing Downloads/home fallback behavior.

Pin the snapshot before file Introduction planning, without creating directories
or files. On Linux, retain a CLOEXEC directory descriptor (O_PATH); configured
symlink aliases may resolve once when explicitly pinning the root. Subsequent
planning uses that retained descriptor; filesystem replacement/rename of the root
cannot change its authority. Display paths remain normal user-facing paths, not
/proc descriptor paths, and diagnostics contain no directory/filename values.
Directory pin failure is an explicit transfer failure, never fallback to a different
root. Text/Wi-Fi transfers do not create/open a file destination unnecessarily.

After explicit Accept, create peer subdirectories/files descriptor-relatively:
normal relative components only, no following child symlinks, O_EXCL leaf creation,
no truncation of collisions. Preserve existing umask-derived file/directory modes.
Keep the same pinned root for planning, metadata and creation. No filesystem
creation before consent. Non-Linux retains its checked canonical-root materializer
with the selected path pinned per request; no Linux descriptor claim there.

Admission bounds remain the existing TCP/BLE/session budgets; no background
watcher, timer, unbounded worker or protocol/crypto change. Retain roots until their
request lifetime ends. Settings may later supply an already acquired directory
capability through this seam; only its owning daemon/store will persist values.

## S1 scoped verification inventory before code/tests

| Artifact | Branches / interactions | Level / planned methods | Categories |
|---|---|---|---|
| ReceivePolicy/ReceiveRoot | independent owners, old/new snapshot, defaults, root unavailable, rename/replacement, normal/parent/absolute paths, symlink child, collision keeps original bytes | real temporary-directory unit 2 | Positive,N1,N2,N4,N5,N7,N9,N10,N12 |
| Inbound planning/Accept | no pre-consent creation, exact selected root, policy edit/root rename before Accept, original relative metadata and folder names, post-consent descriptor creation | existing real inbound fixture + 1 new table-driven method | Positive,N4,N5,N7,N9,N10,N12 |
| TCP/BLE propagation | actual injected policy on both entry points; shared clones keep owner identity, existing lifecycle/transport tests | affected manager/inbound/GATT suites | Positive,N5,N7,N9 |

Use actual policy/filesystem/session logic; no domain mocks or sleeps. Important
assertions kill global state, late root lookup, followed symlink, truncation and
pre-consent creation. N3/N6 use existing transport/session contracts; S1 introduces
no time/billing/new persistent schema (N8/N11; N6 migration belongs to S2). N12 is
applicable to real root/open/create failures. No whole-product coverage claim.

## Observations

Plan and S1 authority/inventory recorded before production changes. No user settings
were read/migrated, daemon/tray installed or live peer tests performed. Previous
notification/settings-review checks remain historical, not new S1 observations.

S1 implemented: no CUSTOM_DOWNLOAD static remains. TCP/BLE admissions snapshot
the owner-local policy; file Introduction retains a root descriptor through Accept.
Normal display paths and existing folder protocol remain intact. Real inbound
fixtures now use owned temporary roots instead of the user's Downloads directory.

Actual S1 checks: 26 inbound, 50 manager, 14 GATT and 2 receive methods passed;
strict core all-target Clippy and nightly formatting passed. The unused import
found during development was removed. Existing bluer future-incompatibility notice
remains. Three new methods primarily cover N5 (2) and N7 (1), with the secondary
categories above; these are focused Mode A checks, not whole-product coverage.
All fixture processes exited. Terminal closed. A final fixture lifetime correction
keeps the non-directory file alive during its negative assertion; its affected
method is rechecked below, without repeating unaffected suites.
