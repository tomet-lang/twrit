//! `python-api` guard and documentation generator.
//!
//! Extracts Tomet API documentation (`@kind(api)`) from Python files using `rustpython-parser`,
//! inspecting modules, classes, methods, functions, and their docstrings.
//!
//! Enforces that generated `.tmt` files in the repository match current Python sources and docstrings.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rustpython_ast::{Arguments, Expr, ExprConstant, Mod, Stmt};
use rustpython_parser::{Mode, parse};
use tomet_ast::{
    Block, Document, ElementValue, Inline, Paragraph, Placement, Section, Sigil, Span, Text, Value,
};
use tomet_tree::{ElementExt, element_new};
use twrit::writ::Rule;
use twrit::{Outcome, plural};

/// Options controlling extraction of Python code documentation.
#[derive(Debug, Clone, Default)]
pub struct PythonExtractOptions {
    /// Include private items (names starting with `_`).
    pub private: bool,
}

/// A parsed representation of a Python module.
#[derive(Debug, Clone)]
pub struct PythonModuleSource {
    pub module_name: String,
    pub file_path: PathBuf,
    pub source_text: String,
}

/// Configuration for `python-api` read from a Rule.
#[derive(Debug, Clone)]
pub struct PythonApiConfig {
    pub out_dir: PathBuf,
    pub packages: Vec<String>,
    pub private: bool,
}

impl PythonApiConfig {
    pub fn read(rule: &Rule, root: &Path) -> Result<Self> {
        let entries = rule.map("api").with_context(|| {
            format!(
                "{}: `@rule({})` guards `python-api` but declares no `api:` parameter",
                rule.path.display(),
                rule.id
            )
        })?;

        let mut out_str = None;
        let mut packages = Vec::new();
        let mut private = false;

        for (key, val) in entries {
            match key.as_str() {
                "out" => {
                    if let Value::String(s) = val {
                        out_str = Some(s.clone());
                    } else {
                        anyhow::bail!("`api.out` must be a string");
                    }
                }
                "packages" => {
                    if let Some(list) = twrit::writ::as_list(val) {
                        for item in list {
                            if let Value::String(s) = item {
                                packages.push(s.clone());
                            }
                        }
                    } else {
                        anyhow::bail!("`api.packages` must be a list of strings");
                    }
                }
                "private" => {
                    if let Value::String(s) = val {
                        private = s == "true";
                    }
                }
                _ => {}
            }
        }

        let out_dir = match out_str {
            Some(dir) => root.join(dir),
            None => root.join("docs/api"),
        };

        Ok(PythonApiConfig {
            out_dir,
            packages,
            private,
        })
    }
}

/// Extracts docstring from body of module, class, or function.
fn extract_docstring(body: &[Stmt]) -> Option<String> {
    if let Some(Stmt::Expr(expr_stmt)) = body.first() {
        if let Expr::Constant(ExprConstant { value, .. }) = &*expr_stmt.value {
            if let rustpython_ast::Constant::Str(s) = value {
                return Some(s.clone());
            }
        }
    }
    None
}

/// Formats function arguments into a readable string like `(a, b=None, *args, **kwargs)`.
fn format_arguments(args: &Arguments) -> String {
    let mut parts = Vec::new();

    for arg in &args.posonlyargs {
        let mut s = arg.def.arg.to_string();
        if arg.default.is_some() {
            s.push_str("=...");
        }
        parts.push(s);
    }

    for arg in &args.args {
        let mut s = arg.def.arg.to_string();
        if arg.default.is_some() {
            s.push_str("=...");
        }
        parts.push(s);
    }

    if let Some(vararg) = &args.vararg {
        parts.push(format!("*{}", vararg.arg));
    }

    for arg in &args.kwonlyargs {
        let mut s = arg.def.arg.to_string();
        if arg.default.is_some() {
            s.push_str("=...");
        }
        parts.push(s);
    }

    if let Some(kwarg) = &args.kwarg {
        parts.push(format!("**{}", kwarg.arg));
    }

    format!("({})", parts.join(", "))
}

fn dummy() -> Span {
    Span::default()
}

fn text(s: &str) -> Inline {
    Inline::Text(Text {
        value: s.to_string(),
        span: dummy(),
    })
}

