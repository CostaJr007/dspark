//! Spec-visible oracle: doctests and helpers from the prompt, never hidden tests.

use serde::Deserialize;
use std::fs;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Deserialize)]
pub struct OracleFailure {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub input: String,
    #[serde(default)]
    pub expected: String,
    #[serde(default)]
    pub actual: String,
    #[serde(default)]
    pub message: String,
}

pub fn python_cmd() -> String {
    if let Ok(p) = std::env::var("PYTHON") {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    for candidate in &["python3", "python", "py"] {
        if Command::new(candidate)
            .arg("-c")
            .arg("import sys; sys.exit(0)")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return candidate.to_string();
        }
    }
    "python".to_string()
}

/// Run prompt-visible checks against candidate Python (doctest + encode/decode roundtrip).
pub fn run_python_spec_oracle(code: &str, specification: &str) -> Vec<OracleFailure> {
    if code.trim().is_empty() {
        return vec![OracleFailure {
            kind: "empty".into(),
            input: String::new(),
            expected: String::new(),
            actual: String::new(),
            message: "candidate code is empty".into(),
        }];
    }

    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir();
    let code_path = dir.join(format!("dspark_oracle_code_{}_{}.py", std::process::id(), seq));
    let spec_path = dir.join(format!("dspark_oracle_spec_{}_{}.py", std::process::id(), seq));
    let runner_path = dir.join(format!("dspark_oracle_run_{}_{}.py", std::process::id(), seq));
    if fs::write(&code_path, code).is_err() || fs::write(&spec_path, specification).is_err() {
        return vec![OracleFailure {
            kind: "io".into(),
            input: String::new(),
            expected: String::new(),
            actual: String::new(),
            message: "failed to write oracle temp files".into(),
        }];
    }
    let runner = r#"# -*- coding: utf-8 -*-
import doctest, json, sys, traceback
code_path, spec_path = sys.argv[1], sys.argv[2]
code = open(code_path, encoding="utf-8").read()
spec = open(spec_path, encoding="utf-8").read()
ns = {}
failures = []
try:
    exec(compile(code, "<candidate>", "exec"), ns)
except Exception as e:
    failures.append({"kind":"exec","input":"","expected":"","actual":"","message":traceback.format_exc(limit=2)})
    print(json.dumps(failures))
    sys.exit(0)

finder = doctest.DocTestFinder()
runner = doctest.DocTestRunner(verbose=False, optionflags=doctest.ELLIPSIS | doctest.NORMALIZE_WHITESPACE)
for name, obj in list(ns.items()):
    if not callable(obj):
        continue
    for t in finder.find(obj, name, globs=dict(ns)):
        nfail, nrun = runner.run(t, out=lambda s: None)
        if nfail:
            failures.append({"kind":"doctest","input":name,"expected":"docstring examples","actual":"failed","message":"%s docstring examples failed" % name})

# Spec-visible encode/decode roundtrip when both helpers appear in the prompt.
enc = next((v for k,v in ns.items() if k.startswith("encode_") and callable(v)), None)
dec = next((v for k,v in ns.items() if k.startswith("decode_") and callable(v)), None)
if enc and dec:
    for s in ["", "a", "ab", "abc", "abcdef", "abcdefgh"]:
        try:
            got = dec(enc(s))
            if got != s:
                failures.append({"kind":"roundtrip","input":repr(s),"expected":repr(s),"actual":repr(got),"message":"decode(encode(s)) != s"})
                break
        except Exception as e:
            failures.append({"kind":"roundtrip","input":repr(s),"expected":repr(s),"actual":str(e),"message":"decode(encode(s)) raised"})
            break
print(json.dumps(failures))
"#;
    if fs::write(&runner_path, runner).is_err() {
        let _ = fs::remove_file(&code_path);
        let _ = fs::remove_file(&spec_path);
        return vec![OracleFailure {
            kind: "io".into(),
            input: String::new(),
            expected: String::new(),
            actual: String::new(),
            message: "failed to write oracle runner".into(),
        }];
    }

    let mut child = match Command::new(python_cmd())
        .arg(&runner_path)
        .arg(&code_path)
        .arg(&spec_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = fs::remove_file(&code_path);
            let _ = fs::remove_file(&spec_path);
            let _ = fs::remove_file(&runner_path);
            return vec![OracleFailure {
                kind: "spawn".into(),
                input: String::new(),
                expected: String::new(),
                actual: String::new(),
                message: format!("python spawn failed: {e}"),
            }];
        }
    };
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut timed_out = false;
    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();

    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                if let Some(mut s) = child.stdout.take() {
                    let _ = std::io::Read::read_to_end(&mut s, &mut stdout_buf);
                }
                if let Some(mut e) = child.stderr.take() {
                    let _ = std::io::Read::read_to_end(&mut e, &mut stderr_buf);
                }
                break;
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(30)),
            _ => {
                timed_out = true;
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
        }
    }

    let _ = fs::remove_file(&code_path);
    let _ = fs::remove_file(&spec_path);
    let _ = fs::remove_file(&runner_path);

    if timed_out {
        return vec![OracleFailure {
            kind: "timeout".into(),
            input: String::new(),
            expected: String::new(),
            actual: String::new(),
            message: "spec oracle execution timed out".into(),
        }];
    }

    let stdout = String::from_utf8_lossy(&stdout_buf).into_owned();
    let stderr = String::from_utf8_lossy(&stderr_buf).into_owned();

    if stdout.trim().is_empty() {
        return vec![OracleFailure {
            kind: "execution_error".into(),
            input: String::new(),
            expected: String::new(),
            actual: stderr.trim().to_string(),
            message: "spec oracle produced no stdout output".into(),
        }];
    }

    serde_json::from_str::<Vec<OracleFailure>>(stdout.trim()).unwrap_or_else(|_| {
        vec![OracleFailure {
            kind: "parse".into(),
            input: String::new(),
            expected: String::new(),
            actual: stderr.trim().to_string(),
            message: stdout.chars().take(200).collect(),
        }]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctest_catches_wrong_filter() {
        let spec = r#"from typing import List
def filter_by_substring(strings: List[str], substring: str) -> List[str]:
    """Filter strings that contain substring
    >>> filter_by_substring([], 'a')
    []
    >>> filter_by_substring(['abc', 'bacd', 'cde', 'array'], 'a')
    ['abc', 'bacd', 'array']
    """
"#;
        let wrong = r#"from typing import List
def filter_by_substring(strings: List[str], substring: str) -> List[str]:
    """Filter strings that contain substring
    >>> filter_by_substring([], 'a')
    []
    >>> filter_by_substring(['abc', 'bacd', 'cde', 'array'], 'a')
    ['abc', 'bacd', 'array']
    """
    return [s for s in strings if s == substring]
"#;
        let fails = run_python_spec_oracle(wrong, spec);
        assert!(!fails.is_empty(), "expected doctest failures, got {fails:?}");
    }

    #[test]
    fn roundtrip_catches_bad_decode() {
        let spec = "def encode_cyclic(s):\n    return s\n\ndef decode_cyclic(s):\n    pass\n";
        let code = r#"
def encode_cyclic(s):
    groups = [s[(3 * i):min((3 * i + 3), len(s))] for i in range((len(s) + 2) // 3)]
    groups = [(group[1:] + group[0]) if len(group) == 3 else group for group in groups]
    return "".join(groups)

def decode_cyclic(s):
    return s
"#;
        let fails = run_python_spec_oracle(code, spec);
        assert!(
            fails.iter().any(|f| f.kind == "roundtrip"),
            "expected roundtrip failure, got {fails:?}"
        );
    }
}