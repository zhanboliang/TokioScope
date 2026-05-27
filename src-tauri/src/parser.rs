use crate::prefs::{DEFAULT_BLOCKING_SLOTS, DEFAULT_WORKER_THREADS};
use serde::{Deserialize, Serialize};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, ExprPath, Macro};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Flavor {
    CurrentThread,
    MultiThread,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub flavor: Flavor,
    pub worker_threads: u32,
    pub blocking_slots: u32,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            flavor: Flavor::MultiThread,
            worker_threads: DEFAULT_WORKER_THREADS,
            blocking_slots: DEFAULT_BLOCKING_SLOTS,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagLevel {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub level: DiagLevel,
    pub line: u32,
    pub col: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrimitiveHit {
    pub kind: &'static str,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParseReport {
    pub ok: bool,
    pub runtime: RuntimeConfig,
    pub diagnostics: Vec<Diagnostic>,
    pub primitives: Vec<PrimitiveHit>,
    pub functions: Vec<FunctionSig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionSig {
    pub name: String,
    pub line: u32,
    pub is_async: bool,
}

/// Parse the source and produce a `ParseReport`. Never panics; syntax errors
/// become diagnostics with `ok: false`.
pub fn analyze(source: &str) -> ParseReport {
    match syn::parse_file(source) {
        Ok(file) => {
            let mut runtime = RuntimeConfig::default();
            let mut diagnostics = Vec::new();
            let mut functions = Vec::new();

            for item in &file.items {
                if let syn::Item::Fn(f) = item {
                    let is_async = f.sig.asyncness.is_some();
                    functions.push(FunctionSig {
                        name: f.sig.ident.to_string(),
                        line: f.sig.ident.span().start().line as u32,
                        is_async,
                    });

                    for attr in &f.attrs {
                        if is_tokio_main_attr(attr) {
                            apply_tokio_main_attr(attr, &mut runtime, &mut diagnostics);
                        }
                    }
                }
            }

            let mut visitor = PrimitiveVisitor::default();
            visitor.visit_file(&file);
            for d in visitor.diagnostics {
                diagnostics.push(d);
            }

            ParseReport {
                ok: !diagnostics.iter().any(|d| d.level == DiagLevel::Error),
                runtime,
                diagnostics,
                primitives: visitor.hits,
                functions,
            }
        }
        Err(err) => {
            let span = err.span();
            let start = span.start();
            ParseReport {
                ok: false,
                runtime: RuntimeConfig::default(),
                diagnostics: vec![Diagnostic {
                    level: DiagLevel::Error,
                    line: start.line as u32,
                    col: start.column as u32,
                    message: err.to_string(),
                }],
                primitives: vec![],
                functions: vec![],
            }
        }
    }
}

fn is_tokio_main_attr(attr: &syn::Attribute) -> bool {
    let path = attr.path();
    if path.is_ident("main") {
        return false;
    }
    let segs: Vec<String> = path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    segs.len() == 2 && segs[0] == "tokio" && segs[1] == "main"
}

fn apply_tokio_main_attr(
    attr: &syn::Attribute,
    runtime: &mut RuntimeConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Parse the attribute args: #[tokio::main(flavor = "current_thread", worker_threads = 4)]
    let meta_list = match &attr.meta {
        syn::Meta::Path(_) => {
            // Bare #[tokio::main] — multi_thread default
            return;
        }
        syn::Meta::List(list) => list,
        syn::Meta::NameValue(_) => return,
    };

    let parsed = meta_list.parse_args_with(
        syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated,
    );

    let parsed = match parsed {
        Ok(p) => p,
        Err(e) => {
            diagnostics.push(Diagnostic {
                level: DiagLevel::Warning,
                line: attr.span().start().line as u32,
                col: 0,
                message: format!("无法解析 #[tokio::main] 参数: {e}"),
            });
            return;
        }
    };

    for nv in parsed {
        let key = nv.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
        match key.as_str() {
            "flavor" => {
                if let Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    match s.value().as_str() {
                        "current_thread" => {
                            runtime.flavor = Flavor::CurrentThread;
                            runtime.worker_threads = 1;
                        }
                        "multi_thread" => runtime.flavor = Flavor::MultiThread,
                        other => diagnostics.push(Diagnostic {
                            level: DiagLevel::Warning,
                            line: s.span().start().line as u32,
                            col: 0,
                            message: format!("未知 flavor: {other},将按 multi_thread 处理"),
                        }),
                    }
                }
            }
            "worker_threads" => {
                if let Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(n),
                    ..
                }) = &nv.value
                {
                    if let Ok(v) = n.base10_parse::<u32>() {
                        if runtime.flavor == Flavor::MultiThread {
                            runtime.worker_threads = v.max(1).min(32);
                        } else {
                            diagnostics.push(Diagnostic {
                                level: DiagLevel::Warning,
                                line: n.span().start().line as u32,
                                col: 0,
                                message: "current_thread flavor 下忽略 worker_threads".into(),
                            });
                        }
                    }
                }
            }
            other => diagnostics.push(Diagnostic {
                level: DiagLevel::Info,
                line: nv.span().start().line as u32,
                col: 0,
                message: format!("忽略未知 tokio::main 参数: {other}"),
            }),
        }
    }
}

