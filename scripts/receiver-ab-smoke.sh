#!/usr/bin/env bash
# Hardware A/B probe for the debug-only slot-0 advertising recovery switch.
# Usage: bash scripts/receiver-ab-smoke.sh baseline|skip-deferred|periodic-10
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: bash scripts/receiver-ab-smoke.sh baseline|skip-deferred|periodic-10

Runs one rQuickShare session and captures a timestamped log under
~/Downloads/rquickshare-smoke/ab-logs/.
For foreground trials, choose Show from the tray and leave the window open:
active BLE fallback should keep scanning continuously.
For background trials, leave the window minimized; fallback uses a 5s/25s
scan/idle cycle. Record which mode you tested.
After receiving the file, select Quit from the tray, NOT Ctrl+C:
graceful shutdown unregisters the mDNS service.
Repeat trials with identical device, network and visibility settings.
USAGE
}

case "${1:-}" in
  baseline)
    variant=baseline
    refresh=1
    periodic=30
    ;;
  skip-deferred)
    variant=skip-deferred
    refresh=0
    periodic=30
    ;;
  periodic-10)
    variant=periodic-10
    refresh=1
    periodic=10
    ;;
  -h|--help)
    usage
    exit 0
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac

binary=/usr/bin/rquickshare
if [[ ! -x "$binary" ]]; then
  printf 'Error: %s is not installed or executable.\n' "$binary" >&2
  exit 1
fi

# An already-running Tauri instance can consume the command line and keep its
# old environment. Do not kill it: closing the tray app is a user action.
if pgrep -u "$(id -u)" -x rquickshare >/dev/null; then
  printf 'Error: rQuickShare is running. Exit from its tray before the cold trial.\n' >&2
  exit 1
fi

log_dir="${HOME}/Downloads/rquickshare-smoke/ab-logs"
mkdir -p "$log_dir"
stamp="$(date -u +%Y%m%dT%H%M%S.%NZ)"
log_file="${log_dir}/${variant}-${stamp}.log"
printf 'Variant: %s (slot0 deferred=%s, periodic advertising=%ss)\n' "$variant" "$refresh" "$periodic"
printf 'Log: %s\n' "$log_file"
printf 'Keep the app open with Show in tray for foreground (continuous-scan) trials,\n'
printf 'or leave it minimized for background (5s/25s fallback-scan) trials.\n'
printf 'Share one image from Pixel; then select Quit from tray to stop gracefully.\n'
printf 'Do NOT use Ctrl+C here: it can prevent mDNS service unregistration.\n'

# This switch is intentionally ignored by release builds. The diagnostic
# startup line in the log proves that the installed binary is a debug build.
set +e
RQS_DIAG_SLOT0_ADV_REFRESH="$refresh" RQS_DIAG_RX_PERIODIC_ADV_SECS="$periodic" RQS_LOG=trace "$binary" 2>&1 | tee "$log_file"
app_status=${PIPESTATUS[0]}
set -e

if ! grep -q "diagnostic slot0 deferred advertising refresh enabled=" "$log_file"; then
  printf '\nWarning: slot0 debug diagnostic marker missing; do not use this run for A/B results.\n' >&2
fi
if ! grep -q "diagnostic periodic advertising refresh interval=${periodic}s" "$log_file"; then
  printf '\nWarning: expected periodic debug marker absent; rebuild and install the current dev debug .deb.\n' >&2
fi

# A sudden SIGINT/SIGTERM may leave a stale randomized mDNS endpoint on Android.
# Do not infer a daemon bug until a trial completed with a graceful tray Quit.
if ! grep -q "MDnsServer: service unregistered" "$log_file"; then
  printf '\nWarning: mDNS graceful unregistration was not confirmed.\n' >&2
  printf 'Quit via tray instead of Ctrl+C. This run cannot establish whether stale Android devices were cleaned up.\n' >&2
fi

printf '\nA/B signals from %s:\n' "$log_file"
grep -E 'diagnostic slot0|diagnostic periodic|passive FE2C monitor|active fallback matched|slot0 read|slot0 requested|periodic advertising refresh|deferred advertising refresh|weave notify|weave session|accepted inbound client|BWU:|state: Some\(Finished\)|state: Some\(Cancelled\)|state: Some\(Disconnected\)|tray_quit|service unregistered' "$log_file" | tail -n 90 || true
if [[ "$app_status" -eq 255 ]] && grep -q 'tray_quit' "$log_file" && grep -q 'MDnsServer: service unregistered' "$log_file"; then
  printf '\nNormal tray Quit confirmed (legacy Tauri exit code 255).\n'
elif [[ "$app_status" -ne 0 ]]; then
  printf '\nWarning: application exited with code %s; check the log and shutdown markers.\n' "$app_status" >&2
fi
printf '\nKeep the complete log for the comparison: %s\n' "$log_file"
