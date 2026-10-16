# Named tmpfs source acceptance and memory accounting review

2026-10-16. Mode A, direct dev. Guide: /home/morozhenko/.codex/AGENTS.md,
sections3/3.1/11/12/13. The user selected option1 on2026-10-16; implementation
of that scoped contract change is authorized. Scope is an installed ordinary-file acceptance limitation,
not previews, a new payload type or an unrelated storage refactor.

## Observed failure and root cause

Installed native12 Desktop1 Begin succeeds, but AppendFiles rejects an owned
ordinary read-only named /tmp file with InvalidInput. Abort and observer closure
release staging; no Commit or Send occurs. The same owned descriptor staging
flow on the repository disk filesystem passes with ack1/aborted and zero final
daemon source/handle/byte/transfer resources. Fixtures are removed.

A local kernel probe found GET_SEALS returns SEAL only for the named /tmp file,
but EINVAL on the repository disk filesystem. OpenedFile::adopt classifies every
successful GET_SEALS as memory-source backing and requires all immutable seals.
This broad tmpfs rejection was explicitly frozen in native-image.md, rather
than merely an accidental missing error case. Ordinary files on a tmpfs /tmp
are therefore unsupported even though they are valid user-selected files.

The user confirmed installed tray/card existence and an available Android peer;
no actual Android transfer has yet occurred. Nearby endpoint counts are not
identity/version/content/consent evidence. Installed evidence is in
evidence/native-installed-acceptance-2026-10-16.json.

## Invariants to preserve

- File names never authorize reopening; only the held read-only regular FD does.
- Anonymous seal-capable sources still require WRITE/GROW/SHRINK/SEAL, size1..32MiB
  and the existing128MiB aggregate memory-source reservation.
- Writable/O_PATH/nonregular descriptors remain rejected. No display-name flag
  or caller-declared source kind can exempt an anonymous source from accounting.
- Ordinary filesystem sources have the existing mutable-file/read-at-size model;
  accepting one must not imply an immutable snapshot of its contents.
- Preparation does not send, and Abort/disconnect releases owned descriptors.
- No wire/settings/API shape changes, source deletion or automatic retry.

## Reviewed options (option1 selected)

| Option | Description | Advantages | Disadvantages and risks |
|---|---|---|---|
| 1 — recommended: distinguish named filesystem files from anonymous memory | Fully immutable seal-capable sources keep their existing charge. A read-only regular seal-capable file with actual kernel link count above0 follows ordinary filesystem-source handling. Anonymous seal-capable files still require complete seals and bounds. Read actual size after immutable seal validation. | Supports normal /tmp and tmpfs-mounted user directories without another copy, storage queue or path-based authority. Clear distinction between user-selected filesystem files and application-owned anonymous snapshots. Preserves anonymous-source immutability and admission guards. | Deliberately narrows the128MiB memory-source budget to anonymous/fully sealed sources rather than all tmpfs files. A linked tmpfs source can change or be unlinked by its existing writer; its pinned bytes can remain in RAM until readers release it, just as an ordinary filesystem capability remains owned. This accounting-policy change must be accepted explicitly. Kernel link/seal properties need focused regression coverage. |
| 2 — materialize tmpfs sources into a private disk snapshot | On explicit preparation, copy mutable seal-capable filesystem input into an owned private disk source before ordinary daemon admission; retain strict anonymous memory rules. | Avoids retaining a mutable tmpfs source as an uncharged daemon file; provides a stable snapshot and supports files larger than32MiB subject to disk budgets. | Adds a persistent spool lifecycle, extra full-file read/write, temporary disk consumption, quota and crash-cleanup responsibilities and failure before selection completes. Source can change during capture, so copy consistency still needs a defined contract. More substantial architecture and privacy/storage behavior than the existing descriptor flow. |

Option1 is recommended for ordinary sharing: an explicitly selected filesystem
file should use the same descriptor model regardless of filesystem placement.
The application does not allocate a second copy or introduce a spool owner.
The user selected this recommendation. Named mutable filesystem capabilities
are outside the anonymous-source memory budget even on tmpfs; this does not claim
they consume no RAM. Anonymous/fully sealed sources retain the existing charge.

