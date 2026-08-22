//! Main entrypoint for the DSpark Rust CLI.

use clap::{Parser, Subcommand};
use colored::*;
use dspark::benchmark::DSparkBenchmarkRunner;
use dspark::client::{DeepSeekClient, LocalLLMClient, ModelClient};
use dspark::curator::DeepSeekCurator;
use dspark::mcp::run_mcp_server;
use dspark::pipeline::DSparkPipeline;
use dspark::repl::start_repl;
use dspark::search::WebSearchEngine;
use dspark::pair::DsparkPair;
use dspark::util::read_file_or_string;
use dspark::DSparkAgent;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "dspark")]
#[command(about = "DSpark engine: creator drafts, curator audits I/O")]
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
    /// Show creator/curator pair (~/.dspark/pair.toml)
    Pair,
    /// Start interactive terminal REPL
    Interactive {
        #[arg(short, long)]
        generator: Option<String>,
        #[arg(short, long)]
        curator: Option<String>,
        #[arg(short, long, default_value = "bloomberg")]
        theme: String,
    },
    /// Live web search for docs or error fixes
    Search {
        query: String,
        #[arg(short = 'n', long = "sources", default_value_t = 5)]
        limit: usize,
        /// Search + fetch top pages into a research brief
        #[arg(long)]
        deep: bool,
    },
    /// Fetch and convert a webpage to clean Markdown
    Fetch { url: String },
    /// Audit a code file against specifications using DeepSeek Reasoner
    Audit {
        file: String,
        #[arg(short, long)]
        spec: String,
        #[arg(short, long)]
        lang: Option<String>,
        #[arg(short, long)]
        json: bool,
    },
    /// Refine code to cover edge cases and I/O contracts
    Refine {
        file: String,
        #[arg(short, long)]
        spec: String,
        #[arg(short = 'i', long = "in-place")]
        in_place: bool,
        #[arg(short, long)]
        out: Option<String>,
        #[arg(short, long)]
        lang: Option<String>,
    },
    /// Arbitrate between two or more candidate implementations
    Arbitrate {
        files: Vec<String>,
        #[arg(short, long)]
        spec: String,
        #[arg(short, long)]
        lang: Option<String>,
    },
    /// Run the full dual-model pipeline (Generate → Curate → Output)
    Run {
        prompt: String,
        #[arg(short, long)]
        draft: Option<String>,
        #[arg(short, long)]
        lang: Option<String>,
        #[arg(short, long)]
        out: Option<String>,
        #[arg(short, long)]
        generator: Option<String>,
        #[arg(short, long)]
        curator: Option<String>,
        /// Skip live web research before generating
        #[arg(long = "no-research")]
        no_research: bool,
    },
    /// Run HumanEval Pass@1 benchmark (baseline vs DSpark dual-engine)
    Bench {
        #[arg(short, long, default_value = "gpt-4o-mini")]
        generator: String,
        #[arg(short, long, default_value = "deepseek-v4-pro")]
        curator: String,
        #[arg(short = 'n', long, default_value_t = 5)]
        limit: usize,
        #[arg(short, long, default_value_t = 0)]
        start: usize,
        #[arg(short, long)]
        all: bool,
        #[arg(short, long)]
        json: bool,
    },
    /// Scan, list and test local offline LLMs (Ollama, LM Studio, vLLM)
    Local {
        #[arg(short, long)]
        url: Option<String>,
        #[arg(short, long)]
        test: Option<String>,
    },
    /// Run DSpark as a Model Context Protocol (MCP) server
    Mcp,
    /// Test connection to DeepSeek API
    TestConnection {
        #[arg(short, long)]
        model: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if !cli.task.is_empty() && cli.command.is_none() {
        let task_prompt = cli.task.join(" ");
        println!(
            "{}: {}",
            "Executing DSpark One-Shot Task".dimmed(),
            task_prompt.bold()
        );
        let agent = DSparkAgent::new(None)?;
        let result = agent.execute_task(&task_prompt).await?;
        println!("\n{}\n", result);
        return Ok(());
    }

    match cli.command {
        None => {
            let pair = DsparkPair::load();
            start_repl(&pair.creator, &pair.curator, "bloomberg").await?;
        }
        Some(Commands::Pair) => {
            let pair = DsparkPair::load();
            println!("creator={}", pair.creator);
            println!("curator={}", pair.curator);
            println!("research={}", pair.research);
            println!("pair_file={}", DsparkPair::pair_path().display());
            if let Some(w) = pair.same_family_warning() {
                eprintln!("{}", w.yellow());
            }
        }
        Some(Commands::Interactive {
            generator,
            curator,
            theme,
        }) => {
            let pair = DsparkPair::load();
            start_repl(
                generator.as_deref().unwrap_or(&pair.creator),
                curator.as_deref().unwrap_or(&pair.curator),
                &theme,
            )
            .await?;
        }
        Some(Commands::Search { query, limit, deep }) => {
            let engine = WebSearchEngine::new();
            println!(
                "{}",
                format!(
                    "Searching via {} for: '{}'...",
                    WebSearchEngine::provider_name(),
                    query
                )
                .dimmed()
            );
            if deep {
                let report = engine.research_topic(&query, limit.min(3)).await;
                println!("\n{}\n", report);
            } else {
                let results = engine.search(&query, limit).await?;
                if results.is_empty() {
                    println!(
                        "{}",
                        "No search results. Set TAVILY_API_KEY for ranked search, or check the network."
                            .yellow()
                    );
                } else {
                    println!(
                        "\n{}",
                        format!("=== Web Search Results for: '{}' ===", query)
                            .blue()
                            .bold()
                    );
                    for (i, res) in results.iter().enumerate() {
                        println!("{}. {}", i + 1, res.title.green().bold());
                        println!("   {}: {}", "URL".dimmed(), res.url);
                        println!("   {}\n", res.snippet);
                    }
                }
            }
        }
        Some(Commands::Fetch { url }) => {
            let engine = WebSearchEngine::new();
            println!("{}", format!("Fetching: {}...", url).dimmed());
            let content = engine.fetch_url(&url, 8000).await?;
            println!("\n{}\n", content);
        }
        Some(Commands::Audit {
            file,
            spec,
            lang,
            json,
        }) => {
            let code = read_file_or_string(&file)?;
            let spec = read_file_or_string(&spec)?;
            let pair = DsparkPair::load();
            let curator = DeepSeekCurator::with_model(&pair.curator)?;
            println!(
                "{}",
                format!("Auditing '{}' with curator {}...", file, pair.curator).dimmed()
            );
            let result = curator.audit(&code, &spec, lang.as_deref()).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let verdict_colored = match result.verdict.as_str() {
                    "APPROVED" => result.verdict.to_string().green().bold(),
                    "NEEDS_REVISION" => result.verdict.to_string().yellow().bold(),
                    _ => result.verdict.to_string().red().bold(),
                };
                println!(
                    "\n=== DSPARK AUDIT VERDICT: {} (Score: {}/100) ===\n",
                    verdict_colored, result.score
                );
                println!("Summary: {}\n", result.summary);
                if !result.critical_issues.is_empty() {
                    println!("Critical Issues:");
                    for issue in &result.critical_issues {
                        println!("  [!] {}", issue.red());
                    }
                    println!();
                }
                if !result.edge_cases.is_empty() {
                    println!("Edge Case Analysis:");
                    for ec in &result.edge_cases {
                        let status = if ec.handled_properly {
                            "✓ Handled".green().to_string()
                        } else {
                            format!("✗ Flaw ({} Risk): {}", ec.risk_level, ec.remedy)
                                .red()
                                .to_string()
                        };
                        println!("  - {}: {}", ec.case, status);
                    }
                    println!();
                }
                if let Some(cx) = &result.complexity {
                    println!("Complexity: Time {}, Space {}\n", cx.time, cx.space);
                }
                if !result.suggested_improvements.is_empty() {
                    println!("Suggested Improvements:");
                    for imp in &result.suggested_improvements {
                        println!("  * {}", imp);
                    }
                    println!();
                }
            }
        }
        Some(Commands::Refine {
            file,
            spec,
            in_place,
            out,
            lang,
        }) => {
            let code = read_file_or_string(&file)?;
            let spec = read_file_or_string(&spec)?;
            let pair = DsparkPair::load();
            let curator = DeepSeekCurator::with_model(&pair.curator)?;
            let result = curator.refine(&code, &spec, None, lang.as_deref()).await?;
            if in_place && Path::new(&file).is_file() {
                fs::write(&file, &result.refined_code)?;
                println!("Refined code written in-place to {}", file);
            } else if let Some(dest) = out {
                fs::write(&dest, &result.refined_code)?;
                println!("Refined code written to {}", dest);
            } else {
                println!("{}", result.refined_code);
            }
        }
        Some(Commands::Arbitrate { files, spec, lang }) => {
            let curator = DeepSeekCurator::new()?;
            let mut candidates = Vec::new();
            for f in &files {
                candidates.push(read_file_or_string(f)?);
            }
            let spec = read_file_or_string(&spec)?;
            let result = curator
                .arbitrate(&candidates, &spec, lang.as_deref())
                .await?;
            println!("\n=== DSPARK ARBITRATION RESULT ===");
            println!("Winner: Candidate #{}", result.winner_index);
            println!("Rationale: {}\n", result.rationale);
            println!("Optimal Synthesized Code:");
            println!("{}", result.synthesized_code);
        }
        Some(Commands::Run {
            prompt,
            draft,
            lang,
            out,
            generator,
            curator,
            no_research,
        }) => {
            let pair = DsparkPair::load();
            let generator = generator.unwrap_or_else(|| pair.creator.clone());
            let curator = curator.unwrap_or_else(|| pair.curator.clone());
            println!(
                "{}",
                format!("Pipeline creator={} curator={}", generator, curator).dimmed()
            );
            let pipeline = DSparkPipeline::with_models(&generator, &curator)?;
            let draft_code = match draft {
                Some(path) => Some(read_file_or_string(&path)?),
                None => None,
            };
            let do_research = !no_research;
            if do_research {
                println!(
                    "{}",
                    format!(
                        "Researching live docs via {}...",
                        WebSearchEngine::provider_name()
                    )
                    .dimmed()
                );
            }
            let res = pipeline
                .run(
                    &prompt,
                    draft_code.as_deref(),
                    lang.as_deref(),
                    do_research,
                )
                .await?;
            println!("\n=== DSPARK PIPELINE COMPLETED ===");
            println!(
                "Audit Verdict: {} (Score: {}/100)",
                res.audit_result.verdict, res.audit_result.score
            );
            if let Some(re) = &res.reaudit_result {
                println!(
                    "Re-audit after refine: {} (Score: {}/100)",
                    re.verdict, re.score
                );
            }
            println!("Refined by Curator: {}\n", res.refined);
            if let Some(dest) = out {
                fs::write(&dest, &res.final_code)?;
                println!("Final verified code written to {}", dest);
            } else {
                println!("Final Verified Code:");
                println!("{}", res.final_code);
            }
        }
        Some(Commands::Bench {
            generator,
            curator,
            limit,
            start,
            all,
            json,
        }) => {
            let runner = DSparkBenchmarkRunner::new(&generator, &curator)?;
            println!("\n{}", "=== ⚡ DSPARK AI BENCHMARK SUITE ===".cyan().bold());
            println!("  Dataset   : Official OpenAI HumanEval (164 tasks)");
            println!("  Generator : {} (Mass Code Generation)", generator.yellow());
            println!(
                "  Curator   : {} (LLM-as-a-Verifier Audit & Refinement)",
                curator.green()
            );
            println!(
                "{}",
                "Running Pass@1 evaluation: Baseline vs DSpark Dual-Engine...\n".dimmed()
            );
            let report = runner
                .run_official_humaneval_benchmark(
                    if all { None } else { Some(limit) },
                    start,
                    |msg| println!("  {} {}", "➜".dimmed(), msg),
                )
                .await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "\n{}",
                    format!("=== 📊 BENCHMARK RESULTS ({}) ===\n", report.dataset_name)
                        .blue()
                        .bold()
                );
                println!("  Generator                : {}", report.generator_model);
                println!("  Curator                  : {}", report.curator_model);
                println!("  Total Problems Evaluated : {}", report.total_problems);
                println!(
                    "  Baseline Pass@1 Rate     : {} ({}/{})",
                    format!("{:.1}%", report.baseline_pass_rate).red(),
                    report.baseline_passed_count,
                    report.total_problems
                );
                println!(
                    "  DSpark Dual-Engine Rate  : {} ({}/{})",
                    format!("{:.1}%", report.dspark_pass_rate).green(),
                    report.dspark_passed_count,
                    report.total_problems
                );
                println!(
                    "  Empirical Accuracy Gain  : {}",
                    format!("{:+.1}%", report.accuracy_delta).green()
                );
                println!(
                    "  Rescued (fail→pass)      : {}  |  Regressed (pass→fail): {}\n",
                    report.rescued_count, report.regress_count
                );
                println!("  Detailed Task Breakdown:");
                for r in &report.results {
                    let base_status = if r.baseline_passed {
                        "PASS".green().to_string()
                    } else {
                        "FAIL".red().to_string()
                    };
                    let dspark_status = if r.dspark_passed {
                        "PASS".green().to_string()
                    } else {
                        "FAIL".red().to_string()
                    };
                    println!("    * [{}] {}", r.problem_id, r.title);
                    println!(
                        "      - Baseline: {} ({:.0}ms) | DSpark Dual: {} (Score: {}/100, Contraexamples: {})",
                        base_status,
                        r.baseline_time_ms,
                        dspark_status,
                        r.curator_score,
                        r.contra_examples_detected
                    );
                }
                println!();
            }
        }
        Some(Commands::Local { url, test }) => {
            println!("\n{}", "=== 💻 DSPARK LOCAL & OFFLINE LLM SCANNER ===\n".cyan().bold());
            let active = LocalLLMClient::detect_active_endpoints().await;
            if active.is_empty() {
                println!("{}", "  [!] No active local LLM servers detected on localhost.\n".yellow());
                println!("  To run models locally (100% free and private):");
                println!("    1. Install Ollama: https://ollama.com");
                println!("    2. Start a model: ollama run qwen2.5-coder:1.5b");
                println!("    3. Or start LM Studio with local server enabled on port 1234\n");
            } else {
                println!(
                    "{}",
                    format!("  [✓] Found {} active local LLM endpoint(s):", active.len()).green()
                );
                for s in &active {
                    println!("    * {}: {}", s.name.bold(), s.v1_url);
                    if let Ok(client) = LocalLLMClient::new(Some(&s.v1_url), None) {
                        let models = client.list_models().await;
                        if models.is_empty() {
                            println!("      (Server running, no models pulled yet)");
                        } else {
                            println!("      Available local models:");
                            for m in models {
                                println!("        - {}", m.cyan());
                            }
                        }
                    }
                }
                println!();
            }
            if let Some(model) = test {
                println!("  Testing generation with local model '{}'...", model.bold());
                let client = ModelClient::from_spec(&format!("local:{}", model)).or_else(|_| {
                    LocalLLMClient::new(url.as_deref(), Some(&model)).map(ModelClient::Local)
                })?;
                let res = client
                    .complete(
                        "Write a python one-liner to reverse a list.",
                        None,
                        0.2,
                        false,
                    )
                    .await?;
                println!("\n  {}\n{}\n", "[✓] Model Response:".green(), res);
            }
        }
        Some(Commands::Mcp) => run_mcp_server().await?,
        Some(Commands::TestConnection { model }) => {
            let client = if let Some(m) = model {
                DeepSeekClient::with_model(Some(&m))?
            } else {
                DeepSeekClient::new()?
            };
            println!(
                "Connecting to DeepSeek API at {} (model: {})...",
                client.base_url, client.model
            );
            let res = client
                .complete(
                    "Ping. Respond with 'DSpark Online (Rust)'.",
                    None,
                    0.0,
                    false,
                )
                .await?;
            println!("{}: {}", "Success".green().bold(), res.trim());
        }
    }

    Ok(())
}
