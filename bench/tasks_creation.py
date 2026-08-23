"""Six mini software-creation tasks with PRE-REGISTERED pytest suites.

Each task gives the models freedom of design but grades against an objective,
pre-written contract suite. The suite imports the candidate module and checks
behavior — no LLM judging involved.
"""

# Template: candidate code is written to solution.py; suite runs against it.
CREATION_TASKS = [
    {
        "id": "create-lru-ttl",
        "description": (
            "Implement a LRUCache class with TTL expiration.\n"
            "Requirements:\n"
            "- LRUCache(capacity: int), capacity >= 1\n"
            "- get(key) -> value or None (missing/expired)\n"
            "- put(key, value, ttl_seconds=None): ttl None means never expires\n"
            "- Least-recently-used key is evicted when over capacity\n"
            "- get/put refresh recency; expired entries are invisible AND evictable\n"
            "Use time.monotonic() for time."
        ),
        "suite": """
import time
from solution import LRUCache

def test_basic_put_get():
    c = LRUCache(2)
    c.put("a", 1); c.put("b", 2)
    assert c.get("a") == 1 and c.get("b") == 2

def test_eviction():
    c = LRUCache(2)
    c.put("a", 1); c.put("b", 2); c.put("c", 3)
    assert c.get("a") is None
    assert c.get("c") == 3

def test_recency_refresh():
    c = LRUCache(2)
    c.put("a", 1); c.put("b", 2); c.get("a"); c.put("c", 3)
    assert c.get("a") == 1 and c.get("b") is None

def test_ttl_expiry():
    c = LRUCache(2)
    c.put("k", "v", ttl_seconds=0.05)
    assert c.get("k") == "v"
    time.sleep(0.08)
    assert c.get("k") is None

def test_no_ttl_never_expires():
    c = LRUCache(1)
    c.put("k", 42)
    time.sleep(0.03)
    assert c.get("k") == 42
""",
    },
    {
        "id": "create-csv-parser",
        "description": (
            "Implement parse_csv(text: str) -> list[list[str]].\n"
            "Requirements:\n"
            "- Split on commas and newlines; handle quoted fields with \"\"\n"
            "- Quotes may contain commas and newlines\n"
            "- Empty trailing line is ignored; empty fields preserved\n"
            "- Raise ValueError on unclosed quote"
        ),
        "suite": """
from solution import parse_csv

def test_simple():
    assert parse_csv("a,b\\nc,d") == [["a","b"],["c","d"]]

def test_quoted_comma():
    assert parse_csv('"x,y",z') == [["x,y","z"]]

def test_quoted_newline():
    assert parse_csv('"line1\\nline2",end') == [["line1\\nline2","end"]]

def test_escaped_quotes():
    D = chr(34)
    csv_line = D + 'say ' + D + D + 'hi' + D + D + D + ',ok'
    assert parse_csv(csv_line) == [['say ' + D + 'hi' + D, "ok"]]

def test_empty_fields():
    assert parse_csv("a,,c") == [["a","","c"]]

def test_unclosed_quote():
    import pytest
    with pytest.raises(ValueError):
        parse_csv('"oops')
""",
    },
    {
        "id": "create-rate-limiter",
        "description": (
            "Implement a TokenBucket class.\n"
            "- TokenBucket(rate_per_sec: float, capacity: int)\n"
            "- allow(n: int = 1) -> bool: consumes n tokens if available else False\n"
            "- Tokens refill continuously at rate_per_sec using time.monotonic()\n"
            "- Burst up to capacity allowed when bucket is full"
        ),
        "suite": """
import time
from solution import TokenBucket

def test_burst_to_capacity():
    b = TokenBucket(rate_per_sec=1, capacity=3)
    assert b.allow() and b.allow() and b.allow()
    assert not b.allow()

def test_refill_over_time():
    b = TokenBucket(rate_per_sec=20, capacity=1)
    assert b.allow()
    assert not b.allow()
    time.sleep(0.07)
    assert b.allow()

def test_multi_token_request():
    b = TokenBucket(rate_per_sec=1, capacity=5)
    assert b.allow(4)
    assert not b.allow(2)
    assert b.allow(1)

def test_invalid_args():
    import pytest
    with pytest.raises(ValueError):
        TokenBucket(rate_per_sec=-1, capacity=1)
""",
    },
    {
        "id": "create-diff-parser",
        "description": (
            "Implement apply_diff(original: str, diff: str) -> str.\n"
            "diff lines follow unified hunk style WITHOUT headers:\n"
            "' '-prefixed = removed, '+'-prefixed = added, ' ' prefix = context.\n"
            "Apply hunks in order; raise ValueError if a context/removal line "
            "does not match the original text at the current position."
        ),
        "suite": """
from solution import apply_diff

def test_context_only():
    assert apply_diff("a\\nb\\nc", " a\\n b\\n c") == "a\\nb\\nc"

def test_addition():
    assert apply_diff("a\\nc", " a\\n+b\\n c") == "a\\nb\\nc"

def test_removal():
    assert apply_diff("a\\nb\\nc", " a\\n-b\\n c") == "a\\nc"

def test_mixed():
    out = apply_diff("x\\ny\\nz", " x\\n-y\\n+Y!\\n z")
    assert out == "x\\nY!\\nz"

def test_mismatch_raises():
    import pytest
    with pytest.raises(ValueError):
        apply_diff("a", "-b")
""",
    },
    {
        "id": "create-interval-merge",
        "description": (
            "Implement merge_intervals(intervals: list[tuple[int,int]]) "
            "-> list[tuple[int,int]].\n"
            "- Merge overlapping or adjacent closed intervals\n"
            "- Input unsorted; must not mutate input; output sorted ascending\n"
            "- Empty input returns []; raise ValueError on start > end"
        ),
        "suite": """
from solution import merge_intervals

def test_overlap():
    assert merge_intervals([(1,3),(2,6),(8,10)]) == [(1,6),(8,10)]

def test_adjacent():
    assert merge_intervals([(1,2),(2,3)]) == [(1,3)]

def test_unsorted_and_no_mutate():
    src = [(5,7),(1,3)]
    out = merge_intervals(src)
    assert out == [(1,3),(5,7)] and src == [(5,7),(1,3)]

def test_empty():
    assert merge_intervals([]) == []

def test_invalid():
    import pytest
    with pytest.raises(ValueError):
        merge_intervals([(3,1)])
""",
    },
    {
        "id": "create-trie-autocomplete",
        "description": (
            "Implement a Trie class.\n"
            "- insert(word: str), starts_with(prefix: str) -> bool, "
            "search(word: str) -> bool (exact)\n"
            "- autocomplete(prefix: str, limit: int = 5) -> list[str]: up to limit "
            "words with that prefix, shortest-first then alphabetical\n"
            "- search/starts_with on empty string: search False; starts_with True"
        ),
        "suite": """
from solution import Trie

def test_insert_search():
    t = Trie(); t.insert("cat")
    assert t.search("cat") and not t.search("ca") and not t.search("dog")

def test_starts_with():
    t = Trie(); t.insert("hello")
    assert t.starts_with("hel") and t.starts_with("") and not t.starts_with("world")

def test_autocomplete_order():
    t = Trie()
    for w in ["bat","bar","barn","ba","b"]:
        t.insert(w)
    assert t.autocomplete("ba") == ["ba","bar","bat","barn"]
    assert t.autocomplete("ba", limit=2) == ["ba","bar"]

def test_autocomplete_empty_prefix():
    t = Trie(); t.insert("z"); t.insert("a")
    assert t.autocomplete("") == ["a","z"]

def test_limit_zero():
    t = Trie(); t.insert("x")
    assert t.autocomplete("x", limit=0) == []
""",
    },
]
