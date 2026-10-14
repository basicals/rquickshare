# Native signed APT distribution

2026-10-14. Mode A. User selected1 in native-update-review.md: a dedicated signed
native APT channel, system package-manager ownership. Guide
/home/morozhenko/.codex/AGENTS.md. Direct dev; no installation/enrollment, live
manager, real signing key, external publication/push or sibling project change.

## Analysis and plan before code

The native Debian builder/verifier already validates exact assets, ELF identity,
dependencies and no maintainer scripts. It fixes every artifact to +native1, which
cannot distinguish successive native builds to APT. Legacy release jobs build
Tauri and its version fetch is not native provenance. There is no configured APT
host/key or package-manager UI contract. Tools dpkg-scanpackages, gpg/gpgv, apt-get
and apt-cache are available; apt-ftparchive is not. Use the existing Debian index
tool plus a small exact SHA256 Release serializer, not a private package resolver.

1. [x] U1: tracked native package revision; strict positive bounded revision and
   release/debug version validation; expose validated metadata from package verify.
   Preserve binaries/service behavior and existing package inventory.
2. [ ] U2: local immutable repository prepare/sign/verify/bootstrap utilities.
   Validate native release packages, generate standard Packages/gzip/by-hash/Release,
   sign only using an explicitly supplied external GPG home/fingerprint and verify
   with an isolated public keyring. No key creation or publication in production.
3. [ ] U3: isolated contract tests with ephemeral test-only signing authority and
   actual APT acquisition/candidate/download; native CI runs affected checks and
   emits an explicitly unsigned repository staging artifact, with no signing secrets.
4. [ ] U4: actual rebuilt release package + unsigned repository stage; document exact
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

Plan frozen before code. U1–U4 pending. The previous login package/checks remain
recorded in native-autostart.md. Production publication needs a separately chosen
URI/key custody; complete local reviewable tooling before asking for that binding.

U1 complete: revision2 is tracked in packaging/release.toml; strict positive bounded
revision/native version and real dpkg ordering are covered by the existing package
metadata table. Package verification now returns its already validated fields for
repository admission. Python package5/0 passed1.244s. No Rust binary, service,
settings or public wire behavior changed. Terminal closed.
