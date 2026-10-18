# Automated tagged release and independent package versioning

2026-10-18. Mode A, user selected option B: GitHub Actions signs and publishes
after an immutable `vMAJOR.MINOR.PATCH` tag. This supersedes the earlier
operator-only signing selection **for public release CI**, while preserving the
isolated local signer/recovery tools as an optional disaster-recovery path.
This is a new explicitly approved security trust boundary; no release key,
secret, bucket object or tag is created by this source task.

## Frozen public contract

- Product: **rQuickShare**. Targets Linux, Windows and macOS in the roadmap;
  only a physically accepted OS/architecture gets release artifacts.
- First intended product release: `1.0.0`, Git tag `v1.0.0` (not beta,
  not `native-*`). Later tags `vMAJOR.MINOR.PATCH`.
- Native Debian package: `rquickshare-native`; version `1.0.0-1`;
  source binary remains `/usr/bin/rquickshare`. Crate `rqs_lib` and
  historical Tauri remain independently versioned.
- Keep private R2 bucket `rquickshare-native-apt`, read-only Worker route
  `https://download.morozhenko.com/dist/rQuickShare/`, and APT suite
  `rquickshare-native-ubuntu26.04`. Same immutable pool .deb supports
  direct installation and signed APT. Current ABI only; no invented platforms.
- No automatic installation or APT source enrollment on a user's machine.
  Original `rquickshare` cannot coexist at the same filesystem paths.
  Debian `Conflicts` + `Replaces` and exact installed migration tests
  are mandatory before first production promotion.

## Automated release trust boundary

1. A tag push starts the native release workflow. Validate **exact tag
   syntax**, match native product version, and require tag commit equals the
   current protected `master` HEAD; fail closed for dev/untrusted refs.
2. Build and run offline verification **without secrets** on declared
   Ubuntu 26.04 ABI. Require successful strict 10x Mode B evidence from
   the exact release source commit and stage package/unsigned APT bytes
   as GitHub artifacts; do not invoke arbitrary workflow code from PRs.
3. The isolated `apt-release` GitHub Environment holds only the
   *signing subkey* exported without the certifying/primary secret, its
   passphrase, pinned public keyring and bucket-scoped R2 credentials.
   GitHub CI can now sign, so compromise of protected tag/master/workflow
   or secrets could forge trusted releases; this is an explicit residual
   risk of user-selected unattended option B. Never upload a private
   primary key or recovery archive to GitHub or R2.
4. In the protected job, import the encrypted subkey into a fresh
   temporary `GNUPGHOME`. Validate pinned primary/public keyring;
   refuse extra secret primary keys. Use loopback passphrase from a
   private temporary file (never command arguments/output).
5. For existing channels, fetch and independently verify the prior
   published authenticated snapshot from private R2 before signing,
   to enforce monotonic versions and same-version identical package bytes.
   Never infer an empty channel from a network/auth error.
6. Sign `InRelease`/Release.gpg, verify the entire package/index hash
   chain and exact commit. Publish to R2 through the existing
   conditional immutable-object admission and last-step CAS `InRelease`.
   No overwrite/delete/unauthenticated fallback on publication failure.
7. Publish corresponding GitHub release only after R2 publication and
   public HTTPS/isolated APT checks succeed. A GitHub tag by itself does
   not prove a package was publicly released.
8. Signed APT index metadata expires in seven days. Provide an
   independently authenticated refresh path that preserves package bytes
   and prior archive authority, with explicit failure monitoring. A
   new application version is **not** required to refresh indexes.

Secrets are not available in the repository or to this agent. Their
registration, access-scoped environment rules, subkey-only export and
end-to-end R2 acceptance require the repository operator. Do not
print or ask the user to paste secrets in chat.

## Stages and rollback

| Stage | Planned change | Gate / rollback |
| --- | --- | --- |
| S0 | Freeze this architecture/identity contract | Docs-only commit |
| S1 | Decouple native SemVer from `rqs_lib`, package/name/version metadata and migrated package ownership | Tests for valid/invalid Debian versions, dpkg comparisons, conflicting files |
| S2 | Align APT producer/verifier/bootstrap and Worker allowlist with new immutable pool identity | Existing real signing/APT/Worker tests, negative path grammar and hash checks |
| S3 | Create tag-gated workflow, protected CI subkey/bootstrap, authenticated previous-snapshot path, online publisher and metadata refresh | Offline CI-contract tests; no secrets during ordinary preflight |
| S4 | Verify all affected tests/CI and update SSOT/handoff | Exact-source evidence, atomic commits |
| S5 | Operator provisions GitHub Environment and secrets, verifies signed test publication and rollback, and manually authorizes the first public tag | **Not performed in this source task** |

Explicit non-goals: full Google Quick Share parity, macOS/Windows ports,
new desktop UX, new R2 bucket/route, automatic enrollment, publishing
`v1.0.0` now, replacing the installed package, or modifying active
signing/recovery material. Existing transfers and Linux settings remain
untouched. Scope is reversible per atomic commit until public signed
artifacts or a new package identity are installed/published.
