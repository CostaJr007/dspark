use dspark::utils::ast_resolver::{create_resolver, CodeBlock, DependencyResolver};

fn make_block(name: &str, code: &str) -> CodeBlock {
    CodeBlock {
        function_name: name.to_string(),
        code: code.to_string(),
        line_count: 1,
    }
}

#[test]
fn test_detects_simple_dependency() {
    let resolver = create_resolver();

    let blocks = vec![
        make_block("helper", "fn helper(x: i32) -> i32 { x * 2 }"),
        make_block("main", "fn main() { let y = helper(5); }"),
    ];

    let (graph, is_valid) = resolver.resolve(&blocks, "rust");

    assert!(is_valid);
    assert!(!graph.has_cycle());

    let sorted = graph.topological_sort();
    assert_eq!(sorted.len(), 2);
    assert_eq!(sorted[0].function_name, "helper");
    assert_eq!(sorted[1].function_name, "main");
}

#[test]
fn test_detects_cycle() {
    let resolver = create_resolver();

    let blocks = vec![
        make_block("a", "fn a() { b(); }"),
        make_block("b", "fn b() { a(); }"), // Cycle!
    ];

    let (_, is_valid) = resolver.resolve(&blocks, "rust");
    assert!(!is_valid); // Cycle detected
}

#[test]
fn test_python_support() {
    let resolver = create_resolver();

    let blocks = vec![
        make_block("helper", "def helper(x): return x * 2"),
        make_block("main", "def main(): y = helper(5)"),
    ];

    let (graph, is_valid) = resolver.resolve(&blocks, "python");
    assert!(is_valid);
    let sorted = graph.topological_sort();
    assert_eq!(sorted.len(), 2);
    assert_eq!(sorted[0].function_name, "helper");
    assert_eq!(sorted[1].function_name, "main");
}
