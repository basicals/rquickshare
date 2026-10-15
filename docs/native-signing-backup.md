# Encrypted recovery backup in the user's existing Google Drive

2026-10-15. Mode A. Direct dev. User selected existing Google Drive ArcVault,
provided /home/morozhenko/morozhenko/secrets/. This supersedes proposed R2 backup;
no extra machine, token, bucket or cloud account. Existing public release identity
and same-PC custody remain unchanged. Read native-local-signing.md,
native-signing-recovery-review.md, local signing code and the agent guide.
Google Drive skill read; no document-format skill applies to encrypted binary data.

Current checkpoint: the encrypted recovery archive itself was downloaded directly
from Drive on 2026-10-15T11:22:40Z with exact20559 bytes/SHA256, matching the already
restored local ciphertext. Three public sidecars still await independent readback;
the first sidecar attempt returned rclone exit3 without a classified HTTP/provider
reason. Full four-file acceptance remains open; do not redownload verified archive.

## Destination observations and ownership

Exact local folder exists: /home/morozhenko/morozhenko/secrets/ArcVault, canonical,
caller-owned0700, on existing fuse.rclone mount /home/morozhenko/morozhenko whose
observed source is morozhenko:. Derived remote target is
morozhenko:secrets/ArcVault. Only scoped recovery filenames may be written there.
Do not read unrelated secret files, alter mount/config/auth/sharing or create a new
ArcVault folder. Drive connector folder searches ArcVault/vault returned no items;
use the user-provided existing mount/remote, not guessed connector folder IDs.
Direct rclone metadata preflight currently reports quota/rate errors; raw backend
errors are suppressed. Mounted-file presence is not proof of completed cloud sync.

## Plan before code

1. [x] B1: implement scripts/native_signer_backup.py with bounded fixed recovery
   inventory/manifest, authenticated GPG symmetric encryption through system
   Pinentry, safe fresh-only restore and real temporary restoration checks.
2. [x] B2: focused pure archive/path/tamper/refusal tests plus real protected GPG
   encryption/decryption/restore/non-release signature/revocation integration.
   No domain mocks; test-only Pinentry injection, no ambient GPG or cloud fixture.
3. [x] B3: create actual encrypted backup on private local disk; verify decryption,
   restoration, exact primary/subkey protection and non-release signature. This
   requires archive-password entry and original key-password entry in Pinentry.
4. [ ] B4: copy ciphertext/receipt/checksum only to the selected ArcVault. Verify
   direct remote metadata and fresh downloaded ciphertext SHA256/size; record
   quota/blocking outcomes honestly. Never claim cloud success from mount cache.
5. [x] B5: SSOT/atomic commits, operator restore instructions and source exports.

Rollback code/receipt commits independently. Preserve active signing profiles.
Unsent staging can be removed explicitly; never silently delete active keys or
replace a partially restored identity. Cloud backup removal must be explicit and
scoped to the exact new file; no sync/delete/broad cleanup or retention changes.

## Frozen recovery format and invariants

Copy only identity.json, archive-keyring.gpg, the three protected private key files
from authority/signer matching validated public keygrips, and the expected primary's
automatically generated revocation certificate. No sockets, trust database, logs,
user configuration, unrelated GPG homes or extra files. Do not archive/restore an
arbitrary pinentry-program config; generate the known safe agent policy instead.
Source homes must be idle; refuse an existing agent rather than stopping it.

Plain recovery payload is a bounded uncompressed deterministic USTAR in memory:
six fixed source files plus canonical manifest binding names/sizes/SHA256 and the
expected primary. Maximum4MiB archive, maximum1MiB each member; regular0600 files
only, bounded count, no absolute/traversal/duplicate/PAX/link/device/directory
members. Parse into bounded memory; never use extractall. Validate exact inventory,
manifest digests, identity and public key before constructing any private profile.

GPG symmetric AES256, SHA256 S2K, maximum supported iterated S2K count65011712,
no compression, no embedded filename, no symmetric passphrase cache. Use a fresh
private0700 crypto home with the existing enforced16-character Pinentry policy;
no password args/env/loopback, no password copied to cloud/receipt/Git. Bound
ciphertext and GPG decrypted output. Check GPG success before accepting plaintext;
no ignored MDC/integrity errors. Plain recovery payload never reaches Google Drive.
Only ciphertext and non-secret checksum/receipt/instructions may be published.

