# Native release signing on the existing Linux workstation

Current production checkpoint: same-PC keys have been initialized through user
Pinentry and verified by an actual non-release subkey signature/gpgv check.
Primary6D8854762CF0C86881138A56695A0888D09FA616. Both dedicated agents are closed.
Default private root is ~/.rquickshare-signing. Encrypted recovery was created,
restored/verified and copied to the selected existing Google Drive ArcVault mount;
independent cloud ciphertext readback passed; three public sidecars remain pending
(native-signing-backup.md). Release approval/
publication remain open; earlier pending-initialization notes are history.

2026-10-14. Mode A. Direct dev. User explicitly rejected additional machines/VMs/
hardware and selected the existing Linux PC. This supersedes the separate-host
requirement in native-signer-custody-review.md and earlier R2 custody notes.
Guide /home/morozhenko/.codex/AGENTS.md read. No renewed host-choice question.

## Accepted boundary and limitations

A dedicated private state directory contains two GPG homes: authority (certification
primary plus signing subkey) and signer (usable signing subkey only). Build and
publisher commands do not receive these homes. This is local role/credential
separation on one OS/user, not an independent host or protection from compromise
of the workstation/account. The locally stored encrypted authority is not called
an offline primary; a recovery copy on separately chosen storage is still needed
before production enrollment. No new hardware/service/account is required by this
implementation. Do not make an unattended signing service or put private keys in CI.

Use a certification-only Ed25519 primary (five-year validity), one Ed25519 signing
subkey (one-year validity), and a public organization-only UID. Rotation/extension
requires explicit operator action; no automatic trust replacement. Preserve the
existing single-primary verifier and canonical request approval. Neither key profile
nor a source SHA alone attests the build's provenance. The existing seven-day signed
metadata lifetime and its refresh ownership remain production acceptance gates.

## Milestones before code

1. [x] L1: document local custody and implement a small operator CLI in
   scripts/native_signer_local.py: initialize a fresh explicit private state,
   export only the public keyring/identity, inspect profile roles/protection.
2. [x] L2: focused deterministic parsing/path/refusal tests and one real GPG
   protected-key/subkey-only-profile integration with owned test-only Pinentry.
   Verify real approved snapshot signing through the existing wrapper; no domain
   logic mocks, global GPG state or unchanged whole-suite repetitions.
3. [x] L3: record measured checks, local one-command operator handoff/recovery,
   CI discovery compatibility, atomic commits and exact refreshed source export.

## Frozen initialization contract

No ambient ~/.gnupg, environment secret, secret CLI option, loopback or generated
password. Production initialize uses normal system Pinentry for the user's
passphrase. No production key initialization during this development turn: the
operator must enter their own password locally. Prepare/test all commands first.
Default CLI state is ~/.rquickshare-signing; allow an explicit absolute
canonical state path, never an existing directory, symlink or repository child.
Parent must exist, be owned by the caller and not group/world writable. Root and
GPG homes0700, state/config/temporary secret files0600, no overwrite/resume magic.

Fresh agent config enforces a16-character passphrase, disables loopback and
external password caching, and bounds ordinary agent cache to60s/default300s/max.
These are local key-cache policy, not scheduling sleeps. Refuse unprotected keys
using agent KEYINFO protection status, not assumptions about Pinentry dialogs.

Generate certification-only primary, check exact public fingerprint/keygrip/roles,
add one signing-only subkey, export public keyring, export protected subkeys to a
private temporary file and import into signer. Remove temporary export in finally.
Validate KEYINFO: authority has exactly the two protected local disk keys, signer
has exactly the protected signing subkey and no usable primary. Validate public
keyring identity and write public metadata only after successful checks. Kill only
agents started in this command's exclusively new/owned GPG homes before returning;
never stop ambient/user/other-agent GPG processes. Inspection must not start/kill
an agent or print private paths/key material/arbitrary tool output.

On failure retain the newly created private directory for recovery rather than
silently deleting generated identity material. No ready manifest on partial
initialization; refuse existing output on retry. Operator can inspect/recover or
explicitly discard the failed profile after review. Do not start another key
identity automatically. No directory rename while GPG agents own socket paths.

