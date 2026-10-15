# Google certificate provider: authorization and implementation review

2026-10-15. Mode A; direct dev. Scope: read-only primary-source research after the
completed native7 PairedKey foundation. Guide: /home/morozhenko/.codex/AGENTS.md
sections3/3.1/11/13. No login, credential read, account API request, installation
or runtime change in the review. The user selected option1: supported authorization
only; continue independent parity with manual consent.

## Findings and exact boundaries

Native7 has a real bounded certificate/token verifier and an explicitly empty
production source. Supplying that source requires authenticated acquisition,
local signing identity/publication, peer metadata resolution, cache validity,
revocation and rotation ownership. A downloaded certificate or userinfo identity
alone does not provide Google Contacts/Your devices behavior.

[Google's pinned certificate manager](https://github.com/google/nearby/blob/d128b5fca30b3fa8d4642f8c75dd9e314e090e74/sharing/certificates/nearby_share_certificate_manager_impl.cc)
uses account/device-bound QuerySharedCredentials/PublishDevice operations. It
does not download without a signed-in account. This identifies an account/RPC
dependency, not public third-party API eligibility. Existing primary receipts
remain in native-trust-research evidence; no account API was called.

[Official desktop OAuth guidance](https://developers.google.com/identity/protocols/oauth2/native-app)
describes the app's registered client, PKCE and a desktop loopback redirect.
It does not itself grant permission or access to arbitrary service scopes.
The [public scope catalog](https://developers.google.com/identity/protocols/oauth2/scopes)
contains no match for Nearby/nearbysharing-pa in this review. Searches of Google's
developer docs for Nearby Sharing/Identity OAuth produced no supported certificate
provider integration. This is a bounded negative research result, not proof that
no private/partner authorization path can exist. Ordinary OpenID/userinfo scopes
must not be represented as certificate-service access.

## Independent implementation discovered and inspected

Scotty's main README claims own-device/contacts support, but explicitly labels its
separate account component experimental/unofficial. Inspecting source, rather
than copying the capability table, revealed the actual authorization boundary.

Pinned optional provider:
[scotty-mydevices3ef70c4a3bdf935e49e89b3ab59c2104d1c83f1f](https://github.com/scottydev40/scotty-mydevices/tree/3ef70c4a3bdf935e49e89b3ab59c2104d1c83f1f),
7 selected public files with exact sizes/SHA256 in
[provider research evidence](evidence/native-certificate-provider-2026-10-15.json).
Main project inspected atfe5aec038de4feb4615fd69c9330a9e61c84026e; no package or
installer was executed. Source ownership/license is independent of supported
service access; no legal determination or Google approval is inferred here.

- [Authorization description](https://github.com/scottydev40/scotty-mydevices/blob/3ef70c4a3bdf935e49e89b3ab59c2104d1c83f1f/docs/AUTHORIZATION.md): the authors call the
  gpsoauth-compatible exchange unsupported and distinguish it from public OAuth.
- [Token provider](https://github.com/scottydev40/scotty-mydevices/blob/3ef70c4a3bdf935e49e89b3ab59c2104d1c83f1f/src/token_provider.cc): it loads a master token
  from Secret Service and exchanges it at an Android authentication endpoint for
  a Nearby bearer. Request app/signature fields identify Google Play services,
  rather than rQuickShare's own registered desktop OAuth client.
- [Sign-in helper](https://github.com/scottydev40/scotty-mydevices/blob/3ef70c4a3bdf935e49e89b3ab59c2104d1c83f1f/src/signin_dialog.cc): an embedded WebEngine
  observes an oauth_token cookie, exchanges it and saves credentials. This is
  not a normal external-browser PKCE flow. No such helper was run here.
- [REST client](https://github.com/scottydev40/scotty-mydevices/blob/3ef70c4a3bdf935e49e89b3ab59c2104d1c83f1f/src/nearby_rest_client.cc): fixed Nearby v1
  device PATCH/publicCertificates transport uses a bearer. It is a different
  surface from the modern Identity RPCs in the pinned Google manager. Public
  source shows how it is attempted, not actual current server eligibility or
  Android/Windows interoperability for our account/application.

Inference: adopting that account exchange adds a high-value master credential,
embedded sign-in/cookie handling and an unsupported server dependency to the
product. Ordinary consent to continue development did not authorize those changes.
Separate-process isolation and Secret Service reduce exposure but do not convert
the exchange into an approved/public OAuth integration or prove server support.
No certainty of account sanctions, legal status or successful access is claimed.

## Significant product/security alternatives

| Option | Scope | Advantages | Costs and risks |
|---|---|---|---|
| 1 — supported authorization only; continue independent parity (recommended) | Retain manual PIN/Accept and the verifier. Keep operational Google-account integration gated on a concrete supported client/scope/service authorization path. Continue independent sharing work such as previews/resume while preserving the account gap explicitly. | Clear app identity and credential ownership; no broad master-token dependency or impersonated app fields; avoids coupling working transfers to an unstable sign-in mechanism. Preserves an eventual supported-provider seam. | Google Account/Contacts/Your devices and remembered Google-peer trust remain incomplete. A supported/partner access path is not currently established and may require external coordination; this option cannot claim full parity today. |
| 2 — explicitly experimental optional account provider | Design an isolated opt-in process for the researched unofficial account exchange, private Secret Service custody, narrow authenticated daemon boundary and bounded certificate transport/cache/rotation. Freeze the full security/recovery contract before code. No automatic sign-in/backend calls from selecting the plan. | Provides a concrete experimental route seen in readable primary implementation; credential/network ownership can be separated from transfer processing. May allow real certificate-acquisition investigation if the user later authorizes an account trial. | High-value master token and embedded-cookie exchange; request identity differs from our app; unsupported service may reject/change behavior. No proven server/peer acceptance. Requires substantial auth/storage/protocol/recovery/UX work and explicit opt-in, never silent anonymous downgrade or a full-parity claim. |

Recommend1 for quality, clear authority and maintenance, even though it leaves
an important parity gap. The existence of a working-looking third-party sign-in
path is insufficient to silently grant the application broader account access.
Option2 is a material product/security tradeoff, not a routine implementation
detail. The user selected option1 on2026-10-15; option2 is not authorized.

Neither option authorizes reading existing Google/Drive/browser credentials,
reusing unrelated app client IDs, entering credentials through chat, changing
the user's mount/auth configuration or signing/publishing native7. The earlier
Drive browser-login refusal remains respected and is not bypassed by this review.
If option2 is selected, account login/trial must be separately explicit; implementation
and offline fixtures can be prepared first with no real credentials or requests.

## Plan and continuation after selection

1. [x] P0: inspect Google source/developer docs and the independent provider's
   exact source/authorization; record receipts, supported-versus-experimental
   distinction, options, limitations and rollback before code.
2. [x] P1 after selection: option1 selected; account acquisition is a known external
   dependency and analyze the next independent sharing slice before freezing its
   material UX/security contract. No inert account toggle. The next scoped
   prerequisite is [bounded byte ingress](native-byte-ingress.md).
3. [ ] P2 only if2: freeze process/client ownership, fixed endpoint/auth identity,
   least credential exposure, secret store failure/lock/logout/revoke semantics,
   bounded HTTP/page/certificate limits, replay/refresh/cache/clock/rotation and
   crash recovery. Separate local signer/publication from peer resolution and
   manual consent. Document why each granted authority is necessary.
4. [ ] P3 only after P2: implement offline adapters and actual crypto/cache/provider
   tests with injected HTTP/clock/secret-store dependencies, never mocked domain
   verification. Keep sensitive tokens out of IPC/logs/Git/exports. No live login
   or account service request until explicitly authorized and concrete/reviewable.
5. [ ] P4: run affected development checks once; record actual observations and
   atomic commits. Native7 remains the current measured package until a future
   source-changing slice is built/verified. Preserve all recovery/release artifacts.

Review-only rollback is one docs commit. Later provider rollback must preserve
manual sharing and explicitly revoke/remove only the credentials it owns; never
touch unrelated application tokens or user accounts. No secret store, auth
surface or account capability has been enabled by this analysis.

## Actual verification

Read-only public-source/docs research. Downloaded files were bounded, pinned and
hashed; no downloaded code ran. No unaffected Rust/GTK/package tests or rebuilds.
Checked 63 local links, JSON and all7 pinned source sizes/digests.
Whitespace checked with git diff --cached --check before the atomic review commit.
Owned research commands exited; no watcher or running terminal is needed.
Terminal closed.
