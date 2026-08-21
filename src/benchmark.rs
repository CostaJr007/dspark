//! HumanEval Pass@1 benchmark: baseline generator vs DSpark dual-engine.

use crate::client::ModelClient;
use crate::curator::DeepSeekCurator;
use crate::util::extract_code_blocks;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

const HUMANEVAL_URL: &str =
    "https://github.com/openai/human-eval/raw/master/data/HumanEval.jsonl.gz";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanEvalTask {
    pub task_id: String,
    pub prompt: String,
    pub entry_point: String,
    #[serde(default)]
    pub canonical_solution: String,
    pub test: String,
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
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkReport {
    pub dataset_name: String,
    pub total_problems: usize,
    pub baseline_passed_count: usize,
    pub dspark_passed_count: usize,
    pub baseline_pass_rate: f64,
    pub dspark_pass_rate: f64,
    pub accuracy_delta: f64,
    pub results: Vec<TaskEvaluationResult>,
}

pub struct DSparkBenchmarkRunner {
    generator: ModelClient,
    curator: DeepSeekCurator,
}

impl DSparkBenchmarkRunner {
    pub fn new(generator_model: &str, curator_model: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            generator: ModelClient::from_spec(generator_model)?,
            curator: DeepSeekCurator::with_model(curator_model)?,
        })
    }

    pub async fn load_official_humaneval() -> Result<Vec<HumanEvalTask>, Box<dyn std::error::Error>> {
        let cache_dir = dataset_cache_dir()?;
        let cache_file = cache_dir.join("HumanEval.jsonl");
        if !cache_file.exists() {
            let http = reqwest::Client::builder()
                .user_agent("DSpark-Benchmark/0.1.0")
                .build()?;
            let bytes = http.get(HUMANEVAL_URL).send().await?.bytes().await?;
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
            tasks.push(serde_json::from_str::<HumanEvalTask>(line)?);
        }
        Ok(tasks)
    }

    pub async fn run_official_humaneval_benchmark(
        &self,
        limit: Option<usize>,
        start_idx: usize,
        mut progress: impl FnMut(&str),
    ) -> Result<BenchmarkReport, Box<dyn std::error::Error>> {
        let all = Self::load_official_humaneval().await?;
        let end = match limit {
            Some(n) => (start_idx + n).min(all.len()),
            None => all.len(),
        };
        let tasks = &all[start_idx.min(all.len())..end];

        let mut results = Vec::new();
        for (idx, task) in tasks.iter().enumerate() {
            let task_name = format!("{} ({})", task.task_id, task.entry_point);
            progress(&format!("[{}/{}] Evaluating {}...", idx + 1, tasks.len(), task_name));

            let t0 = Instant::now();
            let prompt = format!(
                "Complete the following Python function following its docstring strictly:\n\n{}\n\nReturn only the complete Python code implementing this function.",
                task.prompt
            );
            let baseline_code = match self.generator.complete(&prompt, None, 0.2, false).await {
                Ok(raw) => extract_code_blocks(&raw),
                Err(_) => String::new(),
            };
            let baseline_time = t0.elapsed().as_secs_f64() * 1000.0;
            let baseline_passed =
                execute_humaneval_sandbox(&baseline_code, &task.entry_point, &task.test);

            let t1 = Instant::now();
            let (dspark_code, curator_score, contra_count) = match self
                .curator
                .audit(&baseline_code, &task.prompt, Some("python"))
                .await
            {
                Ok(audit) => {
                    let contra = audit.counter_examples.len() + audit.critical_issues.len();
                    let code = if audit.is_approved() && audit.refined_code.is_none() {
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
                            Ok(r) => r.refined_code,
                            Err(_) => baseline_code.clone(),
                        }
                    };
                    (code, audit.score, contra)
                }
                Err(_) => (baseline_code.clone(), 50, 0),
            };
            let dspark_time = t1.elapsed().as_secs_f64() * 1000.0;
            let dspark_passed =
                execute_humaneval_sandbox(&dspark_code, &task.entry_point, &task.test);

            results.push(TaskEvaluationResult {
                problem_id: task.task_id.clone(),
                title: task.entry_point.clone(),
                baseline_passed,
                dspark_passed,
                baseline_time_ms: baseline_time,
                dspark_time_ms: dspark_time,
                curator_score,
                contra_examples_detected: contra_count,
            });
        }

        let total = results.len();
        let base_pass = results.iter().filter(|r| r.baseline_passed).count();
        let dspark_pass = results.iter().filter(|r| r.dspark_passed).count();
        let base_rate = if total == 0 {
            0.0
        } else {
            (base_pass as f64 / total as f64) * 100.0
        };
        let dspark_rate = if total == 0 {
            0.0
        } else {
            (dspark_pass as f64 / total as f64) * 100.0
        };

        Ok(BenchmarkReport {
            dataset_name: "OpenAI HumanEval (Official)".into(),
            total_problems: total,
            baseline_passed_count: base_pass,
            dspark_passed_count: dspark_pass,
            baseline_pass_rate: base_rate,
            dspark_pass_rate: dspark_rate,
            accuracy_delta: dspark_rate - base_rate,
            results,
        })
    }
}

fn dataset_cache_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let cache = home.join(".dspark").join("datasets");
    fs::create_dir_all(&cache)?;
    Ok(cache)
}

fn python_cmd() -> &'static str {
    if Command::new("python")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        "python"
    } else {
        "py"
    }
}

fn execute_humaneval_sandbox(code: &str, entry_point: &str, test_code: &str) -> bool {
    let mut combined = String::new();
    combined.push_str(code);
    combined.push_str("\n\n# --- OFFICIAL OPENAI TEST HARNESS ---\n");
    combined.push_str(test_code);
    combined.push_str("\n\nimport sys\ntry:\n    check(");
    combined.push_str(entry_point);
    combined.push_str(")\n    print(\"ALL_TESTS_PASSED_OFFICIAL\")\nexcept Exception as e:\n    print(f\"FAILED: {e}\")\n    sys.exit(1)\n");
    let tmp = std::env::temp_dir().join(format!("dspark_he_{}.py", std::process::id()));
    if fs::write(&tmp, combined).is_err() {
        return false;
    }
    let output = Command::new(python_cmd())
        .arg(&tmp)
        .output();
    let _ = fs::remove_file(&tmp);
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            out.status.success() && stdout.contains("ALL_TESTS_PASSED_OFFICIAL")
        }
        Err(_) => false,
    }
}
