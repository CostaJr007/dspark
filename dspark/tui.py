"""
DSpark Full-Featured Terminal User Interface (TUI).
Faithfully reproduces the Grok Build & Claude Code interactive environment,
with support for authentic Bloomberg Terminal Amber Theme, Grok Cyan, and Matrix Green.
"""

import os
import sys
from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Optional

from prompt_toolkit import PromptSession
from prompt_toolkit.auto_suggest import AutoSuggestFromHistory
from prompt_toolkit.completion import WordCompleter
from prompt_toolkit.formatted_text import HTML
from prompt_toolkit.history import InMemoryHistory
from prompt_toolkit.styles import Style

from rich.console import Console
from rich.markdown import Markdown
from rich.panel import Panel
from rich.syntax import Syntax
from rich.table import Table
from rich.text import Text

from .agent import DSparkAgent
from .client import LocalLLMClient, create_model_client


console = Console()


@dataclass
class ColorTheme:
    name: str
    primary_color: str        # Hex / ANSI
    secondary_color: str
    accent_green: str
    accent_yellow: str
    accent_red: str
    border_style: str
    prompt_fg: str
    toolbar_bg: str
    toolbar_key_bg: str


THEMES: Dict[str, ColorTheme] = {
    "bloomberg": ColorTheme(
        name="Bloomberg Terminal (Amber Classic)",
        primary_color="#ff9900",       # Bloomberg Amber
        secondary_color="#ffaa00",
        accent_green="#00ff66",        # Ticker Green
        accent_yellow="#ffcc00",       # Amber Gold
        accent_red="#ff3333",          # Alert Red
        border_style="bold #ff9900",
        prompt_fg="#ff9900",
        toolbar_bg="#111111",
        toolbar_key_bg="#222222",
    ),
    "grok": ColorTheme(
        name="Grok Build (Electric Cyan)",
        primary_color="#00ffcc",
        secondary_color="#89b4fa",
        accent_green="#a6e3a1",
        accent_yellow="#f9e2af",
        accent_red="#f38ba8",
        border_style="bright_cyan",
        prompt_fg="#00ffcc",
        toolbar_bg="#1e1e2e",
        toolbar_key_bg="#313244",
    ),
    "matrix": ColorTheme(
        name="Matrix Phosphor (Hacker Green)",
        primary_color="#00ff41",
        secondary_color="#008f11",
        accent_green="#00ff41",
        accent_yellow="#adff2f",
        accent_red="#ff0033",
        border_style="bold #00ff41",
        prompt_fg="#00ff41",
        toolbar_bg="#0d1117",
        toolbar_key_bg="#161b22",
    ),
}

SLASH_COMMANDS = [
    "/theme",
    "/models",
    "/search",
    "/fetch",
    "/audit",
    "/refine",
    "/local",
    "/files",
    "/read",
    "/sh",
    "/clear",
    "/help",
    "/exit",
]


