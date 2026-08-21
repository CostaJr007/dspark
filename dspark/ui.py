"""
DSpark UI Theme & TUI Renderer.
Recreates the sleek, modern aesthetic of Grok Build & Claude Code:
Rounded box panels, neon status badges, formatted tables, and syntax highlighting.
"""

import shutil
import sys
from typing import Any, Dict, List, Optional


# ANSI Color & Style Palette
class Theme:
    RESET = "\033[0m"
    BOLD = "\033[1m"
    DIM = "\033[2m"
    ITALIC = "\033[3m"
    UNDERLINE = "\033[4m"

    # Brand Colors (Cyan / Electric Blue / Emerald / Magenta)
    CYAN = "\033[38;5;51m"
    BLUE = "\033[38;5;39m"
    GREEN = "\033[38;5;48m"
    EMERALD = "\033[38;5;42m"
    YELLOW = "\033[38;5;220m"
    ORANGE = "\033[38;5;208m"
    MAGENTA = "\033[38;5;198m"
    PURPLE = "\033[38;5;141m"
    RED = "\033[38;5;196m"
    GRAY = "\033[38;5;245m"
    DARK_GRAY = "\033[38;5;238m"
    WHITE = "\033[38;5;255m"

    # Backgrounds
    BG_DARK = "\033[48;5;234m"
    BG_CYAN = "\033[48;5;24m"


def get_width(max_width: int = 86) -> int:
    cols = shutil.get_terminal_size(fallback=(86, 24)).columns
    return min(cols - 2, max_width)


def render_grok_banner(workspace: str) -> None:
    w = get_width()
    inner = w - 4

    top = f"{Theme.CYAN}╭" + "─" * (w - 2) + f"╮{Theme.RESET}"
    bot = f"{Theme.CYAN}╰" + "─" * (w - 2) + f"╯{Theme.RESET}"
    mid = f"{Theme.CYAN}├" + "─" * (w - 2) + f"┤{Theme.RESET}"
    bar = f"{Theme.CYAN}│{Theme.RESET}"

    # Status Pills
    pill_deepseek = f"{Theme.GREEN}●{Theme.RESET} DeepSeek-V4"
    pill_openai = f"{Theme.BLUE}●{Theme.RESET} OpenAI"
    pill_kimi = f"{Theme.MAGENTA}●{Theme.RESET} Kimi WebSearch"
    pill_local = f"{Theme.YELLOW}●{Theme.RESET} Local Offline"

    print()
    print(top)
    
    # Title Line
    title_text = f" ⚡ {Theme.BOLD}{Theme.WHITE}DSPARK{Theme.RESET} {Theme.DIM}v0.1.0{Theme.RESET}  {Theme.GRAY}│ Dual-Engine Speculative AI & Autonomous CLI{Theme.RESET}"
    # Calculate visible len
    print(f"{bar}  {title_text:<{inner + 18}} {bar}")
    
    print(mid)
    
    # Pills line
    pills_line = f" {pill_deepseek}   {pill_openai}   {pill_kimi}   {pill_local}"
    print(f"{bar} {pills_line:<{inner + 28}} {bar}")

    # Workspace line
    short_ws = workspace if len(workspace) < (inner - 14) else "..." + workspace[-(inner - 17):]
    ws_line = f" {Theme.GRAY}Workspace:{Theme.RESET} {Theme.WHITE}{short_ws}{Theme.RESET}"
    print(f"{bar} {ws_line:<{inner + 14}} {bar}")

    print(bot)
    print(f"{Theme.DIM}Type your instruction in natural language, {Theme.CYAN}/help{Theme.RESET}{Theme.DIM} for commands, {Theme.CYAN}/exit{Theme.RESET}{Theme.DIM} to quit.{Theme.RESET}\n")


def render_prompt_box() -> str:
    w = get_width()
    top = f"{Theme.CYAN}╭── {Theme.BOLD}{Theme.GREEN}⚡ DSpark{Theme.RESET}{Theme.CYAN} " + "─" * (w - 14) + f"╮{Theme.RESET}"
    return f"{top}\n{Theme.CYAN}│{Theme.RESET} {Theme.BOLD}{Theme.WHITE}❯{Theme.RESET} "


def render_prompt_bottom() -> None:
    w = get_width()
    bot = f"{Theme.CYAN}╰" + "─" * (w - 2) + f"╯{Theme.RESET}"
    print(bot)


