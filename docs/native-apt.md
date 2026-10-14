# Native signed APT distribution

2026-10-14 custody update: user selected the existing Linux workstation; no
additional host/VM/token. Protected local certification/signing profiles and
operator initialization are implemented/tested in [local signing](native-local-signing.md).
Earlier separate-host requirements below are superseded. Production keys are now
initialized and real subkey/gpgv verification passed; recovery-copy destination,
release approval and actual publication acceptance remain open.

2026-10-14 current checkpoint: user selected
https://download.morozhenko.com/dist/rQuickShare/ (exact case) for production APT.
A narrow read-only Worker keeps the dedicated bucket separate from Picturix.
See [shared-prefix route SSOT](native-download-route.md) for current deployment/check status;
earlier domain options/pending-choice notes below are historical. Isolated signer
host and real public primary fingerprint still gate signed publication/enrollment.

Current follow-up: the user selected Cloudflare R2 and isolated outside-CI signing.
[native-r2.md](native-r2.md) records the created private bucket, checked conditional
publisher, public request/approval handoff, By-Hash:force bootstrap and actual
HTTP APT projection checks. Production domain routing/signing identity remain
unbound; earlier U1–U4 measurements below are preserved without repeated checks.

2026-10-14. Mode A. User selected1 in native-update-review.md: a dedicated signed
native APT channel, system package-manager ownership. Guide
/home/morozhenko/.codex/AGENTS.md. Direct dev; no installation/enrollment, live
manager, real signing key, external publication/push or sibling project change.

## Analysis and plan before code

The native Debian builder/verifier already validates exact assets, ELF identity,
dependencies and no maintainer scripts. At stage entry it fixed every artifact to +native1, which
cannot distinguish successive native builds to APT. Legacy release jobs build
Tauri and its version fetch is not native provenance. There is no configured APT
host/key or package-manager UI contract. Tools dpkg-scanpackages, gpg/gpgv, apt-get
and apt-cache are available; apt-ftparchive is not. Use the existing Debian index
tool plus a small exact SHA256 Release serializer, not a private package resolver.

1. [x] U1: tracked native package revision; strict positive bounded revision and
   release/debug version validation; expose validated metadata from package verify.
   Preserve binaries/service behavior and existing package inventory.
2. [x] U2: local immutable repository prepare/sign/verify/bootstrap utilities.
   Validate native release packages, generate standard Packages/gzip/by-hash/Release,
   sign only using an explicitly supplied external GPG home/fingerprint and verify
   with an isolated public keyring. No key creation or publication in production.
3. [x] U3: isolated contract tests with ephemeral test-only signing authority and
   actual APT acquisition/candidate/download; native CI runs affected checks and
   emits an explicitly unsigned repository staging artifact, with no signing secrets.
4. [x] U4: actual rebuilt release package + unsigned repository stage; document exact
   measured checks, trust bootstrap, signing/promotion handoff, atomic commits and
   exact source/artifact export. Bind production URI/key custody only after review.

Rollback repository tooling/CI first, then package revision changes. No native
settings/data/wire/UI change. No application HTTP polling or installation worker:
APT/system update tooling owns network checks/installations under OS user policy.
This is the selected distribution ownership, not a new background network grant.

## Frozen staging and trust contracts

Current build ABI is Ubuntu26.04; suite rquickshare-native-ubuntu26.04, component
main, actual amd64/arm64 only, Origin rQuickShare, Label rQuickShare Native. Never
advertise a Debian/Ubuntu24 compatible build from this ABI. Native package revision
is tracked separately from Rust semver and incremented for every changed release.
Debug artifacts are never admitted. Same source/version across architectures;
one release package per supplied architecture, no invented missing architecture.

Fresh output only, temporary sibling stage then atomic rename; no overwrite or
symlink/special-file input/output. Release contains SHA256 hashes and sizes of
exact Packages/Packages.gz, Acquire-By-Hash=yes, UTC Date and Valid-Until seven days
later. The bounded validity protects against indefinitely replayed metadata; a
publisher must refresh/sign before expiry even with no code release. Serializer
clock is injected for tests. Verification rejects expiry/future dates, altered
fields, unknown paths, wrong architecture/version/identity, digest/size mismatch,
truncated gzip and dangling/mismatched by-hash copies. Package hashes are streamed.

Only public key material may enter bootstrap/artifacts. Expected full primary key
fingerprint is explicit and checked independently; signing subkeys must chain to
it. gpgv verifies both InRelease (exact cleartext Release) and Release.gpg with a
temporary empty home and explicit public keyring. No ambient trust/key discovery,
keyserver retrieval, trust-all, unsigned fallback, private-key export or executable
remote installer. SHA256 sidecars alone are not authentication.

