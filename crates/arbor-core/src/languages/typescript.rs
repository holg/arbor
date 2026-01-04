//! TypeScript/JavaScript parser implementation.
//!
//! This handles TS, TSX, JS, and JSX files. Tree-sitter's TypeScript
//! grammar is comprehensive enough to handle most JS patterns too.

use crate::languages::LanguageParser;
use crate::node::{CodeNode, NodeKind, Visibility};
use tree_sitter::{Language, Node, Tree};

pub struct TypeScriptParser;

impl LanguageParser for TypeScriptParser {
    fn language(&self) -> Language {
        tree_sitter_typescript::language_typescript()
    }

    fn extensions(&self) -> &[&str] {
        &["ts", "tsx", "js", "jsx", "mts", "cts", "mjs", "cjs"]
    }

    fn extract_nodes(&self, tree: &Tree, source: &str, file_path: &str) -> Vec<CodeNode> {
        let mut nodes = Vec::new();
        let root = tree.root_node();

        // We'll do a recursive traversal to find interesting nodes
        extract_from_node(&root, source, file_path, &mut nodes, None);

        nodes
    }
}

/// Recursively extracts nodes from the AST.
fn extract_from_node(
    node: &Node,
    source: &str,
    file_path: &str,
    nodes: &mut Vec<CodeNode>,
    parent_name: Option<&str>,
) {
    let kind = node.kind();

    match kind {
        // Functions
        "function_declaration" | "function" => {
            if let Some(code_node) = extract_function(node, source, file_path, parent_name) {
                nodes.push(code_node);
            }
        }

        // Arrow functions assigned to variables
        "lexical_declaration" | "variable_declaration" => {
            if let Some(code_node) = extract_arrow_function(node, source, file_path) {
                nodes.push(code_node);
            }
        }

        // Classes
        "class_declaration" | "class" => {
            if let Some(code_node) = extract_class(node, source, file_path) {
                let class_name = code_node.name.clone();
                nodes.push(code_node);

                // Extract methods within the class
                if let Some(body) = node.child_by_field_name("body") {
                    for i in 0..body.child_count() {
                        if let Some(child) = body.child(i) {
                            extract_from_node(&child, source, file_path, nodes, Some(&class_name));
                        }
                    }
                }
                return; // Don't recurse again, we handled children
            }
        }

        // Methods inside classes
        "method_definition" => {
            if let Some(code_node) = extract_method(node, source, file_path, parent_name) {
                nodes.push(code_node);
            }
        }

        // Interfaces
        "interface_declaration" => {
            if let Some(code_node) = extract_interface(node, source, file_path) {
                nodes.push(code_node);
            }
        }

        // Type aliases
        "type_alias_declaration" => {
            if let Some(code_node) = extract_type_alias(node, source, file_path) {
                nodes.push(code_node);
            }
        }

        // Import statements
        "import_statement" => {
            if let Some(code_node) = extract_import(node, source, file_path) {
                nodes.push(code_node);
            }
        }

        // Export statements (named exports, default exports)
        "export_statement" => {
            // The export might wrap a function or class, extract those
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    let child_kind = child.kind();
                    if matches!(
                        child_kind,
                        "function_declaration" | "class_declaration" | "lexical_declaration"
                    ) {
                        extract_from_node(&child, source, file_path, nodes, parent_name);
                    }
                }
            }
        }

        _ => {}
    }

    // Recurse into children for most node types
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_from_node(&child, source, file_path, nodes, parent_name);
        }
    }
}

/// Extracts a function declaration.
fn extract_function(
    node: &Node,
    source: &str,
    file_path: &str,
    parent_name: Option<&str>,
) -> Option<CodeNode> {
    let name_node = node.child_by_field_name("name")?;
    let name = get_text(&name_node, source);

    let qualified_name = match parent_name {
        Some(parent) => format!("{}.{}", parent, name),
        None => name.clone(),
    };

    let kind = if parent_name.is_some() {
        NodeKind::Method
    } else {
        NodeKind::Function
    };

    // Check for async keyword
    let is_async = has_modifier(node, source, "async");

    // Check for export
    let is_exported = is_node_exported(node);

    // Build signature
    let signature = build_function_signature(node, source);

    // Extract references (function calls within the body)
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
            .with_visibility(if is_exported {
                Visibility::Public
            } else {
                Visibility::Private
            })
            .with_references(references)
            .as_async_if(is_async)
            .as_exported_if(is_exported),
    )
}

