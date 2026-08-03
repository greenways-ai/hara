import copy
import importlib.util
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("validate_evidence.py")
SPEC = importlib.util.spec_from_file_location("validate_evidence", MODULE_PATH)
VALIDATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VALIDATOR)


def complete_document():
    measurements = []
    for runtime in VALIDATOR.ARTIFACTS:
        for workload in VALIDATOR.WORKLOADS:
            measurements.append({"runtime": runtime, "workload": workload, "status": "ok",
                                 "prepare_ns": 1, "first_ns": 2, "steady_ns": 1,
                                 "throughput_per_sec": 1.0, "checksum": "42"})
    return {"schema": "hara-benchmark-evidence/v2", "profile": "standard",
            "environment": {key: "x" for key in ("run_id", "timestamp", "platform", "machine", "cpu", "git_revision", "browser")},
            "corpus": {"id": "hara-algorithms-v1"}, "measurements": measurements,
            "startup": {runtime: {"p50_ns": 1} for runtime in VALIDATOR.ARTIFACTS},
            "artifacts": {runtime: {key: 1 for key in ("base_bytes", "workload_delta_bytes", "raw_total_bytes", "transfer_bytes")} for runtime in VALIDATOR.ARTIFACTS},
            "language_measurements": [{"hara_runtime": "hara-rust-full"}],
            "http_measurements": [
                {"server": server, "route": route, "hara_runtime": "hara-rust-full"}
                for server in ("hoplite-raw", "hoplite-request", "hoplite-request+hta")
                for route in ("/hello", "/json", "/delay")
            ]}


class EvidenceValidationTest(unittest.TestCase):
    def test_complete_matrix_is_publishable(self):
        self.assertEqual([], VALIDATOR.validate(complete_document()))

    def test_missing_cell_and_wrong_comparison_tier_fail(self):
        document = complete_document()
        document["measurements"].pop()
        document["language_measurements"][0]["hara_runtime"] = "hara-rust-vm"
        errors = VALIDATOR.validate(document)
        self.assertTrue(any("missing measurement" in error for error in errors))
        self.assertIn("language comparisons must use hara-rust-full", errors)

    def test_missing_hoplite_mode_fails(self):
        document = complete_document()
        document["http_measurements"] = [
            row for row in document["http_measurements"]
            if row["server"] != "hoplite-request+hta"
        ]
        errors = VALIDATOR.validate(document)
        self.assertTrue(any("missing HTTP measurement: hoplite-request+hta" in error for error in errors))


if __name__ == "__main__": unittest.main()
