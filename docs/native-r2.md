# Native APT on Cloudflare R2 with external signing

2026-10-14. Mode A. User selected R2 storage and isolated signing outside CI.
Guide /home/morozhenko/.codex/AGENTS.md; Cloudflare and Wrangler skills read.
Direct dev. No production key creation, installation/enrollment, live daemon/peer,
branch/push or unrelated project mutation. Existing local APT contracts/checks
remain authoritative in native-apt.md.

## Analysis and plan before code

Wrangler4.151.0 installed in task-only /tmp/rqs-r2-tools.9Yex3O, Node24.21.0.
Existing OAuth identifies one account; no login/credential export is needed. Do
not place its email/token/config in Git. Production domain and actual external
signer host/hardware are still unspecified. R2 is selected, not a grant to invent
those values or generate a release identity on this workstation.

1. [x] R1: inspect R2 account readiness; reserve a dedicated private empty bucket
   if supported/absent, without enabling r2.dev, custom domain or object exposure.
   Record actual outcome; never alter an unrelated bucket or billing enrollment.
2. [ ] R2: bounded toolkit-independent publication owner and R2 S3 adapter. Verify
   signed snapshots/previous authority first; create immutable pool/by-hash/audit
   snapshot objects, then compare-and-swap only the public InRelease commit point.
   No unsigned upload, deletion, overwrite of immutable bytes or blind write retry.
3. [ ] R3: external signer handoff/explicit approval manifest and operator commands,
   protected publisher separation, least-privilege credentials and pending concrete
   binding. No signing service/API or CI signing key is silently introduced.
4. [ ] R4: focused domain/adapter checks once, actual account/bucket observations,
   dry-run/publication plan for existing artifacts, SSOT/atomic commits/export.
   Public enrollment/promotion requires a chosen domain and real signed authority.

Rollback new publisher/handoff independently of existing APT producer; leave the
new empty private bucket recoverable rather than deleting account resources.
Rust/UI/settings/transfer behavior and existing package bytes remain unchanged.

## Frozen ownership and publication contract

Dedicated bucket rquickshare-native-apt, Standard class; no automatic deletion or
lifecycle expiry. Default/private unless explicitly bound to the chosen domain.
Direct custom-domain R2 serves the standard APT tree; no Worker, server-side signer
or public upload endpoint is needed for this selected static archive. Cloudflare
documents r2.dev as development/rate-limited, so it is not the production URI.

R2's conditional PutObject is atomic; use If-None-Match:* for immutable objects
and first InRelease, If-Match:observed-etag for replacing InRelease. Wrangler
manages account/bucket readiness, but its object CLI does not expose these write
conditions: use the documented S3 SDK for the publication adapter, not unsafe
check-then-unconditional-put commands. Credentials are explicit bucket-scoped R2
object read/write inputs, never ambient AWS discovery or the build job's secrets.
Disable SDK write retries; one admitted write owns its outcome/readback.

Candidate and supplied previous snapshots must pass existing pinned-key signature,
cleartext, package/hash/tree and revision checks. Read the actual current InRelease
through the authenticated storage API, compare bytes with the expected previous
authenticated snapshot, retain its exact ETag. A new bucket requires explicit
first-release. An already-current exact candidate may be acknowledged idempotently;
a competing generation conflicts rather than overwrites. Root Release/Release.gpg
are not uploaded: their pair cannot be atomically committed. Modern target APT uses
the single signed InRelease; missing/unsupported metadata fails closed. All three
signed files remain together in immutable audit snapshots.

Upload every immutable pool/by-hash object first, verify existing/create-raced
objects by streamed bounded SHA256/size, keep historical objects. Candidate normal
Packages files are stored only inside immutable audit snapshots; public index
objects use by-hash, already advertised by the signed Release. APT's target clients
must use by-hash (explicit bootstrap By-Hash:force) rather than mutable index paths.
This avoids a loser publisher changing the winner's public index/metadata files.
InRelease is the sole mutable public commit point, Cache-Control:no-store;
immutable objects use long immutable cache headers. Custom-domain cache rules must
respect that distinction and must not cache negative responses to new object keys.

