//! Interactive Terminal REPL (Grok Build style TUI) in Rust.

use crate::agent::DSparkAgent;
use colored::*;
use std::io::{self, Write};

const BANNER: &str = r#"
  ____  ____                    _    
 |  _ \/ ___| _ __   __ _ _ __ | | __
 | | | \___ \| '_ \ / _` | '__|| |/ /
 | |_| |___) | |_) | (_| | |   |   < 
 |____/|____/| .__/ \__,_|_|   |_|\_\
             |_|                      
  ⚡ Dual-LLM Speculative Engine & Autonomous CLI [Rust Edition]
  [Grok-Build Runtime + Kimi WebSearch + DeepSeek Verifier]
"#;

pub async fn start_repl() -> Result<(), Box<dyn std::error::Error>> {
    let agent = DSparkAgent::new(None)?;

    println!("{}", BANNER.cyan().bold());
    println!("{}: {:?}", "Workspace".dimmed(), agent.working_dir);
    println!("{}", "Type your instruction, /help for slash commands, /exit to quit.\n".dimmed());

    let stdin = io::stdin();
    let mut input_buffer = String::new();

    loop {
        print!("{} ", "DSpark>".green().bold());
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
                continue;
            }
            "/help" | "help" => {
                println!("\n{}", "=== DSpark Interactive Commands (Rust) ===".blue().bold());
                println!("  {}       - Deep web search for documentation (Kimi style)", "/search <query>".green());
                println!("  {}          - Fetch clean Markdown from URL", "/fetch <url>".green());
                println!("  {}          - Read local file", "/read <file>".green());
                println!("  {}         - Run local shell command (e.g. cargo test, git diff)", "/sh <command>".green());
                println!("  {}                - Clear terminal screen", "/clear".green());
                println!("  {}                 - Exit session\n", "/exit".green());
                continue;
            }
            _ => {}
        }

        if let Some(query) = line.strip_prefix("/search ") {
            println!("{}", format!("Searching web for: {}...", query).dimmed());
            match agent.search_engine.search(query, 5).await {
                Ok(results) => {
                    println!("\n{}", format!("=== Web Search Results for: '{}' ===", query).blue().bold());
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

        // Standard Natural Language Task -> Execute Metacognitive Protocol
        println!("{}", "Executing Metacognitive Reasoning Engine...".dimmed());
        match agent.execute_task(line).await {
            Ok(output) => println!("\n{}\n", output),
            Err(e) => println!("{}: {}", "Error".red(), e),
        }
    }

    Ok(())
}