#[derive(Default)]
struct PrimitiveVisitor {
    hits: Vec<PrimitiveHit>,
    diagnostics: Vec<Diagnostic>,
}

impl<'ast> Visit<'ast> for PrimitiveVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(ExprPath { path, .. }) = node.func.as_ref() {
            let path_str = path_to_string(path);
            let line = node.span().start().line as u32;
            match path_str.as_str() {
                "tokio::spawn" | "::tokio::spawn" => {
                    self.hits.push(PrimitiveHit { kind: "tokio::spawn", line })
                }
                "tokio::task::spawn_blocking"
                | "::tokio::task::spawn_blocking"
                | "tokio::spawn_blocking" => self.hits.push(PrimitiveHit {
                    kind: "tokio::task::spawn_blocking",
                    line,
                }),
                "tokio::time::sleep" | "::tokio::time::sleep" => {
                    self.hits.push(PrimitiveHit { kind: "tokio::time::sleep", line })
                }
                "tokio::task::yield_now" | "::tokio::task::yield_now" => {
                    self.hits.push(PrimitiveHit { kind: "tokio::task::yield_now", line })
                }
                s if s.starts_with("tokio::") => self.diagnostics.push(Diagnostic {
                    level: DiagLevel::Warning,
                    line,
                    col: 0,
                    message: format!("未支持的 tokio API: {s}(将按未插桩处理)"),
                }),
                _ => {}
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        // catches `.await` expressions indirectly via Await expression below
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_await(&mut self, node: &'ast syn::ExprAwait) {
        self.hits.push(PrimitiveHit {
            kind: ".await",
            line: node.span().start().line as u32,
        });
        syn::visit::visit_expr_await(self, node);
    }

    fn visit_macro(&mut self, node: &'ast Macro) {
        let path_str = path_to_string(&node.path);
        let line = node.span().start().line as u32;
        match path_str.as_str() {
            "tokio::join" | "join" => {
                self.hits.push(PrimitiveHit { kind: "tokio::join!", line })
            }
            "println" => self.hits.push(PrimitiveHit { kind: "println!", line }),
            _ => {}
        }
        syn::visit::visit_macro(self, node);
    }
}

pub(crate) fn path_to_string(path: &syn::Path) -> String {
    let prefix = if path.leading_colon.is_some() { "::" } else { "" };
    let joined = path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");
    format!("{prefix}{joined}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_multi_thread_default() {
        let src = r#"
            #[tokio::main]
            async fn main() {
                tokio::spawn(async {});
            }
        "#;
        let report = analyze(src);
        assert!(report.ok);
        assert_eq!(report.runtime.flavor, Flavor::MultiThread);
        assert_eq!(report.runtime.worker_threads, DEFAULT_WORKER_THREADS);
        assert!(report.primitives.iter().any(|p| p.kind == "tokio::spawn"));
    }

    #[test]
    fn detects_current_thread_flavor() {
        let src = r#"
            #[tokio::main(flavor = "current_thread")]
            async fn main() {}
        "#;
        let report = analyze(src);
        assert_eq!(report.runtime.flavor, Flavor::CurrentThread);
        assert_eq!(report.runtime.worker_threads, 1);
    }

    #[test]
    fn detects_worker_threads_override() {
        let src = r#"
            #[tokio::main(flavor = "multi_thread", worker_threads = 8)]
            async fn main() {}
        "#;
        let report = analyze(src);
        assert_eq!(report.runtime.worker_threads, 8);
    }

    #[test]
    fn warns_on_unknown_tokio_api() {
        let src = r#"
            #[tokio::main]
            async fn main() {
                let _ = tokio::sync::Mutex::new(());
            }
        "#;
        let report = analyze(src);
        assert!(report.diagnostics.iter().any(|d| matches!(d.level, DiagLevel::Warning)));
    }

    #[test]
    fn syntax_error_returns_error_diag() {
        let src = "fn main() { let x = ";
        let report = analyze(src);
        assert!(!report.ok);
        assert!(matches!(report.diagnostics[0].level, DiagLevel::Error));
    }
}
