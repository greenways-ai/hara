import importlib.util
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("run.py")
SPEC = importlib.util.spec_from_file_location("runtime_benchmark", MODULE_PATH)
BENCHMARK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BENCHMARK)


def measurement(runtime, workload, steady_ns):
    return {
        "runtime": runtime,
        "workload": workload,
        "analysis": {"steady_ns": steady_ns},
    }


class RegressionRulesTest(unittest.TestCase):
    def test_runtime_specific_sources_are_explicit_and_never_fall_back_silently(self):
        workload = {
            "id": "mutable-map-build",
            "source": "unused",
            "sources": {"bb": "(transient {})", "hara-rust-bytecode": "(to-mutable {})"},
        }
        self.assertEqual(
            BENCHMARK.workload_for_runtime(workload, "bb")["source"],
            "(transient {})",
        )
        self.assertIsNone(BENCHMARK.workload_for_runtime(workload, "hara-truffle"))

    def test_accepts_ratio_at_threshold(self):
        data = {"measurements": [
            measurement("bb", "arithmetic", 100),
            measurement("hara-rust-bytecode", "arithmetic", 90),
        ]}
        baseline = {"rules": [{
            "runtime": "hara-rust-bytecode",
            "workload": "arithmetic",
            "relative_to": "bb",
            "max_ratio": 0.90,
        }]}
        self.assertEqual(BENCHMARK.check_regressions(data, baseline), [])

    def test_reports_regression_and_missing_measurements(self):
        data = {"measurements": [
            measurement("bb", "arithmetic", 100),
            measurement("hara-rust-bytecode", "arithmetic", 95),
        ]}
        baseline = {"rules": [
            {"runtime": "hara-rust-bytecode", "workload": "arithmetic",
             "relative_to": "bb", "max_ratio": 0.90},
            {"runtime": "hara-rust-trace-checked", "workload": "arithmetic",
             "relative_to": "bb", "max_ratio": 0.60},
        ]}
        failures = BENCHMARK.check_regressions(data, baseline)
        self.assertIn("ratio 0.950 exceeds 0.900", failures[0])
        self.assertIn("missing measurement", failures[1])


if __name__ == "__main__":
    unittest.main()