/// Extracts arrow functions assigned to const/let.
fn extract_arrow_function(node: &Node, source: &str, file_path: &str) -> Option<CodeNode> {
    // Look for pattern: const foo = () => {} or const foo = async () => {}
    for i in 0..node.child_count() {
        if let Some(declarator) = node.child(i) {
            if declarator.kind() == "variable_declarator" {
                let name_node = declarator.child_by_field_name("name")?;
                let value_node = declarator.child_by_field_name("value")?;

                if value_node.kind() == "arrow_function" {
                    let name = get_text(&name_node, source);
                    let is_async = has_modifier(&value_node, source, "async");
                    let is_exported = is_node_exported(node);

                    let signature = build_arrow_signature(&value_node, source, &name);
                    let references = extract_call_references(&value_node, source);

                    return Some(
                        CodeNode::new(&name, &name, NodeKind::Function, file_path)
                            .with_lines(
                                node.start_position().row as u32 + 1,
                                node.end_position().row as u32 + 1,
                            )
                            .with_bytes(node.start_byte() as u32, node.end_byte() as u32)
                            .with_column(name_node.start_position().column as u32)
                            .with_signature(signature)
                            .with_references(references)
                            .as_async_if(is_async)
                            .as_exported_if(is_exported),
                    );
                }
            }
        }
    }
    None
}

/// Extracts a class declaration.
fn extract_class(node: &Node, source: &str, file_path: &str) -> Option<CodeNode> {
    let name_node = node.child_by_field_name("name")?;
    let name = get_text(&name_node, source);
    let is_exported = is_node_exported(node);

    Some(
        CodeNode::new(&name, &name, NodeKind::Class, file_path)
            .with_lines(
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
            )
            .with_bytes(node.start_byte() as u32, node.end_byte() as u32)
            .with_column(name_node.start_position().column as u32)
            .with_visibility(if is_exported {
                Visibility::Public
            } else {
                Visibility::Private
            })
            .as_exported_if(is_exported),
    )
}

/// Extracts a method within a class.
fn extract_method(
    node: &Node,
    source: &str,
    file_path: &str,
    parent_name: Option<&str>,
) -> Option<CodeNode> {
    let name_node = node.child_by_field_name("name")?;
    let name = get_text(&name_node, source);

    let qualified_name = match parent_name {
        Some(parent) => format!("{}.{}", parent, name),
        None => name.clone(),
    };

    let is_async = has_modifier(node, source, "async");
    let is_static = has_modifier(node, source, "static");
    let signature = build_function_signature(node, source);
    let references = extract_call_references(node, source);

    // Check visibility modifiers
    let visibility = detect_visibility(node, source);

    Some(
        CodeNode::new(&name, &qualified_name, NodeKind::Method, file_path)
            .with_lines(
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
            )
            .with_bytes(node.start_byte() as u32, node.end_byte() as u32)
            .with_column(name_node.start_position().column as u32)
            .with_signature(signature)
            .with_visibility(visibility)
            .with_references(references)
            .as_async_if(is_async)
            .as_static_if(is_static),
    )
}

/// Extracts an interface declaration.
fn extract_interface(node: &Node, source: &str, file_path: &str) -> Option<CodeNode> {
    let name_node = node.child_by_field_name("name")?;
    let name = get_text(&name_node, source);
    let is_exported = is_node_exported(node);

    Some(
        CodeNode::new(&name, &name, NodeKind::Interface, file_path)
            .with_lines(
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
            )
            .with_bytes(node.start_byte() as u32, node.end_byte() as u32)
            .with_column(name_node.start_position().column as u32)
            .with_visibility(if is_exported {
                Visibility::Public
            } else {
                Visibility::Private
            })
            .as_exported_if(is_exported),
    )
}

/// Extracts a type alias.
fn extract_type_alias(node: &Node, source: &str, file_path: &str) -> Option<CodeNode> {
    let name_node = node.child_by_field_name("name")?;
    let name = get_text(&name_node, source);
    let is_exported = is_node_exported(node);

    Some(
        CodeNode::new(&name, &name, NodeKind::TypeAlias, file_path)
            .with_lines(
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
            )
            .with_bytes(node.start_byte() as u32, node.end_byte() as u32)
            .with_column(name_node.start_position().column as u32)
            .as_exported_if(is_exported),
    )
}

