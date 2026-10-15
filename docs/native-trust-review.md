# Authenticated peer identity and trusted devices review

2026-10-15. Mode A, direct dev. Analysis/research complete; significant security/
protocol selection pending. Guide: /home/morozhenko/.codex/AGENTS.md sections3,
3.1,11,13. No production authentication, consent, persistence or UI change in
this review. Generic continuation does not authorize an auto-accept boundary.

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
2. [ ] T1 for option1: freeze a bounded typed full-token/certificate/verdict model,
   role-specific verification and anonymous-versus-authentication-failure rules;
   plan affected branches/negative interactions before code. Preserve existing
   wire tags and anonymous manual-PIN interoperability. Do not use untrusted
   CertificateInfo as a trusted provider or allow peer-declared Success to verify.
3. [ ] T2: implement actual cryptographic verifier with injected certificate source
   and clock, no account dependency/global key. Deterministic signatures/token
   fixtures: correct role/token/key; changed role/token/key/signature; malformed
   DER/keys; missing/expired/future certificate; token/hash length boundaries;
   optional signature; conflicting/unknown results; exact retries/no promotion.
   Raw token/private key/name/certificate metadata absent from Debug/log/IPC.
4. [ ] T3: integrate selected verdict boundary in inbound/outbound state machines
   through owned session context. Stage changes only after exact behavior is
   documented; manual receipt and admitted-session invariants remain intact.
   Use duplex/loopback protocol fixtures, no live radio or private credentials.
5. [ ] T4: research supported certificate acquisition/provenance/lifetime/rotation,
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
