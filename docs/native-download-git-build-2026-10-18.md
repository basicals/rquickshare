# Native download Worker: Git build verification

2026-10-18. MODE B: testing/verification only. Scope is `infra/native-download` in
`morozhenko-apps/rquickshare`. This is a deployment-pipeline audit, not a new
package publication, production Worker behavior change or signing decision.
Read [native-download-route.md](native-download-route.md) and
[infra/native-download/README.md](../infra/native-download/README.md) first.

## Initial repository scan and invariant review

- `dev` is the development branch; `master` is the actual default/release
  branch. No `main` exists at the initial checkpoint.
- `wrangler.jsonc` names `rquickshare-download`, with a single case-sensitive
  `https://download.morozhenko.com/dist/rQuickShare/*` route, private
  `ARCHIVE` R2 bucket, no workers.dev/preview URLs and redacted query logs.
- `src/policy.ts` implements the public-key allowlist, byte ranges and ETags;
  `src/worker.ts` implements read-only HTTP serving with no write or fallback.
- `test/policy.test.mjs` has four tests, `test/worker.test.mjs` five, and
  `test/runtime.test.mjs` one real local workerd/R2 plus isolated signed APT
  integration. Existing Node engine/toolchain target is Node 24.
- `native-package-smoke.yml` validates this Worker only on manual dispatch;
  normal `dev` pushes run Preflight but do not execute this Worker test suite.
  `native-tag-publish.yml` also calls `wrangler deploy` when the release
  pipeline is authorized. This is a separate possible deployment writer and must
  be retained until its release ownership is explicitly reviewed.

## Test inventory, branch coverage and N1–N12 review

| Production artifact | Risk / level | Existing test scenarios and required branch assertions |
| --- | --- | --- |
| `policy.ts::objectPolicy` | High, unit | Accepted immutable/mutable keys, versions/architecture/sidecars; wrong host, scheme, prefix, queries, credentials, path escape, encoding, size, unknown/private objects |
| `policy.ts::byteRange` | High, unit | Open/suffix/bounded ranges and minimum/maximum; invalid, reversed, multirange, overflow, unsatisfiable sizes |
| `policy.ts::matchesEtag` | Medium, unit | Matching/nonmatching strong and weak validators, wildcard, list, null and excessively long input |
| `worker.ts::serve`, `failure`, `valid`, `headers`, default fetch adapter | High, HTTP/contract | GET/HEAD/405/404; mutable freshness; ETag/304; range/206/416; invalid R2 lengths, missing/changed/throwing storage, fixed no-PII error and streaming |
| `wrangler.jsonc`, generated `Env` | Critical, build/contract | Strict types, generated-binding file stable, bounded route/name/R2 settings reviewed; bundle dry-run |
| workerd+R2+HTTP+APT bridge | Critical, integration | Local actual Worker and R2, approved public keys, signed test-only APT download, by-hash, wrong signing-key refusal, isolated filesystem and cleanup |

The positive/negative matrix for these existing scenarios maps N1 (bad input), N2
(bounds), N3 (read failures), N4 (ETag/re-read consistency), N5 (changed object
between `head`/`get`), N6 (invalid metadata), N7 (unauthorized methods/paths),
N9 (interrupted/read errors), N10 (path/secret redaction) and N12 (missing/corrupt
storage) to existing tests. N8 (locale/time) and N11 (monetization) are not
applicable to this stateless read-only projection. All identified branches must
remain under the existing tests; the CI change adds no new production branch.

Key interaction matrix: mutable/immutable × GET/HEAD × Range/If-Range/If-None-Match;
missing/changed/corrupt object × metadata/body; valid/invalid host/path × method.
Test inventory is 10 existing top-level test scenarios with table-driven subcases,
not 10 newly written tests. Per-file coverage map: `policy.ts` three exported
functions, `worker.ts` one exported serving function plus its three internal
helpers/default adapter, `wrangler.jsonc` one deployment contract. Exact numeric
branch coverage has not been measured; do not describe this as 100% coverage.
Mutation targets include changing an allowlist predicate, object maximum, range
bound, conditional validator, storage metadata check or cache policy.

## Plan and acceptance (documentation before implementation)

1. [x] Review source, current deployment evidence and branch ancestry.
2. [x] Add a credential-free, path-filtered GitHub Actions Worker verification on
   `dev`/`master` and PRs. Pin Node 24, install only local APT/GPG test
   prerequisites and locked Worker dependencies. Run type generation/diff,
   TypeScript check, all 10 existing tests ten consecutive times and Wrangler
   dry-run. No remote Worker deploy or release package build.
