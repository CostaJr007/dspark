"""
Unit tests for AST Parser and Contract Compiler.
"""

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


def test_ast_parser_extraction():
    funcs = extract_functions_and_docstrings(SAMPLE_CODE)
    assert len(funcs) == 1
    fn = funcs[0]
    assert fn["name"] == "calculate_discount"
    assert fn["args"] == ["price", "discount_percent"]
    assert len(fn["preconditions"]) == 2
    assert len(fn["postconditions"]) == 2


def test_infer_contracts_from_ast():
    contracts = infer_contracts_from_ast(SAMPLE_CODE)
    assert len(contracts) == 1
    c = contracts[0]
    assert c.function_name == "calculate_discount"
    assert "price >= 0" in c.preconditions
    assert "result <= price" in c.postconditions


def test_contract_compiler_to_pytest():
    contract = IOContract(
        function_name="calculate_discount",
        preconditions=["price >= 0"],
        postconditions=["result <= price"],
    )
    test_harness = ContractCompiler.compile_to_pytest(
        source_code=SAMPLE_CODE,
        contracts=[contract],
    )
    assert "def test_contract_compliance_calculate_discount():" in test_harness
    assert "calculate_discount" in test_harness