Bootstrap generates reviewable deb822 .sources with HTTPS URI, exact suite/actual
architectures and Signed-By scoped to an explicit fingerprint/public keyring.
It writes only a fresh output directory, never /etc/apt or a user trust store.
HTTPS URI rejects credentials, query/fragment, whitespace and control characters.
Production URL, signing authority/custody and publication transport are unbound;
parameterized local tools do not decide who controls the production trust anchor.

Signing/promotion verify packages and the entire repository before release. A
publisher must compare the previous authenticated snapshot: reject downgrade or
same-version different bytes, retain old immutable pool/by-hash objects while
clients may reference them, publish metadata last/atomically, and keep rollback
packages as a higher revision. No application binary replacement, service restart,
login grant or trust enrollment is introduced. Existing no-maintainer-script
packages leave live mapped processes/transfers alone; installed upgrades/removal,
manager cache refresh and mixed old-process/new-package behavior remain acceptance.

## Verification inventory before tests

Mode A focused checks once, not a ten-run Mode B gate. Existing Python package5
plus new repository scenarios: version/revision boundaries and dpkg ordering;
Release/time/URI/fingerprint tables; actual signed stage with wrong key/signature/
cleartext/index/gzip/by-hash/package/path tamper; duplicate/debug/architecture and
same-version drift/downgrade; immutable output failure; actual isolated APT update,
candidate and download with no host lists/status/trust/config/hooks or installation.
Fake only external signing authority (ephemeral real GPG keys); do not mock repository
validation or APT. Test-only expired/future clocks are explicit. Close test-owned
GPG agents; no HTTP server/watcher. Use local file transport only for isolated APT.
Applicable Positive,N1,N2,N3,N4,N5,N6,N7,N8,N9,N10,N12; N11 not applicable.

## Primary references inspected

https://manpages.debian.org/unstable/apt/apt-secure.8.en.html
https://manpages.debian.org/unstable/apt/sources.list.5.en.html
APT verifies signed Release and index/package checksum links. Signed-By scopes
trust; by-hash avoids metadata/index replacement races. A compromised signing
authority remains a risk. Reading this contract does not prove a production host,
key bootstrap, installed update or physical peer behavior. apt-ftparchive web fetch
failed; use installed Debian tool documentation and actual APT acceptance below.

## Current state

U1–U4 local implementation/checks/artifacts complete. The previous login package
checks remain in native-autostart.md. Production publication still needs a chosen
URI/key custody; native-apt-publication-review.md records that binding decision.
No production trust enrollment or externally published APT channel is claimed.

U1 complete: revision2 is tracked in packaging/release.toml; strict positive bounded
revision/native version and real dpkg ordering are covered by the existing package
metadata table. Package verification now returns its already validated fields for
repository admission. Python package5/0 passed1.244s. No Rust binary, service,
settings or public wire behavior changed. Terminal closed.

U2/U3 complete in source: immutable local prepare/sign/verify/bootstrap, actual
Debian indexes, exact signatures/cleartext/public key authority, SHA256 chain,
seven-day freshness, same-version bytes/downgrade/architecture retention and
reviewable scoped bootstrap. Linux renameat2(RENAME_NOREPLACE) protects local
promotion against a concurrent output creator; no check-then-overwrite fallback.
Gzip verification compares bounded deterministic bytes rather than decompressing
an unbounded attacker input. Tree inventory rejects links/special files before
reading or passing them to gpgv. Bootstrap derives architectures from a verified
signed snapshot, never arbitrary caller labels. Files/directories are normalized
to0644/0755 independently of umask. No ambient GPG or APT trust/config is used.

Initial focused repository6/0 passed4.682s. Source review tightened URI authority/
empty userinfo/query/fragment, signature trailing-data framing, bounded sidecars/
Release before external verification, atomic no-replace and signed bootstrap.
Final affected repository7/0 passed6.753s, including two-architecture metadata-only
fixtures, actual APT candidate/download and wrong-key refusal. Test-only GPG
agents were closed by their exact isolated home. No production signing authority,
ARM binary execution, host APT configuration/status/hooks, HTTP server or package
installation was used. Python package5/0 remains the measured U1 run above; no
unnecessary repeat or ten-run/coverage claim. Native CI source now runs both
scoped modules and uploads separate unsigned staging after real package checks;
no remote workflow run or publication claim. Terminal closed.

## Local operator commands

