#!/usr/bin/env python3
"""Analyze one Linux receiver log without publishing its private contents."""

import argparse
import json
import re
import sys
from pathlib import Path


MAX_LOG_BYTES = 16 * 1024 * 1024
VARIANTS = {"baseline": (True, 30), "skip-deferred": (False, 30), "periodic-10": (True, 10)}
ANSI = re.compile(r"\x1b\[[0-9;]*m")
ENVELOPE = re.compile(
    r"^\d{4}-\d{2}-\d{2}T\S+\s+(?:TRACE|DEBUG|INFO|WARN|ERROR)\s+"
    r"(?P<target>[A-Za-z_][A-Za-z_0-9:]*): (?P<body>.*)$"
)
CHANNEL = re.compile(
    r'^rs2js_channelmessage: ChannelMessage \{ id: "(?P<id>(?:[^"\\]|\\.)*)", '
    r"direction: LibToFront, action: None, rtype: Some\((?P<direction>Inbound|Outbound)\), "
    r"state: Some\((?P<state>[A-Za-z]+)\),"
)
SLOT0_DIAG = "ReceiverGattServer: diagnostic slot0 deferred advertising refresh enabled="
PERIOD_DIAG = "ReceiverAdvertiser: diagnostic periodic advertising refresh interval="


def analyze(text: str, variant: str, experiment: str = "periodic") -> dict:
    """Return allowlisted evidence; identifiers are used only for local correlation."""
    if variant not in VARIANTS or experiment not in ("periodic", "slot0"):
        raise ValueError("unsupported variant or experiment")
    expected_slot0, expected_period = VARIANTS[variant]
    events = []
    markers = {"start": [], "slot0": [], "period": [], "quit": [], "unregister": []}
    sessions = {}
    failures = set()
    uncertain = set()
    observations = {"slot0_zero_reads": 0, "scan_starts": 0, "scan_stops": 0}

    def event(kind, line):
        events.append({"type": kind, "line": line})

    for line_number, raw in enumerate(text.splitlines(), 1):
        match = ENVELOPE.fullmatch(ANSI.sub("", raw))
        if match is None:
            continue
        target, body = match.group("target", "body")
        if target == "rquickshare::logger" and body == "Finished setting up logging! yay!":
            markers["start"].append(line_number)
            event("app_start", line_number)
        elif target == "rqs_lib::hdl::gatt" and body.startswith(SLOT0_DIAG):
            value = body[len(SLOT0_DIAG):]
            markers["slot0"].append(line_number)
            if value != str(expected_slot0).lower():
                failures.add("slot0_treatment_mismatch")
            event("slot0_diagnostic", line_number)
        elif target == "rqs_lib::hdl::blea" and body.startswith(PERIOD_DIAG):
            markers["period"].append(line_number)
            if body[len(PERIOD_DIAG):] != f"{expected_period}s":
                failures.add("periodic_treatment_mismatch")
            event("periodic_diagnostic", line_number)
        elif target == "rquickshare" and body == "tray_quit":
            markers["quit"].append(line_number)
            event("tray_quit", line_number)
        elif target == "rqs_lib::hdl::mdns" and body == "MDnsServer: service unregistered: OK":
            markers["unregister"].append(line_number)
            event("mdns_unregistered", line_number)
        elif target == "rqs_lib::hdl::gatt" and re.fullmatch(
            r"ReceiverGattServer: slot0 read offset=0, returned=\d+ bytes", body
        ):
            observations["slot0_zero_reads"] += 1
            event("slot0_zero_read", line_number)
        elif target == "rqs_lib::hdl::ble" and body in (
            "BleListener: active fallback scan started",
            "BleListener: active fallback scan stopped",
        ):
            started = body.endswith("started")
            observations["scan_starts" if started else "scan_stops"] += 1
            event("scan_started" if started else "scan_stopped", line_number)
        elif target == "rqs_lib::hdl::ble" and body == "BleListener: using active BLE discovery fallback":
            event("active_fallback", line_number)
        elif target == "rqs_lib::hdl::ble_monitor" and re.fullmatch(
            r"PassiveBleMonitor: FE2C advertisement monitor active on \S+", body
        ):
            event("passive_monitor", line_number)
        elif target == "rqs_lib::hdl::ble_monitor" and body == "PassiveBleMonitor: matched Quick Share FE2C advertisement":
            event("fe2c_detected", line_number)
        elif target == "rqs_lib::hdl::gatt" and body == "ReceiverGattServer: weave notify subscription started":
            event("weave_subscription", line_number)
        elif target == "rquickshare" and body.startswith("rs2js_channelmessage: ChannelMessage {"):
            channel = CHANNEL.match(body)
            if channel is None:
                # A malformed event must not be hidden by a later successful session.
                uncertain.add("uncorrelated_channel_record")
                continue
            if channel["direction"] != "Inbound":
                continue
            identity = channel["id"]
            if not identity:
                uncertain.add("uncorrelated_channel_record")
                continue
            state = channel["state"]
            if state not in ("WaitingForUserConsent", "Finished", "Rejected", "Cancelled", "Disconnected"):
                continue
            session = sessions.setdefault(identity, {"finished": False, "failed": False, "pending": False})
            if state == "WaitingForUserConsent":
                if session["finished"] or session["failed"]:
                    uncertain.add("reused_transfer_identity")
                session["pending"] = True
            elif state == "Finished":
                session["finished"] = True
                session["pending"] = False
            elif not session["finished"]:
                session["failed"] = True
                session["pending"] = False
                failures.add("transfer_failed_before_completion")
            event(f"transfer_{state.lower()}", line_number)

    for marker in ("start", "slot0", "period"):
        if len(markers[marker]) != 1:
            uncertain.add(f"expected_one_{marker}_marker")
    finished = sum(session["finished"] for session in sessions.values())
    failed = sum(session["failed"] for session in sessions.values())
    pending = sum(session["pending"] for session in sessions.values())
    if not finished:
        uncertain.add("no_correlated_finished_transfer")
    if pending:
        uncertain.add("incomplete_consent_session")
    if experiment == "slot0" and observations["slot0_zero_reads"] == 0:
        uncertain.add("slot0_path_not_exercised")

    clean_exit = (
        len(markers["quit"]) == len(markers["unregister"]) == 1
        and markers["quit"][0] < markers["unregister"][0]
    )
    if not clean_exit:
        uncertain.add("graceful_exit_not_confirmed")
    if len(markers["start"]) == len(markers["quit"]) == 1:
        start, quit_line = markers["start"][0], markers["quit"][0]
        relevant = [e["line"] for e in events if e["type"].startswith("transfer_")
                    or e["type"] in ("slot0_diagnostic", "periodic_diagnostic", "slot0_zero_read")]
        if not start < quit_line or any(not start < line < quit_line for line in relevant):
            uncertain.add("events_outside_run_boundary")

    verdict = "FAIL" if failures else "INCONCLUSIVE" if uncertain else "PASS"
    return {
        "schema_version": 1,
        "scope": "linux_log_evidence_only",
        "variant": variant,
        "experiment": experiment,
        "verdict": verdict,
        "reasons": sorted(failures | uncertain),
        "transfers": {"finished": finished, "failed": failed, "pending": pending},
        "observations": observations,
        "graceful_exit": clean_exit,
        "events": events,
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log", type=Path)
    parser.add_argument("--variant", required=True, choices=VARIANTS)
    parser.add_argument("--experiment", choices=("periodic", "slot0"), default="periodic")
    args = parser.parse_args(argv)
    try:
        with args.log.open("rb") as source:
            raw = source.read(MAX_LOG_BYTES + 1)
        if len(raw) > MAX_LOG_BYTES:
            raise ValueError("input too large")
        report = analyze(raw.decode("utf-8"), args.variant, args.experiment)
    except (OSError, UnicodeError, ValueError):
        # Do not echo filenames or OS exception text into a shareable report.
        print("Unable to analyze input: require a readable UTF-8 log of at most 16 MiB.", file=sys.stderr)
        return 2
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
