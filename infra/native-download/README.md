# Native APT download adapter

Production base: https://download.morozhenko.com/dist/rQuickShare/ (case sensitive).
Only this exact HTTPS route belongs to rquickshare-download. Existing Picturix
routes, DNS and app-dist objects are outside its ownership. The private
rquickshare-native-apt bucket remains without a public custom domain/r2.dev URL.

See [route SSOT](../../docs/native-download-route.md) for decisions, measured checks,
deployment identity and outstanding acceptance. [Publication](../../docs/native-r2.md)
owns signing, immutable objects and conditional InRelease promotion. This Worker
only projects approved public keys to streams; it never publishes/signs/lists.

## Local checks

Node24+, Python3.11+, APT, dpkg-deb, GPG/gpgv, Linux. No Cloudflare credentials
or production network is required for tests. Native CI contains the same gate.

```sh
cd infra/native-download
npm ci --ignore-scripts --no-audit --no-fund
npm run types
git diff --exit-code -- worker-configuration.d.ts
npm run check
npm test
npm run dry-run
```

Generated binding Env uses pinned Workers types. Tests include actual workerd/R2,
an isolated signed TEST ONLY channel and real APT update/candidate/download with
wrong-key refusal. All storage, GPG state and HTTP/runtime owners are temporary.
Production signing keys, package installation and host APT settings are not used.

## Operations and rollback

Review committed source/config and compare current zone routes with the recorded
baseline before `npm run deploy`. Wrangler owns only the one declared route and
Worker. Never repoint download.morozhenko.com or bind the entire hostname here.
Do not add broad routes, redirects, origin fetch fallback, cache overrides or
public audit snapshot access. Production binding is read with head/get only.

Empty/unpublished channel returns404 with no-store, including the prefix root.
That is expected until a real approved signing authority/publication exists.
Errors carry no-store; mutable InRelease always returns fresh complete bytes.
Immutable files support streaming, ETags and single byte ranges. HEAD ignores Range.

Rollback: delete only the route ID recorded in the SSOT with the authenticated
Cloudflare Workers Routes API; this returns this prefix to its former R2 origin.
Then delete the new rquickshare-download Worker if no longer needed. Do not delete
the dedicated bucket, sibling routes, DNS records or app-dist objects. Verify the
Picturix manifest before/after. Package/signing/publisher commits roll back separately.
