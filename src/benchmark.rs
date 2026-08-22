//! HumanEval Pass@1 benchmark: baseline generator vs DSpark dual-engine.

use crate::client::ModelClient;
use crate::cost::{usd_for, TokenUsage};
use crate::curator::DeepSeekCurator;
use crate::oracle::python_cmd;
use crate::util::extract_code_blocks;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const HUMANEVAL_URL: &str =
    "https://github.com/openai/human-eval/raw/master/data/HumanEval.jsonl.gz";
const HUMANEVAL_PLUS_URL: &str =
    "https://github.com/evalplus/humanevalplus_release/releases/download/v0.1.10/HumanEvalPlus-OriginFmt.jsonl.gz";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetKind {
    HumanEval,
    HumanEvalPlus,
}

impl DatasetKind {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_lowercase().as_str() {
            "humaneval" | "he" | "official" => Ok(Self::HumanEval),
            "humaneval-plus" | "humaneval+" | "evalplus" | "plus" => Ok(Self::HumanEvalPlus),
            other => Err(format!(
                "unknown dataset `{other}` (use humaneval or humaneval-plus)"
            )),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::HumanEval => "OpenAI HumanEval (Official)",
            Self::HumanEvalPlus => "EvalPlus HumanEval+",
        }
    }

    fn cache_file(self) -> &'static str {
        match self {
            Self::HumanEval => "HumanEval.jsonl",
            Self::HumanEvalPlus => "HumanEvalPlus-OriginFmt.jsonl",
        }
    }

    fn url(self) -> &'static str {
        match self {
            Self::HumanEval => HUMANEVAL_URL,
            Self::HumanEvalPlus => HUMANEVAL_PLUS_URL,
        }
    }

    fn sandbox_timeout(self) -> Duration {
        match self {
            Self::HumanEval => Duration::from_secs(15),
            Self::HumanEvalPlus => Duration::from_secs(45),
        }
    }
}

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanEvalTask {
    pub task_id: String,
    pub prompt: String,
    pub entry_point: String,
    #[serde(default)]
    pub canonical_solution: String,
    pub test: String,
    #[serde(default)]
    pub plus_test: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskEvaluationResult {
    pub problem_id: String,
    pub title: String,
    pub baseline_passed: bool,
    pub dspark_passed: bool,
    pub baseline_time_ms: f64,
    pub dspark_time_ms: f64,
    pub curator_score: u32,
    pub contra_examples_detected: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curator_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkReport {
    pub dataset_name: String,
    pub generator_model: String,
    pub curator_model: String,
    pub total_problems: usize,
    pub baseline_passed_count: usize,
    pub dspark_passed_count: usize,
    pub rescued_count: usize,
    pub regress_count: usize,
    /// Rescues / generator failures × 100. 0 if the generator never failed.
    pub rescue_rate: f64,
    /// Regressions / generator successes × 100. 0 if the generator never passed.
    pub regression_rate: f64,
    pub baseline_pass_rate: f64,
    pub dspark_pass_rate: f64,
    pub accuracy_delta: f64,
    pub mean_baseline_time_ms: f64,
    pub mean_dspark_time_ms: f64,
    pub wall_time_ms: f64,
    pub generator_usage: TokenUsage,
    pub curator_usage: TokenUsage,
    pub dual_cost_usd: f64,
    pub flagship_cost_usd: f64,
    pub flagship_model: String,
    /// Generator-only (no curator). Used for the live flagship-alone row.
    #[serde(default)]
    pub baseline_only: bool,
    pub results: Vec<TaskEvaluationResult>,
}

pub struct DSparkBenchmarkRunner {
    generator: ModelClient,
    curator: DeepSeekCurator,
    generator_model: String,
    curator_model: String,
}

impl DSparkBenchmarkRunner {
    pub fn new(generator_model: &str, curator_model: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            generator: ModelClient::from_spec(generator_model)?,
            curator: DeepSeekCurator::with_model(curator_model)?,
            generator_model: generator_model.to_string(),
            curator_model: curator_model.to_string(),
        })
    }

    pub async fn load_official_humaneval() -> Result<Vec<HumanEvalTask>, Box<dyn std::error::Error>> {
        Self::load_dataset(DatasetKind::HumanEval).await
    }

    pub async fn load_dataset(
        kind: DatasetKind,
    ) -> Result<Vec<HumanEvalTask>, Box<dyn std::error::Error>> {
        let cache_dir = dataset_cache_dir()?;
        let cache_file = cache_dir.join(kind.cache_file());
        if !cache_file.exists() {
            let http = reqwest::Client::builder()
                .user_agent("DSpark-Benchmark/0.1.0")
                .build()?;
            let bytes = http.get(kind.url()).send().await?.bytes().await?;
            let mut decoder = GzDecoder::new(&bytes[..]);
            let mut jsonl = String::new();
            decoder.read_to_string(&mut jsonl)?;
            fs::write(&cache_file, jsonl)?;
        }

        let raw = fs::read_to_string(&cache_file)?;
        let mut tasks = Vec::new();
        for line in raw.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let mut task: HumanEvalTask = serde_json::from_str(line)?;
            if let Some(plus) = task.plus_test.take() {
                if !plus.trim().is_empty() && !task.test.contains(&plus) {
                    task.test.push_str("\n\n");
                    task.test.push_str(&plus);
                }
            }
            tasks.push(task);
        }
        Ok(tasks)
    }

    pub async fn run_official_humaneval_benchmark(
        &self,
        limit: Option<usize>,
        start_idx: usize,
        progress: impl FnMut(&str),
    ) -> Result<BenchmarkReport, Box<dyn std::error::Error>> {
        self.run_dataset(DatasetKind::HumanEval, limit, start_idx, false, progress)
            .await
    }

    pub async fn run_dataset(
        &self,
        kind: DatasetKind,
        limit: Option<usize>,
        start_idx: usize,
        baseline_only: bool,
        mut progress: impl FnMut(&str),
    ) -> Result<BenchmarkReport, Box<dyn std::error::Error>> {
        let all = Self::load_dataset(kind).await?;
        let end = match limit {
            Some(n) => (start_idx + n).min(all.len()),
            None => all.len(),
        };
        let tasks = &all[start_idx.min(all.len())..end];
        let wall = Instant::now();

        let mut results = Vec::new();
        for (idx, task) in tasks.iter().enumerate() {
            let task_name = format!("{} ({})", task.task_id, task.entry_point);
            progress(&format!(
                "[{}/{}] Evaluating {}...",
                idx + 1,
                tasks.len(),
                task_name
            ));

            let t0 = Instant::now();
            let prompt = format!(
                "Complete the following Python function following its docstring strictly:\n\n{}\n\nReturn only the complete Python code implementing this function.",
                task.prompt
            );
            let mut baseline_error = None;
            let baseline_raw = match self.generator.complete(&prompt, None, 0.2, false).await {
                Ok(raw) => raw,
                Err(e) => {
                    let msg = truncate_err(&e.to_string());
                    progress(&format!("    baseline error: {msg}"));
                    baseline_error = Some(msg);
                    String::new()
                }
            };
            let baseline_code = assemble_solution(&task.prompt, &baseline_raw, &task.entry_point);
            let baseline_time = t0.elapsed().as_secs_f64() * 1000.0;
            let baseline_passed = execute_humaneval_sandbox(
                &baseline_code,
                &task.entry_point,
                &task.test,
                kind.sandbox_timeout(),
            );

            let t1 = Instant::now();
            let mut curator_error = None;
            let (dspark_code, curator_score, contra_count) = if baseline_only {
                (baseline_code.clone(), 0, 0)
            } else {
                match self
                .curator
                .audit(&baseline_code, &task.prompt, Some("python"))
                .await
            {
                Ok(audit) => {
                    let contra = audit.counter_examples.len() + audit.critical_issues.len();
                    let code = if !audit.must_revise() {
                        baseline_code.clone()
                    } else {
                        let mut feedback: Vec<String> = audit.critical_issues.clone();
                        for ce in &audit.counter_examples {
                            feedback.push(format!(
                                "Counter-example input `{}` fails: expected `{}`",
                                ce.failing_input, ce.expected_behavior
                            ));
                        }
                        let fb = if feedback.is_empty() {
                            "Ensure 100% boundary safety.".to_string()
                        } else {
                            feedback.join("\n")
                        };
                        match self
                            .curator
                            .refine(&baseline_code, &task.prompt, Some(&fb), Some("python"))
                            .await
                        {
                            Ok(r) => {
                                if r.refined_code.trim().is_empty() {
                                    baseline_code.clone()
                                } else {
                                    assemble_solution(
                                        &task.prompt,
                                        &r.refined_code,
                                        &task.entry_point,
                                    )
                                }
                            }
                            Err(e) => {
                                let msg = truncate_err(&e.to_string());
                                progress(&format!("    refine error: {msg}"));
                                curator_error = Some(msg);
                                baseline_code.clone()
                            }
                        }
                    };
                    (code, audit.score, contra)
                }
                Err(e) => {
                    let msg = truncate_err(&e.to_string());
                    progress(&format!("    audit error: {msg}"));
                    curator_error = Some(msg);
                    (baseline_code.clone(), 0, 0)
                }
            }
            };
            let dspark_time = t1.elapsed().as_secs_f64() * 1000.0;
            let dspark_passed = if baseline_only {
                baseline_passed
            } else {
                execute_humaneval_sandbox(
                    &dspark_code,
                    &task.entry_point,
                    &task.test,
                    kind.sandbox_timeout(),
                )
            };

            progress(&format!(
                "    baseline={} dual={} score={}/100",
                if baseline_passed { "PASS" } else { "FAIL" },
                if dspark_passed { "PASS" } else { "FAIL" },
                curator_score
            ));

            results.push(TaskEvaluationResult {
                problem_id: task.task_id.clone(),
                title: task.entry_point.clone(),
                baseline_passed,
                dspark_passed,
                baseline_time_ms: baseline_time,
                dspark_time_ms: dspark_time,
                curator_score,
                contra_examples_detected: contra_count,
                baseline_error,
                curator_error,
            });
        }

        let total = results.len();
        let base_pass = results.iter().filter(|r| r.baseline_passed).count();
        let dspark_pass = results.iter().filter(|r| r.dspark_passed).count();
        let rescued = results
            .iter()
            .filter(|r| !r.baseline_passed && r.dspark_passed)
            .count();
        let regress = results
            .iter()
            .filter(|r| r.baseline_passed && !r.dspark_passed)
            .count();
        let base_rate = rate(base_pass, total);
        let dspark_rate = rate(dspark_pass, total);
        let mean_base = mean_ms(results.iter().map(|r| r.baseline_time_ms), total);
        let mean_dual = mean_ms(results.iter().map(|r| r.dspark_time_ms), total);
        let gen_fail = total.saturating_sub(base_pass);
        let rescue_rate = rate(rescued, gen_fail);
        let regression_rate = rate(regress, base_pass);
        let generator_usage = self.generator.usage();
        let curator_usage = self.curator.usage();
        let dual_cost_usd =
            usd_for(&self.generator_model, generator_usage) + usd_for(&self.curator_model, curator_usage);
        let flagship_model = "gpt-4o".to_string();
        let flagship_cost_usd = usd_for(&flagship_model, generator_usage);

        Ok(BenchmarkReport {
            dataset_name: kind.name().into(),
            generator_model: self.generator_model.clone(),
            curator_model: self.curator_model.clone(),
            total_problems: total,
            baseline_passed_count: base_pass,
            dspark_passed_count: dspark_pass,
            rescued_count: rescued,
            regress_count: regress,
            rescue_rate,
            regression_rate,
            baseline_pass_rate: base_rate,
            dspark_pass_rate: dspark_rate,
            accuracy_delta: dspark_rate - base_rate,
            mean_baseline_time_ms: mean_base,
            mean_dspark_time_ms: mean_dual,
            wall_time_ms: wall.elapsed().as_secs_f64() * 1000.0,
            generator_usage,
            curator_usage,
            dual_cost_usd,
            flagship_cost_usd,
            flagship_model,
            baseline_only,
            results,
        })
    }
}

