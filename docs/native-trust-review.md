# Authenticated peer identity and trusted devices review

2026-10-15. Mode A, direct dev. User selected option1: Google-protocol
authentication foundation and provider research. Guide:
/home/morozhenko/.codex/AGENTS.md sections3,3.1,11,13. This selection does not
authorize an auto-accept boundary, account access, enrollment or new pairing UX.

## Repository findings

Analyzed at386ed1a (visibility native6 handoff complete):

- `core_lib/src/utils.rs::gen_ecdsa_keypair` creates a new random P-256 key.
  Inbound/outbound handshake calls use it per session. It is not a persistent
  peer identity and cannot safely implement a remembered-device fingerprint.
- `hdl/inbound.rs` and `hdl/outbound.rs` send random6-byte secret_id_hash and
  random72-byte signed_data when opening PairedKey exchange. Incoming exchange
  only requires the nested frame, then sends Unable. The remote result handlers
  only require its presence: they do not distinguish Success/Fail/Unable/Unknown.
  This is current unauthenticated-peer behavior, not evidence of identity checks.
  Manual native consent/PIN remains required. Encryption/HMAC proves integrity
  inside the established session; it does not establish a returning peer identity.
- The32-byte UKEY2 authentication token is derived locally, reduced to the
  displayed PIN, and not retained as typed input to certificate verification.
  Any verifier needs the full session-bound token and role separation; the short
  displayed PIN is not signing material or a durable identifier.
- `proto_src/wire_format.proto` models PairedKeyEncryption/Result/CertificateInfo/
  PublicCertificate. No handwritten Rust reader uses certificate_info. Modeling
  fields alone provides neither a certificate acquisition flow nor provenance.
- `sessions.rs` has bounded consent capabilities/rounds, readiness and exact
  acknowledgements. A future automatic policy must go through those invariants,
  not mutate handler state or spoof a GUI Accept. Existing receive-root policy,
  path limits, quotas and explicit clipboard/open behavior must remain enforced.

Names, IP/MAC/endpoint IDs, a self-asserted Success, random PairedKey bytes and
ephemeral UKEY2 keys are explicitly unsuitable remembered-trust authorities.
No existing-session fields will be relabeled as verified identity.

## Primary-source observations and limits

Read public Google source on2026-10-15 and retained bounded public file receipts
in [research evidence](evidence/native-trust-research-2026-10-15.json). The pinned
revisiond128b5fca30b3fa8d4642f8c75dd9e314e090e74 is the latest commit returned
for the runner path, not a Google Android/Windows release-version attestation.

