# Native APT on Cloudflare R2 with external signing

2026-10-14 custody update: user selected the existing Linux workstation; no
additional host/VM/token. Protected local certification/signing profiles and
operator initialization are implemented/tested in [local signing](native-local-signing.md).
Earlier separate-host requirements below are superseded. Production keys are now
initialized and real subkey/gpgv verification passed. Recovery destination is the
existing Google Drive ArcVault; encrypted backup/local restore/mounted copy passed,
independent cloud readback remains pending (native-signing-backup.md). Release
approval and actual publication acceptance remain open.

2026-10-14 current checkpoint: user selected
https://download.morozhenko.com/dist/rQuickShare/ (exact case) for production APT.
A narrow read-only Worker keeps the dedicated bucket separate from Picturix.
See [shared-prefix route SSOT](native-download-route.md) for current deployment/check status;
earlier domain options/pending-choice notes below are historical. Isolated signer
host and real public primary fingerprint still gate signed publication/enrollment.

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
2. [x] R2: bounded toolkit-independent publication owner and R2 S3 adapter. Verify
   signed snapshots/previous authority first; create immutable pool/by-hash/audit
   snapshot objects, then compare-and-swap only the public InRelease commit point.
   No unsigned upload, deletion, overwrite of immutable bytes or blind write retry.
3. [x] R3: external signer handoff/explicit approval manifest and operator commands,
   protected publisher separation, least-privilege credentials and pending concrete
   binding. No signing service/API or CI signing key is silently introduced.
4. [x] R4: focused domain/adapter checks once, actual account/bucket observations,
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

2026-10-14 continuation: user asked to resume interrupted work. Complete independent
R2–R4 local checks/handoff first; no domain/routing mutation is authorized by that
continuation. The previously recorded publisher is source-only until these checks.
Add a canonical public signing request (schema1, source commit, pinned primary,
candidate file digest/size inventory and authenticated previous InRelease hash).
Approve the independently reviewed request SHA256 explicitly before invoking the
existing local signing primitive on the selected isolated host; compare request
again after signing so mismatched bytes cannot be handed to publication. This
does not authenticate the asserted source/build provenance by itself.

Focused checks include one actual HTTP loopback APT acquisition against the public
projection (InRelease + pool + by-hash only), generated By-Hash:force sources and
no mutable index/root Release fallback. An ephemeral test-only signing subkey
must verify under its pinned primary. SDK retries/no ambient discovery/conditional
request fields and body cleanup are checked with a real botocore modeled client.
Test-only HTTP/GPG owners close/join in finally; no user services are touched.
Remove only the task-created .wrangler/cache/wrangler-account.json and empty
parent cache directories; never add account cache/credential files to Git.

Initial R2/private-signer contract run7/0 passed6.916s. Actual loopback HTTP APT
accepted/downloaded the public projection without root Release/Release.gpg or
normal Packages paths, using by-hash only; wrong public authority still failed.
Real ephemeral signing subkey verified under its pinned primary. Conditional
publication is tested through a fake external store, SDK request shape through
botocore Stubber: this is not a real R2 PUT/caching/permissions acceptance claim.
A bootstrap By-Hash:force assertion and existing file-transport helper changed;
run those two affected existing APT scenarios once before finalization.

CI must install the optional pinned SDK in its own Python venv before the existing
test_native_*.py discovery (no desktop dependency or signing/publish credentials).
Record docs/commands and remove the task-owned account cache; no package README
asset/Rust change means no package rebuild or unchanged Rust suite rerun.

## Completed local stages and measured checks

R2/R3 local implementation complete: conditional publication rules and optional
boto3 adapter are independently injectable. Public signing request/approval is
implemented in scripts/native_signer.py; only the explicitly approved canonical
request digest permits invoking the existing isolated GPG primitive. It binds the
previous authenticated InRelease, candidate file hashes/sizes, public primary and
asserted source commit. Signed copied input is checked before local output commit.
Public signing subkey verification remains pinned to its primary fingerprint.
This guard is not a source-provenance attestation or a substitute for operator review.

R4 local checks complete: publication/signer/S3/HTTP APT7/0 in6.916s; affected
bootstrap1/0 in0.996s and existing isolated file-transport APT1/0 in0.691s. Real
loopback HTTP APT fetched InRelease/by-hash/pool without mutable Packages or root
Release fallback, and rejected a wrong key. External R2 writes are modeled by a
conditional fake store, SDK contracts by botocore Stubber. Conditions/cache/body
length/SHA256/cleanup, denial/409/412/500/network classification, same/current/next
publication, conflicting immutable/CAS, positive admitted readback and no blind
retry/promotion after uncertainty are exercised. Seven new test methods contain
table-driven branches; no coverage/ten-run or real-R2-PUT claim.