fn rate(pass: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (pass as f64 / total as f64) * 100.0
    }
}

fn mean_ms(iter: impl Iterator<Item = f64>, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        iter.sum::<f64>() / total as f64
    }
}

fn truncate_err(msg: &str) -> String {
    let one_line = msg.replace('\n', " ").replace('\r', " ");
    if one_line.len() > 240 {
        format!("{}…", one_line.chars().take(240).collect::<String>())
    } else {
        one_line
    }
}

fn assemble_solution(prompt: &str, generated: &str, entry_point: &str) -> String {
    let gen = extract_code_blocks(generated);
    if !gen.contains(&format!("def {entry_point}")) {
        return format!("{}\n{}", prompt.trim_end(), gen);
    }
    let mut prefix = String::new();
    for line in prompt.lines() {
        let t = line.trim();
        if (t.starts_with("import ") || t.starts_with("from "))
            && !gen.lines().any(|g| g.trim() == t)
        {
            prefix.push_str(line);
            prefix.push('\n');
        }
    }
    for helper in python_top_level_defs(prompt) {
        if helper.name != entry_point && !gen.contains(&format!("def {}", helper.name)) {
            prefix.push_str(&helper.source);
            prefix.push_str("\n\n");
        }
    }
    if prefix.is_empty() {
        gen
    } else {
        format!("{}{}", prefix, gen)
    }
}

