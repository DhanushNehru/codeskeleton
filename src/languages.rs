//! Per-language tree-sitter configurations.
//!
//! Each supported language defines which AST node types correspond to
//! classes, functions, imports, and calls. This data-driven approach
//! means adding a new language is config, not code.

use tree_sitter::Language;

/// A supported source language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedLanguage {
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Rust,
    Go,
    Java,
    C,
}

impl SupportedLanguage {
    /// Determine language from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "py" => Some(Self::Python),
            "js" | "jsx" => Some(Self::JavaScript),
            "ts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "rs" => Some(Self::Rust),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "c" | "h" => Some(Self::C),
            _ => None,
        }
    }

    /// Get the tree-sitter Language for this language.
    pub fn ts_language(&self) -> Language {
        match self {
            Self::Python => tree_sitter_python::language(),
            Self::JavaScript => tree_sitter_javascript::language(),
            Self::TypeScript => tree_sitter_typescript::language_typescript(),
            Self::Tsx => tree_sitter_typescript::language_tsx(),
            Self::Rust => tree_sitter_rust::language(),
            Self::Go => tree_sitter_go::language(),
            Self::Java => tree_sitter_java::language(),
            Self::C => tree_sitter_c::language(),
        }
    }

    /// Get the extraction specification for this language.
    pub fn spec(&self) -> LanguageSpec {
        match self {
            Self::Python => PYTHON_SPEC,
            Self::JavaScript | Self::Tsx => JS_SPEC,
            Self::TypeScript => TS_SPEC,
            Self::Rust => RUST_SPEC,
            Self::Go => GO_SPEC,
            Self::Java => JAVA_SPEC,
            Self::C => C_SPEC,
        }
    }

    /// Human-readable language name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Python => "Python",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Tsx => "TSX",
            Self::Rust => "Rust",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::C => "C",
        }
    }
}

/// Extraction rules for a language — which AST node types to look for.
#[derive(Debug, Clone, Copy)]
pub struct LanguageSpec {
    /// Node types that represent classes/structs/traits.
    pub class_types: &'static [&'static str],
    /// Node types that represent functions/methods.
    pub function_types: &'static [&'static str],
    /// Node types that represent import statements.
    pub import_types: &'static [&'static str],
    /// Node types that represent call expressions.
    pub call_types: &'static [&'static str],
    /// Field name used to extract the name of a definition.
    pub name_field: &'static str,
    /// Field name for function body.
    pub body_field: &'static str,
    /// Fallback child types to find a name when name_field is missing.
    pub name_fallback_types: &'static [&'static str],
    /// Fallback child types to find a body when body_field is missing.
    pub body_fallback_types: &'static [&'static str],
    /// Field name on call nodes for the callee.
    pub call_function_field: &'static str,
    /// Whether to add "()" suffix to function labels.
    pub function_label_parens: bool,
}

// ──────────────────────────────────────────────────────────────────────────────
// Language specifications
// ──────────────────────────────────────────────────────────────────────────────

const PYTHON_SPEC: LanguageSpec = LanguageSpec {
    class_types: &["class_definition"],
    function_types: &["function_definition"],
    import_types: &["import_statement", "import_from_statement"],
    call_types: &["call"],
    name_field: "name",
    body_field: "body",
    name_fallback_types: &[],
    body_fallback_types: &[],
    call_function_field: "function",
    function_label_parens: true,
};

const JS_SPEC: LanguageSpec = LanguageSpec {
    class_types: &["class_declaration"],
    function_types: &["function_declaration", "method_definition"],
    import_types: &["import_statement"],
    call_types: &["call_expression"],
    name_field: "name",
    body_field: "body",
    name_fallback_types: &[],
    body_fallback_types: &[],
    call_function_field: "function",
    function_label_parens: true,
};

const TS_SPEC: LanguageSpec = LanguageSpec {
    class_types: &["class_declaration"],
    function_types: &["function_declaration", "method_definition"],
    import_types: &["import_statement"],
    call_types: &["call_expression"],
    name_field: "name",
    body_field: "body",
    name_fallback_types: &[],
    body_fallback_types: &[],
    call_function_field: "function",
    function_label_parens: true,
};

const RUST_SPEC: LanguageSpec = LanguageSpec {
    class_types: &["struct_item", "enum_item", "trait_item"],
    function_types: &["function_item"],
    import_types: &["use_declaration"],
    call_types: &["call_expression"],
    name_field: "name",
    body_field: "body",
    name_fallback_types: &["identifier", "type_identifier"],
    body_fallback_types: &["field_declaration_list", "declaration_list", "block"],
    call_function_field: "function",
    function_label_parens: true,
};

const GO_SPEC: LanguageSpec = LanguageSpec {
    class_types: &["type_declaration"],
    function_types: &["function_declaration", "method_declaration"],
    import_types: &["import_declaration"],
    call_types: &["call_expression"],
    name_field: "name",
    body_field: "body",
    name_fallback_types: &["type_identifier"],
    body_fallback_types: &["block"],
    call_function_field: "function",
    function_label_parens: true,
};

const JAVA_SPEC: LanguageSpec = LanguageSpec {
    class_types: &["class_declaration", "interface_declaration"],
    function_types: &["method_declaration", "constructor_declaration"],
    import_types: &["import_declaration"],
    call_types: &["method_invocation"],
    name_field: "name",
    body_field: "body",
    name_fallback_types: &["identifier"],
    body_fallback_types: &["constructor_body", "block"],
    call_function_field: "name",
    function_label_parens: true,
};

const C_SPEC: LanguageSpec = LanguageSpec {
    class_types: &["struct_specifier"],
    function_types: &["function_definition"],
    import_types: &["preproc_include"],
    call_types: &["call_expression"],
    name_field: "name",
    body_field: "body",
    name_fallback_types: &["identifier"],
    body_fallback_types: &["compound_statement"],
    call_function_field: "function",
    function_label_parens: true,
};
