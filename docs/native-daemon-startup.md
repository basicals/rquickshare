# Native daemon startup and visibility recovery

## Current native settings checkpoint (2026-10-13)

[native-settings.md](native-settings.md) now owns normal native startup/settings.
With no override, the daemon reads the private schema-1 native store under
`$XDG_CONFIG_HOME/io.github.morozhenko_apps.RQuickShare/native-settings.json` (BaseDirs
fallback). If absent, it reads the original legacy file without modifying it.
Only explicit Settings1 Save imports/replaces data with a private exact backup.
The daemon holds LaunchGuard across writes and joined restart generations.

Invalid whole-file configuration or unavailable receive-directory identity now
keeps a configuration-only Stopped service available for the secondary GTK editor,
without starting discovery/receive transports. Invalid legacy visibility alone
still starts hidden with a durable warning. Successful explicit save/live adoption
clears it. Save does not implicitly start a stopped service or interrupt transfers;
Settings offers an explicit idle-guarded start/restart and reconnect.

`--check` validates the selected source and custom-directory identity read-only.
`--settings PATH` keeps its original developer read-only semantics: no native
writer/import is activated, and GTK persistence is unavailable. Original legacy
configuration is preserved. Oversized/unreadable/unsafe storage cannot be silently
reset. The original slice and observations below are historical; its Manager1 wire
reference is superseded by the existing Manager2 plus Settings1 contracts.

2026-10-12. Mode A. The user selected option 2 from native-daemon-startup-review.md
and rejected a manual configuration-file repair workflow. This SSOT records the
implementation sequence before code. Existing descriptor/state/ownership contracts
remain authoritative. Valid settings retain their meaning; no automatic trust or
new fresh-install visibility default is introduced.

## Accepted recovery behavior

Invalid visibility in an otherwise valid settings object becomes effective
Invisible plus a persistent visibility-recovery warning. Do not rewrite the file
on startup, populate missing defaults or clear the warning through a transient
toast. The current desktop must present a durable warning and ordinary explicit
visibility choices, including choosing Hidden. A successful explicit save clears
the warning and applies the chosen visibility; failed save leaves the runtime and
warning unchanged. Unknown fields remain preserved by the existing store. Later
native GTK settings must expose this same interaction, not ask users to edit JSON.

The native daemon will expose `GetStartupWarnings() -> as` on Manager1, with the
fixed warning code `visibility_recovered` or an empty list. No raw value/file path
is exposed. The warning is a startup fact, stable for that daemon lifetime; native
settings mutation/reload belongs to the later GTK settings slice. This executable
does not replace the GUI yet; there is no native repair-screen claim in this slice.

Missing visibility retains the existing Visible fresh-install default. Null,
wrong type, negative/fractional/overflow/out-of-range values are invalid visibility
and recover hidden. Whole-file read/JSON/object/size errors reject startup before
transport activity, preserving original bytes. This distinction prevents silently
discarding the port, download path or unrelated preferences. No automatic backup,
reset or data migration is introduced.

## Implementation milestones

- [x] E0: Record selected policy, inventory and rollback, commit before code (`823cf68`).
- [x] E1: Add shared bounded read-only settings parsing; remove invalid visibility
  panic; preserve valid values; implement desktop startup validation, persistent
  warning and explicit visibility save. Verify core/desktop/frontend affected checks.
- [x] E2: Add Linux native executable and process adapter: bounded local Unix user
  bus resolution, fresh nonzero epoch, shared exclusive desktop/native launch
  guard, registered SIGINT/SIGTERM, retained bus/runtime with joined shutdown and
  fixed diagnostics. Add startup-warning wire fixture and offline checks.
- [x] E3: Record actual results, update build/handoff documentation and commit.

E1 and E2 are independently reversible. Do not switch default desktop packaging,
install activation/autostart/service-menu files or register on the user's session.
No transport framing/crypto, file capability or terminal/replay retention change.
Continuous production deployment remains gated on retention and physical checks.

## Concrete settings/launch contracts

- Read at most 1 MiB plus a one-byte overflow probe from a regular file; Linux
  opens nonblocking before type checking so FIFO replacement cannot hang startup.
  Error diagnostics contain fixed categories, not stored values or paths.
- The settings adapter preserves `.settings.json` in the old AppData directory
  (`dev.mandre.rquickshare`). Optional port/null map to absent; present integers
  must fit u16, including the old automatic port zero. Optional download_path/null
  maps to absent; strings retain their PathBuf meaning, rejecting NUL/empty values.
  Desktop-only keys are ignored by the read model and not removed from the store.
- GUI validates bytes before logger/store bootstrap because tauri-plugin-store
  build ignores load errors. Invalid-visibility startup skips default writes;
  frontend missing-autostart bootstrap also avoids automatic writes in that state.