struct PyDef {
    name: String,
    source: String,
}

fn python_top_level_defs(src: &str) -> Vec<PyDef> {
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        let indent = lines[i].len() - trimmed.len();
        if indent == 0 && trimmed.starts_with("def ") {
            let name = trimmed
                .trim_start_matches("def ")
                .split('(')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            let start = i;
            i += 1;
            while i < lines.len() {
                let l = lines[i];
                if l.is_empty() || l.starts_with(' ') || l.starts_with('\t') || l.starts_with('#') {
                    i += 1;
                    continue;
                }
                break;
            }
            out.push(PyDef {
                name,
                source: lines[start..i].join("\n"),
            });
        } else {
            i += 1;
        }
    }
    out
}

fn dataset_cache_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let cache = home.join(".dspark").join("datasets");
    fs::create_dir_all(&cache)?;
    Ok(cache)
}

pub fn reports_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = home.join(".dspark").join("benchmarks");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn execute_humaneval_sandbox(
    code: &str,
    entry_point: &str,
    test_code: &str,
    timeout: Duration,
) -> bool {
    let mut combined = String::new();
    combined.push_str(code);
    combined.push_str("\n\n# --- OFFICIAL OPENAI TEST HARNESS ---\n");
    combined.push_str(test_code);
    combined.push_str("\n\nimport sys\ntry:\n    check(");
    combined.push_str(entry_point);
    combined.push_str(")\n    print(\"ALL_TESTS_PASSED_OFFICIAL\")\nexcept Exception as e:\n    print(f\"FAILED: {e}\")\n    sys.exit(1)\n");
    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "dspark_he_{}_{}.py",
        std::process::id(),
        seq
    ));
    if fs::write(&tmp, combined).is_err() {
        return false;
    }
    let mut child = match Command::new(python_cmd())
        .arg(&tmp)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => {
            let _ = fs::remove_file(&tmp);
            return false;
        }
    };
    let deadline = Instant::now() + timeout;
    let passed = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = child
                    .stdout
                    .take()
                    .and_then(|mut s| {
                        let mut buf = Vec::new();
                        std::io::Read::read_to_end(&mut s, &mut buf).ok()?;
                        Some(buf)
                    })
                    .unwrap_or_default();
                let text = String::from_utf8_lossy(&stdout);
                break status.success() && text.contains("ALL_TESTS_PASSED_OFFICIAL");
            }
            Ok(None) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(50));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    let _ = fs::remove_file(&tmp);
    passed
}

