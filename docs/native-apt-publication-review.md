# Native APT production binding review

2026-10-14. Mode A analysis after native-apt.md U1–U4 local completion. No
production signing authority, hosting endpoint, trust enrollment or external
publication exists in this work. Guide /home/morozhenko/.codex/AGENTS.md3/3.1:
choose the concrete security/operational boundary before implementing deployment.

## Reviewable completed work

scripts/native_apt.py prepares immutable unsigned snapshots, signs via an explicit
external GPG home, verifies both signatures and the complete package/index/hash
chain, compares previous authenticated releases and emits scoped public bootstrap.
Actual APT accepted/downloaded the signed isolated test channel and rejected a
wrong key. Production package0.11.5+native2 amd64 and unsigned stage are verified;
no ephemeral test key is a production trust anchor. See native-apt.md for exact
commands, results, package hash, seven-day metadata validity and rollback.

Repository origin currently points to morozhenko-apps/rquickshare on GitHub. Its
existing workflows build artifacts; this does not establish Pages setup, a custom
domain, artifact bucket, remote signer, production public fingerprint or secret
custody. Never infer production trust from that Git remote or create a release
identity on behalf of the user without choosing custody/recovery.

## Required hosting input

Provide the intended credential-free HTTPS base URI and its operator, or choose
hosting before binding .sources/publication permissions.

1. Recommended long-term: a dedicated static artifact store/CDN on a controlled
   HTTPS domain. Benefits: installation URL independent of source hosting, immutable
   pool/by-hash retention, constrained publish credentials and artifact lifecycle.
   Costs: configure domain/storage/CDN and monitor refresh, storage and billing.
   Risks: domain/provider/credential loss or incorrect object retention/promotion.
   Better architectural fit for a durable binary archive when infrastructure is
   available; does not imply a provider/domain/account has already been selected.
2. GitHub Pages project hosting. Benefits: reuse repository platform and static
   Actions publication, no separate server. Costs: Pages permission/setup, immutable
   object retention inside successive site artifacts and storage/bandwidth quotas.
   Risks: source/hosting operational coupling and quota-related unavailability.
   The default project-site URL shape can suggest a candidate, not prove that a
   particular endpoint is configured or authorized. Existing Pages/custom-domain
   state must be checked before deployment; no branch or site is created now.

In either case publish immutable objects before metadata or an atomic site snapshot,
retain still referenced authenticated pool/by-hash objects, and refresh metadata
before expiry. A standalone current snapshot must not replace a public history
without merging/retention. Do not silently enable a source or write /etc/apt.

## Signing custody choice

1. Recommended: an isolated signer outside build CI, optionally backed by hardware
   or an HSM; an offline primary and separately scoped signing subkey. Benefits:
   private signing material never enters build jobs, independent release approval,
   revocation/rotation and audit boundary. Costs: provision/operator/recovery and
   authenticated signing requests plus metadata refresh automation. Risks: signer
   outage, loss of recovery material or a signing oracle if it blindly signs CI
   bytes. The signer must enforce reviewed release provenance, previous snapshot,
   monotonic versions and authorized refresh; isolation alone is insufficient.
2. A separate protected CI signing environment with a dedicated signing subkey;
   keep the primary offline. Benefits: automated release/metadata refresh without
   a separate signing service, clear separation from untrusted build jobs. Costs:
   protected environment/reviewers, scoped secrets, hardened job/dependency policy,
   key rotation/revocation and independent provenance validation. Risks: CI/control-
   plane/job compromise can expose signing material; signing must never run on PR
   code or share secrets with the build. More practical infrastructure, a broader
   trust boundary than option1. Never upload private key material as an artifact.

Prefer1 for the strongest clear trust ownership and recovery. A concrete remote
signer/hardware interface or CI custody grant remains necessary to implement the
chosen production integration; current local GPG tools support either placement.
Only full public fingerprint and nonsecret configuration should be sent in chat.

## Primary references inspected

https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages
https://docs.github.com/en/pages/getting-started-with-github-pages/github-pages-limits
Official Pages documentation establishes static/project/custom-domain hosting and
quota constraints. It does not prove this project's current site availability.
APT trust/freshness references are in native-apt.md. Signing custody ranking is an
architectural assessment, not a claim that a remote service already exists.

## Handoff

Local implementation/verification is complete and reversible. Production binding
awaits HTTPS URI/operator and signing custody choice. No external changes while
those required security/operational inputs remain missing. Installed package
upgrade/removal/active-transfer and actual KDE/Google acceptance remain separate.
