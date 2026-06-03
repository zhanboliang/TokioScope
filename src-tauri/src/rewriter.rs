use crate::parser::path_to_string;
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, ExprAwait, ExprCall, ExprPath, Macro};

/// Result of preparing an entry file: the instrumented source (with
/// `#[tokio::main]` stripped and `main` renamed to `__ts_user_main`) and whether
/// such a main was actually found.
pub struct EntryResult {
    pub code: String,
    pub found_main: bool,
}

/// Instrument one file's supported primitives (`tokio::spawn`, `.await`,
/// `sleep`, `println!`, `tokio::join!`) in place, **without** touching any entry
/// point. Used for every `.rs` file when tracing a whole project.
///
/// On parse failure returns the source verbatim so the file still compiles —
/// the per-file best-effort fallback for project tracing.
pub fn rewrite_file(source: &str) -> String {
    let mut file = match syn::parse_file(source) {
        Ok(f) => f,
        Err(_) => return source.to_owned(),
    };
    let mut visitor = Rewriter;
    visitor.visit_file_mut(&mut file);
    prettyplease::unparse(&file)
}

/// Instrument a file **and** strip `#[tokio::main]` / rename `main` →
/// `__ts_user_main`, so the caller can drive the runtime via the tracer. No
/// entry stub is appended. On parse failure the source is returned verbatim.
pub fn prepare_entry(source: &str) -> EntryResult {
    let mut file = match syn::parse_file(source) {
        Ok(f) => f,
        Err(_) => {
            return EntryResult {
                code: source.to_owned(),
                found_main: false,
            }
        }
    };

    let mut found_main = false;
    for item in file.items.iter_mut() {
        if let syn::Item::Fn(f) = item {
            let mut keep_attrs = Vec::new();
            let mut had_tokio_main = false;
            for attr in f.attrs.drain(..) {
                if is_tokio_main(&attr) {
                    had_tokio_main = true;
                } else {
                    keep_attrs.push(attr);
                }
            }
            f.attrs = keep_attrs;
            if had_tokio_main && f.sig.ident == "main" {
                f.sig.ident = syn::Ident::new("__ts_user_main", f.sig.ident.span());
                found_main = true;
            }
        }
    }

    let mut visitor = Rewriter;
    visitor.visit_file_mut(&mut file);

    EntryResult {
        code: prettyplease::unparse(&file),
        found_main,
    }
}

/// Rewrite a single self-contained snippet for the template crate: strip
/// `#[tokio::main]` (we drive the runtime ourselves), instrument primitives, and
/// append the `__ts_user_entry` the template's `main.rs` calls.
///
/// On parse failure returns the original source unchanged so the wrapper crate's
/// compile error message stays authoritative.
pub fn rewrite(source: &str) -> String {
    if syn::parse_file(source).is_err() {
        return source.to_owned();
    }
    let EntryResult { mut code, found_main } = prepare_entry(source);
    if found_main {
        code.push_str(
            "\npub fn __ts_user_entry() -> impl std::future::Future<Output = ()> + Send + 'static { __ts_user_main() }\n",
        );
    } else {
        // No `#[tokio::main]` found; emit a no-op entry so the harness compiles.
        code.push_str(
            "\npub fn __ts_user_entry() -> impl std::future::Future<Output = ()> + Send + 'static { async {} }\n",
        );
    }
    code
}

fn is_tokio_main(attr: &syn::Attribute) -> bool {
    let path = attr.path();
    let segs: Vec<String> = path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    segs.len() == 2 && segs[0] == "tokio" && segs[1] == "main"
}

struct Rewriter;

impl VisitMut for Rewriter {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        let line = expr.span().start().line as u32;