[Google's verifier](https://github.com/google/nearby/blob/d128b5fca30b3fa8d4642f8c75dd9e314e090e74/sharing/paired_key_verification_runner.cc)
uses a role-prefixed session token, a resolved public certificate, signature
verification and certificate-associated authentication-token hashes. Missing
identity can yield Unable; known invalid verification yields Fail. Fail is
absorbing when outcomes combine. Random signature/hash fallbacks are deliberate
unknown-peer behavior, not valid credentials. Therefore our unconditional
result handling cannot serve as an authenticated trust decision.

[Public certificate implementation](https://github.com/google/nearby/blob/d128b5fca30b3fa8d4642f8c75dd9e314e090e74/sharing/certificates/nearby_share_decrypted_public_certificate.cc)
verifies ECDSA-SHA256 with the certificate key and uses certificate secret material
for authentication-token hashes; it also decrypts metadata. A copied public key
without acquisition/provenance/lifetime rules does not create account membership.

[Private certificate implementation](https://github.com/google/nearby/blob/d128b5fca30b3fa8d4642f8c75dd9e314e090e74/sharing/certificates/nearby_share_private_certificate.cc)
and [constants](https://github.com/google/nearby/blob/d128b5fca30b3fa8d4642f8c75dd9e314e090e74/sharing/certificates/constants.h)
have validity intervals and certificate rotation. Permanently pinning one observed
certificate as a physical device would require a separately authenticated rotation
link; name equality is insufficient. These source observations do not establish
supported backend access for this app.

[Google's Windows/Android help](https://support.google.com/android/answer/13801258?hl=en)
describes automatic acceptance between one's own devices signed into the same
Google Account. Local permission records would be a separate product feature.
No Google login, contacts query, private API access or account authorization was
attempted. Certificate distribution/enrollment/rotation across stock peers remains
unproven; this review must not promise that a local trust list enables it.

## Significant alternatives

| Option | Scope | Advantages | Costs and risks |
|---|---|---|---|
| 1 — Google-protocol authentication foundation first (recommended) | Introduce bounded typed certificate/token/verification results using the existing PairedKey wire exchange, explicit missing/invalid/verified outcomes and real cryptographic fixtures. Research/freeze the supported certificate provider/enrollment and rotation path before trust persistence or auto-accept UI. Preserve manual acceptance until the identity and policy gates are genuinely satisfied. | Builds the prerequisite for Google-compatible identity; avoids a parallel proprietary authentication scheme and prevents unverified input from becoming trusted. Cryptography, certificate provenance and user permissions have distinct ownership. | This stage does not deliver account login or automatic acceptance. Certificate acquisition/backend eligibility may remain unavailable; need follow-up provider/security selection and versioned real-peer validation. Incorrect result mapping could break anonymous transfers; absent identity must preserve manual flow while explicit authentication failure must not become trusted. |
| 2 — separate rQuickShare-only local pairing first | Design an explicit user-verified enrollment flow and persistent key possession proof between controlled rQuickShare peers, plus revocation/rotation and a separately labeled local permission policy. Stock Google peers continue using manual acceptance until their own identity path is implemented. | Can provide a self-contained local identity without depending on Google account services; clear enrollment/revocation UX under our control. | Introduces another wire/security protocol and persistent private credentials, additional pairing UX, downgrade/cross-protocol/recovery risks. It does not deliver remembered trust for stock Android/Google Windows or Contacts/Your devices parity; supporting two identity systems is an enduring maintenance cost. |

Recommend1 for the user's full Google interoperability goal and architectural
correctness. A trust store is downstream of verified identity, not a substitute
for it. Option2 is coherent only if a separate local pairing product/compatibility
scope is explicitly wanted; do not quietly embed custom bytes into Google fields.

Selection of1 authorizes the scoped authentication foundation and provider
research, not Google account access, backend credentials, trust-store enrollment,
auto-accept, PIN removal or a new visible UI. Those later significant contracts
must be concrete and separately selected. Selection of2 requires freezing its
new pairing/security/storage/recovery contracts before implementation.

## Plan after selection and before implementation

1. [x] T0: inspect core session identity, PairedKey handling, consent boundary and
   primary Google source. Record exact pinned source digests, alternatives,
   unknowns and rollback. No unaffected tests/package rebuild for this review.
2. [x] T1 for option1: freeze a bounded typed full-token/certificate/verdict model,
   role-specific verification and anonymous-versus-authentication-failure rules;
   plan affected branches/negative interactions before code. Preserve existing
   wire tags and anonymous manual-PIN interoperability. Do not use untrusted
   CertificateInfo as a trusted provider or allow peer-declared Success to verify.
3. [x] T2: implement actual cryptographic verifier with injected certificate source
   and clock, no account dependency/global key. Deterministic signatures/token
   fixtures: correct role/token/key; changed role/token/key/signature; malformed
   DER/keys; missing/expired/future certificate; token/hash length boundaries;
   optional signature; conflicting/unknown results; exact retries/no promotion.
   Raw token/private key/name/certificate metadata absent from Debug/log/IPC.
4. [x] T3: integrate selected verdict boundary in inbound/outbound state machines
   through owned session context. Stage changes only after exact behavior is
   documented; manual receipt and admitted-session invariants remain intact.
   Use duplex/loopback protocol fixtures, no live radio or private credentials.
5. [x] T4: research supported certificate acquisition/provenance/lifetime/rotation,
   document evidence and unknowns. Freeze the provider contract before real account
   access, import/enrollment or persistent trust. Do not build a nonfunctional
   Trusted-device toggle when the sender cannot be authenticated.
6. [ ] T5: run affected checks once, document observed counts/failures/limits and
   atomic commits. New native package only after source behavior changes and
   verification; native6 remains current throughout this analysis. Physical
   Android/Google Windows acceptance stays explicitly open.

Risks: confusing unauthenticated encryption with identity; certificate rotation
silently dropping/reassigning trust; malicious metadata/log leakage; invalid proof
falling back to privileged acceptance; short PIN as durable identity; custom
protocol represented as Google parity. Mitigate with separate verified evidence
and permission types, bounded provider inputs, explicit outcomes and unchanged
consent until downstream contracts are selected.

Rollback T0 is documentation-only. Later roll back consumers with their verifier
as a coherent slice; no selected database migration or enrolled account exists
yet. Preserve native6 and all existing signing/recovery artifacts. No private
identity data, account access, live services, signing or publication is needed for
the proposed offline foundation.

## Actual review verification

Only read-only source/official documentation research and documentation changes
were performed. Public-source download sizes and SHA256 are recorded in evidence;
not an executed interoperability or crypto test. Checked61 local links in the
three changed documents, JSON and all5 pinned source sizes/digests. Whitespace
checked using git diff --cached --check. Source tree was clean on dev at start.
Owned research commands exited; no watcher or background service needed.
Terminal closed. No unaffected Rust/GTK/package checks repeated.

## T1 frozen foundation contract (before code)

Implement a private `peer_authentication` domain module, injected per admitted
request. It has separate full32-byte SessionToken, resolved PeerCertificate,
local certificate hash material and Success/Unable/Fail outcomes. No new public
Rust/IPC enrollment endpoint or persisted private key. Production CertificateSource
is explicitly empty until a supported, user-authorized provider is designed;
offline fixtures inject trusted material through the same domain seam. Raw
CertificateInfo/device names/endpoints never populate that source. No test-only
bypass, global store or network call from the verifier.

- Capture the full UKEY2 auth token once after key derivation, separate from the
  displayed PIN. Reject wrong token length and rebinding. Use redacted Debug and
  zeroize token/certificate secret material on drop using existing dependencies.
  A missing token can never verify a peer. Existing direct Introduction fixtures
  without a prior handshake may remain Unable, never Success.
- Snapshot the injected source once per token binding, with provider errors
  distinguishable from missing peer identity. Unavailable source permits only
  anonymous/manual flow. Malformed known credentials fail closed; do not silently
  drop a known invalid certificate to become anonymous. Inject the clock rather
  than read it inside crypto primitives. Clock failure cannot verify identity.
- Peer key is bounded DER SubjectPublicKeyInfo P-256 (at most128 bytes), parsed
  using existing p256 ECDSA-SHA256. Validate not-before < not-after and checked
  interval arithmetic. Public certificate validity uses Google's30-minute skew
  tolerance, inclusive lower/exclusive upper. Local certificate hash material
  uses its exact half-open interval. Expired/future known peer identity is Fail;
  no renewable cached Success across time changes or repeated verification.
- Signature verifies the opposite role prefix (sender01 / receiver02) plus the
  full token. Primary or optional DER signature may prove the same resolved peer
  key; optional proof never changes key identity. Both at most72 bytes; hash at
  most6 bytes. Unknown-peer random fallback remains Unable. Known invalid/missing
  signature is Fail. Unsupported QR/provisioning remains outside authentication;
  no interpretation or trust promotion from those fields.
- Certificate token hash is6-byte HKDF-SHA256 with token as input, certificate
  secret as salt and empty info (not HMAC-only). Use constant-time comparison.
  At most9 local32-byte secret records (bounded source snapshot, no private
  signing key) allow a received hash to prove knowledge of local certificate
  material. Missing/expired local material yields Unable; it never substitutes
  for remote signature proof. The secret_id_hash alone does not identify sender.
- Keep signature outcome distinct from combined verification. Send the signature
  outcome in PairedKeyResult; combine local hash proof, signature proof and remote
  result with absorbing Fail and non-upgrading Unable. Self-asserted remote
  Success never upgrades local Unable. Status0/missing/unknown/provisioning is
  Unable, Fail2 aborts, Success1/Unable3 map directly. No protobuf additions.
- Explicit Fail or bounded/malformed authentication input aborts before inbound
  Introduction/consent or outbound Introduction/payload. Ordinary anonymous peers
  continue manual consent and PIN. This deliberately fixes ignored remote Fail,
  within the selected protocol-authentication behavior. No automatic Accept.
- Same-frame replay remains idempotent but rechecks certificate validity/current
  clock. A conflicting repeated proof/result fails; keep only one bounded digest
  and result, not a history. Rebinding never resets a failed result. A rejected
  proof cannot be replaced by later success to regain admission.
- Outgoing signed_data stays Google's random72-byte unknown-local fallback: this
  slice creates no local signing identity. With valid resolved peer material,
  outgoing hash binds to that peer certificate/token; otherwise random6-byte
  fallback. Actual stock-peer verification is still unproven. Source certificate
  fingerprints must not be exposed/logged as account or physical-device identity.

### Planned scoped verification inventory

| Artifact | Branches/interactions | Level / new methods | Categories |
|---|---|---|---|
| SessionToken/source snapshot | length0/31/32/33, token rebind, source once/error/empty, independent contexts, no token | unit / 2 | Positive,N1,N2,N4,N5,N7,N10 |
| Certificate material / validity | bounded SPKI/truncated/wrong curve/point/trailing bytes, invalid intervals/overflow, lower/upper tolerance boundaries, clock unavailable, local max9/10, secret lengths31/32/33 | unit / 2 | Positive,N1,N2,N6,N7,N8 |
| Signature verifier | both roles, token/key/role mutation, malformed DER/scalars/trailing bytes, primary/optional valid/invalid/absent/oversize combinations, unknown identity | actual P-256 crypto / 3 | Positive,N1,N2,N4,N7,N10 |
| Hash / result combination | independent fixed HKDF vector, wrong token/secret/hash/length, local time bounds, all Success/Unable/Fail combinations, absent/unknown remote result, failure absorbing, conflicting/replayed proof/result and expired replay | unit / 3 | Positive,N1,N2,N4,N5,N7,N8 |
| Handler integration | actual encrypted inbound/outbound PairedKey result and reject-before-Introduction; anonymous Success/Unable/default still manual; token captured by full TCP/duplex handshakes | real loopback/duplex / 2 + existing suites | Positive,N3,N4,N7,N9,N10 |

Estimate12 new table-driven methods, refined if independent invariants require
more. Use deterministic fixture keys/signatures and injected time/source; real
verifier/handlers are subjects. No live credentials/radios. Assertions kill role/
token substitution, omitted certificate bounds, Fail downgrade, remote-Success
promotion, open interval mistakes, ignored provider failure and replay resets.
N11 inapplicable (no billing); N12 no new storage, existing receive/settings fault
tests remain; N9/N3 use existing owned cancellation/stream-failure suites. Full
product Mode B coverage or ten-run stability is not part of this development slice.

### Additional primary evidence

The pinned Google `connections/implementation/encryption_runner.cc` obtains the
raw authentication token from UKEY2 verification bytes; this confirms using the
full auth derivation, not the displayed decimal PIN. `certificates/common.cc`
defines HKDF hash and time tolerance. `nearby_share_certificate_manager_impl.cc`
uses account/RPC-managed certificate download; those sources are researched,
not authorization to invoke that backend. New public receipts extend the existing
research JSON. Actual provider access/enrollment/rotation remains a later security
decision; this foundation must not falsely advertise remembered Google devices.

## T2/T3 implementation checkpoint and observed verification

Implemented the private domain owner with immutable provider snapshot, explicit
Unbound/Empty/Available/Unavailable source state, injected clock, bounded SPKI/
certificate/DER/hash handling, zeroized retained tokens/secrets and redacted
Debug. Known malformed/invalid/expired proof fails; missing identity stays Unable.
Google's role-prefixed ECDSA-SHA256 and6-byte HKDF are used without new dependencies.
Remote Success does not promote local Unable; Fail/rebinding/conflicting replay
are absorbing. Final remote-result admission rechecks current peer/local validity.

Both actual handlers retain the full derived token, construct their outgoing hash
through the owner, send the verified signature result and reject Fail before
Introduction. Existing native receipt/consent/storage gates are unchanged. No
provider consumes peer CertificateInfo or network/account data in production.
Production uses AnonymousSource; trusted certificate fixtures prove the crypto
seam, not operational Google Account integration.

The first affected10 domain methods passed in1.60s (605 filtered). Independent
reference fixture contains a public deterministic test key's SPKI/signature from
system OpenSSL and an HKDF expected value computed with Python stdlib; the
temporary fixture signing-key file was deleted. It is not a production key or a
claim of a Google-generated interoperability vector. Two additional real handler
methods exercise encrypted PairedKey replies and actual state/Introduction gates
over controlled streams. Full affected core execution is pending at this checkpoint.

The integrated full core library run subsequently passed617/0 in21.37s. All12
planned new methods are present (10 domain,2 handler); primary non-overlapping
attribution: Positive1,N1=2,N2=1,N4=1,N7=5,N8=1,N10=1. Further positive/boundary/
race/privacy/cancellation interactions overlap; these are not extra methods or
whole-product coverage percentages. Strict core all-features/all-target Clippy
passed27.82s with the existing upstream bluer future-incompatibility observation.
Final review samples the injected clock once at result admission and zeroizes
the temporary full-token derivation after PIN projection, as well as the retained
token on drop. The affected domain check and compile/lint are repeated only for
these final changes; the617 full run is not mislabeled as later final execution.

## T4 provider research and remaining operational identity gate

Read pinned `sharing/nearby_connections_manager_impl.cc`,
`connections/implementation/encryption_runner.cc` and
`sharing/certificates/nearby_share_certificate_manager_impl.cc`; exact9-file
public-source receipts are in the research evidence. The encryption runner requests
32 UKEY2 verification bytes and passes them as the raw authentication token. This
matches the existing core derivation; the decimal PIN stays presentation only.

The [Google certificate manager](https://github.com/google/nearby/blob/d128b5fca30b3fa8d4642f8c75dd9e314e090e74/sharing/certificates/nearby_share_certificate_manager_impl.cc)
uses account/device identity, managed certificate storage/schedulers and a
certificate download client. It skips download without a logged-in account.
This establishes a missing integration dependency; it does not establish a
supported third-party authorization method or permission to call Google's service.
The repository has no selected authorized account/provider credential, enrollment
or authenticated rotation-link implementation. No request to that backend was
made; only public source was read. It is not justified to build account UI or
auto-accept on a self-asserted peer certificate at this stage.

Selected boundary for this foundation: a private injected source supplies one
already resolved peer certificate and at most9 local hash records, owned by the
session snapshot. It never resolves by peer display name or accepts wire
CertificateInfo. Production source is empty; this intentional limit is recorded
in source/evidence, not hidden by a feature flag or represented as Google Account
support. The verifier can check authentic certificate possession in fixtures;
operational stock-peer identity requires a separately reviewed acquisition,
authorization, storage/revocation and rotation contract with physical evidence.
Provider research for the scoped foundation is complete; that later product/
security decision remains open. Manual PIN/Accept remains the working default.

## Final source verification and candidate plan

After the clock-sampling correction,10 affected domain methods passed1.80s
(607 filtered). Token derivation uses a Zeroizing guard so error/abandonment also
clears the temporary bytes; successful PIN projection explicitly clears them
before returning. Final real binary integrations passed modern_protocol4/0 in
0.00s and native_daemon4/0 in0.06s. Final strict core all-features/all-target
Clippy passed10.48s; nightly format and whitespace checks passed. No changes
were needed to the lockfile, dependencies, protobuf/TS schema, IPC or GTK UI.
Unchanged GTK/Tauri UI suites are not repeated; actual coupled daemon/GTK release
compilation and offline package checks follow. No ten-run or physical-peer claim.

Advance package revision to7 for the changed daemon at the same Rust version.
After this source commit, build/verify the uninstalled native7 package with the
existing private GTK4/Layer Shell root/shlibs metadata. Prepare a fresh unsigned
APT and public unapproved signing request with exact source assertion; do not
sign, install or publish. Native6 remains current until native7 actually passes.
Record package/source/request hashes and metadata validity in new machine evidence,
then export exact committed source/public artifacts with a receipt. Preserve all
older candidates and backup artifacts. Do not use stale native2/native6 export
helpers without adapting/verifying their exact version and feature assumptions.