Restore requires independently supplied primary fingerprint and a fresh canonical
caller-owned0700 destination under a safe parent. Use validated names, exclusive
private file creation and generated agent config. Import the exact public keyring,
compare identity/keygrips and actual KEYINFO protected disk key sets. Sign a
non-release challenge with the restored signing subkey and verify exact primary/
subkey/SHA256 with gpgv. In another disposable public-only profile, import a copy
of the recovery revocation certificate and verify it revokes exactly this primary;
never import revocation into active profiles. Agents belong exclusively to newly
created homes and are closed in finally. Failure preserves partial restore for
review but removes its readiness manifest; no overwrite or automatic key rotation.

Backup creation decrypts and verifies its own ciphertext before committing a fresh
private output directory. Ciphertext is content-addressed; sidecar is integrity,
not independent authentication. Read-only cloud publication must use bounded exact
paths and readback, no cached-only success or indefinite retry/polling. Quota errors
may delay cloud acceptance without invalidating a verified local backup.

## Scoped verification inventory before tests

Fixed inventory and source-idle guards; protected source layout; canonical manifest,
member limits/counts/digests; unknown/traversal/link/duplicate/malformed/oversized TAR;
wrong primary, corrupt/missing metadata and changed file; safe fresh restore/output
paths; exact restored roles/protection/public identity; ciphertext tamper/wrong
password/cancel; real restored subkey signature and revocation usability; owned
agent/temp cleanup and no active-key mutation. Table-driven where rules share
branches. One appropriate development run; rerun affected failures/changes only.
Existing native CI discovers test_native_*.py with GPG/Python prerequisites already
present. No desktop/Worker/Rust rebuild, unchanged suite or real Drive test upload.

## Primary references

https://gnupg.org/documentation/manuals/gnupg/GPG-Esoteric-Options.html
https://rclone.org/drive/
https://rclone.org/commands/rclone_copyto/
Installed rclone1.75.1 lsjson help and current GPG options are checked. Google Drive
backend supports cloud metadata/content readback; FUSE cache is a separate layer.

## B1/B2 implementation and measured checks

Implemented create/verify/restore operator CLI, fixed six-file inventory plus
canonical manifest, private atomic no-replace output, bounded in-memory TAR/GPG,
restored protected-key proof and disposable revocation test. Crypto/restoration
staging always lives beside the private source custody, never beside a potentially
cloud-mounted final output. Ciphertext budget includes64KiB encryption overhead.

Initial module discovery accidentally included an imported TestCase class:5 prior
local-custody methods passed again and4 new backup methods passed; one new actual
backup scenario failed an incorrect byte-equality expectation after unlocking.
Total10 methods18.570s. Fixed import to module alias; loader now discovers exactly
5 backup methods, so future native CI does not duplicate those prior methods.
GPG legitimately rewraps its own protected private-file representation on first
use. Restore now enforces exact bytes before GPG use, the exact real signature and
KEYINFO protection after use; all other recovered files remain byte-identical.
This replaces an invalid file-format assumption with stronger material/protection
checks, not a relaxed identity assertion. Source profile bytes remain unchanged.

Affected actual encrypt/decrypt/restore/tamper/wrong-password/wrong-primary/signature/
revocation/cleanup scenario passed1/0 in13.749s. After fixing local-only secret
staging, that affected scenario passed1/0 in12.668s. Expanded table-driven unsafe
member/size/schema cases passed1/0 in4.999s. Other4 initially passing backup methods
were not repeated. Syntax/CLI help/discovery-count/whitespace passed. No production
key, live GPG profile, cloud object, mount or account config was changed by tests.

Canonical TAR checks reject traversal/absolute/link/directory/FIFO/PAX/duplicates,
malformed/truncated/extra bytes and changed digests; schema rejects bool/float/
string aliases. Cancellation/oversize creates no ready backup. Concurrent output
creation preserves foreign contents. Existing source agent is neither reused nor
stopped. Development tests use owned TEST ONLY Pinentry and close their agents.

## Production operation prepared