/// Extracts an import statement.
fn extract_import(node: &Node, source: &str, file_path: &str) -> Option<CodeNode> {
    // Get the import source (the "from 'module'" part)
    let source_node = node.child_by_field_name("source")?;
    let module_path = get_text(&source_node, source);

    // Clean up quotes
    let module_path = module_path.trim_matches(|c| c == '"' || c == '\'');

    Some(
        CodeNode::new(module_path, module_path, NodeKind::Import, file_path)
            .with_lines(
                node.start_position().row as u32 + 1,
                node.end_position().row as u32 + 1,
            )
            .with_bytes(node.start_byte() as u32, node.end_byte() as u32),
    )
}

// ============================================================================
// Helper functions
// ============================================================================

/// Gets text content of a node.
fn get_text(node: &Node, source: &str) -> String {
    source[node.byte_range()].to_string()
}

/// Checks if a node has a specific modifier keyword.
fn has_modifier(node: &Node, source: &str, modifier: &str) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            let text = get_text(&child, source);
            if text == modifier {
                return true;
            }
        }
    }
    false
}

/// Checks if a node is exported (wrapped in export_statement).
fn is_node_exported(node: &Node) -> bool {
    if let Some(parent) = node.parent() {
        return parent.kind() == "export_statement";
    }
    false
}

/// Detects visibility from TypeScript/ES2022 modifiers.
fn detect_visibility(node: &Node, source: &str) -> Visibility {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            let text = get_text(&child, source);
            match text.as_str() {
                "public" => return Visibility::Public,
                "private" => return Visibility::Private,
                "protected" => return Visibility::Protected,
                _ => {}
            }
        }
    }
    Visibility::Public // Default for class members
}

/// Builds a function signature string.
fn build_function_signature(node: &Node, source: &str) -> String {
    // Try to extract name, params, and return type
    let name = node
        .child_by_field_name("name")
        .map(|n| get_text(&n, source))
        .unwrap_or_default();

    let params = node
        .child_by_field_name("parameters")
        .map(|n| get_text(&n, source))
        .unwrap_or_else(|| "()".to_string());

    let return_type = node
        .child_by_field_name("return_type")
        .map(|n| get_text(&n, source))
        .unwrap_or_default();

    if return_type.is_empty() {
        format!("{}{}", name, params)
    } else {
        format!("{}{}{}", name, params, return_type)
    }
}

/// Builds an arrow function signature.
fn build_arrow_signature(node: &Node, source: &str, name: &str) -> String {
    let params = node
        .child_by_field_name("parameters")
        .or_else(|| node.child_by_field_name("parameter"))
        .map(|n| get_text(&n, source))
        .unwrap_or_else(|| "()".to_string());

    format!("{}{}", name, params)
}

/// Extracts function call references from a node's body.
fn extract_call_references(node: &Node, source: &str) -> Vec<String> {
    let mut refs = Vec::new();
    collect_calls(node, source, &mut refs);
    refs.sort();
    refs.dedup();
    refs
}

/// Recursively collects function call names.
fn collect_calls(node: &Node, source: &str, refs: &mut Vec<String>) {
    if node.kind() == "call_expression" {
        // Get the function being called
        if let Some(func_node) = node.child_by_field_name("function") {
            let call_name = get_text(&func_node, source);
            // Skip common built-ins and method chains on objects
            if !call_name.contains('.') || call_name.starts_with("this.") {
                refs.push(call_name);
            } else if let Some(parts) = call_name.split('.').last() {
                // For chains like foo.bar.baz(), we capture 'baz'
                refs.push(parts.to_string());
            }
        }
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_calls(&child, source, refs);
        }
    }
}

// Builder pattern helpers as a trait extension
trait CodeNodeExt {
    fn as_async_if(self, cond: bool) -> Self;
    fn as_static_if(self, cond: bool) -> Self;
    fn as_exported_if(self, cond: bool) -> Self;
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

    fn as_exported_if(self, cond: bool) -> Self {
        if cond {
            self.as_exported()
        } else {
            self
        }
    }
}