pub fn print_report(report: &BenchmarkReport) {
    use std::io::{self, Write};
    let mut out = io::stdout();
    let _ = writeln!(
        out,
        "\n=== BENCHMARK RESULTS ({}) ===\n",
        report.dataset_name
    );
    let _ = writeln!(out, "  Generator               : {}", report.generator_model);
    let _ = writeln!(out, "  Curator                 : {}", report.curator_model);
    let _ = writeln!(out, "  Total problems          : {}", report.total_problems);
    let _ = writeln!(
        out,
        "  Baseline Pass@1         : {:.1}% ({}/{})",
        report.baseline_pass_rate, report.baseline_passed_count, report.total_problems
    );
    let _ = writeln!(
        out,
        "  Dual-engine Pass@1      : {:.1}% ({}/{})",
        report.dspark_pass_rate, report.dspark_passed_count, report.total_problems
    );
    let _ = writeln!(
        out,
        "  Δ Pass@1 (net gain)     : {:+.1}%",
        report.accuracy_delta
    );
    let _ = writeln!(
        out,
        "  Rescue rate             : {:.1}%  ({}/{} generator failures)",
        report.rescue_rate,
        report.rescued_count,
        report.total_problems.saturating_sub(report.baseline_passed_count)
    );
    let _ = writeln!(
        out,
        "  Regression rate         : {:.1}%  ({}/{} generator successes)",
        report.regression_rate, report.regress_count, report.baseline_passed_count
    );
    let _ = writeln!(
        out,
        "  Dual cost (this slice)  : ${:.6}  ({} in / {} out tokens)",
        report.dual_cost_usd,
        report.generator_usage.prompt_tokens + report.curator_usage.prompt_tokens,
        report.generator_usage.completion_tokens + report.curator_usage.completion_tokens
    );
    let _ = writeln!(
        out,
        "  Flagship {} only (est.) : ${:.6}",
        report.flagship_model, report.flagship_cost_usd
    );
    let _ = writeln!(
        out,
        "  Mean time baseline      : {:.0} ms",
        report.mean_baseline_time_ms
    );
    let _ = writeln!(
        out,
        "  Mean time dual-engine   : {:.0} ms",
        report.mean_dspark_time_ms
    );
    let _ = writeln!(
        out,
        "  Wall time               : {:.1} s\n",
        report.wall_time_ms / 1000.0
    );
    let _ = writeln!(out, "  Task breakdown:");
    for r in &report.results {
        let base = if r.baseline_passed { "PASS" } else { "FAIL" };
        let dual = if r.dspark_passed { "PASS" } else { "FAIL" };
        let _ = writeln!(
            out,
            "    * [{}] {}  baseline={} ({:.0}ms)  dual={} ({:.0}ms, score {}/100, contra {})",
            r.problem_id,
            r.title,
            base,
            r.baseline_time_ms,
            dual,
            r.dspark_time_ms,
            r.curator_score,
            r.contra_examples_detected
        );
        if let Some(err) = &r.baseline_error {
            let _ = writeln!(out, "      baseline error: {err}");
        }
        if let Some(err) = &r.curator_error {
            let _ = writeln!(out, "      curator error: {err}");
        }
    }
    let _ = writeln!(out);
}