Private local output (fresh only):
~/.rquickshare-signing/recovery-drive-2026-10-15. Run create with system Pinentry;
choose/retain an archive password, re-enter it to verify decryption, then enter the
original key password to prove the restored signing subkey works. Passwords are
not tool arguments/output. Initial production fingerprint remains
6D8854762CF0C86881138A56695A0888D09FA616. Expected signing subkey remains
1F5150D98D94C43109FBA2503E7DE744C1014357.

Successful output contains only content-addressed ciphertext, SHA256 sidecar,
public receipt and RESTORE.md. Copy those four exact artifacts to the already
verified ArcVault folder, without replacing existing files. For cloud acceptance,
read metadata and download fresh bytes directly through the existing rclone remote,
compare exact SHA256/size; no remote plaintext or new share permission. Keep the
verified local ciphertext if remote quota prevents completed upload/readback.

## B3 actual local recovery verification / B4 preflight

Actual encrypted backup created successfully with user system Pinentry. Primary
6D8854762CF0C86881138A56695A0888D09FA616 and signing subkey
1F5150D98D94C43109FBA2503E7DE744C1014357 match the existing release identity.
Protected profiles, non-release signature and revocation verification all passed.
Ciphertext20559 bytes, SHA256
a9de8d32972119de7ec747849c76c10e292543d0b984bb28ba0b948f0220f9bf.
Verified local output: ~/.rquickshare-signing/recovery-drive-2026-10-15.
All crypto/recovery profiles were task-owned and removed/closed; active custody
was not overwritten or revoked. No release was signed/published.

Direct rclone folder metadata is still unavailable: HTTP403/rateLimitExceeded,
not a confirmed storage-capacity or authentication failure. Existing mount remains
available. Do not repeat this failed preflight in a tight loop or change auth/config.
Ground the folder once through exact-name connector metadata if possible; otherwise
use the already observed user-provided mount. Copy only four ciphertext/public
artifacts with exclusive no-overwrite creation. Namespace sidecar/receipt/RESTORE
names using the ciphertext stem, so generic names cannot collide with unrelated
ArcVault data. Attempt one bounded fresh remote readback after writing; if quota
continues, record mounted copy/cloud-unverified, not durable cloud success.

## Actual mounted copy and remaining cloud acceptance

Four exact encrypted/public artifacts were created exclusively under the existing
ArcVault path, with content-addressed project-prefixed names; no other files were
replaced/read or sharing/config/mount changed. flush/fsync and readback from the
mount succeeded for all four sizes/SHA256. These observations may use the existing
VFS cache and are explicitly not cloud persistence acceptance. Independent rclone
cat after copying still failed HTTP403/rateLimitExceeded on the first artifact;
no repeated read attempts for the other three files.

Public result: docs/evidence/native-signing-backup-2026-10-15.json. The encrypted
backup itself/private profiles/passwords never enter Git or source/chat exports.
The backup's receipt.json describes local verification and correctly retains
cloud_copy_verified=false; separate cloud evidence must establish persistence.

Drive connector exact-name folder search also returned no items. A browser fallback
previously reached Google sign-in; the user declined browser access. No credentials
were read/typed by the agent, no new OAuth permissions/sharing were granted. A
subsequent attempt to close the task-created login tab reported that it no longer
belonged to the browser session; closure was not confirmed and no other tabs were
touched. Do not request browser access again. Do not re-encrypt, repeat key
generation or overwrite existing files just to retry cloud readback.

B1/B2/B3 complete, B4 mounted copy complete but independent cloud acceptance
pending quota relief. B5 current source/evidence/operator
handoff complete. No production APT release signature/publication, trust enrollment
or native installed/Google peer claim. Existing distribution route remains intact.

## Quota recovery readback plan (2026-10-15)

The user declined browser access, then explicitly requested waiting for quota
recovery and independently checking the saved backup. Browser sign-in is no longer
an available fallback. Use only the existing rclone remote, read-only, scoped to
the four filenames already recorded in the public evidence. No auth/config/mount
changes, unrelated secret reads, uploads, replacement or deletion.

1. Attempt one bounded sequential direct readback after the elapsed overnight
   interval. Disable application/low-level retries, limit request rate, cap each
   output at expected size plus one byte, and compare exact size/SHA256 against
   the previously recorded local-copy evidence. Never print payloads/raw errors.
2. Stop immediately on quota failure or mismatch. If quota still blocks access,
   schedule a thread follow-up no more often than hourly; do not poll in a loop.
   Retry only unverified artifacts and stay quiet on unchanged quota failures.
