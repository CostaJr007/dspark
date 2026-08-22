//! Official-benchmark runner. Separate binary so it never overwrites the TUI.

use clap::Parser;
use dspark::benchmark::{
    is_cross_family, print_comparison, print_report, print_thesis_table, reports_dir,
    DatasetKind, DSparkBenchmarkRunner,
};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn progress_line(msg: &str) {
    println!("  {msg}");
    let _ = io::stdout().flush();
}

#[derive(Parser)]
#[command(name = "dspark-bench")]
#[command(about = "Thesis bench: HumanEval / HumanEval+ Pass@1 — creator × curator")]
#[command(after_help = "\
THESIS PROTOCOL:
  Rescue rate     = curator fixes / generator failures
  Regression rate = curator breaks / generator successes
  Δ Pass@1        = dual Pass@1 − baseline Pass@1
  Cost            = list-price USD from prompt/completion tokens

EXAMPLES:
  dspark-bench --thesis --list-pairs
  dspark-bench --dataset humaneval-plus --slice 40 --creator gpt-4o-mini --curator deepseek-v4-pro
  dspark-bench --thesis
  dspark-bench --dataset humaneval --all --pair gpt-4o-mini/deepseek-v4-pro
")]
struct Args {
    /// Creator model(s). Alias of --generator.
    #[arg(long = "creator", value_delimiter = ',')]
    creator: Vec<String>,
    /// Creator model(s). Repeat or comma-separate.
    #[arg(short, long, value_delimiter = ',')]
    generator: Vec<String>,
    /// Curator model(s). Repeat or comma-separate.
    #[arg(short, long, default_value = "deepseek-v4-pro", value_delimiter = ',')]
    curator: Vec<String>,
    /// Explicit pair(s) `creator/curator`.
    #[arg(long)]
    pair: Vec<String>,
    /// humaneval | humaneval-plus
    #[arg(long, default_value = "humaneval")]
    dataset: String,
    /// Number of tasks. 164 (or --all) = full official suite.
    #[arg(long)]
    slice: Option<usize>,
    /// Number of tasks (alias of --slice).
    #[arg(short = 'n', long, default_value_t = 8)]
    limit: usize,
    #[arg(long, default_value_t = 0)]
    start: usize,
    /// Full dataset (164 tasks).
    #[arg(long)]
    all: bool,
    /// Thesis protocol: HumanEval+ × gpt-4o-mini and deepseek-v4-flash vs deepseek-v4-pro.
    /// Full 164 unless --slice is set. Adds a live gpt-4o flagship-alone row when OPENAI_API_KEY is set.
    #[arg(long)]
    thesis: bool,
    #[arg(long)]
    matrix: bool,
    #[arg(long)]
    include_same: bool,
    #[arg(long)]
    list_pairs: bool,
    #[arg(long)]
    json: bool,
    /// Skip curator: generator Pass@1 only (live flagship-alone).
    #[arg(long)]
    baseline_only: bool,
    /// Extra live flagship-alone model (default gpt-4o when --thesis and OPENAI_API_KEY is set).
    #[arg(long)]
    flagship: Option<String>,
    #[arg(short, long)]
    out: Option<PathBuf>,
}

#[derive(Clone)]
struct Pair {
    creator: String,
    curator: String,
}

impl Pair {
    fn label(&self) -> String {
        format!("{} / {}", self.creator, self.curator)
    }
}