def render_help_panel() -> None:
    w = get_width()
    inner = w - 4
    top = f"{Theme.BLUE}╭── {Theme.BOLD}{Theme.WHITE}Available Slash Commands{Theme.RESET}{Theme.BLUE} " + "─" * (w - 30) + f"╮{Theme.RESET}"
    bot = f"{Theme.BLUE}╰" + "─" * (w - 2) + f"╯{Theme.RESET}"
    bar = f"{Theme.BLUE}│{Theme.RESET}"

    commands = [
        ("/search <query>", "🔍 Deep Web Search for docs, APIs and errors (Kimi style)"),
        ("/fetch <url>", "📄 Fetch URL and convert to clean Markdown documentation"),
        ("/audit <file> -s <spec>", "⚖️  Audit code against strict I/O contracts & edge cases"),
        ("/refine <file> -s <spec>", "🛠️  Refine code in-place guided by counter-examples"),
        ("/local", "💻 Scan, list and test local offline models (Ollama/LM Studio)"),
        ("/files [path]", "📁 List files and directories in current workspace"),
        ("/read <file>", "📖 Read and preview file content directly in terminal"),
        ("/sh <command>", "⚡ Run local terminal command (e.g. pytest, git status)"),
        ("/clear", "🧹 Clear terminal screen"),
        ("/exit", "🚪 Exit interactive session"),
    ]

    print(f"\n{top}")
    for cmd, desc in commands:
        cmd_fmt = f"{Theme.GREEN}{cmd:<26}{Theme.RESET}"
        desc_fmt = f"{Theme.GRAY}{desc}{Theme.RESET}"
        line = f"  {cmd_fmt} {desc_fmt}"
        print(f"{bar} {line:<{inner + 12}} {bar}")
    print(f"{bot}\n")


def render_search_results(query: str, results: List[Any]) -> None:
    w = get_width()
    top = f"{Theme.MAGENTA}╭── {Theme.BOLD}{Theme.WHITE}Web Search Results: '{query}'{Theme.RESET}{Theme.MAGENTA} " + "─" * max(2, (w - len(query) - 28)) + f"╮{Theme.RESET}"
    bot = f"{Theme.MAGENTA}╰" + "─" * (w - 2) + f"╯{Theme.RESET}"
    bar = f"{Theme.MAGENTA}│{Theme.RESET}"

    print(f"\n{top}")
    for i, r in enumerate(results, 1):
        print(f"{bar}  {Theme.BOLD}{Theme.WHITE}{i}. {r.title}{Theme.RESET}")
        print(f"{bar}     {Theme.DIM}{r.url}{Theme.RESET}")
        snippet_lines = r.snippet[:120].replace("\n", " ")
        print(f"{bar}     {Theme.GRAY}{snippet_lines}...{Theme.RESET}")
        if i < len(results):
            print(f"{bar}  " + "·" * (w - 6))
    print(f"{bot}\n")


def render_audit_panel(verdict: str, score: int, summary: str, critical: List[str], counter_examples: List[Any]) -> None:
    w = get_width()
    color = Theme.GREEN if score >= 85 else (Theme.YELLOW if score >= 60 else Theme.RED)
    verdict_badge = f"{color}{Theme.BOLD}[{verdict} - {score}/100]{Theme.RESET}"

    top = f"{color}╭── {Theme.BOLD}{Theme.WHITE}DSpark Formal Audit Verdict{Theme.RESET} {verdict_badge} " + "─" * max(2, (w - len(verdict) - 40)) + f"╮{Theme.RESET}"
    bot = f"{color}╰" + "─" * (w - 2) + f"╯{Theme.RESET}"
    bar = f"{color}│{Theme.RESET}"

    print(f"\n{top}")
    print(f"{bar}  {Theme.BOLD}Summary:{Theme.RESET} {Theme.WHITE}{summary}{Theme.RESET}")

    if critical:
        print(f"{bar}")
        print(f"{bar}  {Theme.RED}{Theme.BOLD}Critical Issues Identified:{Theme.RESET}")
        for c in critical:
            print(f"{bar}    {Theme.RED}✖{Theme.RESET} {c}")

    if counter_examples:
        print(f"{bar}")
        print(f"{bar}  {Theme.YELLOW}{Theme.BOLD}Counter-Examples Synthesized (Failing Cases):{Theme.RESET}")
        for ce in counter_examples:
            inp = getattr(ce, "failing_input", str(ce))
            exp = getattr(ce, "expected_behavior", "")
            print(f"{bar}    {Theme.YELLOW}▲{Theme.RESET} Input: `{inp}` → Expected: `{exp}`")

    print(f"{bot}\n")
