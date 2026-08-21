"""
Example 02: Arbitrate between two candidate implementations.
"""

import os
import sys

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from dspark import DeepSeekCurator

CANDIDATE_A = '''
def fibonacci(n: int) -> int:
    if n <= 0:
        return 0
    elif n == 1:
        return 1
    return fibonacci(n - 1) + fibonacci(n - 2)
'''

CANDIDATE_B = '''
def fibonacci(n: int) -> int:
    if n <= 0:
        return 0
    if n == 1:
        return 1
    a, b = 0, 1
    for _ in range(2, n + 1):
        a, b = b, a + b
    return b
'''

SPECIFICATION = "Calculate the N-th Fibonacci number efficiently for n up to 100,000 with O(1) auxiliary space."

def main():
    curator = DeepSeekCurator()
    print("Arbitrating between Candidate A (Recursive) and Candidate B (Iterative)...")
    result = curator.arbitrate(
        candidates=[CANDIDATE_A, CANDIDATE_B],
        specification=SPECIFICATION,
        language="python"
    )

    print(f"\nWinning Candidate Index: #{result.winner_index}")
    print(f"Rationale: {result.rationale}\n")
    print("Synthesized Optimal Code:")
    print(result.synthesized_code)


if __name__ == "__main__":
    main()