Python syntax checks, native workflow YAML/contents:read and whitespace passed.
CI now installs the pinned optional SDK in a task venv before scoped discovery,
not the desktop runtime. Native package/Rust/service code and package README assets
are unchanged; prior actual package/Rust checks are preserved without repetition.
No rebuild or official source-version signing was claimed. Removed only this
task-created Wrangler account cache, ignore future .wrangler output. All owned
terminal/test HTTP/GPG owners closed; Terminal closed.

Actual account observations remain R1: private empty dedicated bucket, no custom
domain, r2.dev disabled. Existing download.morozhenko.com/app-dist/Picturix unchanged.
Production hostname/routing choice is requested in native-r2-domain-review.md.
Operational signer host/hardware/release fingerprint and bucket-scoped publication
credential are also missing; never paste secrets into chat. These gate real signing,
R2 object PUT/public promotion/enrollment, not completed local implementation.

## Operator handoff

Use scripts/requirements-r2.txt in an isolated Python venv. Build CI has no GPG
private key or R2 write credential and uploads unsigned artifacts only. Independently
audit the exact unsigned snapshot/source/build before creating a public request.
The following are templates for the selected external signer, not configured hosts,
keys, grants or commands to enroll this workstation. Retain the previous complete
authenticated snapshot locally; --first-release is only for an empty new channel.

```sh
python3 scripts/native_signer.py request \
  --repository /audited/unsigned-snapshot \
  --keyring /audited/public-primary-and-subkeys.gpg \
  --fingerprint FULL_UPPERCASE_PRIMARY_FINGERPRINT \
  --source-commit FULL_LOWERCASE_COMMIT \
  --previous /audited/previous-signed-snapshot \
  --output /existing-parent/signing-request
```

Transfer only the request, public keyring, candidate files and expected baseline
to the isolated signer. Review source/build provenance independently and approve
request SHA256 via a trusted human process. A hash copied from untrusted CI is
not approval; the wrapper deliberately requires an explicit approval argument.

```sh
python3 scripts/native_signer.py sign \
  --repository /audited/unsigned-snapshot \
  --keyring /audited/public-primary-and-subkeys.gpg \
  --fingerprint FULL_UPPERCASE_PRIMARY_FINGERPRINT \
  --source-commit FULL_LOWERCASE_COMMIT \
  --previous /audited/previous-signed-snapshot \
  --request /audited/signing-request/request.json \
  --approve-sha256 INDEPENDENTLY_APPROVED_REQUEST_DIGEST \
  --gnupghome /external-isolated-signing-home \
  --output /existing-parent/signed-snapshot
python3 scripts/native_r2.py plan \
  --repository /audited/signed-snapshot \
  --keyring /audited/public-primary-and-subkeys.gpg \
  --fingerprint FULL_UPPERCASE_PRIMARY_FINGERPRINT \
  --previous /audited/previous-signed-snapshot
```

Verify the returned signed snapshot/public fingerprint again in the publisher
environment. The publisher needs explicit account identity and bucket-scoped object
read/write credentials in RQS_R2_ACCESS_KEY_ID/RQS_R2_SECRET_ACCESS_KEY; the CLI never
prints or accepts secret values as arguments and does not use ambient AWS keys.
Replace plan with publish and add --account-id PUBLIC_ACCOUNT_ID only after the
concrete domain/cache/authority binding is reviewed. SDK total_max_attempts=1;
10s connect/30s read limits surface explicit Unavailable/Uncertain, not retry or
a guessed successful completion. Accepted object PUT cannot be retracted.

Offline primary: retain encrypted offline backup/revocation material independently
from build and publisher. Provision a separately scoped signing subkey on the
selected isolated signer; never make its private key a repository/CI/output artifact.
Concrete hardware/host/custodian and authorized refresh process must be supplied
before production provisioning. Metadata expires after seven days, so that process
must review/sign/publish refreshed metadata while preserving version/package bytes.
No unauthorised cron, background signing daemon or key-generation action is included.
Rotation/revocation/bootstrap fingerprints require independent confirmation; the
current exact-single-primary verifier fails closed on a multi-primary keyring until
a deliberate rollover contract is reviewed.

## Remaining acceptance and recoverability

Domain/DNS conflict and exact route/cache/TLS readiness, public HTTP APT signature/
by-hash/download, actual R2 bucket-scoped condition/checksum/denial/readback, isolated
signer recovery/refresh and installed upgrade/removal/live-transfer/KDE/Google
acceptance remain open. Local tests do not prove any of those external outcomes.
Native2 package/unsigned snapshot keep the earlier native-apt.md provenance and
expiry; no production key was invented to make an apparently publishable artifact.
Rollback local signing/publisher/test/CI commits independently; retain the empty
private bucket rather than deleting it. Direct dev only, no push or sibling edits.