- The daemon accepts only `--help`, `--check` and development `--settings PATH`.
  Check performs bounded validation only, without bus, lock, transport or writes.
  Normal startup uses existing AppData settings and user runtime directory.
- Resolve an explicit DBUS_SESSION_BUS_ADDRESS as a bounded single local Unix
  path/abstract address, with percent decoding and optional syntactically valid
  GUID. Reject alternatives, duplicate/unknown keys, malformed values and other
  transports. No implicit system-bus/TCP/unixexec/autolaunch fallback. Without an
  address use the validated user runtime directory's bus socket. GUID is not
  Google identity or an extra authentication guarantee.
- A shared owner lock under the user runtime directory is nonblocking, CLOEXEC,
  no-follow on its final file component, user-owned regular/private and retained
  through core stop. Validate the private runtime directory; never unlink a held
  lock, which would create competing lock inodes. It covers the updated legacy
  desktop/native executable, not arbitrarily old external builds or core fixtures.
- Install signal handlers before async startup. Retain startup/runtime owners;
  cancellation of bus connect/start must close/join retained ownership. No detached
  signal loop, timeout-based kill, process::exit or async Drop cleanup claim.
- Headless core is background, not an imaginary visible window. Native executable
  logs only fixed startup/status errors; it does not enable SDK/core payload traces
  through RUST_LOG. Diagnostic collection/redaction remains later work.

## Focused verification inventory

| Artifact | Scenarios | Level |
|---|---|---|
| Settings parsing/read | Existing valid modes/defaults/options; all invalid visibility types/bounds; corrupt/non-object/oversized/unreadable/nonregular source; exact byte cap; original bytes unchanged | Unit + disposable files |
| Desktop recovery | Effective hidden/visible status from backend; durable warning; every explicit mode; Hidden repair; save failure preserves warning/runtime; no bootstrap default writes in recovered state | Unit/component, backend pure seams |
| User bus parsing | Filesystem/abstract/percent/optional GUID; bounds; missing/duplicate/unknown/alternatives/unsupported transport; malformed environment; no fallback from explicit invalid input | Unit + local socket |
| Shared owner guard | Two contenders, release/reacquire, symlink/directory/permissions/foreign owner checks, CLOEXEC, no unlink; no core startup by rejected owner | Kernel file/lock |
| Process lifetime | Check/help no side effects; fresh nonzero epochs; early stop, occupied core port, missing/private bus, startup warning wire, joined SIGINT/SIGTERM/connection loss cleanup | Private broker/owned process fixtures |

Interactions: valid whole file x invalid visibility x no rewrite; save success/fail
x all three choices; GUI hidden/show x durable warning; native/legacy x same lock;
stop x connect/start/run; explicit address x wrong transport/fallback; restarted
process x epoch/warning persistence. Use real domain/runtime logic; inject external
addresses/files/stop signals only. No sleep polling or weak completion assertions.

N1/N2/N6/N7/N9/N10/N12 apply; N3 startup/socket failure; N4 guard/save repetition;
N5 exclusive owner and stop races; N8 Unicode/address bytes, no clock semantics;
N11 inapplicable. This Mode A plan is scoped verification, not repository-wide
coverage or ten-run stability certification.

## Verification observations

The plan was committed before source changes. Shared settings parsing now supplies
both the native process and legacy startup. Invalid stored visibility is Hidden
with a warning, without startup writes. The legacy frontend reads effective state
from get_receiving_status, not an unchecked numeric map. Its persistent repair
panel works even while outbound content hides the ordinary receive sidebar.
All three choices, including Keep hidden, call an explicit backend save. Only a
successful save updates the core and clears the warning. Event-publication failure
after a successful save is a fixed warning, not a false failed-save response.

Source review found the store's automatic retry could persist a failed visibility
choice after the UI reported failure. Disable automatic saves for this store;
existing setting editors already explicitly save. Valid fresh defaults are saved
only when missing and never populated during recovery. On explicit visibility
save failure restore the old cached field, retain the live Hidden mode and warning,
and prevent an automatic retry. Existing SDK whole-file writes are not upgraded
to a crash-safe transactional settings store in this slice; atomic migration,
concurrent editors and whole-file repair UI remain later settings work. Do not
claim byte preservation after an interrupted explicit SDK write.

The new Linux rquickshare-daemon has check/help/development settings arguments,
read-only settings, bounded local Unix address parsing, nonblocking CLOEXEC socket
connect, a fresh OS-random epoch and SIGINT/SIGTERM handlers registered before async
startup. Stoppable bus construction drops handshake ownership before a pump exists;
after pump construction cancellation joins/drains/closes it. The runtime host still
owns and awaits core shutdown. The shared lock is retained by native, updated
Tauri and the legacy developer core_bin; old external builds and direct library
fixtures are outside that process guard. Default cargo run remains core_bin.
GetStartupWarnings adds one authenticated Manager1 read method; current total is
15 methods plus StateChanged. Warning codes remain fixed and contain no settings.

