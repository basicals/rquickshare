# Hardware test automation and local-access plan

_Status: hardware runner/ADB proposal; offline evidence analysis implemented 2026-10-12. No physical-device automation has been established by this document._

## Goal

Replace repetitive manual Pixel ↔ Ubuntu Quick Share smoke-test steps with a reproducible hardware regression harness. Keep the existing Mode B test inventory, negative-case requirements, coverage/mutation review, and release gates intact. **Automation reduces user intervention; it does not delete scenarios or make a simulated peer equivalent to a physical Pixel.**

Reference matrix: [mode-b-inventory.md](mode-b-inventory.md), section "Desktop notification and Android completion synchronization gate". This plan is separate from the broader [full-feature-roadmap.md](full-feature-roadmap.md).

## Planning estimates (not measurements)

The current remaining hardware matrix contains up to **33 scenario executions across eight groups**; some successful runs can satisfy multiple groups. A/B experiments that fail to exercise the intended code path do not count as completed even if the file transfers.

Estimated user-facing manual interactions/test sessions, assuming reliable automation is built:

| Access level | User-intervention estimate | Preconditions |
|---|---:|---|
| Current remote GitHub + uploaded logs only | 20–30 | User installs packages, operates both devices, exports traces |
| Authorized local Ubuntu control | 8–12 | Reliable terminal/GUI execution, installer and tracing access |
| Authorized local Ubuntu control **and** Pixel over ADB | 2–5 | USB or wireless debugging authorized; Quick Share UI/intent automation proven reliable |

The previous rough estimate was **70–80% less manual work** with desktop access and **up to 85–90%** with Android automation. These numbers are speculative planning ranges, **not coverage percentages, measured productivity improvements, or guaranteed reductions**. ADB can log state and automate some actions, but Android permission dialogs, system Quick Share UX, OEM/API changes and user confirmation may block full unattended execution. The counts refer to human intervention, not a reduced number of checks.

## Scenario-to-automation matrix

| Hardware group | Scenario executions | Ubuntu automation | Additional Pixel/ADB capability | Still needs observation/confirmation |
|---|---:|---|---|---|
| Request notification: accept/reject/peer cancel | 3 | Spawn receiver; capture native KDE notifications, UI and event logs | Trigger distinct peer actions if accessible | Popup visibility/usability, cancellation when UI is locked or hidden |
| Final-state synchronization: image/text | 2 | Capture `WaitingForUserConsent`, `Finished`, frame send and socket close | Capture Android sender state/timestamps, UI video/logcat | Validate what Android actually calls “complete” |
| Cold discovery: foreground vs tray | 6 | Control app window and track FE2C scan/advertiser events | Trigger/share consistently and mark tap time | Real physical discoverability, first contact |
| FEF3 periodic 30s vs 10s | 6 | Build debug variant, set runtime env and parse timed logs | Initiate matched test payload and mark sender events | Causal isolation from background scan duty cycle |
| GATT slot0 deferred refresh enabled/disabled | Up to 6 (conditional) | Toggle diagnostic flag; reject non-slot0 A/B samples | Trigger GATT-bootstrap cases if possible | A successful direct TCP run **does not** count as a slot0 A/B |
| mDNS duplicates after clean exits | 2 | Tray Quit, verify unregister, restart and capture service identity | Observe/search Android device list after each cycle | Confirm whether stale Alcotester entries persist |
| Text/URL/image/folder functional paths | 5 | Generate safe fixture files, verify bytes/hash/tree and local state | Share intents, Android receive/open/verify where permitted | OS-specific system shares, notifications and folder UX |
| Repeated warm inbound transfers | 3 | Keep receiver running, capture separate sessions and outputs | Repeat Quick Share sends | Confirm peer remains discoverable, no stale routes |

**Sum: 33** as a scenario-budget ceiling for this matrix, not 33 separate required physical launches. A single trial can count toward multiple rows only if it independently satisfies each row's preconditions and assertions. A/B comparisons must preserve the same Android/Ubuntu versions, network, visibility, foreground/background mode and payload.

## Architecture for future harness

