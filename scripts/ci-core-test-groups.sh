#!/usr/bin/env bash
# Deterministic dev-CI partition of core Rust tests: stable once, race-sensitive 10x.
# MODE B's final full-suite 10x remains a separate, stricter acceptance gate.
set -euo pipefail

usage() {
  echo 'Usage: bash scripts/ci-core-test-groups.sh stable|stress|list'
}

case "${1:-}" in
  stable|stress|list) group="$1" ;;
  -h|--help) usage; exit 0 ;;
  *) usage >&2; exit 2 ;;
esac
if (( $# != 1 )); then
  usage >&2
  exit 2
fi

# These methods protect concurrency/transport lifecycles. They are
# race-sensitive risk probes, NOT tests known to be intermittently failing.
# Full paths were verified in the exact-source 2026-10-17 core run.
RACE_SENSITIVE=(
  'hdl::inbound::security_tests::receive_reliability::given_peer_eof_during_active_multifile_receive_when_terminated_then_only_published_member_survives'
  'hdl::outbound::security_tests::opened_sources::metadata::given_peer_reset_after_first_real_file_chunk_when_sending_then_no_false_finish_or_source_loss'
  'manager::supervision_tests::managed_tests::given_native_connect_failure_when_owner_releases_then_snapshot_records_disconnection_without_success'
  'service::bus::tests::given_core_bind_failure_when_host_runs_then_runtime_pump_and_bus_are_closed'
  'service::client::controller::tests::given_disconnected_or_slow_view_consumer_when_daemon_changes_then_worker_coalesces_and_fails_closed_without_reconnect'
  'service::runtime::tests::given_lost_observation_when_shutdown_fails_then_core_barrier_is_still_awaited'
  'service::wakeup::tests::given_failed_or_blocked_activation_when_generations_coalesce_then_same_notice_never_respawns_and_shutdown_joins'
  'sessions::tasks::tests::given_parent_shutdown_when_group_is_active_then_children_stop_and_late_admission_fails'
)

if [[ "$group" == list ]]; then
  printf '%s\n' "${RACE_SENSITIVE[@]}"
  exit 0
fi

cd "$(dirname "${BASH_SOURCE[0]}")/../core_lib"

# Rust's --exact test filter can silently run zero tests. Validate the
# inventory and the names before launching either suite.
inventory="$(cargo test --all-features --lib -- --list)"
test_count="$(printf '%s\n' "$inventory" | grep -cE ': test$' || true)"
if (( test_count <= ${#RACE_SENSITIVE[@]} )); then
  echo "Core unit-test inventory is unexpectedly empty or smaller than the stress group" >&2
  exit 1
fi

for test_name in "${RACE_SENSITIVE[@]}"; do
  occurrences="$(printf '%s\n' "$inventory" | grep -Fxc -- "$test_name: test" || true)"
  if [[ "$occurrences" != 1 ]]; then
    echo "Test partition is stale: expected exactly one '$test_name', found $occurrences" >&2
    exit 1
  fi
done

if [[ "$group" == stable ]]; then
  exclusions=()
  for test_name in "${RACE_SENSITIVE[@]}"; do
    exclusions+=(--skip "$test_name")
  done
  expected=$((test_count - ${#RACE_SENSITIVE[@]}))
  echo "Core stable group: $expected unit tests once, ${#RACE_SENSITIVE[@]} race-sensitive tests excluded, integration binaries retained."
  output="$(cargo test --all-features -- "${exclusions[@]}" 2>&1)" || {
    printf '%s\n' "$output"
    exit 1
  }
  printf '%s\n' "$output"
  if ! grep -Fq "test result: ok. $expected passed; 0 failed; 0 ignored;" <<< "$output"; then
    echo "Core stable result does not match the validated unit-test inventory ($expected)" >&2
    exit 1
  fi
  exit 0
fi

echo "Core race-sensitive group: ${#RACE_SENSITIVE[@]} exact tests, ten consecutive attempts."
for attempt in $(seq 1 10); do
  echo "=== Core race-sensitive attempt $attempt/10 ==="
  for test_name in "${RACE_SENSITIVE[@]}"; do
    output="$(cargo test --all-features --lib -- "$test_name" --exact 2>&1)" || {
      printf '%s\n' "$output"
      echo "Race-sensitive failure on attempt $attempt/10: $test_name" >&2
      exit 1
    }
    printf '%s\n' "$output"
    if ! grep -Fq 'test result: ok. 1 passed; 0 failed; 0 ignored;' <<< "$output"; then
      echo "Selected race-sensitive method did not execute exactly once: $test_name" >&2
      exit 1
    fi
  done
done
echo "Core race-sensitive suite: all ten complete attempts passed."
