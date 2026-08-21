//! Agentic tool registry (file, shell, search, curator).

use crate::curator::DeepSeekCurator;
use crate::search::WebSearchEngine;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const SKIP_DIR_NAMES: &[&str] = &[
    ".git",
    "__pycache__",
    "node_modules",
    ".venv",
    "target",
    ".dspark",
];

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool_name: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

impl ToolResult {
    fn ok(tool: &str, output: impl Into<String>) -> Self {
        Self {
            tool_name: tool.into(),
            success: true,
            output: output.into(),
            error: None,
        }
    }

    fn err(tool: &str, error: impl Into<String>) -> Self {
        Self {
            tool_name: tool.into(),
            success: false,
            output: String::new(),
            error: Some(error.into()),
        }
    }
}

pub struct ToolRegistry {
    pub working_dir: PathBuf,
    curator: Option<DeepSeekCurator>,
    search_engine: WebSearchEngine,
}

impl ToolRegistry {
    pub fn new(working_dir: PathBuf, curator: Option<DeepSeekCurator>) -> Self {
        Self {
            working_dir,
            curator,
            search_engine: WebSearchEngine::new(),
        }
    }

    fn resolve(&self, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.working_dir.join(p)
        }
    }

    pub fn read_file(&self, path: &str, start_line: Option<i64>, end_line: Option<i64>) -> ToolResult {
        let full = self.resolve(path);
        match fs::read_to_string(&full) {
            Ok(content) => {
                if start_line.is_some() || end_line.is_some() {
                    let lines: Vec<&str> = content.lines().collect();
                    let s = start_line.unwrap_or(1).max(1) as usize - 1;
                    let e = end_line
                        .map(|n| n as usize)
                        .unwrap_or(lines.len())
                        .min(lines.len());
                    let s = s.min(e);
                    ToolResult::ok("read_file", lines[s..e].join("\n"))
                } else {
                    ToolResult::ok("read_file", content)
                }
            }
            Err(e) => ToolResult::err("read_file", format!("File '{}': {}", path, e)),
        }
    }

    pub fn write_file(&self, path: &str, content: &str) -> ToolResult {
        let full = self.resolve(path);
        if let Some(parent) = full.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                return ToolResult::err("write_file", e.to_string());
            }
        }
        match fs::write(&full, content) {
            Ok(()) => ToolResult::ok(
                "write_file",
                format!("Successfully wrote {} bytes to {}", content.len(), path),
            ),
            Err(e) => ToolResult::err("write_file", e.to_string()),
        }
    }

    pub fn edit_file(&self, path: &str, target_chunk: &str, replacement_chunk: &str) -> ToolResult {
        let full = self.resolve(path);
        match fs::read_to_string(&full) {
            Ok(content) => {
                if !content.contains(target_chunk) {
                    return ToolResult::err(
                        "edit_file",
                        format!("Target chunk not found in {}", path),
                    );
                }
                let new_content = content.replacen(target_chunk, replacement_chunk, 1);
                match fs::write(&full, new_content) {
                    Ok(()) => ToolResult::ok("edit_file", format!("Successfully edited {}", path)),
                    Err(e) => ToolResult::err("edit_file", e.to_string()),
                }
            }
            Err(e) => ToolResult::err("edit_file", format!("File '{}': {}", path, e)),
        }
    }

    pub fn list_files(&self, relative_path: &str) -> ToolResult {
        let target = self.resolve(relative_path);
        if !target.exists() {
            return ToolResult::err(
                "list_files",
                format!("Directory '{}' does not exist.", relative_path),
            );
        }
        let mut entries = Vec::new();
        walk_files(&target, &self.working_dir, &mut entries, 120);
        entries.sort();
        ToolResult::ok("list_files", entries.join("\n"))
    }

    pub fn run_terminal(&self, command: &str, timeout_secs: u64) -> ToolResult {
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", command]);
            c
        };
        cmd.current_dir(&self.working_dir);

        match wait_with_timeout(&mut cmd, Duration::from_secs(timeout_secs)) {
            Ok((code, stdout, stderr)) => {
                let mut out = format!("Exit code: {}\n", code);
                if !stdout.is_empty() {
                    out.push_str(&format!("STDOUT:\n{}\n", stdout));
                }
                if !stderr.is_empty() {
                    out.push_str(&format!("STDERR:\n{}\n", stderr));
                }
                ToolResult {
                    tool_name: "run_terminal".into(),
                    success: code == 0,
                    output: out.trim().to_string(),
                    error: None,
                }
            }
            Err(e) => ToolResult::err("run_terminal", e),
        }
    }

    pub async fn search_web(&self, query: &str) -> ToolResult {
        let report = self.search_engine.research_topic(query, 3).await;
        if report.starts_with("No results found") {
            ToolResult::err("search_web", report)
        } else {
            ToolResult::ok("search_web", report)
        }
    }

    pub async fn verify_with_curator(&self, code: &str, specification: &str) -> ToolResult {
        let Some(curator) = &self.curator else {
            return ToolResult::err("verify_with_curator", "Curator is not configured.");
        };
        match curator.audit(code, specification, None).await {
            Ok(audit) => {
                let summary = format!(
                    "Curator Verdict: {} (Score: {}/100)\nSummary: {}\nCounter-Examples Detected: {}",
                    audit.verdict,
                    audit.score,
                    audit.summary,
                    audit.counter_examples.len()
                );
                ToolResult {
                    tool_name: "verify_with_curator".into(),
                    success: audit.is_approved(),
                    output: summary,
                    error: None,
                }
            }
            Err(e) => ToolResult::err("verify_with_curator", e.to_string()),
        }
    }
}

pub fn walk_files(dir: &Path, root: &Path, out: &mut Vec<String>, max: usize) {
    if out.len() >= max {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for ent in rd.flatten() {
        if out.len() >= max {
            break;
        }
        let path = ent.path();
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if SKIP_DIR_NAMES.iter().any(|s| *s == name) {
            continue;
        }
        if path.is_dir() {
            walk_files(&path, root, out, max);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn wait_with_timeout(
    cmd: &mut Command,
    limit: Duration,
) -> Result<(i32, String, String), String> {
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Command execution failed: {}", e))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_h = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut s) = stdout {
            let _ = std::io::Read::read_to_end(&mut s, &mut buf);
        }
        String::from_utf8_lossy(&buf).into_owned()
    });
    let stderr_h = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut s) = stderr {
            let _ = std::io::Read::read_to_end(&mut s, &mut buf);
        }
        String::from_utf8_lossy(&buf).into_owned()
    });

    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) => {
                if start.elapsed() > limit {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "Command timed out after {} seconds.",
                        limit.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(e) => return Err(e.to_string()),
        }
    };

    Ok((
        status.code().unwrap_or(-1),
        stdout_h.join().unwrap_or_default(),
        stderr_h.join().unwrap_or_default(),
    ))
}