## Implementation and verification plan after selection

1. Record the selected accounting/source-lifecycle contract and rationale here and
   in native-image.md before code. If option2 is selected, freeze spool budgets,
   capture consistency and cleanup ownership before implementation.
2. For option1, add the minimal kernel-property distinction to OpenedFile::adopt;
   preserve immutable-seal-before-size ordering and all descriptor/type guards.
   No proc-link-name heuristic, source path reopen, source unlink or budget raise.
3. Add focused real-kernel regressions for linked tmpfs files at0/1/32MiB+1,
   exact bytes and zero anonymous charge, plus rejection of mutable unlinked
   seal-capable files. Preserve/extend incomplete-seal and immutable memory-size
   rejection coverage. Tests require a writable Linux tmpfs test directory.
4. Run affected file-source/memory-source checks and strict affected lint once;
   rerun only after a failure/change. Record actual results, not ten repetitions.
5. Commit source/docs atomically, build a distinct native package revision from
   the committed code, verify its inventory/checksum and retain native12 intact.
   The installed user's application is not silently stopped or replaced during
   active work; recheck ownership and active transfers before an approved upgrade.
6. Repeat only the failed installed tmpfs staging check with the fixed candidate;
   then continue explicit current Android code/consent/two-way content acceptance.

Rollback points: review-only documentation commit; later scoped source commit;
separate package/installed checkpoint. At the initial review-only checkpoint no source changes, new package, installation
replacement, transfer, signing or publication occurred. All owned
observer/fixture/probe commands exited: Terminal closed.

## Primary references

