# Encrypted recovery backup in the user's existing Google Drive

2026-10-15. Mode A. Direct dev. User selected existing Google Drive ArcVault,
provided /home/morozhenko/morozhenko/secrets/. This supersedes proposed R2 backup;
no extra machine, token, bucket or cloud account. Existing public release identity
and same-PC custody remain unchanged. Read native-local-signing.md,
native-signing-recovery-review.md, local signing code and the agent guide.
Google Drive skill read; no document-format skill applies to encrypted binary data.

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

1. [ ] B1: implement scripts/native_signer_backup.py with bounded fixed recovery
   inventory/manifest, authenticated GPG symmetric encryption through system
   Pinentry, safe fresh-only restore and real temporary restoration checks.
2. [ ] B2: focused pure archive/path/tamper/refusal tests plus real protected GPG
   encryption/decryption/restore/non-release signature/revocation integration.
   No domain mocks; test-only Pinentry injection, no ambient GPG or cloud fixture.
3. [ ] B3: create actual encrypted backup on private local disk; verify decryption,
   restoration, exact primary/subkey protection and non-release signature. This
   requires archive-password entry and original key-password entry in Pinentry.
4. [ ] B4: copy ciphertext/receipt/checksum only to the selected ArcVault. Verify
   direct remote metadata and fresh downloaded ciphertext SHA256/size; record
   quota/blocking outcomes honestly. Never claim cloud success from mount cache.
5. [ ] B5: SSOT/atomic commits, operator restore instructions and source exports.

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
