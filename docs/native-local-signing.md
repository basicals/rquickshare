# Native release signing on the existing Linux workstation

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

1. [ ] L1: document local custody and implement a small operator CLI in
   scripts/native_signer_local.py: initialize a fresh explicit private state,
   export only the public keyring/identity, inspect profile roles/protection.
2. [ ] L2: focused deterministic parsing/path/refusal tests and one real GPG
   protected-key/subkey-only-profile integration with owned test-only Pinentry.
   Verify real approved snapshot signing through the existing wrapper; no domain
   logic mocks, global GPG state or unchanged whole-suite repetitions.
3. [ ] L3: record measured checks, local one-command operator handoff/recovery,
   CI discovery compatibility, atomic commits and exact refreshed source export.

## Frozen initialization contract

No ambient ~/.gnupg, environment secret, secret CLI option, loopback or generated
password. Production initialize uses normal system Pinentry for the user's
passphrase. No production key initialization during this development turn: the
operator must enter their own password locally. Prepare/test all commands first.
Default CLI state is ~/.local/share/rquickshare-signing; allow an explicit absolute
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