pub fn model_family(id: &str) -> &'static str {
    let s = id.to_lowercase();
    if s.contains("gpt") || s.contains("openai") || s.starts_with("o1") || s.starts_with("o3") {
        "openai"
    } else if s.contains("gemini") {
        "gemini"
    } else if s.contains("deepseek") {
        "deepseek"
    } else if s.contains("claude") || s.contains("anthropic") {
        "anthropic"
    } else if s.contains("grok") {
        "xai"
    } else if s.starts_with("local:") || s.starts_with("ollama:") || s.starts_with("lmstudio:") {
        "local"
    } else {
        "other"
    }
}

pub fn is_cross_family(creator: &str, curator: &str) -> bool {
    model_family(creator) != model_family(curator)
}

pub fn print_comparison(reports: &[BenchmarkReport]) {
    if reports.is_empty() {
        return;
    }
    println!("=== PAIR COMPARISON (same official slice) ===\n");
    println!(
        "  {:<22} {:<22} {:>8} {:>8} {:>8} {:>9} {:>9} {:>10}",
        "creator", "curator", "base%", "dual%", "ΔP@1", "rescue%", "regress%", "USD"
    );
    for r in reports {
        println!(
            "  {:<22} {:<22} {:>7.1}% {:>7.1}% {:>+7.1}% {:>8.1}% {:>8.1}% {:>10.4}",
            r.generator_model,
            r.curator_model,
            r.baseline_pass_rate,
            r.dspark_pass_rate,
            r.accuracy_delta,
            r.rescue_rate,
            r.regression_rate,
            r.dual_cost_usd
        );
    }
    println!();
}

