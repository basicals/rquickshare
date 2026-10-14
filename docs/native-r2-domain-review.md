# Existing download domain and native APT routing decision

2026-10-14 current checkpoint: user selected
https://download.morozhenko.com/dist/rQuickShare/ (exact case) for production APT.
A narrow read-only Worker keeps the dedicated bucket separate from Picturix.
See [shared-prefix route SSOT](native-download-route.md) for current deployment/check status;
earlier domain options/pending-choice notes below are historical. Isolated signer
host and real public primary fingerprint still gate signed publication/enrollment.

2026-10-14. Mode A. User asks whether download.morozhenko.com can be shared with
Picturix. Read-only source/account inspection completed; no Picturix code, bucket,
domain, DNS, route, object or cache setting was changed.

## Actual observations

Wrangler4.151.0 reports download.morozhenko.com connected to existing app-dist R2
bucket, domain/ownership/SSL active, minTLS1.2, zone morozhenko.com. Picturix source
README/legal/operations confirms its release manifest at
https://download.morozhenko.com/dist/Picturix/latest.json.
This is an actual existing dependency, not an unused hostname.

The new dedicated rquickshare-native-apt bucket exists, Standard/WEUR, object count0,
no custom domains, r2.dev public access disabled. Its creation did not change
app-dist. Conditional publisher and external signing handoff now pass local domain/SDK/
HTTP APT checks in native-r2.md. These checks do not establish production R2
object writes or authorize a domain/routing change.

## Options requiring selection

1. Recommended: a dedicated single-level subdomain such as apt.morozhenko.com,
   directly connected to rquickshare-native-apt after checking existing DNS/usage.
   Benefits: separate object authority/credentials and hosting lifecycle, standard
   static archive, existing Picturix paths remain on their current origin, no new
   routing service in the download path. Costs: one additional DNS/domain binding,
   separate monitoring/cache policy and public bootstrap URI. Risks: existing DNS
   collision or wrong binding must fail preflight; TLS/domain readiness must be
   verified before enrollment. Best architectural fit for independent application
   distribution ownership, not merely less implementation work.
2. Share download.morozhenko.com with a dedicated /rquickshare/apt/ path, routed by
   a narrowly matched read-only Worker to the new dedicated bucket. Benefits: one
   user-facing download hostname, separate storage/write credentials retained.
   Costs: a routing Worker, deployment permissions, path normalization/streaming/
   cache/HTTP tests and operational ownership. Risks: an overly broad route or
   cache policy could affect Picturix; route matching must leave existing
   /dist/Picturix/* entirely with its current origin. Do not repoint the existing
   hostname to the new bucket or replace existing DNS to implement sharing.

Using a new folder in app-dist is technically possible but shares the bucket's
storage/credential/retention boundary. It is not the recommended isolated bucket
design chosen for this project. R2 domain binding itself maps a hostname to a
bucket, not a separate bucket per URL prefix; explicit routing is needed for
option2. No routing Worker or DNS mutation before selection (AGENTS3/3.1).

## Handoff

Domain choice pending. Existing isolated signer choice remains selected and is not
asked again; concrete signer host/hardware and public release fingerprint are
still missing operational inputs. Keep the private bucket and local work scoped;
do not publish artifacts, create production keys or change Picturix while waiting.

Primary source:
https://developers.cloudflare.com/r2/buckets/public-buckets/
Existing domain state came from authenticated Wrangler read-only list; Picturix
path came from local source. Candidate apt.morozhenko.com is a proposal, not a
claim of available DNS or a configured public service.
