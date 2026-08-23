//! AST Resolver & Sequential Dependency Injection module.
//! Constructs dependency DAGs and topological sorts code blocks using petgraph.

use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeBlock {
    pub function_name: String,
    pub code: String,
    pub line_count: usize,
}

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub graph: DiGraph<CodeBlock, ()>,
    pub node_indices: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    pub fn topological_sort(&self) -> Vec<CodeBlock> {
        match toposort(&self.graph, None) {
            Ok(sorted_indices) => sorted_indices
                .into_iter()
                .filter_map(|idx| self.graph.node_weight(idx).cloned())
                .collect(),
            Err(_) => {
                // If a cycle is detected, preserve initial order as fallback
                self.graph.raw_nodes().iter().map(|n| n.weight.clone()).collect()
            }
        }
    }

    pub fn has_cycle(&self) -> bool {
        is_cyclic_directed(&self.graph)
    }
}

pub struct AstResolver {
    fn_def_re: Regex,
    fn_call_re: Regex,
}

impl AstResolver {
    pub fn new() -> Self {
        Self {
            // Matches Rust, Python, and JS function/method definitions
            fn_def_re: Regex::new(r"(?m)(?:(?:pub\s+)?(?:async\s+)?fn\s+([a-zA-Z_0-9]+)|def\s+([a-zA-Z_0-9]+)|function\s+([a-zA-Z_0-9]+))").unwrap(),
            // Matches candidate identifiers/calls
            fn_call_re: Regex::new(r"\b([a-zA-Z_0-9]+)\s*\(").unwrap(),
        }
    }

    /// Resolves code blocks into a causal dependency graph
    pub fn resolve(&self, code_blocks: &[CodeBlock]) -> (DependencyGraph, bool) {
        let mut graph = DiGraph::new();
        let mut node_indices = HashMap::new();

        // 1. Add nodes
        for block in code_blocks {
            let idx = graph.add_node(block.clone());
            node_indices.insert(block.function_name.clone(), idx);
        }

        // 2. Extract dependencies and add causal edges (Dependency -> Dependent)
        for block in code_blocks {
            if let Some(&caller_idx) = node_indices.get(&block.function_name) {
                let called_functions = self.extract_calls(&block.code);
                for callee in called_functions {
                    if callee != block.function_name {
                        if let Some(&callee_idx) = node_indices.get(&callee) {
                            // Callee must execute before Caller: callee -> caller
                            graph.add_edge(callee_idx, caller_idx, ());
                        }
                    }
                }
            }
        }

        let is_valid = !is_cyclic_directed(&graph);
        (DependencyGraph { graph, node_indices }, is_valid)
    }

    /// Extracts function blocks from a monolithic source string
    pub fn split_into_blocks<'a>(&self, source: &'a str) -> Vec<CodeBlock> {
        let mut blocks = Vec::new();
        let mut current_name = "main".to_string();
        let mut current_lines: Vec<&'a str> = Vec::new();

        for line in source.lines() {
            if let Some(caps) = self.fn_def_re.captures(line) {
                if current_lines.iter().any(|l| !l.trim().is_empty()) {
                    let code = current_lines.join("\n");
                    let count = current_lines.len();
                    blocks.push(CodeBlock {
                        function_name: current_name.clone(),
                        code,
                        line_count: count,
                    });
                    current_lines.clear();
                }

                current_name = caps
                    .get(1)
                    .or_else(|| caps.get(2))
                    .or_else(|| caps.get(3))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "anonymous".to_string());
            }
            current_lines.push(line);
        }

        if current_lines.iter().any(|l| !l.trim().is_empty()) {
            let code = current_lines.join("\n");
            let count = current_lines.len();
            blocks.push(CodeBlock {
                function_name: current_name,
                code,
                line_count: count,
            });
        }

        blocks
    }

    fn extract_calls(&self, code: &str) -> Vec<String> {
        let mut calls = Vec::new();
        for caps in self.fn_call_re.captures_iter(code) {
            if let Some(m) = caps.get(1) {
                calls.push(m.as_str().to_string());
            }
        }
        calls
    }
}
