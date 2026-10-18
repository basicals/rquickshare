# Production native signing custody: concrete deployment decision

> **2026-10-18 release-CI change:** The independently approved
> [automated tagged release decision](native-tagged-release.md) supersedes
> this document's former "never sign in CI" rule for protected public-release
> automation. Only the encrypted signing **subkey**, never the certifying
> primary/recovery material, may enter the isolated GitHub `apt-release`
> environment. The local signer and backup remain valid offline recovery
> paths. Historical observations below remain unchanged.


2026-10-14 decision update: user selected the existing Linux workstation and
rejected additional machines/VMs/hardware. The options below are historical.
Current accepted boundary and implementation plan: native-local-signing.md.

2026-10-14. Mode A. Direct dev. Continuation after the deployed shared download
route. Read native-r2.md operator/remaining gates, native-download-route.md,
native_signer.py and the existing agent guide. Working tree initially clean.

## Current authority and missing inputs

User already selected isolated signing outside build CI, with an offline primary
and a separate signing subkey. Do not ask that architectural choice again.
Selected production URI/Worker is deployed and checked. No production signing key,
public primary fingerprint, signer host/hardware/custodian or scoped publication
credential is provisioned. Existing signing wrapper checks an independently
approved canonical request and does not establish build provenance on its own.

R2 is artifact storage, not the selected GPG signing host. A software key folder
on the ordinary development/build/publisher workstation is not a substitute for
the selected isolated signer. Do not create a production key in a guessed path,
use ephemeral test identities for a release, or put signing secrets in CI/chat.

## Options requiring user selection before implementation

1. Recommended custody target: dedicated signing-only Linux host with an OpenPGP
   hardware token holding the release signing subkey; offline primary/recovery
   material kept separately. Benefits: the token keeps signing private material
   outside ordinary host files; clear separation from build/publish authority.
   Costs: compatible hardware, provisioning and operator access are required.
   Risks: token loss/failure and a compromised signer requesting an unwanted
   signature remain possible; independent request review and tested recovery are
   still required. Best for key custody where this hardware/host is available,
   not a claim that a token validates the build or makes a compromised host safe.
2. Dedicated signing-only Linux host/VM with an encrypted software signing subkey;
   offline primary/recovery material kept separately, no build/publisher credential.
   Benefits: preserves the accepted separate-owner boundary without requiring a
   token, supports a controlled signing workflow. Costs: host/key-file hardening,
   backup protection and operator-controlled unlocking are required. Risks:
   compromise of the unlocked signer can expose/use the software key; VM security
   also depends on its host. Encryption at rest does not protect a running signer.

Both need a concrete available host/hardware description and custodian before
provisioning. Do not prescribe a purchase or assume available infrastructure.
No automated signing service, network signing API, cron or remote host creation
is included in this decision. Metadata currently expires after seven days, so an
explicit refresh owner/process must be completed before enrolling production users.
Hardware vs software custody does not alone resolve that operational requirement.

## Implementation milestones after selection

1. Freeze the selected custody boundary, concrete host/token, public identity,
   offline backup/revocation and recovery ownership before key provisioning.
2. Prepare host-specific operator provisioning/check commands and handoff policy.
   Keep build, signer and publisher credentials separate. Record approval and
   reject unreviewed candidate/source/baseline drift with existing wrapper.
3. Verify the real public primary/subkey fingerprint and a bounded non-release
   signing check on that host; record measured results separately from fixture tests.
4. Review and approve the audited candidate; verify signed output, provision a
   bucket-scoped publisher credential, exercise actual conditional R2 publication
   and public HTTPS APT acceptance. Never install/enroll clients before this gate.
5. Record periodic metadata-refresh/recovery operation and exact public bootstrap.

Risks/rollback: key custody and trust are durable security boundaries, not an
internal naming choice. Before publication local staged outputs can be discarded;
a published accepted signature/key trust cannot be undone by reverting Git.
No production secret, host, key, token, signing request or R2 object was created
in this continuation. Stop at the missing custody decision under AGENTS3/3.1;
no unchanged tests or production preflight polling is needed for documentation.

## Primary reference

https://gnupg.org/documentation/manuals/gnupg/OpenPGP-Key-Management.html
Current official GnuPG documentation supports transferring a selected signing
subkey to an OpenPGP smartcard. This is a custody mechanism, not provenance or
request authorization. Exact algorithm/token compatibility remains host-specific.
