"""
Unit tests for AST Parser and Contract Compiler.
"""

import unittest
from dspark.compiler.parser import extract_functions_and_docstrings, infer_contracts_from_ast
from dspark.compiler.test_harness import ContractCompiler
from dspark.state import IOContract


SAMPLE_CODE = '''
def calculate_discount(price: float, discount_percent: float) -> float:
    """
    Calculates final price after discount.
    PRE: price >= 0
    PRE: 0 <= discount_percent <= 100
    POST: result <= price
    POST: result >= 0
    """
    if price < 0 or discount_percent < 0 or discount_percent > 100:
        raise ValueError("Invalid parameters")
    return price * (1.0 - discount_percent / 100.0)
'''


class TestCompiler(unittest.TestCase):
    def test_ast_parser_extraction(self):
        funcs = extract_functions_and_docstrings(SAMPLE_CODE)
        self.assertEqual(len(funcs), 1)
        fn = funcs[0]
        self.assertEqual(fn["name"], "calculate_discount")
        self.assertEqual(fn["args"], ["price", "discount_percent"])
        self.assertEqual(len(fn["preconditions"]), 2)
        self.assertEqual(len(fn["postconditions"]), 2)

    def test_infer_contracts_from_ast(self):
        contracts = infer_contracts_from_ast(SAMPLE_CODE)
        self.assertEqual(len(contracts), 1)
        c = contracts[0]
        self.assertEqual(c.function_name, "calculate_discount")
        self.assertIn("price >= 0", c.preconditions)
        self.assertIn("result <= price", c.postconditions)

    def test_contract_compiler_to_pytest(self):
        contract = IOContract(
            function_name="calculate_discount",
            preconditions=["price >= 0"],
            postconditions=["result <= price"],
        )
        test_harness = ContractCompiler.compile_to_pytest(
            source_code=SAMPLE_CODE,
            contracts=[contract],
        )
        self.assertIn("def test_contract_compliance_calculate_discount():", test_harness)
        self.assertIn("calculate_discount", test_harness)


if __name__ == "__main__":
    unittest.main()