3. Record actual results and timestamps in public evidence. Only mark independent
   cloud acceptance complete after all four files match. Existing local restore
   and revocation verification remain valid; downloading identical ciphertext
   needs no repeated Pinentry/unlock or another restore run.
4. Stop the follow-up after successful verification, notify the user, and commit
   the result on dev. Changes are limited to documentation/evidence; no production
   code or architecture changes. Rollback the documentation commit independently;
   preserve all existing key custody and backup files regardless of outcome.

### Actual retry and scheduled continuation

2026-10-15T06:23:15Z: one direct bounded rclone cat attempt on the ciphertext
returned HTTP403/rateLimitExceeded in under one second. Zero cloud artifacts
verified; no requests for the remaining three files, no local backup changes.
This does not establish the cause or reset time of the provider restriction.

Created active hourly thread heartbeat rquickshare-arcvault using the app scheduler.
It retries only pending exact artifacts through the existing remote, stops at the
first failure, stays quiet on unchanged quota failures, reports missing/mismatched
data or successful acceptance, and removes itself after all four matches. No
background terminal, busy loop, browser auth or unattended signing is involved.

### Local mount diagnostics plan (2026-10-15)

The user questioned whether local creation followed by moving would solve the
quota failure. Local creation and exclusive mounted copying already happened.
Mode A: inspect only existing mount cache settings and the four exact VFS metadata
entries, compare the four local output artifacts with recorded evidence, and
inspect scoped existing process logs if necessary. No remote calls, duplicate
upload, moves, cache/config changes or interference with the user's mount process.
The mount uses full VFS caching and a five-second write-back delay. Determine
whether these specific uploads remain dirty/pending before suggesting any action;
cached readability alone cannot establish cloud persistence. Record observations
and retain all original files. Documentation-only rollback; no production changes.

### Local mount diagnostic results

All four original artifacts under ~/.rquickshare-signing/recovery-drive-2026-10-15
still match recorded size/SHA256. Their exact VFS cache contents match too. All
four existing metadata entries have Dirty=false and nonempty remote fingerprints;
each fingerprint's MD5 matches its local original. This is consistent with
completed mount writeback, not four pending uploads. It is stronger evidence than
directory visibility, but remains a cached observation, not fresh independent
cloud download acceptance. Public diagnostic values are recorded in the evidence.

The mount and direct CLI use the same canonical existing config path and executable;
no mount RCLONE_* environment or --drive-* backend overrides were observed. No
different-config explanation was found. No backend requests, original/cache writes,
mount restart or auth/config changes occurred during diagnostics. The existing
hourly independent readback remains pending. Re-copying/moving would repeat the
already completed local-create/mounted-copy workflow without addressing the read
failure; preserve the original local recovery copy and existing destination files.

Interpretation checked against upstream rclone1.75.1 vfscache/item.go: _dirty marks
changed data needing writeback; successful _store clears Dirty. Upstream mount
documentation explains full read/write disk buffering and remote size/modtime/hash
fingerprints. MD5 here only compares the existing remote fingerprint; SHA256 is
the recorded recovery integrity check. No authentication claim relies on MD5.

References:
https://rclone.org/commands/rclone_mount/#vfs-file-caching
https://raw.githubusercontent.com/rclone/rclone/v1.75.1/vfs/vfscache/item.go

### First successful independent archive readback

2026-10-15T11:22:40Z scheduled attempt used direct rclone cat against the exact
ciphertext path, bypassing the mount cache. Download succeeded; exact20559 bytes
and SHA256 a9de8d32972119de7ec747849c76c10e292543d0b984bb28ba0b948f0220f9bf
match the locally created/restored archive. This independently establishes that
the encrypted recovery payload reached Drive and could be downloaded intact.
No new unlock/restore is needed for byte-identical ciphertext already verified.

The subsequent .tar.gpg.sha256 sidecar read failed with exit3, with no classified
HTTP/provider reason in the sanitized result. Do not infer quota relief for all
requests or claim a confirmed missing/corrupt file from that result alone. Stop
this run at that failure; receipt/instructions were not requested. Preserve all
local/cloud originals, record the verified archive and failure, and continue only
the three unverified sidecars on later hourly runs. No uploads or replacements.
The user was notified about the successful payload verification and new error.
