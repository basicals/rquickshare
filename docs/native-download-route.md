# Shared download hostname, isolated rQuickShare APT prefix

2026-10-14. Mode A. User selected the exact production base URI
https://download.morozhenko.com/dist/rQuickShare/ (case sensitive), superseding the
independent apt subdomain recommendation. Cloudflare/Wrangler and Workers Best
Practices skills read. Guide /home/morozhenko/.codex/AGENTS.md. Direct dev.

## Analysis and plan before code

Existing hostname is bound to app-dist and Picturix uses /dist/Picturix/latest.json.
Dedicated rquickshare-native-apt remains private/empty; isolated signing selected.
Authenticated zone route list contains no route for download.morozhenko.com and
the proposed new rquickshare-download Worker does not exist (404). Read-only DNS
and cache-rules API requests returned403 with current OAuth scopes; do not broaden
credentials or alter them. Existing R2 custom-domain state/TLS and live Picturix
HTTP preflight provide separate observations, not those unavailable API checks.

1. [x] D1: freeze exact prefix/static-read contract and rollback, capture bounded
   live Picturix manifest baseline; create narrow Worker/config/tests in
   infra/native-download, no changes in Picturix or its bucket/DNS/cache.
2. [x] D2: generated Wrangler types, strict TypeScript/static checks, actual local
   workerd+R2 integration/HTTP path/stream/range/HEAD/error isolation once and dry-run
   build/startup check. Package/client bytes remain unchanged.
3. [ ] D3: deploy the reviewed new Worker with only route
   https://download.morozhenko.com/dist/rQuickShare/* and dedicated R2 binding;
   verify actual route diff, public prefix/error/cache headers and unchanged
   Picturix manifest. No DNS repointing, wildcard-host takeover or app-dist write.
4. [ ] D4: bind producer/bootstrap/operator URI, record actual limitations/results,
   update SSOT/atomic commits/exact source export. Production signing remains gated
   on concrete isolated signer/public fingerprint; no fabricated release authority.

Rollback only the newly created exact route/Worker, restoring R2-origin404 for
this previously unused prefix. Keep dedicated bucket and existing hostname/origin
recoverable. No package install, source push, user settings/transfer/peer change.

## Frozen request/response boundary

Worker is a read-only static adapter to its dedicated R2 binding. Exact HTTPS host
and /dist/rQuickShare/ prefix, only GET/HEAD. Strip this prefix to existing APT
keys; no copy/rewrite of package metadata, no redirect/proxy to arbitrary hosts.
Allow only current InRelease, standard SHA256 by-hash indexes and native release
pool packages/sidecars. Reject private snapshots, mutable Packages/Release fallback,
unknown paths/queries/credentials, malformed escapes or traversal. Canonical URI
encoding needed for a Debian '+' version is permitted after one bounded decode;
decoded slashes/backslashes/controls/dot segments or double encoding are rejected.
Do not list keys or expose signing requests/audit snapshots. No upload/delete API,
signed URL, secret, crypto signer, global mutable request state or background task.

Pass R2 bodies as streams, never buffer a package. HEAD uses head() and no body.
Support one valid byte range on immutable files, including suffix/open-ended;
multi/invalid/unsatisfiable range returns416, If-Range mismatch gets the full body.
Pin ranged get to observed immutable ETag; a changed object returns503 rather than
mix old metadata/new bytes. R2 read errors return a finite503, no origin bypass or
recursive fetch. Conditional ETag GET/HEAD may return304 for immutable objects.
Mutable InRelease ignores Range/conditionals and always reads current bytes.

Successful immutable objects have a long immutable cache header, no-store for
InRelease and all errors; no Cache API/edge cache writes/negative caching. Fixed
content types, nosniff, quoted R2 httpEtag, bounded body sizes. Structured error
logging contains only a fixed event, never arbitrary path/body/account detail.
Disable invocation logs, workers.dev and preview URLs; no broad request logging.
Explicit route config preserves all existing routes, DNS and app-dist bindings.

## Verification inventory before tests

Focused Node/table domain tests for host/prefix/path/method, encoding/traversal,
same fixed key projection, no private/sibling access, range boundaries/If-Range,
ETag/HEAD/empty/missing/changed/read-failure and streaming without buffering.
Actual local workerd/R2 fixture integration and real APT HTTP projection under the
chosen prefix; isolated test server/runtime disposal, no production fixture upload.
Strict generated types/config/dry run/startup. Live deployment: exact route diff,
pre/post bounded Picturix manifest status/hash, native404 no-store on empty bucket,
POST refusal and no accidental root/private-object access. Preserve measured prior
Python/core/native results; no ten repetitions or irrelevant rebuilds.

## Primary references inspected

https://developers.cloudflare.com/workers/configuration/routing/routes/
https://developers.cloudflare.com/r2/api/workers/workers-api-reference/
https://developers.cloudflare.com/workers/best-practices/workers-best-practices/
https://developers.cloudflare.com/api/resources/workers/subresources/routes/methods/list/
Routes are case sensitive in path and most-specific pattern wins. R2 binding
supports head/get/conditional/range and streaming body. Current generated types
and Wrangler schema are checked before deploying; old skill snippets are not API
authority. Failed route-precedence web URL was replaced by the actual Routes page.

## Current state

D1/D2 complete locally; D3 deployment and D4 final handoff pending. User selected URI and authorized scoped route setup; no renewed
architecture/permission question is needed for that exact binding. Missing signer
identity gates production signing/enrollment only, not the selected read-only route.

## Development observations

Picturix baseline: curl returned200, 565 bytes, SHA256
73a2d36a8a1e4b37ba9ddd05266f66323b9a763a845a9de45defbc88238b7fdc,
ETag cf94ee5d7e30761435102fc0b4a01bb3 at2026-10-14T17:46:53Z.
Earlier Python urllib preflight returned403; curl establishes a separate successful
observation, not proof of its cause. No origin/WAF settings changed.

HEAD ignores Range per HTTP semantics and uses only R2 head. If-Range accepts a
matching strong ETag; date/weak/nonmatching validators receive the full response.
Generated Env uses pinned current Workers types. Package-lock pins local tooling;
no npm install scripts or global tool changes. Runtime integration owns and disposes
Miniflare/R2, loopback server and temporary TEST ONLY GPG identity.

D2 measurements: strict TypeScript passed. Nine policy/handler tests passed in
the initial10-test run; runtime test initially failed on Miniflare5 config changes
and fixture stdout framing, before production handling. Updated against installed
current schema (config/manifest/env) and isolated fixture stdout. Affected real
workerd/R2/APT scenario then passed1/0 in1.533s (test1.069s). Actual APT signature,
by-hash, encoded-plus .deb download/bytes and wrong-key refusal passed. Dry-run
bundle5.90KiB/gzip2.17KiB, only dedicated ARCHIVE binding. No fixture uploaded.

D4 implementation: native_apt bootstrap CLI defaults to the selected production
base while retaining explicit --uri for isolated channels. Its existing strict
verification and public Python API remain unchanged. Runtime fixture checks this
URI equals Worker policy. Native CI adds a separate credential-free local gate;
no deployment/signing credentials or remote workflow run.

After binding the bootstrap default, the affected real runtime/APT scenario passed
again1/0 in1.482s (test1.006s). Python syntax/CLI help, exact route/bucket/logging
config, CI YAML/contents:read and whitespace passed. No other passing scenario was
repeated. All owned test HTTP/GPG/workerd processes disposed; Terminal closed.