Use Python>=3.11, Debian dpkg-dev, gnupg/gpgv and apt on Linux. Examples require
explicit previously reviewed production inputs; placeholders are not configured
endpoints, keys or grants. The prepare step needs no secret and never executes
package maintainer code. Its package verifier compares assets to this checkout;
use the matching audited source when signing older artifacts.

```sh
python3 scripts/native_deb.py build
python3 scripts/native_apt.py prepare \
  --package native_desktop/target/packages/rquickshare_0.11.5+native2_amd64.deb \
  --output /absolute/existing-parent/native-apt-unsigned
python3 scripts/native_apt.py sign \
  --repository /absolute/existing-parent/native-apt-unsigned \
  --output /absolute/existing-parent/native-apt-signed \
  --gnupghome /explicit/external-signing-home \
  --keyring /explicit/audited-public-keyring.gpg \
  --fingerprint FULL_UPPERCASE_PRIMARY_FINGERPRINT \
  --first-release
python3 scripts/native_apt.py verify \
  --repository /absolute/existing-parent/native-apt-signed \
  --keyring /explicit/audited-public-keyring.gpg \
  --fingerprint FULL_UPPERCASE_PRIMARY_FINGERPRINT
python3 scripts/native_apt.py bootstrap \
  --repository /absolute/existing-parent/native-apt-signed \
  --uri https://EXPLICIT_REVIEWED_REPOSITORY_URI \
  --keyring /explicit/audited-public-keyring.gpg \
  --fingerprint FULL_UPPERCASE_PRIMARY_FINGERPRINT \
  --output /absolute/existing-parent/native-apt-bootstrap
```

For every subsequent signing operation replace --first-release with --previous
/path/to/previous-authenticated-snapshot. The prior metadata may be expired when
used only as an authenticated anti-downgrade baseline; new metadata must be fresh.
Refreshing metadata reuses the exact published packages at a new Date, preserving
version/hash identity. Both signing and verify can compare the previous snapshot.
No shell command downloads/executes an installer. Bootstrap only emits public
keyring/FINGERPRINT/.sources; independent fingerprint confirmation must precede
reviewed system enrollment. APT signatures grant archive authority, not a package
code audit; production trust scope, key rotation/revocation and release approvals
need the concrete authority/hosting binding before deployment.

Publishing is deliberately not implemented against an invented host. Keep old
pool and by-hash objects for at least all active metadata/cache references; a
fresh standalone snapshot is not authorization to delete the old public tree.
Use immutable object publication followed by metadata-last/atomic promotion.
Promotion/refresh job, production signing-key lifecycle and real installed
upgrade/removal/session behavior are pending the concrete production binding.

Final standards review: sources.list(5) specifies a10-second default future-date
allowance. Align the verifier with actual APT instead of the initial ten-minute
tolerance; adjust only the boundary table and rerun that affected scenario.

The corrected affected date/bootstrap scenario1/0 passed1.436s. No other unchanged
scenario was repeated. Python syntax compilation, native workflow YAML parse with
contents:read permission assertion and whitespace checks passed. Rust code was
unchanged, so existing core/native checks are preserved rather than rerun.

U4 local complete: actual locked release package0.11.5+native2 amd64 built/verified,
core/native cached0.71s/0.56s; exact10-file assets/modes/owner/checksum/ELF/dependency
checks and extracted offline GUI/daemon --help + private settings fixture passed.
SHA256 5821ad8c85556b603461876b7c6a5484c01048feb2dbf4e741c04058880ec3c7.
Actual unsigned repository prepared from those bytes at
native_desktop/target/apt-unsigned-native2; Date2026-10-14 16:10:26 UTC,
Valid-Until2026-10-21 16:10:26 UTC. Exact Release/index/gzip/by-hash/pool admission
passed. This output has no production signature and must not be enrolled as a
trusted repository. Ephemeral test authorities were not exported as release keys.

Current source boundaries: package revision and local APT production tools/CI
staging only; no new HTTP client/UI/update installer, native protocol or daemon
behavior change. Known core feature-specific and merged-/usr loader notices remain
visible. All owned terminals exited (Terminal closed). No real manager, package
install, host APT lists/status/hooks/trust, production key, publication/push, peer,
clipboard or sibling project mutation. Rollback commits in reverse stage order.

Next: choose concrete hosting and signing custody before publishing/enrollment.
Then prepare reviewable production release/refresh/rotation/revocation workflow,
retain authenticated snapshots and confirm fingerprint independently. Installed
upgrade/removal during transfers and actual OS/KDE/system-update behavior remain
acceptance; isolated APT update/download does not prove installed lifecycle or
Google interoperability. Exact committed source and unsigned artifact are exported
with separately recorded SHA256/provenance in the output checkpoint.
