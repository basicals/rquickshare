"""Deterministic contracts for the offline receiver-evidence analyzer."""

import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location(
    "analyze_receiver_smoke", Path(__file__).resolve().parents[1] / "analyze_receiver_smoke.py"
)
analyzer = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(analyzer)


def record(target, body):
    return f"2026-10-12T08:00:00Z DEBUG {target}: {body}"


def transfer(state, identity="peer-private", direction="Inbound", tail="meta: None, error: None }"):
    return record("rquickshare", f'rs2js_channelmessage: ChannelMessage {{ id: "{identity}", '
                  f'direction: LibToFront, action: None, rtype: Some({direction}), '
                  f'state: Some({state}), {tail}')


def fixture(variant="baseline"):
    enabled, period = analyzer.VARIANTS[variant]
    return [
        record("rquickshare::logger", "Finished setting up logging! yay!"),
        record("rqs_lib::hdl::gatt", analyzer.SLOT0_DIAG + str(enabled).lower()),
        record("rqs_lib::hdl::blea", analyzer.PERIOD_DIAG + f"{period}s"),
        transfer("WaitingForUserConsent"),
        transfer("Finished"),
        record("rquickshare", "tray_quit"),
        record("rqs_lib::hdl::mdns", "MDnsServer: service unregistered: OK"),
    ]


def analyze(lines, variant="baseline", experiment="periodic"):
    return analyzer.analyze("\n".join(lines), variant, experiment)


