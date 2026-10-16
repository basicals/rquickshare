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

## Concrete options requiring user selection

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
separate package/installed checkpoint. No source changes, new package, installation
replacement, transfer, signing or publication occurs in this review. All owned
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
- [ ] T4: increment package revision to13, commit source/docs, build/verify the
  distinct package without overwriting native12, and record exact source/hash.
- [ ] T5: under the user's existing application-replacement/repair authorization,
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
