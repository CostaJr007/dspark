"""
Example 01: Audit candidate code against specification with DeepSeek Curator.
"""

import os
import sys

# Ensure dspark is importable
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from dspark import DeepSeekCurator

SAMPLE_CODE = '''
def find_median_sorted_arrays(nums1, nums2):
    merged = sorted(nums1 + nums2)
    n = len(merged)
    if n % 2 == 1:
        return float(merged[n // 2])
    else:
        return (merged[n // 2 - 1] + merged[n // 2]) / 2.0
'''

SPECIFICATION = """
Given two sorted arrays nums1 and nums2 of size m and n respectively, 
return the median of the two sorted arrays.
The overall run time complexity must be O(log (m+n)).
Empty arrays should be handled gracefully.
"""

def main():
    print("Initializing DSpark DeepSeek Curator...")
    curator = DeepSeekCurator()

    print("\nAuditing draft implementation against O(log(m+n)) requirement...")
    result = curator.audit(
        code=SAMPLE_CODE,
        specification=SPECIFICATION,
        language="python"
    )

    print(f"\nVerdict: {result.verdict.value} (Score: {result.score}/100)")
    print(f"Summary: {result.summary}")
    print(f"Time Complexity identified: {result.complexity.get('time')}")
    print(f"Optimal: {result.complexity.get('optimal')}")

    if result.critical_issues:
        print("\nCritical Issues Flagged by DeepSeek:")
        for issue in result.critical_issues:
            print(f" - {issue}")

    if result.refined_code:
        print("\n--- DeepSeek Refined O(log(m+n)) Implementation ---")
        print(result.refined_code)


if __name__ == "__main__":
    main()