/// Parses docstring lines into paragraphs for Tomet AST.
fn parse_doc_to_blocks(doc: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut current_para = Vec::new();

    for line in doc.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !current_para.is_empty() {
                blocks.push(Block::Paragraph(Paragraph {
                    content: vec![text(&current_para.join(" "))],
                    span: dummy(),
                }));
                current_para.clear();
            }
        } else {
            current_para.push(trimmed.to_string());
        }
    }

    if !current_para.is_empty() {
        blocks.push(Block::Paragraph(Paragraph {
            content: vec![text(&current_para.join(" "))],
            span: dummy(),
        }));
    }

    blocks
}

/// Extracts documentation for a Python module into a Tomet [`Document`].
pub fn extract_module_doc(
    module_name: &str,
    source: &str,
    options: &PythonExtractOptions,
) -> Result<Document> {
    let ast = parse(source, Mode::Module, module_name)
        .map_err(|e| anyhow::anyhow!("failed to parse Python source for {module_name}: {e}"))?;

    let Mod::Module(mod_ast) = ast else {
        anyhow::bail!("expected Module AST for {module_name}");
    };

    let mut blocks = Vec::new();

    // 1. Header: @kind(api) and @meta{ module: "..." }
    let mut kind_el = element_new(Sigil::named("kind")).with_placement(Placement::Block);
    kind_el.args = Some(Value::String("api".to_string()));
    blocks.push(Block::Element(kind_el));

    let meta_pairs = vec![("module".to_string(), Value::String(module_name.to_string()))];
    let mut meta_el = element_new(Sigil::named("meta")).with_placement(Placement::Block);
    meta_el.value = Some(ElementValue::from_map(Value::Map(meta_pairs)));
    blocks.push(Block::Element(meta_el));

    // 2. Module section: =[ module_name ]
    blocks.push(Block::Section(Section::new(
        1,
        vec![text(module_name)],
        dummy(),
    )));

    // 3. Module docstring
    if let Some(doc) = extract_docstring(&mod_ast.body) {
        blocks.extend(parse_doc_to_blocks(&doc));
    }

    // 4. Classes and Functions
    for stmt in &mod_ast.body {
        match stmt {
            Stmt::ClassDef(cls) => {
                let name = cls.name.as_str();
                if !options.private && name.starts_with('_') {
                    continue;
                }
                blocks.push(Block::Section(Section::new(
                    2,
                    vec![text(&format!("class {name}"))],
                    dummy(),
                )));
                if let Some(doc) = extract_docstring(&cls.body) {
                    blocks.extend(parse_doc_to_blocks(&doc));
                }

                // Methods in class
                for item in &cls.body {
                    if let Stmt::FunctionDef(func) = item {
                        let fn_name = func.name.as_str();
                        if !options.private && fn_name.starts_with('_') && fn_name != "__init__" {
                            continue;
                        }
                        let sig = format!("def {fn_name}{}", format_arguments(&func.args));
                        blocks.push(Block::Section(Section::new(
                            3,
                            vec![text(&sig)],
                            dummy(),
                        )));
                        if let Some(doc) = extract_docstring(&func.body) {
                            blocks.extend(parse_doc_to_blocks(&doc));
                        }
                    }
                }
            }
            Stmt::FunctionDef(func) => {
                let name = func.name.as_str();
                if !options.private && name.starts_with('_') {
                    continue;
                }
                let sig = format!("def {name}{}", format_arguments(&func.args));
                blocks.push(Block::Section(Section::new(
                    2,
                    vec![text(&sig)],
                    dummy(),
                )));
                if let Some(doc) = extract_docstring(&func.body) {
                    blocks.extend(parse_doc_to_blocks(&doc));
                }
            }
            _ => {}
        }
    }

    Ok(Document::new(blocks, dummy()))
}

/// Formats a Tomet document into a formatted `.tmt` string.
pub fn format_document(doc: &Document) -> String {
    let printed = tomet_printer::document_to_tm(doc);
    tomet_formatter::format_source(&printed)
}

