# Native update distribution review

2026-10-14. Mode A analysis after completed native-autostart.md A1–A4. Guide
/home/morozhenko/.codex/AGENTS.md sections3/3.1 requires selection for the update
trust/distribution boundary. No updater, repository enrollment, signing key,
external publication, package install or live manager change in this review.

## Source observations

Legacy app/main/src/vue_lib/utils.ts getLatestVersion fetches the fixed GitHub
releases/latest endpoint during HomePage initialization, strips the first tag
character and compares semver. Heading.vue opens a fixed releases/latest page on
explicit click. It does not verify/download/install a signed native update.
Native source currently has no updater. The new native package is uninstalled;
existing Tauri release workflows remain migration baseline. A generic latest tag
must not be presented as a confirmed compatible native package/channel.

The Debian package manager owns executable/unit paths and dependency/conflict
handling. A native app must not replace package-owned binaries itself or use the
transfer daemon as a privileged installer. A checksum shipped beside a package
checks consistency, not independently authenticated release provenance.

## Selected option1 (alternatives retained)

1. Recommended: a dedicated signed native APT channel for the current Debian/Ubuntu
   package distribution; the system package manager owns installation/update.
   Benefits: one installation owner, dependency/transaction behavior, authenticated
   repository metadata/checksum chain and ordinary desktop system-update tooling.
   The application may expose version/channel/status and an explicit system-update
   entry, without a second self-updater. Costs: maintain repository metadata,
   monotonic package revisions, signing-key lifecycle, build provenance and release
   publication/hosting. Risks: key/release-pipeline compromise, wrong ABI/channel,
   enrollment or installed live-transfer lifecycle defects. Scope is the accepted
   native Debian path, not an implied all-distribution updater. Repository bootstrap
   and user trust need explicit, reviewable artifacts; no keys/hosts are invented.
   Best architectural fit: preserves existing dpkg ownership and provides normal
   system update discovery rather than asking each user to replace files by hand.
2. A dedicated native GitHub release channel with explicit in-app version checking
   and an authenticated native artifact/manifest, followed by an explicit handoff
   to the OS package installer. Benefits: no separate APT hosting, package manager
   still owns installation; independent native/legacy channel validation. Costs:
   custom release metadata/signature verification, platform/ABI selection and
   package handoff UX. Risks: extra download/install steps, less consistent system
   update discovery, signing/bootstrap and installer recovery still required.
   This is a viable distribution design, not permission for automatic browser
   opening, generic releases/latest reuse or replacing dpkg-owned binaries.

Both need explicit policy for network checks and installed transfer-safe upgrades
before implementation. This selection establishes distribution ownership; detailed
plans and reversible source changes follow the user's answer. Never treat a
sidecar hash or a GitHub version response as proof of a compatible trusted update.

## Primary reference inspected

https://manpages.debian.org/unstable/apt/apt-secure.8.en.html
APT authenticates signed Release metadata and follows repository/package checksum
links; Signed-By scopes repository keys. This does not audit package code or defend
against compromise of the signing authority. This source supports the trust model,
not evidence that this project already operates a signed repository.

## Handoff

User selected1. The implementation plan and measured checks are in native-apt.md.
Production URI/key custody remain unbound; no publication is implied. Completed login artifact/checks remain
in native-autostart.md. Physical package/Google acceptance remains separate.
