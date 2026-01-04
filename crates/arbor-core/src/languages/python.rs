//! Python language parser implementation.
//!
//! Handles .py and .pyi files. Python's AST is relatively
//! straightforward with clear function and class boundaries.

use crate::languages::LanguageParser;
use crate::node::{CodeNode, NodeKind, Visibility};
use tree_sitter::{Language, Node, Tree};

pub struct PythonParser;

impl LanguageParser for PythonParser {
    fn language(&self) -> Language {
        tree_sitter_python::language()
    }

    fn extensions(&self) -> &[&str] {
        &["py", "pyi"]
    }

    fn extract_nodes(&self, tree: &Tree, source: &str, file_path: &str) -> Vec<CodeNode> {
        let mut nodes = Vec::new();
        let root = tree.root_node();

        extract_from_node(&root, source, file_path, &mut nodes, None);

        nodes
    }
}

/// Recursively extracts nodes from the Python AST.
fn extract_from_node(
    node: &Node,
    source: &str,
    file_path: &str,
    nodes: &mut Vec<CodeNode>,
    class_name: Option<&str>,
) {
    let kind = node.kind();

    match kind {
        // Function definitions
        "function_definition" => {
            if let Some(code_node) = extract_function(node, source, file_path, class_name) {
                nodes.push(code_node);
            }
        }

        // Class definitions
        "class_definition" => {
            if let Some(code_node) = extract_class(node, source, file_path) {
                let name = code_node.name.clone();
                nodes.push(code_node);

                // Extract methods within the class
                if let Some(body) = node.child_by_field_name("body") {
                    for i in 0..body.child_count() {
                        if let Some(child) = body.child(i) {
                            extract_from_node(&child, source, file_path, nodes, Some(&name));
                        }
                    }
                }
                return; // Already handled children
            }
        }

        // Import statements
        "import_statement" => {
            if let Some(code_node) = extract_import(node, source, file_path) {
                nodes.push(code_node);
            }
        }

        // From imports
        "import_from_statement" => {
            if let Some(code_node) = extract_from_import(node, source, file_path) {
                nodes.push(code_node);
            }
        }

        // Module-level assignments (could be constants)
        "expression_statement" if class_name.is_none() => {
            // Check if it's a simple assignment at module level
            if let Some(assign) = find_child_by_kind(node, "assignment") {
                if let Some(code_node) = extract_assignment(assign, source, file_path) {
                    nodes.push(code_node);
                }
            }
        }

        _ => {}
    }

    // Recurse into children (but not for classes, handled above)
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_from_node(&child, source, file_path, nodes, class_name);
        }
    }
}

/// Extracts a function or method definition.
fn extract_function(
    node: &Node,
    source: &str,
    file_path: &str,
    class_name: Option<&str>,
) -> Option<CodeNode> {
    let name_node = node.child_by_field_name("name")?;
    let name = get_text(&name_node, source);

    let kind = if class_name.is_some() {
        NodeKind::Method
    } else {
        NodeKind::Function
    };

    let qualified_name = match class_name {
        Some(cls) => format!("{}.{}", cls, name),
        None => name.clone(),
    };

    // Python uses naming convention for visibility
    let visibility = python_visibility(&name);

    // Check for async def
    let is_async = has_async_keyword(node, source);

    // Check for @staticmethod or @classmethod
    let is_static =
        has_decorator(node, source, "staticmethod") || has_decorator(node, source, "classmethod");

    // Build signature
    let signature = build_function_signature(node, source, &name);

    // Get docstring
    let docstring = extract_docstring(node, source);

    // Extract references
    let references = extract_call_references(node, source);

    Some(
        CodeNode::new(&name, &qualified_name, kind, file_path)
            .with_lines(
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
            )
            .with_bytes(node.start_byte() as u32, node.end_byte() as u32)
            .with_column(name_node.start_position().column as u32)
            .with_signature(signature)
            .with_visibility(visibility)
            .with_references(references)
            .with_docstring_if(docstring)
            .as_async_if(is_async)
            .as_static_if(is_static),
    )
}

/// Extracts a class definition.
fn extract_class(node: &Node, source: &str, file_path: &str) -> Option<CodeNode> {
    let name_node = node.child_by_field_name("name")?;
    let name = get_text(&name_node, source);
    let visibility = python_visibility(&name);

    // Get docstring
    let docstring = extract_docstring(node, source);

    Some(
        CodeNode::new(&name, &name, NodeKind::Class, file_path)
            .with_lines(
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
            )
            .with_bytes(node.start_byte() as u32, node.end_byte() as u32)
            .with_column(name_node.start_position().column as u32)
            .with_visibility(visibility)
            .with_docstring_if(docstring),
    )
}

/// Extracts an import statement.
fn extract_import(node: &Node, source: &str, file_path: &str) -> Option<CodeNode> {
    let text = get_text(node, source);
    // Strip "import " prefix
    let module_name = text.strip_prefix("import ")?.trim();

    Some(
        CodeNode::new(module_name, module_name, NodeKind::Import, file_path)
            .with_lines(
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
            )
            .with_bytes(node.start_byte() as u32, node.end_byte() as u32),
    )
}

