//! Interactive Terminal REPL (Grok Build & Bloomberg Terminal style TUI) in native Rust.

use crate::agent::DSparkAgent;
use colored::*;
use std::io::{self, Write};

pub enum ThemeMode {
    Bloomberg,
    Grok,
    Matrix,
}

pub struct ReplState {
    pub theme: ThemeMode,
    pub generator_model: String,
    pub curator_model: String,
}

impl Default for ReplState {
    fn default() -> Self {
        Self {
            theme: ThemeMode::Bloomberg,
            generator_model: "gpt-4o-mini".to_string(),
            curator_model: "deepseek-v4-flash".to_string(),
        }
    }
}

pub fn render_rust_banner(workspace: &std::path::Path, state: &ReplState) {
    let ws_str = workspace.to_string_lossy();
    let theme_name = match state.theme {
        ThemeMode::Bloomberg => "Bloomberg",
        ThemeMode::Grok => "Grok",
        ThemeMode::Matrix => "Matrix",
    };

    println!();
    println!("{}", "╭────────────────────────────────────────────────────────────────────────────────────────────╮".yellow().bold());
    println!("{}", "│   ⚡ DSPARK v0.1.0 (Rust Edition) │ Speculative Dual-Engine & Autonomous CLI                │".yellow().bold());
    println!(
        "│   {}  {}  {}  {}                       │",
        "● DeepSeek-V4".green().bold(),
        "● OpenAI".cyan().bold(),
        "● Kimi Search".magenta().bold(),
        "● Native Rust".yellow().bold()
    );
    println!(
        "│   Generator: {} │ Curator: {} │ Theme: {} │ Workspace: {} │",
        state.generator_model.yellow().bold(),
        state.curator_model.green().bold(),
        theme_name.yellow(),
        ws_str.white()
    );
    println!("{}", "╰────────────────────────────────────────────────────────────────────────────────────────────╯".yellow().bold());
    println!("{}", "Type instruction in natural language, /models to switch models, /theme to change palette, /help for commands.\n".dimmed());
}

pub async fn start_repl() -> Result<(), Box<dyn std::error::Error>> {
    let mut agent = DSparkAgent::new(None)?;
    let mut state = ReplState::default();

    render_rust_banner(&agent.working_dir, &state);

    let stdin = io::stdin();
    let mut input_buffer = String::new();

    loop {
        print!("{} ", "DSpark ❯".yellow().bold());
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
                println!("\n{}", "=== ⚡ DSpark Interactive Commands (Rust Edition) ===".yellow().bold());
                println!("  {}     - Interactively switch active AI models", "/models".green().bold());
                println!("  {}     - Switch color theme (bloomberg, grok, matrix)", "/theme [name]".green().bold());
                println!("  {}     - Deep web search for documentation (Kimi style)", "/search <query>".green().bold());
                println!("  {}       - Fetch clean Markdown documentation from URL", "/fetch <url>".green().bold());
                println!("  {}       - Preview workspace source file", "/read <file>".green().bold());
                println!("  {}      - Execute native shell command (e.g. cargo test, git status)", "/sh <command>".green().bold());
                println!("  {}             - Clear terminal screen", "/clear".green().bold());
                println!("  {}              - Exit session\n", "/exit".green().bold());
                continue;
            }
            "/models" | "models" => {
                println!("\n{}", "╭── 🤖 Select Active AI Models for DSpark Engine ──────────────────────────────────────────╮".yellow().bold());
                println!("│  [1] Ultra-Fast & Cost-Efficient:  gpt-4o-mini + deepseek-v4-flash                        │");
                println!("│  [2] Maximum Reasoning Accuracy:    gemini-3.7-flash + deepseek-v4-pro                    │");
                println!("│  [3] Pure DeepSeek Ecosystem:       deepseek-v4-flash + deepseek-v4-pro                   │");
                println!("│  [q] Cancel / Keep Current                                                                │");
                println!("{}", "╰──────────────────────────────────────────────────────────────────────────────────────────╯".yellow().bold());
                print!("Select option [1-3, q]: ");
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
                    _ => {}
                }
                println!("{}", format!("✓ Models updated: {} + {}", state.generator_model, state.curator_model).green().bold());
                render_rust_banner(&agent.working_dir, &state);
                continue;
            }
            _ => {}
        }

        if let Some(theme_name) = line.strip_prefix("/theme ") {
            match theme_name.trim().to_lowercase().as_str() {
                "bloomberg" => state.theme = ThemeMode::Bloomberg,
                "grok" => state.theme = ThemeMode::Grok,
                "matrix" => state.theme = ThemeMode::Matrix,
                _ => println!("{}", "Unknown theme. Options: bloomberg, grok, matrix".red()),
            }
            render_rust_banner(&agent.working_dir, &state);
            continue;
        }

        if let Some(query) = line.strip_prefix("/search ") {
            println!("{}", format!("Searching web for: {}...", query).dimmed());
            match agent.search_engine.search(query, 5).await {
                Ok(results) => {
                    println!("\n{}", format!("=== 🔍 Web Search Results for: '{}' ===", query).yellow().bold());
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

        if let Some(cmd) = line.strip_prefix("/sh ") {
            println!("{}", format!("Running: {}", cmd).dimmed());
            let out = agent.run_terminal(cmd);
            println!("{}", out);
            continue;
        }

        if let Some(fpath) = line.strip_prefix("/read ") {
            match agent.read_file(fpath) {
                Ok(content) => println!("\n--- {} ---\n{}\n", fpath, content),
                Err(e) => println!("{}: {}", "Read Error".red(), e),
            }
            continue;
        }

        // Natural Language Instruction Execution
        println!("{}", "\n⚡ Executing Metacognitive Reasoning Engine...".yellow().bold());
        match agent.execute_task(line).await {
            Ok(solution) => {
                println!("\n{}", solution);
            }
            Err(e) => {
                println!("{}: {}", "Execution Error".red(), e);
            }
        }
        println!();
    }

    Ok(())
}