3. [x] Observe exact-source CI result; record successes/failures and revise this
   document. Do not claim CI green based on historical runs.
4. [x] Check `master...dev` again, require fast-forward ancestry and green
   verification before moving `master`, without force or merge commit. Observe
   subsequent CI and Cloudflare build where access permits.

Cloudflare Workers Builds configuration is **outside Git**. Operator must
independently confirm in Worker Settings > Build: repository
`morozhenko-apps/rquickshare`, production branch `master`, root directory
`infra/native-download`, Worker name `rquickshare-download` matching
`wrangler.jsonc`, and suitable build/deploy commands. Suggested minimum
build validation is `npm ci --ignore-scripts --no-audit --no-fund && npm run check`;
deploy command `npm run deploy` from that root. A GitHub status check alone
does **not** gate a simultaneous Cloudflare production build. For production
safety, configure any desired checks in Cloudflare's build command and branch
protection before treating them as deployment gates.

Cloudflare Build trigger settings, token scopes, live version and routes cannot
be verified from a GitHub checkout. The historical 2026-10-14 deployment
evidence is not proof of a 2026-10-18 Git-connected deployment. Public live
HTTP probes are inconclusive in this environment (DNS resolution is blocked).
Neither a clean GitHub check nor a successful Worker dry-run proves production
deployment. Do not create an APT signer, new route or alternative public bucket
as part of this verification.

Rollback: revert the isolated workflow/doc commits; neither changes production
HTTP semantics. If a later Cloudflare deployment fails, inspect the Cloudflare
build history and restore only a known-good Worker version, preserving R2,
signing keys, sibling routes and Picturix.

## Verification receipts and follow-up

First Worker workflow: [run 39121334435](https://github.com/morozhenko-apps/rquickshare/actions/runs/39121334435)
on source `26ecc1650533dd3e37bad050d7d0030847d55b36`: **SUCCESS**.
Node 24 and locked install, generated bindings/diff, TypeScript check, ten
complete executions of the existing 10-scenario suite (including real local
workerd/R2/signed test-only APT) and Wrangler dry-run all passed. That is a
scoped 10x Worker verification, not proof of 100% branch coverage or the global
MODE B completion gate. Preflight status for the source is tracked separately.

The follow-up README-only Worker-root change intentionally makes Git build watch
path configurations that include `infra/native-download/*` eligible to trigger
when later fast-forwarded into production. Its exact-source GitHub Actions
verification must be observed before promotion. Cloudflare build settings,
actual deployment and public HTTP acceptance remain independently unverified.

## Production Git build result (2026-10-18)

Before promotion, the exact-source Worker run
[39121547650](https://github.com/morozhenko-apps/rquickshare/actions/runs/39121547650)
and five-job Preflight
[39121547768](https://github.com/morozhenko-apps/rquickshare/actions/runs/39121547768)
both passed on `a6c0935f1cfed21c20b621d38857cb441a5b441e`.
GitHub compare proved `dev` was 44 commits ahead of `master`, 0 behind.
`master` was fast-forwarded with no force and both branch heads were independently
read back equal at that SHA.

Cloudflare's GitHub check
[`Workers Builds: rquickshare-download`](https://github.com/morozhenko-apps/rquickshare/runs/117659138240)
reported **SUCCESS** for that exact SHA on 2026-10-18 11:07:06 UTC.
Its detail URL is the
[production build](https://dash.cloudflare.com/e5501830b73f9b3b7c730555e879856a/workers/services/view/rquickshare-download/production/builds/fd9e5ca2-6763-4f39-abcd-3e03620cab75);
Build ID `fd9e5ca2-6763-4f39-abcd-3e03620cab75`, deployed Worker
version `5969d799-f1d2-4524-b96d-d01b7fc2d586`.
This is a distinct version from the former manually deployed
`3d3a4542-b0c8-4717-a615-bf85efd6fd77`.
A second Worker gate on `master`
[39121920238](https://github.com/morozhenko-apps/rquickshare/actions/runs/39121920238)
also passed. Master Preflight
[39121920293](https://github.com/morozhenko-apps/rquickshare/actions/runs/39121920293)
was still running its desktop Rust job at this record's cutoff; source-identical
five-job Preflight already passed on `dev`.

This final documentation-only commit intentionally carries `[skip ci]`, so it
does not request another production deployment merely to record its receipt.
The deployed Worker source and configuration are unchanged by this note.
Remote runtime HTTP/DNS/sibling-route verification remains unobserved from this
tool environment and is not represented as passed. The release-tag workflow's
separate Wrangler deployment writer remains an operational ownership review.