Linux [memfd_create](https://man7.org/linux/man-pages/man2/memfd_create.2.html)
describes anonymous backing and optional sealing; [F_GET_SEALS](https://man7.org/linux/man-pages/man2/F_GET_SEALS.2const.html)
describes seal queries and restrictions. These explain sealing; actual named
tmpfs behavior above is a measured host observation. Do not rely on a debugging
memfd name as an authority or claim that these pages establish every kernel link
property; confirm that invariant in implementation review if option1 is selected.

## Selected option1 execution plan — 2026-10-16

- [x] T1: document explicit selection and update native-image.md before code.
- [x] T2: add only the kernel link-count guard after the fully sealed branch;
  retain size inspection after seal validation, read-only/regular checks, limits
  and fail-closed errors. No FD path classification or source reopening.
- [x] T3: verify named tmpfs files at0/1/32MiB+1, clone/read and zero anonymous
  charge; mutable unlinked tmpfs rejection; existing anonymous bounds and missing
  seals including SEAL-only rejection. Run affected source suites/lint once.
- [x] T4: increment package revision to13, commit source/docs, build/verify the
  distinct package without overwriting native12, and record exact source/hash.
- [x] T5: under the user's existing application-replacement/repair authorization,
  upgrade the fixed installed package only after confirming no active transfers,
  preparations or unknown owners. Join only the task-owned idle systemd services,
  keep login start/settings intact, launch normally and repeat the failed tmpfs
  staging/abort check. If user work exists, preserve it and defer replacement.
- [ ] T6: record current installed evidence and refresh mutable handoff documents;
  continue the user-operated Android consent/content check without choosing an
  unidentified peer or sending personal files automatically.

This changes acceptance/accounting policy for linked filesystem sources only.
Actual kernel link count is observed at adoption; later source mutation/unlink can
still cause existing clone/preflight failure, never an immutable-file guarantee.
An already adopted FD continues to refer to the held inode. A fresh adoption of
mutable unlinked tmpfs remains rejected. No API/wire/settings schema is changed.
Rollback is the scoped source commit and, if an installed repair is needed, the
retained native12 package; do not downgrade or interrupt active user work blindly.

Focused verification uses the existing Mode A source suites rather than a new
Mode B coverage project. Writable /dev/shm is required by the tmpfs regressions;
assert seal-capable behavior instead of silently substituting a disk fixture.
No sleeps, real peer network, source mocks or repeated full-product suites.

## T2-T3 actual source verification

OpenedFile::adopt now accepts a not-fully-sealed descriptor as an ordinary
filesystem source only when kernel metadata reports at least one actual link.
The fully immutable branch remains first and its size is read after validation.
Read-only/type/name/size and anonymous limits are unchanged. No source path is
opened/unlinked, no budget constant or IPC schema changed. Package revision13
is reserved for this changed candidate; revision12 bytes remain immutable.

Two real-kernel regression methods were added: named tmpfs0/1/32MiB+1 exact
read/clone/charge behavior and mutable unlinked tmpfs rejection with an existing
FD still readable. The existing anonymous seal/bounds table now includes SEAL
alone and asserts link count0 for every case. The Linux /dev/shm fixture is an
explicit seal-capable test prerequisite, not a disk substitution or silent skip.
Existing suites cover writable/O_PATH/nonregular/source-size-change rejection.

One affected source filter execution passed49/0,606 filtered,0.15s; it includes
file_source, memory_source and prepared_source. Strict core all-feature/all-target
Clippy passed8.07s. Its existing upstream bluer future-incompatibility notice
remains visible. Formatting was applied once; no full-suite or ten-run claim.
Logs: /tmp/rqs-tmpfs-source-tests.log and /tmp/rqs-tmpfs-source-clippy.log.

A separate owned kernel probe confirmed memfd link count0 and a rejected attempt
to link it into the test-owned /dev/shm directory (EXDEV18); all resources removed.
Primary pinned Linux v6.17 sources reviewed: [shmem.c](https://github.com/torvalds/linux/blob/v6.17/mm/shmem.c)
clears link count for shmem_file_setup; [memfd.c](https://github.com/torvalds/linux/blob/v6.17/mm/memfd.c)
uses that setup for ordinary memfd; [namei.c](https://github.com/torvalds/linux/blob/v6.17/fs/namei.c)
rejects cross-filesystem links and unlinked inodes without I_LINKABLE. Runtime
classification uses the held descriptor's metadata, never these diagnostic names.
All owned checks/probes exited: Terminal closed.

## Prepared Android fixture (not sent)

Owned private fixture: /home/morozhenko/Documents/Codex/rquickshare-acceptance/2026-10-16/peer-fixtures/rquickshare-android-check.txt,123 bytes, SHA256
36f53d9c2376e0c6c8f911327ab0a3cd067afa1e24989b6c34fbf1a8f2f3230d.
Its private receipt records sent=false/received=false. This is a plain ASCII
interoperability sample with no personal contents or device identifiers, prepared
without invoking any desktop source preparation or peer Send. Recipient choice,
verification code and remote acceptance remain explicit user actions.

## T4 actual package checkpoint

Binary source:1607c8f95e308a533269b31e6ceb8ce7cf21a505. Native13 release compilation passed: daemon196s,
GUI60s, existing helper1.33s. Initial packaging stopped before .deb creation
because dpkg-shlibdeps could not locate the private Layer Shell dependency
metadata. Extracted the actual shlibs from its already-held1.3.0-1 Debian package
and passed it through the existing --shlibs-file option. No code/test changes or
dependency suppression; cached Cargo invocations did not rebuild unchanged code.
Corrected packaging verified11-file inventory, owner/mode, dependencies, ELF,
checksum, extracted help and isolated settings check. Existing native12 remains
intact; nothing signed/published or APT-enrolled.

Package bytes:6416616; SHA256:3d52d836d5b2a4a4832b1ea00b8964e7b3c6a16d1bb2e19926d94b8c22414d06.
Logs:/tmp/rqs-native13-build.log and /tmp/rqs-native13-build-final.log.
The existing nine nonexperimental-core warnings and libc6 usr-merge diversion
notice remain visible; dependency verification passed with actual metadata.
All owned build commands exited: Terminal closed.

T5 next: under existing repair/replacement authorization, recheck the known
owned native12 service PIDs, zero daemon source/transfer resources, and retained
native12 hash; join only those idle services, use pkexec apt-get install --yes
on this exact native13 local package, reload user units without enablement and
launch normally. Authentication remains solely in the system prompt. Preserve
settings, received files and optional login-start state; on a failed upgrade
report actual package state and restore ordinary launch where it remains valid.

## T5 actual installed repair

The documented existing repair/replacement authorization was used. Immediately
before replacement, native12 SHA256 matched, known task-owned service PIDs matched,
the daemon was synchronized/running with zero preparations/handles/bytes/transfers,
and the real own tray source action was enabled. Only those idle services were
joined through normal systemd stop; no force-kill or ambient process interference.
APT exited0 upgrading native12 to0.11.5+native13, with no additional dependencies,
removals or unrelated upgrades. Authentication was solely the system prompt.
User manager metadata reloaded; normal launch restored one running daemon/client.
All11 installed file bytes/modes/root ownership match the native13 archive.
Desktop login start remains disabled, as before. Settings/received files preserved.

Repeated normal launch retained new daemon PID1909694 and desktop PID1909695,
and exactly one own active KDE tray item. Actual /tmp file AppendFiles now ack1;
Abort returned aborted, resource baseline returned to0, fixture/observer closed.
No Commit/Send was used in that repair regression. Previous native12 failure is
preserved in installed evidence rather than rewritten as a pass. No full visual
or current-Google interoperability claim follows from this check.

## T6 handoff and pending user-operated Android check

The installed CLI prepared only the owned123-byte Android fixture, exit0:
/usr/bin/rquickshare send -- /home/morozhenko/Documents/Codex/rquickshare-acceptance/2026-10-16/peer-fixtures/rquickshare-android-check.txt

One subsequent immutable daemon observation found1 source reservation,2 handles,
123 selected bytes and0 transfers: preparation did not send automatically. The
selection belongs to the existing GTK owner and is intentionally left for the
user's explicit device choice/Send. Do not stop that owner or discard its source
as if it were a leaked test process. It may be discarded in the card if the user
defers; no peer was selected or sent to by the agent.

The user was asked to open Android Quick Share receiving, choose their own phone
in the Linux card, compare codes and explicitly accept; result/Android version
remain pending. Windows availability, exact Android/Quick Share version, two-way
hashes, rejection/cancellation/partial outcomes and remaining visual/lifecycle
checks are unverified. Do not mark physical acceptance complete from this source
fix, CLI preparation or a count of nearby endpoints. T6 physical portion remains
open while source/package/installed tmpfs repair is complete.

No signing, production APT update, key/Drive/account access or immutable native12
artifact modification occurred. Logs:/tmp/rqs-native13-install.log and committed
JSON observations. All owned one-shot commands exited: Terminal closed. Installed
application services and the explicitly prepared user-reviewable source remain.

Handoff verification: 58 affected local links resolve; evidence JSON parses.
Runtime/core/native/packaging/build-script source matches binary commit1607c8f;
the later native README edit is documentation only. Native13 source/package
export will record its own final documentation commit. No runtime suite repetition.

## First Android outbound observation

The user reported receiving the prepared file on Android with matching code.
A private immutable daemon snapshot shows one outbound Finished and zero remaining
source/handle/byte resources. Its outgoing terminal details are Unsupported,
without manifest/byte counters. An initial observer assertion expecting the exact
fixture manifest failed; source tests were not changed or weakened. A focused
read established the actual available fields. Do not claim independently checked
123/123 bytes or a remote hash. No peer name/identifier or PIN was logged.

This is a successful first user-confirmed outbound smoke, not full Android/Windows
parity, independently hashed remote contents, reverse sharing or rejection/
interruption acceptance. Exact Android/Quick Share version remains unprovided.
The outgoing metadata/progress projection limitation is retained for the essential
acceptance queue; do not expand the selected cosmetic slice into a new daemon API.
Native13 source/package/installed tmpfs repair is complete. The next user request
is the explicit eight-point card/tray cleanup, tracked in native-card-cleanup.md.
