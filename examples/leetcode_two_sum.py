"""
Example: Solving LeetCode Two Sum with Formal Contracts and CEGAR Verification.
"""

from typing import List


def two_sum(nums: List[int], target: int) -> List[int]:
    """
    Finds two numbers in nums such that they add up to target.
    PRE: len(nums) >= 2
    POST: len(result) == 2
    POST: result[0] != result[1]
    POST: nums[result[0]] + nums[result[1]] == target
    """
    seen: dict[int, int] = {}
    for i, num in enumerate(nums):
        complement = target - num
        if complement in seen:
            return [seen[complement], i]
        seen[num] = i
    raise ValueError("No two sum solution found")


if __name__ == "__main__":
    nums = [2, 7, 11, 15]
    target = 9
    indices = two_sum(nums, target)
    print(f"Indices: {indices}, Values: {[nums[i] for i in indices]}")
