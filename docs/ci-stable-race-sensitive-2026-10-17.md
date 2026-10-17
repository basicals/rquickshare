# Stable/race-sensitive CI split and R2 parity fix

2026-10-17. MODE B (Testing). Direct `dev` headless GitHub only.
Installed native29, uninstalled native30, host, Pixel, physical
Windows/Linux peers, production secrets, published artifacts and
`master` must not be accessed or modified.

## Repository scan and observed failure

User SSOT: `agent.md`, Stage 11 requires **ten consecutive full-suite
successful runs** for final MODE B. That requirement is not replaced by
a cheaper ordinary preflight profile. Existing
`.github/workflows/lint.yml` runs the entire Core Rust suite once,
Vue/Vitest once, Python publication contracts once, and desktop Rust
once. `native-gtk-ui.yml` runs GTK once on affected pushes, with an
optional ten-run manual profile. No formal stable/flaky inventory,
test marker or dedicated stress selector exists. Importantly, no
currently verified test can accurately be labelled 'known flaky';
the eight selected tests below are **race-sensitive risk probes**,
not suppressed/quarantined known defects.

The newly extended `mode-b.yml` ran on source `7107339e` as
GitHub Actions 39063291246. Its first 4-suite loop failed **on attempt
1** in Python `test_native_r2.NativeR2Tests.
test_given_real_s3_model_when_requests_fail_or_complete_then_conditions_no_retry_bounds_and_body_cleanup_follow_contract`.
The exact exception was
`TypeError: Got unexpected keyword argument
'request_checksum_calculation'` from the Ubuntu OS-supplied
`botocore.config.Config`. Other first-iteration groups passed:
Core 704/0 + 8/0 integrations, legacy desktop 10/0, frontend.
GTK and coverage were correctly skipped. The same commit's ordinary
Preflight 39063291248 was **5/5 green**, including pinned R2 Python
contract deps (`scripts/requirements-r2.txt`) installed into a
venv. Therefore the failure is **environment parity**, not proof
that the domain test is flaky or should be moved to a 10x group.
Fix it by using the same pinned venv in full MODE B (single existing
Python test invocation per complete attempt), not by ignoring the
test, weakening an assertion, or changing publisher logic.

## Intent and policy

Normal dev Preflight: split `core_lib` into a complete *stable 1x*
pass that **excludes** a rigorously enumerated race-sensitive
subset, and an *exact 10x* pass of that subset. All other suites
remain once in Preflight. The source manifest validates that every
selected Rust test actually exists in `cargo test --lib -- --list`
before dispatching either group; no silent zero-test filters. The
stable `cargo test --all-features` command retains the existing
integration binaries and exclusions only remove the eight explicit
selected unit-test names. The stress group runs each selected full
test name with `--exact`; no status overrides, conditional ignore,
retries after red or assertion weakening. A red attempt fails CI.
The tests' original test source code and behavior are untouched.

Final `mode-b.yml`: retain the strict **ten complete** loops
(Core+desktop+Vue+Python, then GTK+coverage), with the missing pinned
R2 dependency initialization fixed. This is deliberately **not**
the fast dev profile: a full-suite 10x gate is explicit SSOT. Full
10x is not claimed complete until the exact Actions run succeeds.
No global assertions that these eight are actually flaky.

## Exhaustive scope inventory / branch map

The requested CI split covers these artifacts/branches; test authors'
larger Positive/N1-N12 matrices for their production files stay
authoritative in each individual ledger.

| Artifact | Risk | Validation / branches |
|---|---|---|
| `scripts/ci-core-test-groups.sh` | P0 silent test omissions | unknown mode -> nonzero; all 8 names exist exactly once; stable excludes exactly 8 via full names; stress invokes each name once per 10 attempts; red failure propagates; no ignored results |
| `.github/workflows/lint.yml` | P0 accidental holes | Core stable 1x then stress 10x in one owned job sharing compilation; other jobs/Clippy unmodified |
| `.github/workflows/mode-b.yml` | P0 false red environmental setup | pin boto3/botocore exactly as Preflight; use isolated Python venv for each of ten complete attempts; preserve GTK, existing coverage dependency, all test commands |
| `core_lib` selected race/concurrency tests | P0 potential scheduling conflicts | original real tests and assertions run 10x each; avoid classifying deterministic protocol errors as flaky |
| `docs/mode-b-test-plan.md`, `docs/current-state.md` | P1 outdated claims | note two distinct cadence contracts and proof status |
| Product implementation, GTK runtime, published tools | no touched source | existing 1x/strict 10x suites remain; physical acceptance still OPEN |

## Exact race-sensitive suite inventory (8 unique methods)

