# Native daemon startup and settings error policy

2026-10-12. Mode A. The user selected option 2 and requested an ordinary UI repair
path instead of manual JSON editing. Implementation SSOT: native-daemon-startup.md.
Previous state API work is complete in `7f41750`.

## Source evidence

- `core_lib/src/bin.rs` is a legacy developer runner, not a native daemon. It
  starts RQS::default, whose initial visibility is Visible, does not preserve
  desktop settings, and does not expose the authenticated Manager1 boundary.
- `app/main/src-tauri/tauri.conf.json` identifies the old desktop application as
  `dev.mandre.rquickshare`. Cached tauri-plugin-store 2.5.0 resolves `.settings.json`
  against AppData; cached Tauri 2.12.1 resolves Linux AppData to the data directory
  plus that identifier. The native reader must preserve this location/content,
  not create an empty new settings store.
- `app/main/src-tauri/src/store.rs::get_visibility` maps stored unsigned values
  through `Visibility::from_raw_value`, whose invalid branch in hdl/mdns.rs is
  `unreachable!()`. An invalid unsigned number can panic. Read/type failures in
  the getter return Visible. No corrupt user file or exploit was executed.
- Existing known numeric values are 0=Visible, 1=Invisible and 2=Temporarily.
  Fresh-install/missing-key defaults in the existing app are Visible; valid values
  must retain their existing meaning. No new default or automatic trust is proposed.
- `.settings.json` also contains port/download path and desktop-only keys. The
  first native reader should be bounded, read-only and preserve unrelated fields;
  no migration/write/reset. The daemon must not log setting contents or raw paths.
- SelectionBus already claims the unique well-known name before core startup,
  joins shutdown on stop/bus loss and keeps the runtime retained across waits.
  It accepts a Linux UnixStream; address discovery and OS signal ownership belong
  to the executable. No system-bus, TCP-control or unixexec fallback is authorized.
- The legacy desktop does not claim that native name. A shared process ownership
  guard at desktop/native startup is necessary to enforce the already accepted
  mutually exclusive launch modes. The new service name alone cannot exclude Tauri.

No user settings file was opened, no live service launched, and no package installed
during this analysis. Cached SDK sources and tracked repository sources were read.

## New decision: invalid stored visibility

Global AGENTS sections 3/3.1 require user confirmation for security/product behavior
changes. The existing diary explicitly leaves corrupt visibility recovery undecided
in offline-maintenance.md. Earlier native architecture/descriptor/page approvals
remain valid; this question only fixes startup behavior when saved intent is invalid.

| Option | Behavior | Benefits | Costs / risks |
|---|---|---|---|
| 1 — explicit startup failure (recommended) | Reject invalid visibility before claiming a service/starting discovery or transports; return a fixed actionable settings error; preserve original bytes | No guessed privacy setting, no accidental discoverability, no hidden reinterpretation or reset; repair is explicit | Background sharing is unavailable until the setting is corrected; the future client must explain startup failure |
| 2 — hidden degraded startup | Use Invisible for an invalid visibility field, preserve the original file and surface a persistent configuration warning; retain independently valid remaining settings | Service can stay available for diagnostics and local controls while advertising is hidden | Stored intent is reinterpreted; degraded status needs a new read/UI representation before shipping; hidden advertisements alone are not a network authorization boundary |

Recommend option 1 because this process owns unattended discovery/receive and there
is no implemented native settings-repair UI yet. A readable error is preferable to
inventing the user's privacy preference. The old application must not reintroduce
the same panic/implicit-Visible behavior when used for rollback after this policy is
selected. Apply the checked conversion and startup validation consistently where
settings initialize the core, without rewriting the file.

For both alternatives, an unreadable, oversized or malformed whole settings file
must not be treated as a fresh install or overwritten with defaults. The executable
returns a settings error before network startup; no verified port/download intent
exists to preserve in that case. An absent file/key keeps the existing fresh-install
default, not a new privacy default. Option 2 is limited to a valid settings object
with an invalid visibility field; it is not permission to guess other settings.

## Concrete implementation sequence after choice

1. Commit the selected policy, exact bounded settings/Unix-address/exit contracts,
   scoped branch/negative inventory and rollback points before implementation.
2. Add a native executable entry point and small testable startup adapter using
   the existing core crate. Resolve the user bus through the provided session
   address or standard user runtime directory; support bounded local Unix socket
   addresses only. Reject unsupported transports and malformed explicit values.
   Generate a fresh nonzero epoch from OS-backed randomness for each process.
3. Read the existing settings without modifying them. Validate known visibility,
   representable TCP port and optional download path; desktop-only keys stay in
   the file. Do not use legacy RQS::default as a fallback for failed settings reads.
   The headless service has no foreground window; do not pretend one is visible.
4. Install a shared exclusive process guard for legacy/native app startup before
   either starts transports, retaining it through joined shutdown. This is an
   internal mechanism for the accepted single-owner invariant, not another public
   control endpoint. Failure must preserve existing ownership and settings.
5. Retain NativeRuntime and SelectionBus; subscribe to SIGINT/SIGTERM before
   startup; stop through the cancellation token, then await owned cleanup. Do not
   detach a forever signal task or use process::exit before shutdown. Filter SDK
   diagnostic metadata; fixed startup failures must not expose private settings.
6. Verify pure settings/address/epoch/argument boundaries and real owned process
   cleanup using temporary settings, private brokers, local occupied-port failure
   and OS signals where needed. No BlueZ/mDNS or user-session launch in automated
   tests; injected external lifecycle fixtures must not mock core/domain logic.
7. Run appropriate affected checks once, document actual results and commit atomic
   changes. Add build/run documentation. Keep package activation/install and GUI
   migration separate; terminal/replay retention still needs review before a
   continuously running production service.

Rollback: additive executable/helper code can be removed without data migration;
shared ownership/startup guard changes are an independent checkpoint. Reverting the
visibility fix must never be represented as retaining the selected safety policy.
No autostart/service-menu file is installed until a real native client/chooser and
package lifecycle are ready.

## Handoff

Option 2 is selected; no further visibility policy approval is pending. The current
repository remains on dev with the fourteen-method library adapter and old GUI.
This review is documentation only. No tests were rerun on unchanged source.
All owned read/commit command sessions exited; terminal closed.
