//! Main entrypoint for DSpark Rust CLI.

use clap::{Parser, Subcommand};
use colored::*;
use dspark_cli::curator::DeepSeekCurator;
use dspark_cli::repl::start_repl;
use dspark_cli::search::WebSearchEngine;
use std::fs;

#[derive(Parser)]
#[command(name = "dspark")]
#[command(about = "⚡ DSpark: Dual-LLM Speculative Engine & Autonomous Agent CLI (Rust Edition)")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Optional one-shot instruction prompt
    #[arg(trailing_var_arg = true)]
    task: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive terminal REPL (Grok Build style)
    Interactive,
    /// Perform deep web search for docs or error fixes (Kimi style)
    Search {
        /// Query string
        query: String,
        #[arg(short = 'n', default_value_t = 5)]
        limit: usize,
    },
    /// Fetch and convert a webpage to clean Markdown
    Fetch {
        /// Target URL
        url: String,
    },
    /// Audit a code file against specifications using DeepSeek Reasoner
    Audit {
        /// Code file path
        file: String,
        #[arg(short, long)]
        spec: String,
    },
    /// Test connection to DeepSeek API
    TestConnection,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // If one-shot task provided without subcommand: dspark "task..."
    if !cli.task.is_empty() && cli.command.is_none() {
        let task_prompt = cli.task.join(" ");
        println!("{}: {}", "Executing DSpark One-Shot Task".dimmed(), task_prompt.bold());
        let agent = dspark_cli::DSparkAgent::new(None)?;
        let result = agent.execute_task(&task_prompt).await?;
        println!("\n{}\n", result);
        return Ok(());
    }

    match cli.command {
        None | Some(Commands::Interactive) => {
            start_repl().await?;
        }
        Some(Commands::Search { query, limit }) => {
            let engine = WebSearchEngine::new();
            println!("{}", format!("Searching web for: '{}'...", query).dimmed());
            let results = engine.search(&query, limit).await?;
            println!("\n{}", format!("=== Web Search Results for: '{}' ===", query).blue().bold());
            for (i, res) in results.iter().enumerate() {
                println!("{}. {}", i + 1, res.title.green().bold());
                println!("   {}: {}", "URL".dimmed(), res.url);
                println!("   {}\n", res.snippet);
            }
        }
        Some(Commands::Fetch { url }) => {
            let engine = WebSearchEngine::new();
            println!("{}", format!("Fetching: {}...", url).dimmed());
            let content = engine.fetch_url(&url, 5000).await?;
            println!("\n{}\n", content);
        }
        Some(Commands::Audit { file, spec }) => {
            let code = fs::read_to_string(&file)?;
            let curator = DeepSeekCurator::new()?;
            println!("{}", format!("Auditing '{}' with DeepSeek Reasoner...", file).dimmed());
            let result = curator.audit(&code, &spec).await?;
            println!("\n=== DSPARK AUDIT VERDICT: {:?} (Score: {}/100) ===", result.verdict, result.score);
            println!("Summary: {}\n", result.summary);
            if !result.critical_issues.is_empty() {
                println!("Critical Issues:");
                for issue in &result.critical_issues {
                    println!("  [!] {}", issue.red());
                }
            }
        }
        Some(Commands::TestConnection) => {
            let client = dspark_cli::DeepSeekClient::new()?;
            println!("Connecting to DeepSeek API at {} (model: {})...", client.model, client.model);
            let res = client.complete("Ping. Respond with 'DSpark Online (Rust)'.", None, 0.0, false).await?;
            println!("{}: {}", "Success".green().bold(), res.trim());
        }
    }

    Ok(())
}
