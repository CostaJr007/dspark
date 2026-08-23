//! Interactive terminal REPL (bloomberg / cyan / matrix palettes).

use crate::agent::DSparkAgent;
use crate::agent_loop::SparkAgent;
use crate::client::LocalLLMClient;
use crate::pair::{DEFAULT_CREATOR, DEFAULT_CURATOR};
use crate::util::read_file_or_string;
use colored::*;
use std::io::{self, Write};
use std::path::Path;

#[derive(Clone, Copy)]
pub enum ThemeMode {
    Bloomberg,
    Cyan,
    Matrix,
}

impl ThemeMode {
    fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_lowercase().as_str() {
            "bloomberg" => Some(Self::Bloomberg),
            "cyan" | "spark" => Some(Self::Cyan),
            "matrix" => Some(Self::Matrix),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Bloomberg => "Bloomberg",
            Self::Cyan => "Cyan",
            Self::Matrix => "Matrix",
        }
    }
}

pub struct ReplState {
    pub theme: ThemeMode,
    pub generator_model: String,
    pub curator_model: String,
}

impl ReplState {
    pub fn new(generator: &str, curator: &str, theme: &str) -> Self {
        Self {
            theme: ThemeMode::from_name(theme).unwrap_or(ThemeMode::Bloomberg),
            generator_model: generator.to_string(),
            curator_model: curator.to_string(),
        }
    }
}

impl Default for ReplState {
    fn default() -> Self {
        Self::new(DEFAULT_CREATOR, DEFAULT_CURATOR, "bloomberg")
    }
}

fn paint(theme: ThemeMode, s: &str) -> ColoredString {
    match theme {
        ThemeMode::Bloomberg => s.yellow().bold(),
        ThemeMode::Cyan => s.cyan().bold(),
        ThemeMode::Matrix => s.green().bold(),
    }
}

pub fn render_rust_banner(workspace: &Path, state: &ReplState) {
    let ws_str = workspace.to_string_lossy();
    let theme_name = state.theme.name();
    let border = paint(state.theme, "╭────────────────────────────────────────────────────────────────────────────────────────────╮");
    let bot = paint(state.theme, "╰────────────────────────────────────────────────────────────────────────────────────────────╯");

    println!();
    println!("{}", border);
    println!(
        "{}",
        paint(
            state.theme,
            "│   ⚡ DSPARK v0.1.0 (Rust) │ Speculative Dual-Engine & Autonomous CLI                       │"
        )
    );
    println!(
        "│   {}  {}  {}  {}                       │",
        "● DeepSeek-V4".green().bold(),
        "● OpenAI".cyan().bold(),
        "● Web Search".magenta().bold(),
        "● Native Rust".yellow().bold()
    );
    println!(
        "│   Generator: {} │ Curator: {} │ Theme: {} │ Workspace: {} │",
        state.generator_model.yellow().bold(),
        state.curator_model.green().bold(),
        theme_name.yellow(),
        ws_str.white()
    );
    println!("{}", bot);
    println!(
        "{}",
        "Type instruction in natural language, /models to switch models, /theme to change palette, /help for commands.\n"
            .dimmed()
    );
}

fn print_help(theme: ThemeMode) {
    println!(
        "\n{}",
        paint(theme, "=== ⚡ DSpark Interactive Commands (Rust Edition) ===")
    );
    println!("  {}     - Interactively switch active AI models", "/models".green().bold());
    println!(
        "  {}     - Switch color theme (bloomberg, cyan, matrix)",
        "/theme [name]".green().bold()
    );
    println!(
        "  {}     - Live web search for documentation",
        "/search <query>".green().bold()
    );
    println!(
        "  {}       - Fetch clean Markdown documentation from URL",
        "/fetch <url>".green().bold()
    );
    println!(
        "  {}  - Formal I/O audit against a specification",
        "/audit <file> -s <spec>".green().bold()
    );
    println!(
        "  {} - Surgical in-place refinement",
        "/refine <file> -s <spec>".green().bold()
    );
    println!(
        "  {}             - Scan local LLMs (Ollama / LM Studio)",
        "/local".green().bold()
    );
    println!("  {}      - List workspace files", "/files [path]".green().bold());
    println!("  {}       - Preview workspace source file", "/read <file>".green().bold());
    println!(
        "  {}      - Execute native shell command (e.g. cargo test, git status)",
        "/sh <command>".green().bold()
    );
    println!("  {}             - Clear terminal screen", "/clear".green().bold());
    println!("  {}              - Exit session\n", "/exit".green().bold());
}

fn parse_spec_flag(rest: &str) -> (String, String) {
    if let Some((left, right)) = rest.split_once(" -s ") {
        return (left.trim().to_string(), right.trim().to_string());
    }
    if let Some((left, right)) = rest.split_once(" --spec ") {
        return (left.trim().to_string(), right.trim().to_string());
    }
    let mut parts = rest.splitn(2, ' ');
    (
        parts.next().unwrap_or("").to_string(),
        parts.next().unwrap_or("").to_string(),
    )
}

