"""
DSpark Full-Featured Terminal User Interface (TUI).
Faithfully reproduces the Grok Build & Claude Code interactive environment:
- Reactive prompt_toolkit session with live Tab auto-completion & history
- Live Bottom Status Toolbar (active models, workspace, memory)
- Animated reasoning spinners (rich.status)
- Syntax-highlighted code panels, tables, and tool execution cards
"""

import os
import sys
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

PROMPT_STYLE = Style.from_dict({
    "prompt": "#00ffcc bold",
    "arrow": "#ffffff bold",
    "bottom-toolbar": "bg:#1e1e2e #a6adc8",
    "bottom-toolbar.key": "bg:#313244 #f5c2e7 bold",
    "bottom-toolbar.val": "bg:#1e1e2e #89b4fa bold",
    "bottom-toolbar.cur": "bg:#1e1e2e #a6e3a1 bold",
})

SLASH_COMMANDS = [
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
    Stateful interactive terminal agent loop modeled after Grok Build.
    """

    def __init__(
        self,
        working_dir: Optional[str] = None,
        generator_model: str = "gpt-4o-mini",
        curator_model: str = "deepseek-v4-flash",
    ):
        self.working_dir = os.path.abspath(working_dir or os.getcwd())
        self.generator_model = generator_model
        self.curator_model = curator_model

        self.agent = DSparkAgent(
            working_dir=self.working_dir,
            client=create_model_client(self.generator_model),
        )

        self.completer = WordCompleter(SLASH_COMMANDS, sentence=True)
        self.session = PromptSession(
            history=InMemoryHistory(),
            auto_suggest=AutoSuggestFromHistory(),
            completer=self.completer,
            style=PROMPT_STYLE,
        )

    def render_header(self) -> None:
        grid = Table.grid(expand=True)
        grid.add_column(justify="left")
        grid.add_column(justify="right")

        title = Text("⚡ DSPARK ", style="bold white on #1e1e2e") + Text("v0.1.0 ", style="dim") + Text("│ Dual-Engine Speculative AI & Autonomous CLI", style="cyan")
        status_badges = Text("● DeepSeek-V4  ", style="bold green") + Text("● OpenAI  ", style="bold blue") + Text("● Kimi Search  ", style="bold magenta") + Text("● Local", style="bold yellow")
        grid.add_row(title, status_badges)

        models_info = Text(f"  Generator: ", style="dim") + Text(f"{self.generator_model} ", style="bold yellow") + Text("│ Curator: ", style="dim") + Text(f"{self.curator_model} ", style="bold green") + Text("│ Workspace: ", style="dim") + Text(f"{self.working_dir}", style="white")

        panel = Panel(
            grid,
            subtitle=models_info,
            subtitle_align="left",
            border_style="bright_cyan",
            padding=(0, 1),
        )
        console.print()
        console.print(panel)
        console.print("[dim]Type your instruction, [cyan]/models[/cyan] to switch models, [cyan]/help[/cyan] for commands, [cyan]/exit[/cyan] to quit.[/dim]\n")

    def get_bottom_toolbar(self) -> HTML:
        return HTML(
            f' <b><style fg="#00ffcc">⚡ DSpark</style></b> │ '
            f'<style fg="#cdd6f4">Gen:</style> <b><style fg="#f9e2af">{self.generator_model}</style></b> │ '
            f'<style fg="#cdd6f4">Curator:</style> <b><style fg="#a6e3a1">{self.curator_model}</style></b> │ '
            f'<style fg="#6c7086">Tab: Complete</style>'
        )

    def render_help(self) -> None:
        table = Table(
            title="⚡ Available Interactive Slash Commands",
            title_style="bold cyan",
            border_style="blue",
            expand=True,
        )
        table.add_column("Command", style="bold green", width=26)
        table.add_column("Description", style="dim white")

        commands = [
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
            title_style="bold yellow",
            border_style="yellow",
            expand=True,
        )
        table.add_column("Key", style="bold cyan", width=6, justify="center")
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
        console.print(f"\n[bold green]✓ Models updated![/bold green] Active Pairing: [yellow]{self.generator_model}[/yellow] + [green]{self.curator_model}[/green]\n")
        self.render_header()

    def run(self) -> None:
        self.render_header()

        while True:
            try:
                user_input = self.session.prompt(
                    HTML('<b><style fg="#00ffcc">DSpark</style></b> <style fg="#ffffff">❯</style> '),
                    bottom_toolbar=self.get_bottom_toolbar,
                ).strip()

                if not user_input:
                    continue

                if user_input in ("/exit", "/quit", "exit", "quit"):
                    console.print("[yellow]Exiting DSpark session. Happy coding![/yellow]")
                    break

                elif user_input in ("/clear", "clear"):
                    os.system("cls" if os.name == "nt" else "clear")
                    self.render_header()
                    continue

                elif user_input in ("/help", "help"):
                    self.render_help()
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
                                console.print(f"    - [cyan]{m}[/cyan]")
                        console.print()
                    continue

                elif user_input.startswith("/search "):
                    query = user_input[8:].strip()
                    with console.status(f"[bold magenta]🔍 Searching web with Kimi Engine for: '{query}'...[/bold magenta]", spinner="dots"):
                        results = self.agent.search_engine.search(query, max_results=5)

                    table = Table(title=f"🔍 Web Search Results: '{query}'", title_style="bold magenta", border_style="magenta", expand=True)
                    table.add_column("#", width=3, justify="right")
                    table.add_column("Result / URL / Snippet")

                    for i, r in enumerate(results, 1):
                        txt = Text(f"{r.title}\n", style="bold white") + Text(f"{r.url}\n", style="dim underline") + Text(f"{r.snippet[:140]}...", style="dim")
                        table.add_row(str(i), txt)

                    console.print()
                    console.print(table)
                    console.print()
                    continue

                elif user_input.startswith("/fetch "):
                    url = user_input[7:].strip()
                    with console.status(f"[bold cyan]📄 Fetching & parsing Markdown from: '{url}'...[/bold cyan]", spinner="dots"):
                        content = self.agent.fetch_url(url)
                    console.print(Panel(Markdown(content[:3000]), title=f"📄 Page Content: {url}", border_style="cyan"))
                    continue

                elif user_input.startswith("/files"):
                    parts = user_input.split(maxsplit=1)
                    subpath = parts[1] if len(parts) > 1 else "."
                    files = self.agent.list_files(subpath)
                    console.print(f"\n[bold cyan]Files in {subpath} ({len(files)} items):[/bold cyan]")
                    for f in files:
                        console.print(f"  [dim]•[/dim] {f}")
                    console.print()
                    continue

                elif user_input.startswith("/read "):
                    fpath = user_input[6:].strip()
                    try:
                        content = self.agent.read_file(fpath)
                        syntax = Syntax(content, "python" if fpath.endswith(".py") else "text", line_numbers=True)
                        console.print(Panel(syntax, title=f"📖 {fpath}", border_style="blue"))
                    except Exception as e:
                        console.print(f"[bold red]Error: {e}[/bold red]")
                    continue

                elif user_input.startswith("/sh "):
                    cmd = user_input[4:].strip()
                    with console.status(f"[bold yellow]⚡ Running command: '{cmd}'...[/bold yellow]", spinner="line"):
                        res = self.agent.run_terminal(cmd)
                    console.print(Panel(res, title=f"⚡ Output: {cmd}", border_style="yellow"))
                    continue

                # Standard Natural Language Task -> Execute with Grok-Build live spinner
                with console.status(f"[bold cyan]⚡ Executing Metacognitive Reasoning Engine ({self.generator_model} + {self.curator_model})...[/bold cyan]", spinner="arc"):
                    response = self.agent.execute_task(user_input)

                console.print()
                console.print(Panel(Markdown(response), title="⚡ DSpark Solution", border_style="bright_cyan", padding=(1, 2)))
                console.print()

            except KeyboardInterrupt:
                console.print("\n[yellow]Interrupted. Type /exit to quit.[/yellow]")
            except EOFError:
                break
            except Exception as e:
                console.print(f"\n[bold red]Error: {e}[/bold red]\n")