class EvidenceTests(unittest.TestCase):
    def test_Given_each_variant_When_matching_run_Then_evidence_passes(self):
        for variant in analyzer.VARIANTS:
            with self.subTest(variant=variant):
                # Arrange / Act
                report = analyze(fixture(variant), variant)
                # Assert
                self.assertEqual(report["verdict"], "PASS")
                self.assertEqual(report["reasons"], [])
                self.assertEqual(report["transfers"], {"finished": 1, "failed": 0, "pending": 0})
                self.assertEqual(report["scope"], "linux_log_evidence_only")
                self.assertTrue(report["graceful_exit"])

    def test_Given_missing_markers_When_analyzed_Then_no_pass(self):
        for index, reason in [(0, "expected_one_start_marker"), (1, "expected_one_slot0_marker"),
                              (2, "expected_one_period_marker"), (5, "graceful_exit_not_confirmed"),
                              (6, "graceful_exit_not_confirmed")]:
            with self.subTest(index=index):
                # Arrange
                lines = fixture()
                del lines[index]
                # Act
                report = analyze(lines)
                # Assert
                self.assertEqual(report["verdict"], "INCONCLUSIVE")
                self.assertIn(reason, report["reasons"])

    def test_Given_wrong_or_unknown_knob_When_analyzed_Then_fails(self):
        cases = [(1, analyzer.SLOT0_DIAG + value, "slot0_treatment_mismatch")
                 for value in ("false", "unknown", "true trailing")]
        cases += [(2, analyzer.PERIOD_DIAG + value, "periodic_treatment_mismatch")
                  for value in ("10s", "0s", "121s", "30", "30s trailing")]
        for index, body, reason in cases:
            with self.subTest(body=body):
                # Arrange
                lines = fixture()
                target = "rqs_lib::hdl::gatt" if index == 1 else "rqs_lib::hdl::blea"
                lines[index] = record(target, body)
                # Act
                report = analyze(lines)
                # Assert
                self.assertEqual(report["verdict"], "FAIL")
                self.assertIn(reason, report["reasons"])

    def test_Given_duplicate_boundary_When_analyzed_Then_inconclusive(self):
        for index in (0, 1, 2, 5, 6):
            with self.subTest(index=index):
                # Arrange
                lines = fixture()
                lines.insert(index, lines[index])
                # Act / Assert
                self.assertEqual(analyze(lines)["verdict"], "INCONCLUSIVE")

    def test_Given_concatenated_runs_When_analyzed_Then_inconclusive(self):
        # Arrange / Act / Assert
        self.assertEqual(analyze(fixture() + fixture())["verdict"], "INCONCLUSIVE")

    def test_Given_no_completion_When_analyzed_Then_inconclusive(self):
        for lines in ([], fixture()[:3] + fixture()[5:], fixture()[:4] + fixture()[5:]):
            with self.subTest(lines=len(lines)):
                # Act
                report = analyze(lines)
                # Assert
                self.assertEqual(report["verdict"], "INCONCLUSIVE")
                self.assertIn("no_correlated_finished_transfer", report["reasons"])

    def test_Given_failed_transfer_and_retry_When_analyzed_Then_failure_is_preserved(self):
        for state in ("Rejected", "Cancelled", "Disconnected"):
            for retry_identity in ("peer-private", "retry-private"):
                with self.subTest(state=state, retry=retry_identity):
                    # Arrange
                    lines = fixture()
                    lines[4:5] = [transfer(state), transfer("Finished", retry_identity)]
                    # Act
                    report = analyze(lines)
                    # Assert
                    self.assertEqual(report["verdict"], "FAIL")
                    self.assertEqual(report["transfers"]["failed"], 1)
                    self.assertEqual(report["transfers"]["finished"], 1)
                    self.assertIn("transfer_failed_before_completion", report["reasons"])

    def test_Given_finished_then_disconnect_or_duplicate_When_analyzed_Then_counts_once(self):
        for state in ("Disconnected", "Finished"):
            with self.subTest(state=state):
                # Arrange
                lines = fixture()
                lines.insert(5, transfer(state))
                # Act
                report = analyze(lines)
                # Assert
                self.assertEqual(report["verdict"], "PASS")
                self.assertEqual(report["transfers"], {"finished": 1, "failed": 0, "pending": 0})

    def test_Given_pending_second_session_When_analyzed_Then_inconclusive(self):
        # Arrange
        lines = fixture()
        lines.insert(5, transfer("WaitingForUserConsent", "second-private"))
        # Act
        report = analyze(lines)
        # Assert
        self.assertEqual(report["verdict"], "INCONCLUSIVE")
        self.assertEqual(report["transfers"]["pending"], 1)
        self.assertIn("incomplete_consent_session", report["reasons"])

    def test_Given_reused_finished_identity_When_new_consent_Then_inconclusive(self):
        # Arrange
        lines = fixture()
        lines[5:5] = [transfer("WaitingForUserConsent"), transfer("Finished")]
        # Act / Assert
        self.assertIn("reused_transfer_identity", analyze(lines)["reasons"])

    def test_Given_outbound_or_intermediate_state_When_analyzed_Then_ignored(self):
        for event in (transfer("Finished", "outbound", "Outbound"), transfer("SendingFiles")):
            with self.subTest(event=event):
                # Arrange
                lines = fixture()
                lines.insert(4, event)
                # Act
                report = analyze(lines)
                # Assert
                self.assertEqual(report["verdict"], "PASS")
                self.assertEqual(report["transfers"], {"finished": 1, "failed": 0, "pending": 0})
                self.assertEqual(len([e for e in report["events"] if e["type"].startswith("transfer_")]), 2)

    def test_Given_uncorrelated_record_When_analyzed_Then_inconclusive(self):
        for event in (transfer("Finished", ""), record("rquickshare", "rs2js_channelmessage: ChannelMessage { malformed")):
            with self.subTest(event=event):
                # Arrange
                lines = fixture()
                lines.insert(5, event)
                # Act / Assert
                self.assertIn("uncorrelated_channel_record", analyze(lines)["reasons"])

    def test_Given_slot0_experiment_When_offset_is_nonzero_or_missing_Then_inconclusive(self):
        for offset in (None, 1, 20):
            with self.subTest(offset=offset):
                # Arrange
                lines = fixture()
                if offset is not None:
                    lines.insert(3, record("rqs_lib::hdl::gatt", f"ReceiverGattServer: slot0 read offset={offset}, returned=20 bytes"))
                # Act
                report = analyze(lines, experiment="slot0")
                # Assert
                self.assertEqual(report["verdict"], "INCONCLUSIVE")
                self.assertIn("slot0_path_not_exercised", report["reasons"])

    def test_Given_slot0_zero_read_When_analyzed_Then_path_is_observed(self):
        # Arrange
        lines = fixture()
        lines.insert(3, record("rqs_lib::hdl::gatt", "ReceiverGattServer: slot0 read offset=0, returned=20 bytes"))
        # Act
        report = analyze(lines, experiment="slot0")
        # Assert
        self.assertEqual(report["verdict"], "PASS")
        self.assertEqual(report["observations"]["slot0_zero_reads"], 1)
        self.assertIn({"type": "slot0_zero_read", "line": 4}, report["events"])

    def test_Given_discovery_events_When_analyzed_Then_only_fixed_observations_emitted(self):
        # Arrange
        lines = fixture()
        lines[3:3] = [record("rqs_lib::hdl::ble", "BleListener: active fallback scan started"),
                      record("rqs_lib::hdl::ble", "BleListener: active fallback scan stopped"),
                      record("rqs_lib::hdl::ble", "BleListener: using active BLE discovery fallback"),
                      record("rqs_lib::hdl::ble_monitor", "PassiveBleMonitor: FE2C advertisement monitor active on hci0"),
                      record("rqs_lib::hdl::ble_monitor", "PassiveBleMonitor: matched Quick Share FE2C advertisement"),
                      record("rqs_lib::hdl::gatt", "ReceiverGattServer: weave notify subscription started")]
        # Act
        report = analyze(lines)
        # Assert
        self.assertEqual(report["observations"], {"slot0_zero_reads": 0, "scan_starts": 1, "scan_stops": 1})
        self.assertTrue({"active_fallback", "passive_monitor", "fe2c_detected", "weave_subscription"}.issubset(e["type"] for e in report["events"]))

    def test_Given_failed_reversed_or_fake_unregister_When_analyzed_Then_inconclusive(self):
        for mode in ("failed", "reversed", "trailing", "wrong-target"):
            with self.subTest(mode=mode):
                # Arrange
                lines = fixture()
                if mode == "reversed":
                    lines[5], lines[6] = lines[6], lines[5]
                else:
                    suffix = {"failed": "ERR", "trailing": "OK not really", "wrong-target": "OK"}[mode]
                    target = "rquickshare" if mode == "wrong-target" else "rqs_lib::hdl::mdns"
                    lines[6] = record(target, "MDnsServer: service unregistered: " + suffix)
                # Act / Assert
                self.assertEqual(analyze(lines)["verdict"], "INCONCLUSIVE")

    def test_Given_event_outside_start_quit_When_analyzed_Then_inconclusive(self):
        for mode in ("early", "late", "reversed"):
            with self.subTest(mode=mode):
                # Arrange
                lines = fixture()
                if mode == "early":
                    lines.insert(0, transfer("Finished", "outside"))
                elif mode == "late":
                    lines.append(transfer("Finished", "outside"))
                else:
                    lines[0], lines[5] = lines[5], lines[0]
                # Act / Assert
                self.assertIn("events_outside_run_boundary", analyze(lines)["reasons"])

    def test_Given_ansi_colors_When_analyzed_Then_result_is_unchanged(self):
        # Arrange
        lines = fixture()
        # Act / Assert
        self.assertEqual(analyze(lines), analyze(["\x1b[2m" + line + "\x1b[0m" for line in lines]))

    def test_Given_private_and_marker_like_payload_When_analyzed_Then_no_content_leaks(self):
        # Arrange
        secret = '192.168.1.20 AA:BB:CC:DD:EE:FF private.png PIN=1234 key=secret '
        payload = secret + analyzer.SLOT0_DIAG + 'false state: Some(Cancelled)'
        lines = fixture()
        lines[4] = transfer("Finished", tail=f'meta: Some("{payload}"), error: None }}')
        lines.insert(4, record("rqs_lib::hdl::inbound", payload))
        # Act
        report = analyze(lines)
        serialized = json.dumps(report)
        # Assert
        self.assertEqual(report["verdict"], "PASS")
        for private in ("192.168", "AA:BB", "private.png", "1234", "secret", "peer-private", "2026-10-12"):
            self.assertNotIn(private, serialized)
        self.assertTrue(all(set(event) == {"type", "line"} for event in report["events"]))

    def test_Given_invalid_api_options_When_analyzed_Then_rejected(self):
        for variant, experiment in (("unknown", "periodic"), ("baseline", "unknown")):
            with self.subTest(variant=variant, experiment=experiment):
                # Arrange / Act / Assert
                with self.assertRaises(ValueError):
                    analyzer.analyze("", variant, experiment)