1. **Ubuntu runner**: a local script/executable in `scripts/` starts the installed debug build, installs a known SHA-matched `.deb`, selects `baseline`/`skip-deferred`/`periodic-10`, controls tray/window state when permitted, checks the expected diagnostic flags and performs graceful Quit. Never use `Ctrl+C` as the normal stop procedure; assert `MDnsServer: service unregistered: OK`.
2. **Android bridge (optional)**: use a physically connected Pixel with explicitly authorized USB/Wi-Fi ADB debugging. Record `adb devices`, OS/build, Quick Share version where available, `logcat` and optionally screen recording. Test `am` share intents / UI automation first; do not assume privileged system dialogs are scriptable. Never require root or silently change system security settings.
3. **Correlated event timeline**: record monotonic timestamps on each host plus a clock-offset calibration. Track Android share initiation/device discovery/device selection/connection/consent/completion alongside Linux FE2C/GATT slot0/Weave/BWU/TCP/`Finished`/disconnection/send-ack/notification lifecycle. Avoid treating Linux app startup-to-TCP as tap-to-discovery latency.
4. **Evidence bundle**: each trial outputs metadata (commit SHA, binary build type, OS, USB/BT controllers, variant, run UUID), sanitized event summary, raw logs, optional screen recording and a machine-readable `PASS`/`FAIL`/`INCONCLUSIVE` reason. Redact peer identifiers, MAC addresses, LAN IPs, text content, filenames, pins and session keys before publication. Raw traces remain local unless explicitly approved for upload.
5. **Orchestrator**: group reproducible trials by mode, run bounded repetitions, parse state transitions, flag stale notifications/duplicate peers/timeouts and produce a summary table. Do not silently retry a failure and count the retry as an initial pass. Preserve original failure evidence.
6. **CI boundary**: `dev` retains its fast single-run preflight; physical hardware execution must be explicit and only scheduled where a trusted self-hosted runner with the correct devices is available. Do not route hardware jobs to arbitrary shared runners. Keep full final Mode B 10× suite and package/physical signoff separate. Never auto-promote to `master` solely because local smoke passed.

## Provisioning and security prerequisites

- Treat shell/GUI control, package installation, Bluetooth permissions and ADB authorization as separate explicit grants; least privilege and reversible changes by default.
- `adb devices` must show an authorized physical device; missing/unauthorized ADB is a clear `BLOCKED` result, not a fake pass.
- Do not assume a ChatGPT conversation has direct PC or ADB access just because a repository or GitHub connector is available. Local tool/agent integration must actually be enabled and authorized.
- Use temporary fixture payloads, avoid real clipboard contents or personal files, and never log full transferred text by default.
- Provide cleanup for temporary files, test app instances, persistent notifications and any changed device state; avoid forced termination while transferring.
- Store trial logs outside Git, publish summaries and reproducible instructions. Keep secret material out of GitHub Actions artifacts.

## First implementation slice (after access is available)

1. Add a host-side runner that validates current `dev` debug package SHA, starts trials, confirms clean exit, and exports structured event timelines. Reuse `scripts/receiver-ab-smoke.sh` rather than creating duplicate protocol logic.
2. Instrument notifications and sender-vs-receiver completion to distinguish KDE popup expiry, pending consent, Linux `Finished` and Android terminal state. Fix lifecycle only with regression coverage.
3. Automate two matched discovery modes (foreground continuous scan and background 5s/25s fallback); only then compare 30s vs 10s FEF3 refresh.
4. Add ADB smoke proof of concept; gate UI automation behind feature detection and explicit user authorization.
5. Integrate deterministic non-hardware parts into Mode B; record unresolved physical-only cases in the per-production-file coverage map and final completion gate.

## Advancement criteria

Product feature development on `dev` can continue when immediate discovery/notification/completion blockers are characterized or fixed with regression coverage; it does not require all 33 trials before every feature. A final validated promotion to `master` requires the agreed hardware package smoke, complete Mode B review, stability gate and explicit unresolved-risk acceptance. Keep all baseline evidence and avoid reporting conditional A/B cases as passed when the protocol path was not exercised.

## Offline evidence analysis (implemented 2026-10-12)

After capturing one run with the existing launcher, analyze its log locally:

```bash
python3 scripts/analyze_receiver_smoke.py path/to/baseline.log --variant baseline
python3 scripts/analyze_receiver_smoke.py path/to/skip-deferred.log --variant skip-deferred --experiment slot0
python3 scripts/analyze_receiver_smoke.py path/to/periodic-10.log --variant periodic-10
```

The CLI requires one current-format Fern trace, matching debug treatment markers, correlated inbound state events and a clean tray-Quit/mDNS-unregister sequence. `--experiment slot0` additionally requires an offset-zero slot0 read. A direct-TCP-only run is inconclusive for that experiment. A failed/cancelled attempt remains a failure even if a retry succeeds.

JSON verdicts describe **Linux log evidence only**. They do not prove Android completion, payload byte/hash correctness, foreground/background trial matching, native notification usability, latency causation, or a winning A/B variant. The report emits only fixed event names, source line numbers, counts and reason codes; raw trace contents, timestamps, file paths, peer identifiers and payloads are excluded. Keep raw traces local. The analyzer cannot attest authenticity of supplied logs.

Exit code 0 means a report was generated, including `FAIL` or `INCONCLUSIVE`; callers must inspect `verdict`. Invalid options/unreadable or malformed UTF-8 input/inputs over 16 MiB exit nonzero. Concatenated runs, missing markers, uncorrelated channel records and incomplete sessions cannot produce `PASS`.

Tests use the standard library only: `python3 -m unittest discover -s scripts/tests -v`. The tests run in preflight and every Mode B iteration. The physical Ubuntu runner, package SHA validation, ADB bridge and synchronized Android timeline remain backlog. See [offline-maintenance.md](offline-maintenance.md) for the preregistered parser contract and handoff log.
