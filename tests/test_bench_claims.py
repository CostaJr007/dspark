"""
CI guard for the offline improvement benchmarks: the A/B claims
(bench/compare_cegar_improvements.py) must keep holding on every run.
"""

import importlib.util
import pathlib
import unittest


class TestBenchClaims(unittest.TestCase):

    def test_cegar_improvements_bench_claims(self):
        path = pathlib.Path(__file__).resolve().parent.parent / "bench" / "compare_cegar_improvements.py"
        spec = importlib.util.spec_from_file_location("compare_cegar_improvements", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        self.assertEqual(module.main(), 0)


if __name__ == "__main__":
    unittest.main()