An ambiguous immutable write may be adopted only after exact positive readback;
a missing object does not prove the earlier request cannot still commit. Do not
promote after unresolved object writes. An ambiguous final CAS may be acknowledged
only when exact candidate bytes are observed; otherwise report Uncertain, no blind
CAS replay, deletion or rollback. A subsequent explicit attempt re-reads authority.
ETags are opaque CAS tokens, never substituted for cryptographic package digests.

Signing stays outside build/publish CI. A public request manifest binds candidate
Release and all admitted bytes, expected previous snapshot, source commit and
expected public primary fingerprint. Source commit text is an assertion, not build
provenance. An operator must independently approve the exact request digest/build
provenance before signing on the selected isolated signer. No automatic signing
oracle, uploaded primary key or chat secret. Production signing identity, recovery,
revocation/rotation and metadata refresh operator remain concrete binding inputs.

## Verification inventory before tests

Mode A focused domain cases with a fake external conditional object store, real
snapshot/GPG verification: unsigned/wrong-authority rejection before storage I/O;
first/current/next/no-op, exact bytes/order/headers, immutable conflict/race,
concurrent CAS conflict, denied/failed/ambiguous admission, positive readback and
missing-after-uncertain with no promotion/replay. SDK contract uses a real modeled
S3 client plus external Stubber only; never mock publication logic. Approval digest/
source/previous binding tables; no external signing API or production key. Preserve
existing test results, no repeated Rust or ten-run gate. Actual public domain/APT
and installed lifecycle remain acceptance, not fake-store evidence.

## Primary sources inspected

https://developers.cloudflare.com/r2/buckets/public-buckets/
https://developers.cloudflare.com/r2/reference/consistency/
https://developers.cloudflare.com/r2/api/s3/extensions/
https://developers.cloudflare.com/r2/reference/wrangler-commands/
R2 has strong conditional commits; a missing read cannot settle an unknown in-flight
write. Custom domain is the production serving path, caching is separate from R2
consistency. Official S3 SDK references govern request fields. Local skill numbers/
examples are not treated as current API evidence.

## Current state

R1–R4 pending after this plan. No bucket/domain/object/key modification yet. Skills
do not impose a separate permission gate; user selection authorizes scoped private
bucket setup and reviewable source work. Missing domain/signer identity gates only
dependent public binding and production signing, not local implementation.

R1 complete: dedicated bucket rquickshare-native-apt created2026-10-14 17:02:46 UTC,
Standard, actual locationWEUR, observed object_count0/size0B. Existing other
buckets were only listed and not modified. No r2.dev/custom domain/public setting
was enabled, no object/token/production key uploaded, no billing enrollment change.
Task-only Python venv /tmp/rqs-r2-python.ev80Fc contains boto3/botocore1.43.113
and its pinned dependency graph for SDK contract checks; user Python is unchanged.
Wrangler4.151.0 read/create/info completed; all owned terminals closed.

User domain clarification: download.morozhenko.com already serves app-dist and
Picturix /dist/Picturix/latest.json. Actual Wrangler read confirms domain/SSL active.
New native bucket still has0 objects/no custom domains/r2.dev disabled. Read-only
Picturix guide/source inspection only, no sibling edits. Domain/routing choice is
recorded in native-r2-domain-review.md; recommend independent apt.morozhenko.com
after DNS preflight, alternative a narrowly scoped shared-host Worker route.

R2 source is staged but not yet behavior-tested or wired into CI: native_r2.py
contains verified snapshot planning, immutable create/readback and InRelease CAS
with explicit bucket-scoped credential adapter. Optional boto3 dependency graph
is pinned in requirements-r2.txt. Single PUT uses documented Content-MD5 transport
checksum, while signed SHA256/readback supplies actual content authentication; no
assumed full-object SHA256 S3 feature. R2–R4 tests, signer approval handoff and
By-Hash:force bootstrap/actual APT projection checks remain pending. Do not present
this staged publisher as operational. Production source/transfer code is unchanged.
Pause before significant domain/routing choice under AGENTS3/3.1, not a skill
approval rule. Existing outside-CI signing selection remains recorded.