### Measured checks

- Initial core compilation exposed incorrect rustix API paths and a shadowed stop
  token. Corrected to socket_with, io::fcntl_getfd/FdFlags and mkfifoat, and gave the
  pump its own named token. The completed all-feature invocation ran **429 unit/export
  methods: 428 passed, 1 failed**, no ignored, in 1.24 s. The failure showed that
  tempfile directories do not guarantee XDG runtime's required mode 0700. Correct
  private fixtures explicitly set that mode; the production owner check was not
  relaxed. All **6 affected process methods** then passed.
- The four new native executable integration methods passed (0.05 s), alongside
  the four existing modern_protocol methods (0.00 s). They launch only owned child
  processes with temporary settings/runtime directories and private sockets/brokers.
  Real SIGINT and SIGTERM arrive after observed SASL bytes; child wait and socket EOF
  prove process stop and lock release. Occupied-port startup fails before optional
  BlueZ/mDNS bootstrap and releases the actual private-bus service name.
- Initial desktop compilation caught the existing notification callback's missing
  new AppHandle argument. Updated that explicit-action caller. Desktop's different
  dependency lock also exposed three deprecated digest as_slice calls; portable
  slice indexing preserves the exact byte comparison and wire/crypto behavior.
  Desktop **10 methods** passed. After the later cache rollback/autosave correction,
  both affected backend persistence methods passed again, with strict Clippy.
- After digest conversion the affected core **102 hdl methods** and **40 manager
  supervision methods** passed, including real encrypted loopback transfers. The
  final settings read/invalid/FIFO/dangling-link cases passed all **6 methods**.
  Current core inventory is **429 unit/export + 8 integration = 437**; measurements
  above are complete/affected runs, not a final single 437-method run or a ten-run gate.
- Frontend lint/typecheck and **46 Vitest checks across 8 files** passed (4.30 s),
  including five new recovery checks. Lint initially reported ten formatting-only
  warnings in the new panel and an obsolete existing eslint-env directive under
  flat config. Corrected the panel formatting and removed that unused directive;
  affected files subsequently passed ESLint with --max-warnings 0. No behavior
  change followed the frontend test run, so its suite was not repeated.
- Final all-feature/all-target core Clippy and no-default-feature/all-target desktop
  Clippy passed with --locked -- -D warnings. Nightly formatting and diff checks
  passed. Existing core bluer future-incompatibility notice remains. Desktop lock
  resolved the shared dependency/feature links with **zero package version updates**.
  serde_json moved from dev-only to production; Tokio process enables owned child
  integration fixtures. No remote workflow, GUI/package or physical peer run.

### New regression counts and practical limits

**25 new checks**: 6 shared-settings, 6 process adapter, 2 bus, 4 executable
integration, 2 desktop backend and 5 frontend. Parameterized rows are not counted
again. Primary categories: Positive 7; N1 3; N2 1; N3 1; N5 2; N6 3; N7 2;
N9 3; N10 1; N12 2. N4 repeats/guard release and N8 encoded bytes are secondary
coverage; N11 introduces no billing. Real foreign-UID filesystem ownership, disk
full SDK partial writes, successful live SDK/bootstrap and graphical notification
presentation were not manufactured/certified. No exhaustive line/branch or executed
mutation-tool claim is made.

Assertions kill automatic-visible fallback, removed count/type/port/address guards,
unlocked parallel launch, missing CLOEXEC/no-follow, stale warning disappearance,
early core update before persistence, lost owned startup socket and unjoined process
stop. The subject uses actual settings/core/runtime/locks/brokers; backend persistence
failure injects only the external store adapter. No source data was written or
deleted except disposable fixture settings. No user configuration was read during
development analysis or changed by these checks.

## Handoff and next work

This executable is manually buildable but not activated or the default packaged
app. The old UI now offers visibility recovery; the future GTK client must consume
GetStartupWarnings and provide equivalent explicit settings save/reload. Native
warnings currently remain a startup fact for the process lifetime. The native
headless service's owner lock excludes launching Tauri while it runs; do not tell
users the old GUI can attach to it. A complete native settings client remains a
required migration gate, not an implemented screen.

Next: bounded terminal/replay retention review before continuous operation; native
client/GTK panel and notification/consent presentation; full settings migration and
repair; Dolphin; package/installation and real Google peer validation. Whole-file
corruption still fails read-only startup; this accepted recovery applies to an
invalid visibility field in an otherwise usable settings object.

All owned commands completed; terminal closed. Child tests wait/reap owned daemons
and private brokers; no watchers or user-session service remain. No installation,
activation/autostart/menu registration, user policy write, push or master promotion.