class GrokBuildTUI:
    """
    Interactive terminal agent loop with support for Bloomberg Terminal theme,
    Grok Build layout, and live speculative AI verification.
    """

    def __init__(
        self,
        working_dir: Optional[str] = None,
        generator_model: str = "gpt-4o-mini",
        curator_model: str = "deepseek-v4-flash",
        theme_name: str = "bloomberg",
    ):
        self.working_dir = os.path.abspath(working_dir or os.getcwd())
        self.generator_model = generator_model
        self.curator_model = curator_model
        self.current_theme = THEMES.get(theme_name.lower(), THEMES["bloomberg"])

        self.agent = DSparkAgent(
            working_dir=self.working_dir,
            client=create_model_client(self.generator_model),
        )

        self.completer = WordCompleter(SLASH_COMMANDS, sentence=True)
        self._init_session()

    def _init_session(self) -> None:
        style = Style.from_dict({
            "prompt": f"{self.current_theme.prompt_fg} bold",
            "arrow": "#ffffff bold",
            "bottom-toolbar": f"bg:{self.current_theme.toolbar_bg} #ffffff",
            "bottom-toolbar.key": f"bg:{self.current_theme.toolbar_key_bg} {self.current_theme.primary_color} bold",
            "bottom-toolbar.val": f"bg:{self.current_theme.toolbar_bg} {self.current_theme.accent_yellow} bold",
            "bottom-toolbar.cur": f"bg:{self.current_theme.toolbar_bg} {self.current_theme.accent_green} bold",
        })
        self.session = PromptSession(
            history=InMemoryHistory(),
            auto_suggest=AutoSuggestFromHistory(),
            completer=self.completer,
            style=style,
        )

    def set_theme(self, theme_key: str) -> None:
        key = theme_key.lower().strip()
        if key in THEMES:
            self.current_theme = THEMES[key]
            self._init_session()
            console.print(f"\n[bold green]✓ Switched theme to:[/bold green] [bold {self.current_theme.primary_color}]{self.current_theme.name}[/bold {self.current_theme.primary_color}]\n")
            self.render_header()
        else:
            console.print(f"[bold red]Unknown theme '{theme_key}'. Available: {', '.join(THEMES.keys())}[/bold red]")

    def render_header(self) -> None:
        grid = Table.grid(expand=True)
        grid.add_column(justify="left")
        grid.add_column(justify="right")

        brand_badge = f"[{self.current_theme.primary_color}]⚡ DSPARK[/{self.current_theme.primary_color}] [bold white]v0.1.0[/bold white] │ [dim white]Dual-Engine Speculative AI[/dim white]"
        status_badges = f"[{self.current_theme.accent_green}]● DeepSeek-V4[/{self.current_theme.accent_green}]  [{self.current_theme.primary_color}]● OpenAI[/{self.current_theme.primary_color}]  [{self.current_theme.accent_yellow}]● Kimi WebSearch[/{self.current_theme.accent_yellow}]  [white]● Bloomberg Terminal[/white]"
        grid.add_row(brand_badge, status_badges)

        models_info = (
            f"  [dim white]Generator:[/dim white] [{self.current_theme.accent_yellow} bold]{self.generator_model}[/{self.current_theme.accent_yellow} bold] "
            f"│ [dim white]Curator:[/dim white] [{self.current_theme.accent_green} bold]{self.curator_model}[/{self.current_theme.accent_green} bold] "
            f"│ [dim white]Theme:[/dim white] [{self.current_theme.primary_color}]{self.current_theme.name.split(' ')[0]}[/{self.current_theme.primary_color}] "
            f"│ [dim white]Workspace:[/dim white] [white]{self.working_dir}[/white]"
        )

        panel = Panel(
            grid,
            subtitle=models_info,
            subtitle_align="left",
            border_style=self.current_theme.border_style,
            padding=(0, 1),
        )
        console.print()
        console.print(panel)
        console.print(f"[dim]Type instruction in natural language, [{self.current_theme.primary_color}]/theme[/{self.current_theme.primary_color}] for Bloomberg/Grok, [{self.current_theme.primary_color}]/models[/{self.current_theme.primary_color}] to switch models, [{self.current_theme.primary_color}]/help[/{self.current_theme.primary_color}] for commands.[/dim]\n")

    def get_bottom_toolbar(self) -> HTML:
        return HTML(
            f' <b><style fg="{self.current_theme.primary_color}">⚡ DSPARK</style></b> │ '
            f'<style fg="#ffffff">Gen:</style> <b><style fg="{self.current_theme.accent_yellow}">{self.generator_model}</style></b> │ '
            f'<style fg="#ffffff">Curator:</style> <b><style fg="{self.current_theme.accent_green}">{self.curator_model}</style></b> │ '
            f'<style fg="{self.current_theme.primary_color}">[BLOOMBERG AMBER]</style> │ '
            f'<style fg="#888888">Tab: Complete</style>'
        )

    def render_help(self) -> None:
        table = Table(
            title="⚡ Available Interactive Slash Commands",
            title_style=f"bold {self.current_theme.primary_color}",
            border_style=self.current_theme.border_style,
            expand=True,
        )
        table.add_column("Command", style=f"bold {self.current_theme.primary_color}", width=26)
        table.add_column("Description", style="dim white")

        commands = [
            ("/theme [name]", "🎨 Switch UI theme (bloomberg, grok, matrix)"),
            ("/models", "🤖 Interactively switch active Generator & Curator models"),
            ("/search <query>", "🔍 Kimi-style deep web search for live documentation and error fixes"),
            ("/fetch <url>", "📄 Fetch URL and convert to clean Markdown documentation"),
            ("/audit <file> -s <spec>", "⚖️  Formal reasoning audit against I/O contracts & edge cases"),
            ("/refine <file> -s <spec>", "🛠️  Surgical code refinement guided by counter-examples"),
            ("/local", "💻 Scan and list local offline LLMs (Ollama / LM Studio)"),
            ("/files [path]", "📁 List files and directories in current workspace"),
            ("/read <file>", "📖 Read and preview file content directly in terminal"),
            ("/sh <command>", "⚡ Execute native shell command (e.g. pytest, git status)"),
            ("/clear", "🧹 Clear terminal screen"),
            ("/exit", "🚪 Exit DSpark interactive session"),
        ]
        for cmd, desc in commands:
            table.add_row(cmd, desc)

        console.print()
        console.print(table)
        console.print()

    def render_models_menu(self) -> None:
        local_models = []
        active = LocalLLMClient.detect_active_endpoints()
        for s in active:
            models = LocalLLMClient(base_url=s["v1_url"]).list_models()
            local_models.extend(models)

        table = Table(
            title="🤖 Select Active AI Models for DSpark Engine",
            title_style=f"bold {self.current_theme.primary_color}",
            border_style=self.current_theme.border_style,
            expand=True,
        )
        table.add_column("Key", style=f"bold {self.current_theme.primary_color}", width=6, justify="center")
        table.add_column("Preset Name", style="bold white", width=32)
        table.add_column("Generator Model (Draft)", style="yellow")
        table.add_column("Curator Model (Verifier)", style="green")

        table.add_row("[1]", "Ultra-Fast & Cost-Efficient", "gpt-4o-mini", "deepseek-v4-flash")
        table.add_row("[2]", "Maximum Reasoning Accuracy", "gemini-3.7-flash", "deepseek-v4-pro")
        table.add_row("[3]", "Pure DeepSeek Ecosystem", "deepseek-v4-flash", "deepseek-v4-pro")

        if local_models:
            for idx, lm in enumerate(local_models, start=4):
                table.add_row(f"[{idx}]", f"Local Offline ({lm})", f"local:{lm}", f"local:{lm}")

        table.add_row("[c]", "Custom Configuration", "Type custom...", "Type custom...")
        table.add_row("[q]", "Cancel / Keep Current", f"{self.generator_model}", f"{self.curator_model}")

        console.print()
        console.print(table)
        console.print()

        choice = input("Select option [1-3, c, q]: ").strip().lower()
        if choice == "1":
            self.generator_model = "gpt-4o-mini"
            self.curator_model = "deepseek-v4-flash"
        elif choice == "2":
            self.generator_model = "gemini-3.7-flash"
            self.curator_model = "deepseek-v4-pro"
        elif choice == "3":
            self.generator_model = "deepseek-v4-flash"
            self.curator_model = "deepseek-v4-pro"
        elif choice.isdigit() and int(choice) >= 4 and (int(choice) - 4) < len(local_models):
            lm = local_models[int(choice) - 4]
            self.generator_model = f"local:{lm}"
            self.curator_model = f"local:{lm}"
        elif choice == "c":
            new_g = input("  Enter Generator model (e.g. gpt-4o, qwen2.5-coder:7b): ").strip()
            new_c = input("  Enter Curator model (e.g. deepseek-v4-pro, local:deepseek-r1): ").strip()
            if new_g:
                self.generator_model = new_g
            if new_c:
                self.curator_model = new_c
        elif choice in ("q", ""):
            console.print("[dim]Kept active models.[/dim]\n")
            return

        self.agent.client = create_model_client(self.generator_model)
        console.print(f"\n[bold green]✓ Models updated![/bold green] Active Pairing: [{self.current_theme.accent_yellow}]{self.generator_model}[/{self.current_theme.accent_yellow}] + [{self.current_theme.accent_green}]{self.curator_model}[/{self.current_theme.accent_green}]\n")
        self.render_header()

    def run(self) -> None:
        self.render_header()

        while True:
            try:
                user_input = self.session.prompt(
                    HTML(f'<b><style fg="{self.current_theme.primary_color}">DSpark</style></b> <style fg="#ffffff">❯</style> '),
                    bottom_toolbar=self.get_bottom_toolbar,
                ).strip()

                if not user_input:
                    continue

                if user_input in ("/exit", "/quit", "exit", "quit"):
                    console.print(f"[{self.current_theme.accent_yellow}]Exiting DSpark session. Happy coding![/{self.current_theme.accent_yellow}]")
                    break

                elif user_input in ("/clear", "clear"):
                    os.system("cls" if os.name == "nt" else "clear")
                    self.render_header()
                    continue

                elif user_input in ("/help", "help"):
                    self.render_help()
                    continue

                elif user_input.startswith("/theme"):
                    parts = user_input.split(maxsplit=1)
                    if len(parts) > 1:
                        self.set_theme(parts[1])
                    else:
                        console.print(f"\n[bold {self.current_theme.primary_color}]Available Themes:[/bold {self.current_theme.primary_color}]")
                        for t_key, t_val in THEMES.items():
                            console.print(f"  • [{t_val.primary_color}]{t_key}[/{t_val.primary_color}]: {t_val.name}")
                        console.print(f"\nUsage: [dim]/theme bloomberg[/dim] or [dim]/theme grok[/dim] or [dim]/theme matrix[/dim]\n")
                    continue

                elif user_input in ("/models", "models", "/model", "/select"):
                    self.render_models_menu()
                    continue

                elif user_input in ("/local", "local"):
                    active = LocalLLMClient.detect_active_endpoints()
                    if not active:
                        console.print("\n[yellow][!] No active local LLM detected. Start Ollama (ollama run qwen2.5-coder:1.5b) or LM Studio.[/yellow]\n")
                    else:
                        console.print(f"\n[green][✓] Found {len(active)} active local server(s):[/green]")
                        for s in active:
                            console.print(f"  * [bold]{s['name']}[/bold] ({s['v1_url']})")
                            models = LocalLLMClient(base_url=s["v1_url"]).list_models()
                            for m in models:
                                console.print(f"    - [{self.current_theme.primary_color}]{m}[/{self.current_theme.primary_color}]")
                        console.print()
                    continue

                elif user_input.startswith("/search "):
                    query = user_input[8:].strip()
                    with console.status(f"[bold {self.current_theme.primary_color}]🔍 Searching web with Kimi Engine for: '{query}'...[/bold {self.current_theme.primary_color}]", spinner="dots"):
                        results = self.agent.search_engine.search(query, max_results=5)

                    table = Table(
                        title=f"🔍 Web Search Results: '{query}'",
                        title_style=f"bold {self.current_theme.primary_color}",
                        border_style=self.current_theme.border_style,
                        expand=True,
                    )
                    table.add_column("#", width=3, justify="right")
                    table.add_column("Result / URL / Snippet")

                    for i, r in enumerate(results, 1):
                        txt = Text(f"{r.title}\n", style="bold white") + Text(f"{r.url}\n", style=f"dim {self.current_theme.primary_color} underline") + Text(f"{r.snippet[:140]}...", style="dim")
                        table.add_row(str(i), txt)

                    console.print()
                    console.print(table)
                    console.print()
                    continue

                elif user_input.startswith("/fetch "):
                    url = user_input[7:].strip()
                    with console.status(f"[bold {self.current_theme.primary_color}]📄 Fetching & parsing Markdown from: '{url}'...[/bold {self.current_theme.primary_color}]", spinner="dots"):
                        content = self.agent.fetch_url(url)
                    console.print(Panel(Markdown(content[:3000]), title=f"📄 Page Content: {url}", border_style=self.current_theme.border_style))
                    continue

                elif user_input.startswith("/files"):
                    parts = user_input.split(maxsplit=1)
                    subpath = parts[1] if len(parts) > 1 else "."
                    files = self.agent.list_files(subpath)
                    console.print(f"\n[bold {self.current_theme.primary_color}]Files in {subpath} ({len(files)} items):[/bold {self.current_theme.primary_color}]")
                    for f in files:
                        console.print(f"  [dim]•[/dim] {f}")
                    console.print()
                    continue

                elif user_input.startswith("/read "):
                    fpath = user_input[6:].strip()
                    try:
                        content = self.agent.read_file(fpath)
                        syntax = Syntax(content, "python" if fpath.endswith(".py") else "text", line_numbers=True)
                        console.print(Panel(syntax, title=f"📖 {fpath}", border_style=self.current_theme.border_style))
                    except Exception as e:
                        console.print(f"[bold red]Error: {e}[/bold red]")
                    continue

                elif user_input.startswith("/sh "):
                    cmd = user_input[4:].strip()
                    with console.status(f"[bold {self.current_theme.accent_yellow}]⚡ Running command: '{cmd}'...[/bold {self.current_theme.accent_yellow}]", spinner="line"):
                        res = self.agent.run_terminal(cmd)
                    console.print(Panel(res, title=f"⚡ Output: {cmd}", border_style=self.current_theme.border_style))
                    continue

                # Standard Natural Language Task -> Execute with live spinner
                with console.status(f"[bold {self.current_theme.primary_color}]⚡ Executing Metacognitive Reasoning Engine ({self.generator_model} + {self.curator_model})...[/bold {self.current_theme.primary_color}]", spinner="arc"):
                    response = self.agent.execute_task(user_input)

                console.print()
                console.print(Panel(Markdown(response), title=f"[{self.current_theme.primary_color}]⚡ DSpark Solution[/{self.current_theme.primary_color}]", border_style=self.current_theme.border_style, padding=(1, 2)))
                console.print()

            except KeyboardInterrupt:
                console.print(f"\n[{self.current_theme.accent_yellow}]Interrupted. Type /exit to quit.[/{self.current_theme.accent_yellow}]")
            except EOFError:
                break
            except Exception as e:
                console.print(f"\n[bold red]Error: {e}[/bold red]\n")