fn trim_models(raw: Vec<String>) -> Vec<String> {
    raw.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_pair(raw: &str) -> Result<Pair, String> {
    let s = raw.trim();
    let (a, b) = if let Some((l, r)) = s.split_once('/') {
        (l, r)
    } else if let Some((l, r)) = s.split_once('+') {
        (l, r)
    } else if let Some(idx) = s.rfind(':') {
        (&s[..idx], &s[idx + 1..])
    } else {
        return Err(format!(
            "invalid --pair `{s}` (use creator/curator, e.g. gpt-4o-mini/deepseek-v4-pro)"
        ));
    };
    let creator = a.trim().to_string();
    let curator = b.trim().to_string();
    if creator.is_empty() || curator.is_empty() {
        return Err(format!("invalid --pair `{s}`: empty role"));
    }
    Ok(Pair { creator, curator })
}

fn models_with_keys() -> Vec<String> {
    let mut out = Vec::new();
    if env::var("OPENAI_API_KEY").map(|v| !v.is_empty()).unwrap_or(false) {
        out.push("gpt-4o-mini".into());
    }
    if env::var("DEEPSEEK_API_KEY").map(|v| !v.is_empty()).unwrap_or(false) {
        out.push("deepseek-v4-flash".into());
        out.push("deepseek-v4-pro".into());
    }
    if env::var("GEMINI_API_KEY").map(|v| !v.is_empty()).unwrap_or(false) {
        out.push("gemini-2.5-flash".into());
    }
    if env::var("ANTHROPIC_API_KEY").map(|v| !v.is_empty()).unwrap_or(false) {
        out.push("claude-3-haiku-20240307".into());
    }
    out
}

fn cartesian(creators: &[String], curators: &[String], include_same: bool) -> Vec<Pair> {
    let mut pairs = Vec::new();
    for c in creators {
        for k in curators {
            if !include_same && c.eq_ignore_ascii_case(k) {
                continue;
            }
            pairs.push(Pair {
                creator: c.clone(),
                curator: k.clone(),
            });
        }
    }
    pairs
}

fn thesis_pairs() -> Vec<Pair> {
    vec![
        Pair {
            creator: "gpt-4o-mini".into(),
            curator: "deepseek-v4-pro".into(),
        },
        Pair {
            creator: "deepseek-v4-flash".into(),
            curator: "deepseek-v4-pro".into(),
        },
    ]
}

fn resolve_flagship(args: &Args) -> Option<String> {
    if let Some(raw) = &args.flagship {
        let s = raw.trim();
        if s.is_empty() || s.eq_ignore_ascii_case("none") || s == "-" {
            return None;
        }
        return Some(s.to_string());
    }
    let openai = env::var("OPENAI_API_KEY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    if args.thesis && !args.baseline_only && openai {
        Some("gpt-4o".into())
    } else {
        None
    }
}

fn resolve_pairs(args: &Args) -> Result<Vec<Pair>, Box<dyn std::error::Error>> {
    if args.thesis && args.pair.is_empty() && args.creator.is_empty() && args.generator.is_empty() && !args.matrix {
        return Ok(thesis_pairs());
    }
    if !args.pair.is_empty() {
        let mut pairs = Vec::new();
        for raw in &args.pair {
            let p = parse_pair(raw)?;
            if !args.include_same && p.creator.eq_ignore_ascii_case(&p.curator) {
                continue;
            }
            pairs.push(p);
        }
        return Ok(pairs);
    }
    if args.matrix {
        let models = models_with_keys();
        if models.is_empty() {
            return Err(" --matrix: no API keys in this process".into());
        }
        return Ok(cartesian(&models, &models, args.include_same));
    }
    let mut generators = trim_models(args.creator.clone());
    generators.extend(trim_models(args.generator.clone()));
    if generators.is_empty() {
        generators = vec!["gpt-4o-mini".into(), "deepseek-v4-flash".into()];
    }
    let curators = trim_models(args.curator.clone());
    if curators.is_empty() {
        return Err("need at least one --curator".into());
    }
    Ok(cartesian(&generators, &curators, args.include_same))
}

fn print_pair_plan(pairs: &[Pair], dataset: &str, slice: &str) {
    println!("=== DSPARK THESIS BENCHMARK ===");
    println!("  Dataset   : {dataset}");
    println!("  Slice     : {slice}");
    println!("  Metrics   : Rescue rate, Regression rate, Δ Pass@1, USD/token");
    println!("  Pairs     : {}\n", pairs.len());
    println!(
        "  {:<4} {:<22} {:<22} {:<8} {}",
        "#", "creator", "curator", "family", "note"
    );
    for (i, p) in pairs.iter().enumerate() {
        let cross = is_cross_family(&p.creator, &p.curator);
        let fam = if cross { "cross" } else { "same" };
        let note = if p.creator.eq_ignore_ascii_case(&p.curator) {
            "same-model (confirmation-bias control)"
        } else if !cross {
            "same family"
        } else {
            "asymmetric dual-engine"
        };
        println!(
            "  {:<4} {:<22} {:<22} {:<8} {}",
            i + 1,
            p.creator,
            p.curator,
            fam,
            note
        );
    }
    println!();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let dataset = if args.thesis && args.dataset == "humaneval" {
        DatasetKind::HumanEvalPlus
    } else {
        DatasetKind::parse(&args.dataset)?
    };
    let pairs = resolve_pairs(&args)?;
    if pairs.is_empty() {
        return Err("no pairs to run (all were skipped as identical; pass --include-same)".into());
    }

    let all = args.all || args.slice == Some(164) || (args.thesis && args.slice.is_none());
    let limit = args.slice.unwrap_or(args.limit);
    let slice = if all {
        "all 164".to_string()
    } else {
        format!("{} tasks starting at {}", limit, args.start)
    };
    print_pair_plan(&pairs, dataset.name(), &slice);

    let flagship = resolve_flagship(&args);
    if let Some(ref model) = flagship {
        println!("  Flagship-alone (live): {model}\n");
    }

    if args.list_pairs {
        println!("No API calls (--list-pairs). Re-run without it to execute.");
        return Ok(());
    }

    let mut reports = Vec::new();
    for (idx, pair) in pairs.iter().enumerate() {
        println!("--- [{}/{}] {} ---", idx + 1, pairs.len(), pair.label());
        let runner = DSparkBenchmarkRunner::new(&pair.creator, &pair.curator)?;
        let report = runner
            .run_dataset(
                dataset,
                if all { None } else { Some(limit) },
                args.start,
                args.baseline_only,
                progress_line,
            )
            .await?;
        print_report(&report);
        if args.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        let _ = io::stdout().flush();
        reports.push(report);
    }

    if let Some(model) = flagship {
        println!("--- flagship-alone {model} (generator only) ---");
        let _ = io::stdout().flush();
        let runner = DSparkBenchmarkRunner::new(&model, "deepseek-v4-pro")?;
        let report = runner
            .run_dataset(
                dataset,
                if all { None } else { Some(limit) },
                args.start,
                true,
                progress_line,
            )
            .await?;
        print_report(&report);
        reports.push(report);
    }

    print_comparison(&reports);
    print_thesis_table(&reports);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let default_path = reports_dir()?.join(format!("thesis-{}-{stamp}.json", args.dataset));
    let out_path = args.out.unwrap_or(default_path);
    fs::write(&out_path, serde_json::to_string_pretty(&reports)?)?;
    println!("JSON report: {}", out_path.display());
    Ok(())
}
