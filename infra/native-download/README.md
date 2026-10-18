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

## Git-connected Cloudflare Builds

This Worker lives in a monorepo. In the Cloudflare `rquickshare-download`
Settings > Build screen, verify the connected repository is
`morozhenko-apps/rquickshare`, the **production branch is `master`** (there
is no `main` in this repository), and **root directory** is
`infra/native-download`. Recommended Build command:

```sh
npm ci --ignore-scripts --no-audit --no-fund && npm run check
```

Deploy command: `npm run deploy`. Both commands must run from the Worker root,
where `wrangler.jsonc` and `package.json` live. The Cloudflare build token
needs only the permissions required for the existing Worker, its one route and
its dedicated R2 binding. Inspect Build History after a production push; a
successful GitHub Actions check does not establish a successful Cloudflare build.

The credential-free [Native download Worker verification](../../.github/workflows/native-download-check.yml)
workflow runs a source contract, real local R2/APT test suite 10x and dry-run
for Worker changes on `dev`/`master`. It **does not block Cloudflare's own
production push trigger**; build validation must also be configured in
Cloudflare. The existing release-tag deploy remains a separate deploy writer.
Avoid enabling generic preview routes or public test R2 buckets just to exercise
the Git integration. See [verification record](../../docs/native-download-git-build-2026-10-18.md).