## Test inventory before tests

Public key listing parser: exact cert-only primary + one sign-only subkey, public
fingerprints/keygrips/algorithm/expiry; missing/duplicate/malformed/extra roles fail.
KEYINFO parser: exact protected disk key sets; clear/unknown/token/unexpected keys,
invalid framing and missing successful completion fail. Path policy: existing/
symlink/relative/repository/noncanonical/insecure parent and unsafe ownership fail.
Public state/keyring consistency, permission validation and no-autostart inspection.
Actual GPG initialization with deterministic TEST ONLY Pinentry passphrase, protected
KEYINFO, no usable primary in signer, existing request+sign+gpgv snapshot acceptance,
owned agent cleanup and temporary-export cleanup. No production keys or fixtures
uploaded. Fixture Pinentry is an explicit test dependency, not a production CLI
password bypass. Failures/cancellation preserve private partial state with no
success manifest. Focused run once; rerun affected failures only.

## Primary references inspected

https://gnupg.org/documentation/manuals/gnupg/OpenPGP-Key-Management.html
https://www.gnupg.org/documentation/manuals/gnupg/Agent-Options.html
https://gnupg.org/documentation/manuals/gnupg/Unattended-GPG-key-generation.html
https://lists.gnupg.org/pipermail/gnupg-devel/2015-January/029303.html
Installed GnuPG2.4.8/options and an owned temporary agent's KEYINFO help are checked
before relying on current command/protocol details. Use current official primary
sources and local protocol help, not third-party recipes.

## Measured verification and local operator handoff

L1/L2 implemented. Installed GnuPG2.4.8 supports the selected options. An owned
agent's live KEYINFO help confirms D/local-disk and P/passphrase protection.
Initial focused5-method run: three domain/path tests passed; two cleanup assertions
incorrectly assumed gpg-connect-agent --no-autostart returns nonzero without an
agent. Actual isolated protocol observation: return0, empty stdout, no-agent stderr.
Corrected the test to assert the actual Unix agent socket rejects a connection;
this verifies process cleanup directly, without weakening the custody invariant.
Affected actual protected-key/request/sign/gpgv scenario then passed1/0 in9.325s;
affected cancellation scenario passed1/0 in0.730s. No unchanged methods repeated.
Five methods total cover public identity/roles/expiry; clear/missing/extra key
protection; path/permissions/overwrite guards; actual protected custody and real
subkey release signature; cancellation and retained private partial state.
Initial5-method duration7.162s includes the two incorrect test assertions.

Actual GPG creates protected cert-only primary and sign-only subkey, imports a
subkey-only working profile, and the existing canonical-request wrapper signs a
fixture APT snapshot. gpgv VALIDSIG identifies that exact subkey and expected
primary. Both owned agent sockets are closed after initialization, and again after
fixture signing; temporary secret export is absent. Public metadata drift, extra
private key, unsafe manifest mode and keyring changes fail inspection. Test-only
Pinentry/passphrase and temporary fixture keys are never production identities.

Python syntax/CLI help/whitespace passed. Current host has pinentry-gnome3, graphical
session variables and an existing default state parent. These are readiness
observations, not a successful production password prompt or key-generation claim.
Existing native CI discovers test_native_*.py including the new local suite and
already installs GPG/Python prerequisites; no extra dependency or secret is needed.
Only docs/new scripts changed; native package/Rust/Worker bytes need no rebuild or
unchanged suites/deployment. Prior measured checks remain in their original SSOT.

The development command does not start production Pinentry. When the user can
enter their own password locally, run this single initialization command (absolute
source path may be used from any directory):

```sh
python3 scripts/native_signer_local.py initialize
python3 scripts/native_signer_local.py inspect
```

Default private root: ~/.rquickshare-signing. Pinentry may ask during
key generation/export/import. Passphrase minimum16 characters is enforced; do not
paste it into chat, CLI options, environment, source or a shell script. Production
CLI exposes no test Pinentry/loopback/password bypass. Initialization is explicit,
fresh-only and never runs during a build, import, test discovery or application
startup. No production profile is created until that local input is available.

