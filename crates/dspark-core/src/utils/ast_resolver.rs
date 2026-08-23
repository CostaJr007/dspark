//! Abstract AST Resolver with pluggable implementations (Regex & Tree-sitter).
//! Constructs dependency DAGs and topological sorts code blocks using petgraph.

use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CodeBlock {
    pub function_name: String,
    pub code: String,
    pub line_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    pub graph: DiGraph<CodeBlock, ()>,
    pub node_indices: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, block: CodeBlock) -> NodeIndex {
        let name = block.function_name.clone();
        if let Some(&idx) = self.node_indices.get(&name) {
            idx
        } else {
            let idx = self.graph.add_node(block);
            self.node_indices.insert(name, idx);
            idx
        }
    }

    pub fn add_edge(&mut self, from_callee: &str, to_caller: &str) {
        if let (Some(&from_idx), Some(&to_idx)) = (
            self.node_indices.get(from_callee),
            self.node_indices.get(to_caller),
        ) {
            if from_idx != to_idx {
                self.graph.add_edge(from_idx, to_idx, ());
            }
        }
    }

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

/// Abstract trait for dependency resolution
pub trait DependencyResolver: Send + Sync {
    /// Resolves code blocks into a causal dependency graph
    fn resolve(&self, code_blocks: &[CodeBlock], language: &str) -> (DependencyGraph, bool);
    /// Extracts function blocks from a monolithic source string
    fn split_into_blocks(&self, source: &str) -> Vec<CodeBlock>;
    /// Returns the resolver backend name
    fn name(&self) -> &'static str;
}

/// Factory function deciding backend based on compilation feature
pub fn create_resolver() -> Box<dyn DependencyResolver> {
    #[cfg(feature = "tree-sitter-ast")]
    {
        Box::new(TreeSitterResolver::new())
    }
    #[cfg(not(feature = "tree-sitter-ast"))]
    {
        Box::new(RegexResolver::new())
    }
}

// ===========================================
// Implementation 1: Regex (Fast, zero-C-dependency)
// ===========================================
pub struct RegexResolver {
    fn_def_re: Regex,
    fn_call_re: Regex,
}

pub type AstResolver = RegexResolver;

impl RegexResolver {
    pub fn new() -> Self {
        Self {
            // Matches Rust, Python, and JS function/method definitions
            fn_def_re: Regex::new(r"(?m)(?:(?:pub\s+)?(?:async\s+)?fn\s+([a-zA-Z_0-9]+)|def\s+([a-zA-Z_0-9]+)|function\s+([a-zA-Z_0-9]+))").unwrap(),
            // Matches candidate identifiers/calls
            fn_call_re: Regex::new(r"\b([a-zA-Z_0-9]+)\s*\(").unwrap(),
        }
    }

    pub fn resolve(&self, code_blocks: &[CodeBlock], language: &str) -> (DependencyGraph, bool) {
        DependencyResolver::resolve(self, code_blocks, language)
    }

    pub fn split_into_blocks(&self, source: &str) -> Vec<CodeBlock> {
        DependencyResolver::split_into_blocks(self, source)
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

impl Default for RegexResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyResolver for RegexResolver {
    fn name(&self) -> &'static str {
        "regex"
    }

    fn resolve(&self, code_blocks: &[CodeBlock], _language: &str) -> (DependencyGraph, bool) {
        let mut graph = DependencyGraph::new();

        for block in code_blocks {
            graph.add_node(block.clone());
        }

        for block in code_blocks {
            let called_functions = self.extract_calls(&block.code);
            for callee in called_functions {
                if callee != block.function_name && graph.node_indices.contains_key(&callee) {
                    // Callee must execute before Caller: callee -> caller
                    graph.add_edge(&callee, &block.function_name);
                }
            }
        }

        let is_valid = !graph.has_cycle();
        (graph, is_valid)
    }

    fn split_into_blocks(&self, source: &str) -> Vec<CodeBlock> {
        let mut blocks = Vec::new();
        let mut current_name = "main".to_string();
        let mut current_lines: Vec<&str> = Vec::new();

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
}

// ===========================================
// Implementation 2: Tree-Sitter (Precise AST)
// ===========================================
#[cfg(feature = "tree-sitter-ast")]
pub struct TreeSitterResolver;

#[cfg(feature = "tree-sitter-ast")]
impl TreeSitterResolver {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "tree-sitter-ast")]
impl Default for TreeSitterResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "tree-sitter-ast")]
impl DependencyResolver for TreeSitterResolver {
    fn name(&self) -> &'static str {
        "tree-sitter"
    }

    fn resolve(&self, code_blocks: &[CodeBlock], language: &str) -> (DependencyGraph, bool) {
        let mut graph = DependencyGraph::new();
        for block in code_blocks {
            graph.add_node(block.clone());
        }

        let mut parser = tree_sitter::Parser::new();
        if language == "python" {
            let _ = parser.set_language(&tree_sitter_python::LANGUAGE.into());
        } else {
            let _ = parser.set_language(&tree_sitter_rust::LANGUAGE.into());
        }

        for block in code_blocks {
            if let Some(tree) = parser.parse(&block.code, None) {
                let mut cursor = tree.walk();
                let mut stack = vec![tree.root_node()];

                while let Some(node) = stack.pop() {
                    if node.kind() == "call_expression" || node.kind() == "call" {
                        if let Some(fn_node) = node.child_by_field_name("function") {
                            if let Ok(name) = fn_node.utf8_text(block.code.as_bytes()) {
                                let simple_name = name.split("::").last().unwrap_or(name);
                                if simple_name != block.function_name && graph.node_indices.contains_key(simple_name) {
                                    graph.add_edge(simple_name, &block.function_name);
                                }
                            }
                        }
                    }

                    for child in node.children(&mut cursor) {
                        stack.push(child);
                    }
                }
            }
        }

        let is_valid = !graph.has_cycle();
        (graph, is_valid)
    }

    fn split_into_blocks(&self, source: &str) -> Vec<CodeBlock> {
        // Fallback to line scanner for splitting monolithic sources
        RegexResolver::new().split_into_blocks(source)
    }
}