pub async fn start_repl(
    generator: &str,
    curator: &str,
    theme: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut agent = DSparkAgent::with_models(None, generator, curator)?;
    let mut state = ReplState::new(generator, curator, theme);

    render_rust_banner(&agent.working_dir, &state);

    let stdin = io::stdin();
    let mut input_buffer = String::new();

    loop {
        print!("{} ", paint(state.theme, "DSpark ❯"));
        io::stdout().flush()?;
        input_buffer.clear();

        if stdin.read_line(&mut input_buffer)? == 0 {
            break;
        }

        let line = input_buffer.trim();
        if line.is_empty() {
            continue;
        }

        match line {
            "/exit" | "/quit" | "exit" | "quit" => {
                println!("{}", "Exiting DSpark session. Happy coding!".yellow());
                break;
            }
            "/clear" | "clear" => {
                print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
                render_rust_banner(&agent.working_dir, &state);
                continue;
            }
            "/help" | "help" => {
                print_help(state.theme);
                continue;
            }
            "/local" | "local" => {
                let active = LocalLLMClient::detect_active_endpoints().await;
                if active.is_empty() {
                    println!(
                        "\n{}\n",
                        "[!] No active local LLM detected. Start Ollama (ollama run qwen2.5-coder:1.5b) or LM Studio."
                            .yellow()
                    );
                } else {
                    println!(
                        "\n{}",
                        format!("[✓] Found {} active local server(s):", active.len())
                            .green()
                            .bold()
                    );
                    for s in active {
                        println!("  * {} ({})", s.name.bold(), s.v1_url);
                        if let Ok(client) = LocalLLMClient::new(Some(&s.v1_url), None) {
                            for m in client.list_models().await {
                                println!("    - {}", m.cyan());
                            }
                        }
                    }
                    println!();
                }
                continue;
            }
            "/models" | "models" | "/model" | "/select" => {
                println!(
                    "\n{}",
                    paint(
                        state.theme,
                        "╭── 🤖 Select Active AI Models for DSpark Engine ──────────────────────────────────────────╮"
                    )
                );
                println!("│  [1] Ultra-Fast & Cost-Efficient:  gpt-4o-mini + deepseek-v4-flash                        │");
                println!("│  [2] Maximum Reasoning Accuracy:    gemini-3.7-flash + deepseek-v4-pro                    │");
                println!("│  [3] Pure DeepSeek Ecosystem:       deepseek-v4-flash + deepseek-v4-pro                   │");
                println!("│  [c] Custom                                                                                 │");
                println!("│  [q] Cancel / Keep Current                                                                │");
                println!(
                    "{}",
                    paint(
                        state.theme,
                        "╰──────────────────────────────────────────────────────────────────────────────────────────╯"
                    )
                );
                print!("Select option [1-3, c, q]: ");
                io::stdout().flush()?;
                let mut choice = String::new();
                stdin.read_line(&mut choice)?;
                match choice.trim() {
                    "1" => {
                        state.generator_model = "gpt-4o-mini".to_string();
                        state.curator_model = "deepseek-v4-flash".to_string();
                    }
                    "2" => {
                        state.generator_model = "gemini-3.7-flash".to_string();
                        state.curator_model = "deepseek-v4-pro".to_string();
                    }
                    "3" => {
                        state.generator_model = "deepseek-v4-flash".to_string();
                        state.curator_model = "deepseek-v4-pro".to_string();
                    }
                    "c" => {
                        print!("  Enter Generator model: ");
                        io::stdout().flush()?;
                        let mut g = String::new();
                        stdin.read_line(&mut g)?;
                        print!("  Enter Curator model: ");
                        io::stdout().flush()?;
                        let mut c = String::new();
                        stdin.read_line(&mut c)?;
                        if !g.trim().is_empty() {
                            state.generator_model = g.trim().to_string();
                        }
                        if !c.trim().is_empty() {
                            state.curator_model = c.trim().to_string();
                        }
                    }
                    _ => {
                        println!("{}", "Kept active models.".dimmed());
                        continue;
                    }
                }
                match agent.set_models(&state.generator_model, &state.curator_model) {
                    Ok(()) => println!(
                        "{}",
                        format!(
                            "✓ Models updated: {} + {}",
                            state.generator_model, state.curator_model
                        )
                        .green()
                        .bold()
                    ),
                    Err(e) => println!("{}: {}", "Model Error".red(), e),
                }
                render_rust_banner(&agent.working_dir, &state);
                continue;
            }
            _ => {}
        }

        if let Some(theme_name) = line.strip_prefix("/theme") {
            let theme_name = theme_name.trim();
            if theme_name.is_empty() {
                println!("Available themes: bloomberg, cyan, matrix");
                continue;
            }
            match ThemeMode::from_name(theme_name) {
                Some(t) => state.theme = t,
                None => println!("{}", "Unknown theme. Options: bloomberg, cyan, matrix".red()),
            }
            render_rust_banner(&agent.working_dir, &state);
            continue;
        }

        if let Some(query) = line.strip_prefix("/search ") {
            println!("{}", format!("Searching web for: {}...", query).dimmed());
            match agent.search_engine.search(query, 5).await {
                Ok(results) => {
                    println!(
                        "\n{}",
                        paint(
                            state.theme,
                            &format!("=== 🔍 Web Search Results for: '{}' ===", query)
                        )
                    );
                    for (i, res) in results.iter().enumerate() {
                        println!("{}. {}", i + 1, res.title.green().bold());
                        println!("   {}: {}", "URL".dimmed(), res.url);
                        println!("   {}\n", res.snippet);
                    }
                }
                Err(e) => println!("{}: {}", "Search Error".red(), e),
            }
            continue;
        }

        if let Some(url) = line.strip_prefix("/fetch ") {
            println!("{}", format!("Fetching: {}...", url).dimmed());
            match agent.search_engine.fetch_url(url, 4000).await {
                Ok(content) => println!("\n{}\n", content),
                Err(e) => println!("{}: {}", "Fetch Error".red(), e),
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("/files") {
            let subpath = rest.trim();
            let subpath = if subpath.is_empty() { "." } else { subpath };
            let files = agent.list_files(subpath);
            println!(
                "\n{} ({} items):",
                format!("Files in {}", subpath).cyan(),
                files.len()
            );
            for f in files {
                println!("  {} {}", "•".dimmed(), f);
            }
            println!();
            continue;
        }

        if let Some(cmd) = line.strip_prefix("/sh ") {
            println!("{}", format!("Running: {}", cmd).dimmed());
            println!("{}", agent.run_terminal(cmd));
            continue;
        }

        if let Some(fpath) = line.strip_prefix("/read ") {
            match agent.read_file(fpath) {
                Ok(content) => println!("\n--- {} ---\n{}\n", fpath, content),
                Err(e) => println!("{}: {}", "Read Error".red(), e),
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("/audit ") {
            let (file, spec) = parse_spec_flag(rest);
            match (read_file_or_string(&file), agent.curator()) {
                (Ok(code), Ok(curator)) => {
                    println!("{}", format!("Auditing '{}'...", file).dimmed());
                    match curator.audit(&code, &spec, None).await {
                        Ok(result) => {
                            println!(
                                "\n=== DSPARK AUDIT VERDICT: {} (Score: {}/100) ===",
                                result.verdict, result.score
                            );
                            println!("Summary: {}\n", result.summary);
                            for issue in &result.critical_issues {
                                println!("  [!] {}", issue.red());
                            }
                        }
                        Err(e) => println!("{}: {}", "Audit Error".red(), e),
                    }
                }
                (Err(e), _) => println!("{}: {}", "Read Error".red(), e),
                (_, Err(e)) => println!("{}: {}", "Curator Error".red(), e),
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("/refine ") {
            let (file, spec) = parse_spec_flag(rest);
            match (read_file_or_string(&file), agent.curator()) {
                (Ok(code), Ok(curator)) => {
                    println!("{}", format!("Refining '{}'...", file).dimmed());
                    match curator.refine(&code, &spec, None, None).await {
                        Ok(result) => {
                            if Path::new(&file).is_file() {
                                let _ = std::fs::write(&file, &result.refined_code);
                                println!("{}", format!("Refined code written in-place to {}", file).green());
                            } else {
                                println!("{}", result.refined_code);
                            }
                        }
                        Err(e) => println!("{}: {}", "Refine Error".red(), e),
                    }
                }
                (Err(e), _) => println!("{}: {}", "Read Error".red(), e),
                (_, Err(e)) => println!("{}: {}", "Curator Error".red(), e),
            }
            continue;
        }

        println!(
            "{}",
            paint(state.theme, "\n⚡ Executing Metacognitive Reasoning Engine...")
        );
        match SparkAgent::new(
            agent.working_dir.clone(),
            &state.generator_model,
            &state.curator_model,
        ) {
            Ok(spark) => {
                let on_call = |tool: &str, args: &serde_json::Value| {
                    println!("  {} {}({})", "➜ Tool Call:".cyan(), tool.bold(), args);
                };
                let on_res = |res: &crate::tools::ToolResult| {
                    if res.success {
                        let snippet: String = res.output.chars().take(160).collect();
                        println!("    {} {}", "✓ SUCCESS".green(), snippet.dimmed());
                    } else {
                        println!(
                            "    {} {}",
                            "✖ FAILED".red(),
                            res.error.clone().unwrap_or_default()
                        );
                    }
                };
                match spark
                    .execute_step(line, Some(&on_call), Some(&on_res), 6)
                    .await
                {
                    Ok(solution) => println!("\n{}", solution),
                    Err(e) => match agent.execute_task(line).await {
                        Ok(fallback) => println!("\n{}", fallback),
                        Err(_) => println!("{}: {}", "Execution Error".red(), e),
                    },
                }
            }
            Err(_) => match agent.execute_task(line).await {
                Ok(solution) => println!("\n{}", solution),
                Err(e) => println!("{}: {}", "Execution Error".red(), e),
            },
        }
        println!();
    }

    Ok(())
}