/// Discovers Python files in the repository according to configuration.
pub fn discover_python_modules(root: &Path, packages: &[String]) -> Result<Vec<PythonModuleSource>> {
    let mut modules = Vec::new();

    let search_roots: Vec<PathBuf> = if packages.is_empty() {
        vec![root.to_path_buf()]
    } else {
        packages.iter().map(|p| root.join(p)).collect()
    };

    for base in search_roots {
        if !base.exists() {
            continue;
        }
        for entry in ignore::WalkBuilder::new(&base).hidden(false).build() {
            let entry = entry.context("failed to traverse directory")?;
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            if entry.path().extension().is_none_or(|e| e != "py") {
                continue;
            }

            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap_or(path);
            let mut mod_parts = Vec::new();
            for comp in rel.iter() {
                let s = comp.to_string_lossy();
                if s.ends_with(".py") {
                    let base = s.trim_end_matches(".py");
                    if base != "__init__" {
                        mod_parts.push(base.to_string());
                    }
                } else {
                    mod_parts.push(s.to_string());
                }
            }

            let module_name = if mod_parts.is_empty() {
                "main".to_string()
            } else {
                mod_parts.join(".")
            };

            let source_text = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;

            modules.push(PythonModuleSource {
                module_name,
                file_path: path.to_path_buf(),
                source_text,
            });
        }
    }

    modules.sort_by(|a, b| a.module_name.cmp(&b.module_name));
    Ok(modules)
}

/// Generates API documentation files for all discovered Python modules.
pub fn generate(rule: &Rule, root: &Path) -> Result<usize> {
    let config = PythonApiConfig::read(rule, root)?;
    let modules = discover_python_modules(root, &config.packages)?;
    let options = PythonExtractOptions {
        private: config.private,
    };

    fs::create_dir_all(&config.out_dir)?;

    for module in &modules {
        let doc = extract_module_doc(&module.module_name, &module.source_text, &options)?;
        let formatted = format_document(&doc);
        let out_file = config.out_dir.join(format!("{}.tmt", module.module_name));
        fs::write(&out_file, formatted)?;
    }

    Ok(modules.len())
}

/// Guard check: ensures generated API documentation matches current Python sources.
pub fn check(rule: &Rule, root: &Path) -> Result<Outcome> {
    let config = PythonApiConfig::read(rule, root)?;
    let modules = discover_python_modules(root, &config.packages)?;
    let options = PythonExtractOptions {
        private: config.private,
    };

    let mut violations = Vec::new();
    let mut checked = 0;

    for module in &modules {
        checked += 1;
        let doc = extract_module_doc(&module.module_name, &module.source_text, &options)?;
        let formatted = format_document(&doc);
        let target_file = config.out_dir.join(format!("{}.tmt", module.module_name));

        match fs::read_to_string(&target_file) {
            Ok(existing) => {
                if existing != formatted {
                    let rel_target = target_file
                        .strip_prefix(root)
                        .unwrap_or(&target_file)
                        .display();
                    violations.push(format!("{rel_target} differs from source"));
                }
            }
            Err(_) => {
                let rel_target = target_file
                    .strip_prefix(root)
                    .unwrap_or(&target_file)
                    .display();
                violations.push(format!("{rel_target} does not exist"));
            }
        }
    }

    Ok(Outcome::checked(plural(checked, "module"), violations))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_module_doc() {
        let code = r#"
"""A sample calculation module."""

def add(x, y=10):
    """Adds two numbers."""
    return x + y

class Calculator:
    """A calculator class."""

    def __init__(self, initial=0):
        """Initializes calculator."""
        self.val = initial

    def compute(self, op, *args):
        """Performs calculation."""
        pass

    def _private_method(self):
        pass
"#;
        let doc = extract_module_doc("sample.calc", code, &PythonExtractOptions::default()).unwrap();
        let formatted = format_document(&doc);

        assert!(formatted.contains("@kind(api)"));
        assert!(formatted.contains("=[sample.calc]"));
        assert!(formatted.contains("A sample calculation module."));
        assert!(formatted.contains("==[def add(x, y=...)]"));
        assert!(formatted.contains("Adds two numbers."));
        assert!(formatted.contains("==[class Calculator]"));
        assert!(formatted.contains("A calculator class."));
        assert!(formatted.contains("===[def __init__(self, initial=...)]"));
        assert!(formatted.contains("===[def compute(self, op, *args)]"));
        assert!(!formatted.contains("_private_method"));
    }
}