/// Thesis table: small-model baseline vs dual-engine vs flagship cost.
pub fn print_thesis_table(reports: &[BenchmarkReport]) {
    if reports.is_empty() {
        return;
    }
    let n = reports[0].total_problems;
    println!("=== DSPARK THESIS VALIDATION ===");
    println!("  Dataset : {}", reports[0].dataset_name);
    println!("  Slice   : {n} tasks\n");
    println!(
        "  {:<22} {:<36} {:>10} {:>10} {:>10} {:>14} {:>16}",
        "architecture",
        "models",
        "Pass@1",
        "rescue%",
        "regress%",
        "USD / slice",
        "USD / 1k tasks"
    );
    let duals: Vec<&BenchmarkReport> = reports.iter().filter(|r| !r.baseline_only).collect();
    let flags: Vec<&BenchmarkReport> = reports.iter().filter(|r| r.baseline_only).collect();
    for r in &duals {
        let scale = scale_1k(r.total_problems);
        let gen_usd = usd_for(&r.generator_model, r.generator_usage);
        println!(
            "  {:<22} {:<36} {:>9.1}% {:>9.1}% {:>9.1}% {:>14.5} {:>16.4}",
            "Baseline (small)",
            r.generator_model,
            r.baseline_pass_rate,
            0.0,
            0.0,
            gen_usd,
            gen_usd * scale
        );
        let dual_label = format!("{}+{}", r.generator_model, r.curator_model);
        println!(
            "  {:<22} {:<36} {:>9.1}% {:>9.1}% {:>9.1}% {:>14.5} {:>16.4}",
            "Dual-engine",
            dual_label,
            r.dspark_pass_rate,
            r.rescue_rate,
            r.regression_rate,
            r.dual_cost_usd,
            r.dual_cost_usd * scale
        );
        println!();
    }
    if flags.is_empty() {
        if let Some(r) = duals.first() {
            let scale = scale_1k(r.total_problems);
            println!(
                "  {:<22} {:<36} {:>10} {:>10} {:>10} {:>14.5} {:>16.4}",
                "Flagship (est.)",
                r.flagship_model,
                "—",
                "—",
                "—",
                r.flagship_cost_usd,
                r.flagship_cost_usd * scale
            );
            println!();
        }
        println!(
            "  Flagship cost prices the same generator tokens at {} list rates (not a live run).",
            reports[0].flagship_model
        );
    } else {
        for r in flags {
            let scale = scale_1k(r.total_problems);
            let usd = usd_for(&r.generator_model, r.generator_usage);
            println!(
                "  {:<22} {:<36} {:>9.1}% {:>9.1}% {:>9.1}% {:>14.5} {:>16.4}",
                "Flagship-alone",
                r.generator_model,
                r.baseline_pass_rate,
                0.0,
                0.0,
                usd,
                usd * scale
            );
        }
        println!();
        println!("  Flagship-alone is a live generator-only run on the same slice.");
    }
    println!(
        "  Rescue rate = curator fixes / generator failures. Regression rate = curator breaks / generator successes.\n"
    );
}

fn scale_1k(n: usize) -> f64 {
    if n == 0 {
        0.0
    } else {
        1000.0 / n as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stitch_prepends_missing_imports() {
        let prompt = "from typing import List\n\ndef filter_by_substring(strings: List[str], substring: str):\n    pass\n";
        let gen = "def filter_by_substring(strings: List[str], substring: str):\n    return [s for s in strings if substring in s]\n";
        let out = assemble_solution(prompt, gen, "filter_by_substring");
        assert!(out.contains("from typing import List"));
        assert!(out.contains("def filter_by_substring"));
    }

    #[test]
    fn rescue_and_regression_rates() {
        let rescued = 2usize;
        let gen_fail = 8usize;
        let regress = 1usize;
        let gen_pass = 10usize;
        assert!((rate(rescued, gen_fail) - 25.0).abs() < 1e-9);
        assert!((rate(regress, gen_pass) - 10.0).abs() < 1e-9);
        assert_eq!(rate(1, 0), 0.0);
    }

    #[test]
    fn stitch_keeps_helper_from_prompt() {
        let prompt = "def encode_cyclic(s):\n    return s[1:]+s[:1]\n\ndef decode_cyclic(s):\n    \"\"\"decode\"\"\"\n";
        let gen = "def decode_cyclic(s):\n    return encode_cyclic(encode_cyclic(s))\n";
        let out = assemble_solution(prompt, gen, "decode_cyclic");
        assert!(out.contains("def encode_cyclic"));
        assert!(out.contains("def decode_cyclic"));
    }
}
