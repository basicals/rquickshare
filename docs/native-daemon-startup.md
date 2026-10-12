# Native daemon startup and visibility recovery

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

- [ ] E0: Record selected policy, inventory and rollback, commit before code.
- [ ] E1: Add shared bounded read-only settings parsing; remove invalid visibility
  panic; preserve valid values; implement desktop startup validation, persistent
  warning and explicit visibility save. Verify core/desktop/frontend affected checks.
- [ ] E2: Add Linux native executable and process adapter: bounded local Unix user
  bus resolution, fresh nonzero epoch, shared exclusive desktop/native launch
  guard, registered SIGINT/SIGTERM, retained bus/runtime with joined shutdown and
  fixed diagnostics. Add startup-warning wire fixture and offline checks.
- [ ] E3: Record actual results, update build/handoff documentation and commit.

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

Pending. Existing 419-method core inventory remains in native-dbus-state.md.
