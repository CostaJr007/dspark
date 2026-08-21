import unittest
from dspark.search import HTMLToMarkdownParser, WebSearchEngine


class TestWebSearchEngine(unittest.TestCase):

    def test_html_cleaner(self):
        raw = """
        <html>
            <head><style>body { color: red; }</style></head>
            <body>
                <h1>Documentation Title</h1>
                <p>Here is an explanation of <code>asyncio</code> in Python.</p>
                <pre><code>import asyncio
async def main():
    pass</code></pre>
            </body>
        </html>
        """
        md = HTMLToMarkdownParser.clean_html(raw)
        self.assertIn("# Documentation Title", md)
        self.assertIn("`asyncio`", md)
        self.assertIn("```", md)
        self.assertNotIn("<style>", md)

    def test_search_engine_instance(self):
        engine = WebSearchEngine()
        res = engine.search("python", max_results=2)
        self.assertIsInstance(res, list)


if __name__ == "__main__":
    unittest.main()