        // Rewrite await expressions: `x.await` => `__tracer::trace_await(L, x).await`
        if let Expr::Await(ExprAwait { base, .. }) = expr {
            // Avoid double-wrapping when base is already a __tracer call.
            if !is_already_traced(base) {
                let inner = (*base).clone();
                let wrapped: Expr = syn::parse2(quote! {
                    __tracer::trace_await(#line, #inner)
                })
                .expect("rewriter: trace_await");
                *base = Box::new(wrapped);
            }
            visit_mut::visit_expr_mut(self, expr);
            return;
        }

        // Rewrite call expressions for tokio primitives.
        if let Expr::Call(ExprCall { func, args, .. }) = expr {
            if let Expr::Path(ExprPath { path, .. }) = func.as_ref() {
                let p = path_to_string(path);
                let replacement: Option<TokenStream> = match p.as_str() {
                    "tokio::spawn" | "::tokio::spawn" => {
                        let a = args.iter();
                        Some(quote! { __tracer::spawn(#line, #(#a),*) })
                    }
                    "tokio::task::spawn_blocking"
                    | "::tokio::task::spawn_blocking"
                    | "tokio::spawn_blocking" => {
                        let a = args.iter();
                        Some(quote! { __tracer::spawn_blocking(#line, #(#a),*) })
                    }
                    "tokio::task::yield_now" | "::tokio::task::yield_now" => {
                        Some(quote! { __tracer::yield_now(#line) })
                    }
                    "tokio::time::sleep" | "::tokio::time::sleep" => {
                        let a = args.iter();
                        Some(quote! { __tracer::sleep(#line, #(#a),*) })
                    }
                    _ => None,
                };
                if let Some(ts) = replacement {
                    let new_expr: Expr = syn::parse2(ts).expect("rewriter: tokio call");
                    *expr = new_expr;
                }
            }
        }

        visit_mut::visit_expr_mut(self, expr);
    }

    fn visit_macro_mut(&mut self, mac: &mut Macro) {
        let p = path_to_string(&mac.path);
        let line = mac.span().start().line as u32;
        match p.as_str() {
            "println" => {
                // println!(fmt, args...) => __tracer::println(L, format!(fmt, args...))
                let inner = mac.tokens.clone();
                let new_expr: Expr = syn::parse2(quote! {
                    __tracer::println(#line, format!(#inner))
                })
                .expect("rewriter: println");
                // We cannot replace a macro with an expression in-place via
                // visit_macro_mut. We rely on a second pass via visit_expr_mut
                // detecting println! and rewriting; handled in visit_expr_mut
                // below.
                let _ = new_expr;
            }
            _ => {}
        }
        visit_mut::visit_macro_mut(self, mac);
    }

    fn visit_stmt_mut(&mut self, stmt: &mut syn::Stmt) {
        // Rewrite stand-alone println! and tokio::join! statements.
        if let syn::Stmt::Macro(m) = stmt {
            let p = path_to_string(&m.mac.path);
            let line = m.mac.span().start().line as u32;
            if p == "println" {
                let inner = m.mac.tokens.clone();
                let new_stmt: syn::Stmt = syn::parse2(quote! {
                    __tracer::println(#line, format!(#inner));
                })
                .expect("rewriter: stmt println");
                *stmt = new_stmt;
            } else if p == "tokio::join" || p == "join" {
                let inner = m.mac.tokens.clone();
                let new_stmt: syn::Stmt = syn::parse2(quote! {
                    __tracer::join_enter(#line);
                    let __ts_join_result = tokio::join!(#inner);
                    __tracer::join_exit(#line);
                    let _ = __ts_join_result;
                })
                .unwrap_or_else(|_| {
                    syn::parse2(quote! { let _ = tokio::join!(#inner); }).expect("fallback join")
                });
                *stmt = new_stmt;
                return;
            }
        }
        visit_mut::visit_stmt_mut(self, stmt);
    }
}

fn is_already_traced(expr: &Expr) -> bool {
    if let Expr::Call(ExprCall { func, .. }) = expr {
        if let Expr::Path(ExprPath { path, .. }) = func.as_ref() {
            let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
            return matches!(segs.first().map(String::as_str), Some("__tracer"));
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tokio_main_and_wraps_spawn() {
        let src = r#"
            #[tokio::main]
            async fn main() {
                tokio::spawn(async {});
            }
        "#;
        let out = rewrite(src);
        assert!(!out.contains("#[tokio::main]"));
        assert!(out.contains("__tracer::spawn"));
        assert!(out.contains("__ts_user_main"));
        assert!(out.contains("__ts_user_entry"));
    }

    #[test]
    fn wraps_await_and_sleep() {
        let src = r#"
            #[tokio::main]
            async fn main() {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        "#;
        let out = rewrite(src);
        assert!(out.contains("__tracer::sleep"));
        assert!(out.contains("__tracer::trace_await"));
    }

    #[test]
    fn wraps_println_stmt() {
        let src = r#"
            #[tokio::main]
            async fn main() {
                println!("hi {}", 1);
            }
        "#;
        let out = rewrite(src);
        assert!(out.contains("__tracer::println"));
    }

    #[test]
    fn rewrite_file_instruments_without_entry_stub() {
        // A non-entry project file: instrument primitives, but never rename main
        // or append __ts_user_entry.
        let src = r#"
            pub async fn helper() {
                tokio::spawn(async {});
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        "#;
        let out = rewrite_file(src);
        assert!(out.contains("__tracer::spawn"));
        assert!(out.contains("__tracer::sleep"));
        assert!(!out.contains("__ts_user_entry"));
        assert!(!out.contains("__ts_user_main"));
    }

    #[test]
    fn rewrite_file_passes_through_unparseable() {
        let src = "fn main() { let x = ";
        assert_eq!(rewrite_file(src), src);
    }

    #[test]
    fn prepare_entry_detects_and_renames_main() {
        let with = prepare_entry("#[tokio::main]\nasync fn main() {}\n");
        assert!(with.found_main);
        assert!(with.code.contains("__ts_user_main"));
        assert!(!with.code.contains("tokio::main"));

        let without = prepare_entry("async fn helper() {}\n");
        assert!(!without.found_main);
        assert!(!without.code.contains("__ts_user_main"));
    }
}