class CliTests(unittest.TestCase):
    def invoke(self, args):
        output, errors = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
            result = analyzer.main(args)
        return result, output.getvalue(), errors.getvalue()

    def test_Given_valid_or_empty_log_When_cli_runs_Then_json_and_stable_exit(self):
        for content, verdict in (("\n".join(fixture()), "PASS"), ("", "INCONCLUSIVE")):
            with self.subTest(verdict=verdict), tempfile.TemporaryDirectory() as directory:
                # Arrange
                path = Path(directory) / "input.log"
                path.write_text(content, encoding="utf-8")
                # Act
                first = self.invoke([str(path), "--variant", "baseline"])
                second = self.invoke([str(path), "--variant", "baseline"])
                # Assert
                self.assertEqual(first, second)
                self.assertEqual(first[0], 0)
                self.assertEqual(first[2], "")
                self.assertEqual(json.loads(first[1])["verdict"], verdict)

    def test_Given_failed_evidence_When_cli_runs_Then_report_is_not_execution_error(self):
        # Arrange
        lines = fixture()
        lines[4] = transfer("Cancelled")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.log"
            path.write_text("\n".join(lines), encoding="utf-8")
            # Act
            result, output, errors = self.invoke([str(path), "--variant", "baseline"])
            # Assert
            self.assertEqual(result, 0)
            self.assertEqual(errors, "")
            self.assertEqual(json.loads(output)["verdict"], "FAIL")

    def test_Given_unreadable_or_invalid_utf8_When_cli_runs_Then_no_private_path_leaks(self):
        for mode in ("missing", "directory", "utf8", "oversized"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                # Arrange
                path = Path(directory) / "private-filename.log"
                if mode == "directory":
                    path.mkdir()
                elif mode == "utf8":
                    path.write_bytes(b"\xff")
                elif mode == "oversized":
                    with path.open("wb") as output:
                        output.truncate(analyzer.MAX_LOG_BYTES + 1)
                # Act
                result, output, errors = self.invoke([str(path), "--variant", "baseline"])
                # Assert
                self.assertEqual(result, 2)
                self.assertEqual(output, "")
                self.assertIn("readable UTF-8 log", errors)
                self.assertNotIn(str(path), errors)

    def test_Given_exact_size_limit_When_cli_runs_Then_accepted(self):
        # Arrange
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "input.log"
            path.write_bytes(b" " * analyzer.MAX_LOG_BYTES)
            # Act
            result, output, errors = self.invoke([str(path), "--variant", "baseline"])
            # Assert
            self.assertEqual(result, 0)
            self.assertEqual(errors, "")
            self.assertEqual(json.loads(output)["verdict"], "INCONCLUSIVE")

    def test_Given_invalid_cli_options_When_parsed_Then_nonzero(self):
        for args in (["input"], ["input", "--variant", "unknown"],
                     ["input", "--variant", "baseline", "--experiment", "unknown"]):
            with self.subTest(args=args):
                # Arrange / Act / Assert
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
                    analyzer.main(args)
                self.assertEqual(error.exception.code, 2)


if __name__ == "__main__":
    unittest.main()
