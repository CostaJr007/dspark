"""
Command-line interface and interactive terminal agent for DSpark.
"""

import argparse
import os
import sys
from typing import Optional

try:
    import readline
except ImportError:
    pass

# Ensure UTF-8 output encoding on Windows consoles
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

from .agent import DSparkAgent
from .benchmark import DSparkBenchmarkRunner
from .client import DeepSeekClient, create_model_client, LocalLLMClient
from .curator import DeepSeekCurator
from .mcp_server import run_mcp_server
from .pipeline import DSparkPipeline
from .search import WebSearchEngine
from .ui import (
    Theme,
    render_banner,
    render_prompt_box,
    render_prompt_bottom,
    render_help_panel,
    render_model_selector_menu,
    render_search_results,
    render_audit_panel,
)


def _read_file_or_string(val: str) -> str:
    """If val is an existing file path, read its content. Otherwise return val."""
    if os.path.exists(val):
        with open(val, "r", encoding="utf-8") as f:
            return f.read()
    return val


def start_interactive_session(
    working_dir: Optional[str] = None,
    generator_model: str = "gpt-4o-mini",
    curator_model: str = "deepseek-v4-flash",
    theme: str = "bloomberg",
):
    """Interactive terminal UI (bloomberg, cyan, matrix palettes)."""
    try:
        from .tui import SparkTUI
        tui = SparkTUI(
            working_dir=working_dir,
            generator_model=generator_model,
            curator_model=curator_model,
            theme_name=theme,
        )
        tui.run()
        return
    except Exception as e:
        sys.stderr.write(f"TUI notice: {e}\n")

    current_gen = generator_model
    current_cur = curator_model

    agent = DSparkAgent(
        working_dir=working_dir,
        client=create_model_client(current_gen),
    )
    render_banner(str(agent.working_dir), generator_model=current_gen, curator_model=current_cur)

    while True:
        try:
            prompt_str = render_prompt_box()
            user_input = input(prompt_str).strip()
            render_prompt_bottom()

            if not user_input:
                continue

            if user_input in ("/exit", "/quit", "exit", "quit"):
                print(f"{Theme.YELLOW}Exiting DSpark session. Happy coding!{Theme.RESET}")
                break

            elif user_input in ("/clear", "clear"):
                os.system("cls" if os.name == "nt" else "clear")
                render_banner(str(agent.working_dir), generator_model=current_gen, curator_model=current_cur)
                continue

            elif user_input in ("/help", "help"):
                render_help_panel()
                continue

            elif user_input in ("/models", "models", "/model", "/select"):
                # Collect any running local models
                local_models = []
                active = LocalLLMClient.detect_active_endpoints()
                for s in active:
                    models = LocalLLMClient(base_url=s["v1_url"]).list_models()
                    local_models.extend(models)

                render_model_selector_menu(current_gen, current_cur, local_models)
                choice = input(f"{Theme.CYAN}Select option [1-3, c, q]: {Theme.RESET}").strip().lower()

                if choice == "1":
                    current_gen = "gpt-4o-mini"
                    current_cur = "deepseek-v4-flash"
                elif choice == "2":
                    current_gen = "gemini-3.7-flash"
                    current_cur = "deepseek-v4-pro"
                elif choice == "3":
                    current_gen = "deepseek-v4-flash"
                    current_cur = "deepseek-v4-pro"
                elif choice.isdigit() and int(choice) >= 4 and (int(choice) - 4) < len(local_models):
                    lm = local_models[int(choice) - 4]
                    current_gen = f"local:{lm}"
                    current_cur = f"local:{lm}"
                elif choice == "c":
                    new_g = input(f"  {Theme.GRAY}Enter Generator model (e.g. gpt-4o, qwen2.5-coder:7b): {Theme.RESET}").strip()
                    new_c = input(f"  {Theme.GRAY}Enter Curator model (e.g. deepseek-v4-pro, local:deepseek-r1): {Theme.RESET}").strip()
                    if new_g:
                        current_gen = new_g
                    if new_c:
                        current_cur = new_c
                elif choice in ("q", ""):
                    print(f"  {Theme.GRAY}Kept active models.{Theme.RESET}\n")
                    continue

                agent.client = create_model_client(current_gen)
                print(f"\n  {Theme.GREEN}✓ Models updated! Active Pairing: {Theme.YELLOW}{current_gen}{Theme.RESET} + {Theme.GREEN}{current_cur}{Theme.RESET}\n")
                render_banner(str(agent.working_dir), generator_model=current_gen, curator_model=current_cur)
                continue

            elif user_input in ("/local", "local"):
                active = LocalLLMClient.detect_active_endpoints()
                if not active:
                    print(f"\n{Theme.YELLOW}[!] No active local LLM detected. Start Ollama (ollama run qwen2.5-coder:1.5b) or LM Studio.{Theme.RESET}\n")
                else:
                    print(f"\n{Theme.GREEN}[✓] Found {len(active)} active local server(s):{Theme.RESET}")
                    for s in active:
                        print(f"  * {Theme.BOLD}{s['name']}{Theme.RESET} ({s['v1_url']})")
                        models = LocalLLMClient(base_url=s['v1_url']).list_models()
                        for m in models:
                            print(f"    - {Theme.CYAN}{m}{Theme.RESET}")
                    print()
                continue

            elif user_input.startswith("/search "):
                query = user_input[8:].strip()
                print(f"{Theme.GRAY}Searching web for: {query}...{Theme.RESET}")
                results = agent.search_engine.search(query, max_results=5)
                render_search_results(query, results)
                continue

            elif user_input.startswith("/fetch "):
                url = user_input[7:].strip()
                print(f"{Theme.GRAY}Fetching content from: {url}...{Theme.RESET}\n")
                res = agent.fetch_url(url)
                print(res)
                continue

            elif user_input.startswith("/files"):
                parts = user_input.split(maxsplit=1)
                subpath = parts[1] if len(parts) > 1 else "."
                files = agent.list_files(subpath)
                print(f"\n{Theme.CYAN}Files in {subpath} ({len(files)} items):{Theme.RESET}")
                for f in files:
                    print(f"  {Theme.GRAY}•{Theme.RESET} {f}")
                print()
                continue

            elif user_input.startswith("/read "):
                fpath = user_input[6:].strip()
                try:
                    content = agent.read_file(fpath)
                    print(f"\n{Theme.BLUE}─── {fpath} ───{Theme.RESET}\n{content}\n")
                except Exception as e:
                    print(f"{Theme.RED}Error: {e}{Theme.RESET}")
                continue

            elif user_input.startswith("/sh "):
                cmd = user_input[4:].strip()
                print(f"{Theme.GRAY}Running: {cmd}{Theme.RESET}")
                res = agent.run_terminal(cmd)
                print(res)
                continue

            # Standard Natural Language Task -> Execute Metacognitive Protocol
            print(f"\n{Theme.CYAN}⚡ Executing Metacognitive Reasoning Engine...{Theme.RESET}\n")
            response = agent.execute_task(user_input)
            print(f"{response}\n")

        except KeyboardInterrupt:
            print(f"\n{Theme.YELLOW}Interrupted. Type /exit to quit.{Theme.RESET}")
        except EOFError:
            break
        except Exception as e:
            print(f"\n{Theme.RED}Error: {e}{Theme.RESET}\n")