1. `hdl::inbound::security_tests::receive_reliability::given_peer_eof_during_active_multifile_receive_when_terminated_then_only_published_member_survives` — network EOF during active file-owner cleanup.
2. `hdl::outbound::security_tests::opened_sources::metadata::given_peer_reset_after_first_real_file_chunk_when_sending_then_no_false_finish_or_source_loss` — socket RST during producer/consumer concurrency.
3. `manager::supervision_tests::managed_tests::given_native_connect_failure_when_owner_releases_then_snapshot_records_disconnection_without_success` — managed release/error ordering.
4. `service::bus::tests::given_core_bind_failure_when_host_runs_then_runtime_pump_and_bus_are_closed` — failed startup and bus lifecycle.
5. `service::client::controller::tests::given_disconnected_or_slow_view_consumer_when_daemon_changes_then_worker_coalesces_and_fails_closed_without_reconnect` — slow observer/coalescing.
6. `service::runtime::tests::given_lost_observation_when_shutdown_fails_then_core_barrier_is_still_awaited` — shutdown synchronization.
7. `service::wakeup::tests::given_failed_or_blocked_activation_when_generations_coalesce_then_same_notice_never_respawns_and_shutdown_joins` — concurrent worker wakeups.
8. `sessions::tasks::tests::given_parent_shutdown_when_group_is_active_then_children_stop_and_late_admission_fails` — parent/child shutdown race.

Selection is risk-based and **not** a measured list of intermittently
failing tests. In the absence of positive reproducibility evidence,
renaming them 'flaky' would be incorrect and would hide defects.

## Positive/N1–N12 interaction matrix

Every category assessed across CI contract and selected runtime tests.
E means existing domain tests rerun; P means new CI enforcement;
X means inapplicable to this CI scheduling layer.

| Artifact | Positive | N1 | N2 | N3 | N4 | N5 | N6 | N7 | N8 | N9 | N10 | N11 | N12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| CLI selector | P | P | P | X | P | P | P | P | X | P | X | X | X |
| 1x/10x CI partition | P | P | P | E | P | P | P | E | X | P | E | X | E |
| R2 Python venv parity | P | E | P | E | E | X | P | E | X | E | E | X | E |

N8 (timezones/DST) and N11 (monetization) have no mapping in CI
scheduler/selector; business-domain tests remain unchanged. N12 of
the selector is inapplicable; R2 uses actual temporary storage
tests. N5 and N9 are directly stress-observed through the
eight actual concurrency/teardown probes. N4 means repeated
identical test execution, **not** product retry idempotence by itself.
N10 includes no user credentials/prod paths in CI logs.

Interactions: rust test existence × matching exact selector ×
`--skip` × integration binaries; selected test × ten repeats ×
failure at any attempt; ordinary dev push × manual strict MODE B;
Python botocore version × required SDK options; Mode B complete
4-suite success × native GTK success × dependent coverage.

## Estimates, mutation review, coverage map

8 original Rust test methods are reclassified (no new test methods).
One stable execution excludes exactly these 8 from 704 library tests
and leaves all 8 integration tests; this should yield **696**
core unit tests in stable plus **8 distinct × 10** stress invocations,
provided exact library inventory is unchanged. The previous
704/8 figures are historical, not evidence of the new split.
All other Preflight jobs retain their single-run counts. Strict
MODE B still needs **10 complete** suites and GTK ten times, and
coverage is conditional.

Mutation sensitivity: misspelled test selector must fail before either
group; removing a selector changes documented expected count;
a test marked `ignored` must fail inventory if observed (no hiding);
a stable group that accidentally retains a selected name must fail
the selector-integrity gate; a suite that skips execution must not
be counted as an attempt; any red test aborts. We do not modify
domain test assertions. This is assertion/contract mutation review,
not instrumented mutation testing. Per-production-file coverage is
unchanged: all eight source files still have their exact existing
test methods; only the scheduling cadence changes. Complete
current-branch map and coverage percentages remain OPEN.

## Milestones and rollback

1. [x] Analyze current workflows/SSOT, verify 5/5 Preflight and
   exact Mode B failure; inventory and cadence contract first.
2. [ ] Implement the explicit 8-test selector, stable 1x/stress 10x
   Preflight split with no loss, no retries and strict failure.
3. [ ] Fix botocore parity in Mode B with the same pinned venv
   strategy as Preflight; retain full ten-loop SSOT requirement,
   GTK gate and coverage dependencies.
4. [ ] Verify exact-source CI and record failures and counts. Never
   count a running job as a completed loop.
5. [ ] Update Mode B and current-state SSOT with exact evidence.
   All public-branch/physical acceptance gates remain OPEN.

Rollback: revert scoped script/Preflight patch and re-run 704/8
once; revert only mode-b venv patch to recreate old environment
(which is known broken and therefore not recommended). No files
on the user's machine, installed packages, master or live services
are touched.

## Execution

Pending.