/// Extracts a from...import statement.
fn extract_from_import(node: &Node, source: &str, file_path: &str) -> Option<CodeNode> {
    // Get the module name being imported from
    if let Some(module) = node.child_by_field_name("module_name") {
        let module_name = get_text(&module, source);

        return Some(
            CodeNode::new(&module_name, &module_name, NodeKind::Import, file_path)
                .with_lines(
                    node.start_position().row as u32 + 1,
                    node.end_position().row as u32 + 1,
                )
                .with_bytes(node.start_byte() as u32, node.end_byte() as u32),
        );
    }
    None
}

/// Extracts a module-level assignment (potential constant).
fn extract_assignment(node: Node, source: &str, file_path: &str) -> Option<CodeNode> {
    let left = node.child_by_field_name("left")?;

    // Only handle simple identifiers, not destructuring
    if left.kind() != "identifier" {
        return None;
    }

    let name = get_text(&left, source);

    // Convention: UPPERCASE names are constants
    let kind = if name.chars().all(|c| c.is_uppercase() || c == '_') {
        NodeKind::Constant
    } else {
        NodeKind::Variable
    };

    Some(
        CodeNode::new(&name, &name, kind, file_path)
            .with_lines(
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
            )
            .with_bytes(node.start_byte() as u32, node.end_byte() as u32)
            .with_column(left.start_position().column as u32),
    )
}

// ============================================================================
// Helper functions
// ============================================================================

/// Gets text content of a node.
fn get_text(node: &Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

/// Finds a child node by its kind.
fn find_child_by_kind<'a>(node: &'a Node, kind: &str) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == kind {
                return Some(child);
            }
        }
    }
    None
}

/// Determines visibility from Python naming convention.
fn python_visibility(name: &str) -> Visibility {
    if name.starts_with("__") && !name.ends_with("__") {
        // Name mangled, effectively private
        Visibility::Private
    } else if name.starts_with('_') {
        // Convention: protected/internal
        Visibility::Protected
    } else {
        Visibility::Public
    }
}

/// Checks if function has async keyword.
fn has_async_keyword(node: &Node, source: &str) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if get_text(&child, source) == "async" {
                return true;
            }
        }
    }
    false
}

/// Checks if function has a specific decorator.
fn has_decorator(node: &Node, source: &str, decorator_name: &str) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "decorator" {
                let text = get_text(&child, source);
                if text.contains(decorator_name) {
                    return true;
                }
            }
        }
    }
    false
}

/// Builds a function signature.
fn build_function_signature(node: &Node, source: &str, name: &str) -> String {
    let params = node
        .child_by_field_name("parameters")
        .map(|n| get_text(&n, source))
        .unwrap_or_else(|| "()".to_string());

    let return_type = node
        .child_by_field_name("return_type")
        .map(|n| format!(" -> {}", get_text(&n, source)))
        .unwrap_or_default();

    format!("def {}{}{}", name, params, return_type)
}

/// Extracts docstring from a function or class.
fn extract_docstring(node: &Node, source: &str) -> Option<String> {
    // Docstring is the first expression statement in the body
    // that contains a string
    let body = node.child_by_field_name("body")?;

    for i in 0..body.child_count() {
        if let Some(child) = body.child(i) {
            if child.kind() == "expression_statement" {
                if let Some(string_node) = child.child(0) {
                    if string_node.kind() == "string" {
                        let text = get_text(&string_node, source);
                        // Strip quotes
                        let doc = text
                            .trim_start_matches("\"\"\"")
                            .trim_start_matches("'''")
                            .trim_end_matches("\"\"\"")
                            .trim_end_matches("'''")
                            .trim();
                        return Some(doc.to_string());
                    }
                }
            }
            // Only check the first statement
            break;
        }
    }
    None
}

/// Extracts function call references.
fn extract_call_references(node: &Node, source: &str) -> Vec<String> {
    let mut refs = Vec::new();
    collect_calls(node, source, &mut refs);
    refs.sort();
    refs.dedup();
    refs
}

/// Recursively collects function call names.
fn collect_calls(node: &Node, source: &str, refs: &mut Vec<String>) {
    if node.kind() == "call" {
        if let Some(func_node) = node.child_by_field_name("function") {
            let call_name = get_text(&func_node, source);
            refs.push(call_name);
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_calls(&child, source, refs);
        }
    }
}

// Builder pattern helpers
trait CodeNodeExt {
    fn as_async_if(self, cond: bool) -> Self;
    fn as_static_if(self, cond: bool) -> Self;
    fn with_docstring_if(self, docstring: Option<String>) -> Self;
}

impl CodeNodeExt for CodeNode {
    fn as_async_if(self, cond: bool) -> Self {
        if cond {
            self.as_async()
        } else {
            self
        }
    }

    fn as_static_if(self, cond: bool) -> Self {
        if cond {
            self.as_static()
        } else {
            self
        }
    }

    fn with_docstring_if(mut self, docstring: Option<String>) -> Self {
        self.docstring = docstring;
        self
    }
}