def main():
    parser = argparse.ArgumentParser(
        prog="dspark",
        description="DSpark: Dual-LLM Speculative Engine, Autonomous CLI & Web Research Agent",
    )
    subparsers = parser.add_subparsers(dest="command", help="Available subcommands")

    # Command: search
    search_p = subparsers.add_parser("search", help="Perform deep web search for docs or error fixes")
    search_p.add_argument("query", type=str, help="Search query string")
    search_p.add_argument("--sources", "-n", type=int, default=5, help="Number of search results")

    # Command: fetch
    fetch_p = subparsers.add_parser("fetch", help="Fetch and convert a documentation page to clean Markdown")
    fetch_p.add_argument("url", type=str, help="Target URL to scrape")

    # Command: test-connection
    test_p = subparsers.add_parser("test-connection", help="Test DeepSeek API key and endpoint")
    test_p.add_argument("--model", type=str, default=None, help="Custom model ID")

    # Command: audit
    audit_p = subparsers.add_parser("audit", help="Audit code against specifications using DeepSeek Reasoner")
    audit_p.add_argument("file", type=str, help="Path to code file or inline code string")
    audit_p.add_argument("--spec", "-s", type=str, required=True, help="Specification string or path to requirements file")
    audit_p.add_argument("--lang", "-l", type=str, default=None, help="Programming language")
    audit_p.add_argument("--json", "-j", action="store_true", help="Output raw JSON analysis")

    # Command: refine
    refine_p = subparsers.add_parser("refine", help="Refine code using DeepSeek to ensure zero edge case flaws")
    refine_p.add_argument("file", type=str, help="Path to code file or inline code string")
    refine_p.add_argument("--spec", "-s", type=str, required=True, help="Specification string or path to requirements file")
    refine_p.add_argument("--in-place", "-i", action="store_true", help="Overwrite file in-place with refined code")
    refine_p.add_argument("--out", "-o", type=str, default=None, help="Output destination file")
    refine_p.add_argument("--lang", "-l", type=str, default=None, help="Programming language")

    # Command: arbitrate
    arbitrate_p = subparsers.add_parser("arbitrate", help="Arbitrate between 2 or more candidate files")
    arbitrate_p.add_argument("files", nargs="+", help="Paths to candidate code files")
    arbitrate_p.add_argument("--spec", "-s", type=str, required=True, help="Specification string or path to requirements file")
    arbitrate_p.add_argument("--lang", "-l", type=str, default=None, help="Programming language")

    # Command: run (Pipeline: Generator -> Curator -> Final Code)
    run_p = subparsers.add_parser("run", help="Run full dual-model pipeline (Generate -> Curate -> Output)")
    run_p.add_argument("prompt", type=str, help="Feature request or algorithm specification")
    run_p.add_argument("--draft", "-d", type=str, default=None, help="Optional draft code file to curate instead of generating")
    run_p.add_argument("--lang", "-l", type=str, default=None, help="Programming language")
    run_p.add_argument("--out", "-o", type=str, default=None, help="Output destination file")

    # Command: bench
    bench_p = subparsers.add_parser("bench", help="Run automated Pass@1 benchmark (Official OpenAI HumanEval & Edge-Case Suite)")
    bench_p.add_argument("--official", "-o", type=str, default="humaneval", choices=["humaneval", "custom"], help="Benchmark dataset (default: humaneval)")
    bench_p.add_argument("--generator", "-g", type=str, default="gpt-4o-mini", help="Fast draft generator model (e.g. gpt-4o-mini, gpt-3.5-turbo, deepseek-v4-flash)")
    bench_p.add_argument("--curator", "-c", type=str, default="deepseek-v4-flash", help="Curator & Verifier model (e.g. deepseek-v4-flash, deepseek-v4-pro)")
    bench_p.add_argument("--limit", "-n", type=int, default=5, help="Number of benchmark tasks to evaluate (default: 5)")
    bench_p.add_argument("--start", "-s", type=int, default=0, help="Starting index in dataset (default: 0)")
    bench_p.add_argument("--all", "-a", action="store_true", help="Run all 164 official HumanEval problems")
    bench_p.add_argument("--json", "-j", action="store_true", help="Output raw JSON benchmark report")

    # Command: interactive / repl
    inter_p = subparsers.add_parser("interactive", help="Start interactive terminal coding session")
    inter_p.add_argument("--generator", "-g", type=str, default="gpt-4o-mini", help="Active draft generator model")
    inter_p.add_argument("--curator", "-c", type=str, default="deepseek-v4-flash", help="Active curator & verifier model")
    inter_p.add_argument("--theme", "-t", type=str, default="bloomberg", choices=["bloomberg", "cyan", "matrix"], help="Color theme (default: bloomberg)")

    # Command: mcp
    subparsers.add_parser("mcp", help="Run DSpark as a Model Context Protocol (MCP) server")

    # Command: local (Manage & detect local offline models)
    local_p = subparsers.add_parser("local", help="Scan, list and test local offline LLMs (Ollama, LM Studio, vLLM)")
    local_p.add_argument("--url", "-u", type=str, default=None, help="Custom local endpoint URL (default: auto-detect)")
    local_p.add_argument("--test", "-t", type=str, default=None, help="Test generate with a specific local model")

    # If user provided a single string argument that is not a known command, treat as one-shot task
    if len(sys.argv) == 2 and not sys.argv[1].startswith("-") and sys.argv[1] not in subparsers.choices:
        agent = DSparkAgent()
        print(f"\033[90mExecuting DSpark One-Shot Task: '{sys.argv[1]}'\033[0m\n")
        print(agent.execute_task(sys.argv[1]))
        return

    # If no arguments provided, launch interactive session by default
    if len(sys.argv) == 1:
        start_interactive_session()
        return

    args = parser.parse_args()

    try:
        if args.command == "interactive":
            start_interactive_session(
                generator_model=getattr(args, "generator", "gpt-4o-mini"),
                curator_model=getattr(args, "curator", "deepseek-v4-flash"),
                theme=getattr(args, "theme", "bloomberg"),
            )

        elif args.command == "search":
            engine = WebSearchEngine()
            results = engine.search(args.query, max_results=args.sources)
            print(f"\n\033[1;34m=== Web Search Results for: '{args.query}' ===\033[0m\n")
            for idx, res in enumerate(results, 1):
                print(f"\033[92m{idx}. {res.title}\033[0m")
                print(f"   \033[90mURL:\033[0m {res.url}")
                print(f"   {res.snippet}\n")

        elif args.command == "fetch":
            engine = WebSearchEngine()
            content = engine.fetch_url(args.url)
            print(content)

        elif args.command == "test-connection":
            client = DeepSeekClient(default_model=args.model)
            print(f"Connecting to DeepSeek API at: {client.base_url} (model: {client.default_model})...")
            res = client.complete("Ping. Respond with 'DSpark Online'.", temperature=0.0)
            print(f"Success! Response: {res.strip()}")

        elif args.command == "audit":
            curator = DeepSeekCurator()
            code = _read_file_or_string(args.file)
            spec = _read_file_or_string(args.spec)
            result = curator.audit(code=code, specification=spec, language=args.lang)

            if args.json:
                print(result.raw_response)
            else:
                verdict_color = "\033[92m" if result.is_approved else "\033[93m" if result.verdict.value == "NEEDS_REVISION" else "\033[91m"
                reset_color = "\033[0m"
                print(f"\n{verdict_color}=== DSPARK AUDIT VERDICT: {result.verdict.value} (Score: {result.score}/100) ==={reset_color}\n")
                print(f"Summary: {result.summary}\n")
                
                if result.critical_issues:
                    print("Critical Issues:")
                    for issue in result.critical_issues:
                        print(f"  [!] {issue}")
                    print()

                if result.edge_cases:
                    print("Edge Case Analysis:")
                    for ec in result.edge_cases:
                        status = "✓ Handled" if ec.handled_properly else f"✗ Flaw ({ec.risk_level} Risk): {ec.remedy}"
                        print(f"  - {ec.case}: {status}")
                    print()

                if result.complexity:
                    print(f"Complexity: Time {result.complexity.get('time', 'N/A')}, Space {result.complexity.get('space', 'N/A')}\n")

                if result.suggested_improvements:
                    print("Suggested Improvements:")
                    for imp in result.suggested_improvements:
                        print(f"  * {imp}")
                    print()

        elif args.command == "refine":
            curator = DeepSeekCurator()
            code = _read_file_or_string(args.file)
            spec = _read_file_or_string(args.spec)
            result = curator.refine(code=code, specification=spec, language=args.lang)

            if args.in_place and os.path.exists(args.file):
                with open(args.file, "w", encoding="utf-8") as f:
                    f.write(result.refined_code)
                print(f"Refined code written in-place to {args.file}")
            elif args.out:
                with open(args.out, "w", encoding="utf-8") as f:
                    f.write(result.refined_code)
                print(f"Refined code written to {args.out}")
            else:
                print(result.refined_code)

        elif args.command == "arbitrate":
            curator = DeepSeekCurator()
            candidates = [_read_file_or_string(f) for f in args.files]
            spec = _read_file_or_string(args.spec)
            result = curator.arbitrate(candidates=candidates, specification=spec, language=args.lang)

            print(f"\n=== DSPARK ARBITRATION RESULT ===")
            print(f"Winner: Candidate #{result.winner_index}")
            print(f"Rationale: {result.rationale}\n")
            print("Optimal Synthesized Code:")
            print(result.synthesized_code)

        elif args.command == "run":
            pipeline = DSparkPipeline()
            draft = _read_file_or_string(args.draft) if args.draft else None
            res = pipeline.run(specification=args.prompt, draft_code=draft, language=args.lang)

            print(f"\n=== DSPARK PIPELINE COMPLETED ===")
            print(f"Audit Verdict: {res.audit_result.verdict.value} (Score: {res.audit_result.score}/100)")
            print(f"Refined by Curator: {res.refined}\n")
            
            if args.out:
                with open(args.out, "w", encoding="utf-8") as f:
                    f.write(res.final_code)
                print(f"Final verified code written to {args.out}")
            else:
                print("Final Verified Code:")
                print(res.final_code)

        elif args.command == "bench":
            runner = DSparkBenchmarkRunner(
                generator_model=args.generator,
                curator_model=args.curator,
            )
            dataset_title = "Official OpenAI HumanEval (164 tasks)" if args.official == "humaneval" else "Custom Curated Suite"
            print(f"\n\033[1;36m=== ⚡ DSPARK AI BENCHMARK SUITE ===\033[0m")
            print(f"  \033[90mDataset   :\033[0m \033[1m{dataset_title}\033[0m")
            print(f"  \033[90mGenerator :\033[0m \033[93m{args.generator}\033[0m (Mass Code Generation)")
            print(f"  \033[90mCurator   :\033[0m \033[92m{args.curator}\033[0m (LLM-as-a-Verifier Audit & Refinement)")
            print("\033[90mRunning Pass@1 evaluation: Baseline vs DSpark Dual-Engine...\033[0m\n")

            limit = None if args.all else args.limit
            if args.official == "humaneval":
                report = runner.run_official_humaneval_benchmark(
                    limit=limit,
                    start_idx=args.start,
                    progress_callback=lambda msg: print(f"  \033[90m➜\033[0m {msg}"),
                )
            else:
                report = runner.run_benchmark(progress_callback=lambda msg: print(f"  \033[90m➜\033[0m {msg}"))

            if args.json:
                import json
                print(json.dumps(report.__dict__, default=lambda o: o.__dict__, indent=2))
            else:
                print(f"\n\033[1;34m=== 📊 BENCHMARK RESULTS ({report.dataset_name}) ===\033[0m\n")
                print(f"  Total Problems Evaluated : \033[1m{report.total_problems}\033[0m")
                print(f"  Baseline Pass@1 Rate     : \033[91m{report.baseline_pass_rate:.1f}%\033[0m ({report.baseline_passed_count}/{report.total_problems})")
                print(f"  DSpark Dual-Engine Rate  : \033[92m{report.dspark_pass_rate:.1f}%\033[0m ({report.dspark_passed_count}/{report.total_problems})")
                
                delta_color = "\033[92m" if report.accuracy_delta >= 0 else "\033[91m"
                print(f"  Empirical Accuracy Gain  : {delta_color}+{report.accuracy_delta:.1f}%\033[0m\n")

                print("  Detailed Task Breakdown:")
                for r in report.results:
                    base_status = "\033[92mPASS\033[0m" if r.baseline_passed else "\033[91mFAIL\033[0m"
                    dspark_status = "\033[92mPASS\033[0m" if r.dspark_passed else "\033[91mFAIL\033[0m"
                    print(f"    * [{r.problem_id}] {r.title}")
                    print(f"      - Baseline: {base_status} ({r.baseline_time_ms:.0f}ms) | DSpark Dual: {dspark_status} (Score: {r.curator_score}/100, Contraexamples: {r.contra_examples_detected})")
                print()

        elif args.command == "local":
            print("\n\033[1;36m=== 💻 DSPARK LOCAL & OFFLINE LLM SCANNER ===\033[0m\n")
            active = LocalLLMClient.detect_active_endpoints()

            if not active:
                print("  \033[93m[!] No active local LLM servers detected on localhost.\033[0m\n")
                print("  To run models locally (100% free and private):")
                print("    1. Install Ollama: \033[1mhttps://ollama.com\033[0m")
                print("    2. Start a model in terminal: \033[1mollama run qwen2.5-coder:1.5b\033[0m (or deepseek-r1:1.5b)")
                print("    3. Or start LM Studio with local server enabled on port 1234\n")
            else:
                print(f"  \033[92m[✓] Found {len(active)} active local LLM endpoint(s):\033[0m")
                for s in active:
                    print(f"    * \033[1m{s['name']}\033[0m: {s['v1_url']}")
                    client = LocalLLMClient(base_url=s['v1_url'])
                    models = client.list_models()
                    if models:
                        print("      Available local models:")
                        for m in models:
                            print(f"        - \033[96m{m}\033[0m")
                    else:
                        print("      (Server running, no models pulled yet)")
                print()

            if args.test:
                print(f"  Testing generation with local model '\033[1m{args.test}\033[0m'...")
                client = LocalLLMClient(base_url=args.url)
                res = client.complete("Write a python one-liner to reverse a list.", model=args.test)
                print(f"\n  \033[92m[✓] Model Response:\033[0m\n{res}\n")

        elif args.command == "mcp":
            run_mcp_server()

    except Exception as e:
        sys.stderr.write(f"Error: {e}\n")
        sys.exit(1)


if __name__ == "__main__":
    main()