Use archive-keyring.gpg and the primary fingerprint in identity.json to prepare
and independently review a signing request with native_signer.py. Pass ONLY the
signer child as --gnupghome for normal release signing; never the authority child.
Keep request digest approval explicit. The authority profile is for subkey renewal/
recovery/certification, not routine archive signatures. It remains encrypted local
material on the same PC; it is not an offline custody claim. Before public trust
bootstrap, retain a protected recovery copy of the authority profile and its
openpgp-revocs.d certificate at an operator-chosen recovery location; validate
restoration and metadata-refresh ownership. No backup destination was invented.

Read-only inspect checks public identity/current expiry, exact private file layout
and permissions without starting an agent. Live passphrase protection is checked
at initialization with KEYINFO; later inspect is not a fresh agent/protection
attestation. It never prints private key files or arbitrary GPG output.

On failed/cancelled initialization the private root is retained, identity.json is
absent, temporary export removed and owned agents stopped. Existing output refuses
reinitialization to prevent silent identity replacement; recover/review that state
before explicitly discarding it. Revert this feature commit to remove tooling;
that does not revoke an already created/published identity or delete private data.

L3 complete for prepared/tested tooling and handoff. Actual production key creation
awaits local Pinentry input, then public identity verification/recovery and real
publication acceptance. No extra host/VM/token is requested. No real secret,
production signature/R2 object, enrollment/install, user GPG settings or unrelated
process was changed. All task-owned test agents/terminals closed; Terminal closed.

## Production initialization preflight correction (2026-10-14)

User is at the workstation and explicitly authorized launching Pinentry. Initial
initialize refused before creating state: ~/.local/share is caller-owned0775,
violating the frozen non-group/world-writable parent invariant. No agent/key or
partial profile was created. Home itself is canonical/caller-owned0750.

Before retry: use ~/.rquickshare-signing as the default private root directly under
that secure home. Preserve fresh-state/ownership/permission guards; do not chmod
existing user directories or introduce an implicit insecure-parent exception.
This is an internal storage-default correction within the selected same-PC custody,
not a new signing authority/host. Explicit --state-dir remains available and guarded.
Update operator instructions before retry, check syntax/help/whitespace, then run
real initialization with system Pinentry. Existing insecure-parent tests already
protect the refusal; unchanged key-generation tests need no repetition.

## Actual production profile initialization (2026-10-14)

User explicitly confirmed being at the workstation and entered the passphrase in
system Pinentry. initialize and subsequent read-only inspect succeeded using the
corrected ~/.rquickshare-signing default. Public primary fingerprint:
6D8854762CF0C86881138A56695A0888D09FA616. Public signing subkey:
1F5150D98D94C43109FBA2503E7DE744C1014357. Public keyring SHA256:
2c9f0593c762c4a792ca1d27f0c7911a3f0f935c3e36332c9944700aa763a25b.
Primary expires2031-10-13T18:22:36Z, subkey2027-10-14T18:23:43Z. Current verified
profile layout/protection is the initialized same-PC custody, not an offline host.
No production password/private key was read, logged, exported into Git or CI.

Next bounded verification: use the working signer profile for a fixed-format
non-release challenge with fresh nonce/time, require exactly the expected subkey
and pinned primary in real gpgv status. This message is explicitly not APT metadata,
a release approval or package signature. Run only if its dedicated agent socket
is initially closed, and close the exclusively started agent in finally. Retain
only public identity/challenge/signature evidence; no secret profile copy. Do not
sign/publish a release as a side effect of key verification.

Control challenge verification succeeded with exact signing subkey/primary and
SHA256. Public keyring committed at scripts/keyrings/rquickshare-native-archive.gpg;
public challenge/signature/evidence at docs/evidence/native-signing-2026-10-14/.
Keyring export SHA256 matches the initialized profile. Real agent Unix sockets for
both authority and signer reject connections after completion. No unchanged test
suite repetition, release signature, R2 key backup/object upload or host enrollment.
Private material stays only under the user's private home root, outside Git/output
archives. Recovery copy destination must be selected before key material is sent
to any external storage. The live signature is expressly not a release approval.
