//! Turn TSX source into an swc AST and pull out the root JSX tree.
//!
//! The parser is deliberately thin: it hands the heavy lifting to
//! `swc_ecma_parser`, which is a production-grade, build-time-only dependency
//! (it never ships in the application binary). Lowering — turning the JSX into a
//! `floom_ui_ir::UiDocument` — lives in `lower.rs`.

use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_common::comments::SingleThreadedComments;
use swc_ecma_ast::EsVersion;
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax};

use crate::error::{Result, TsxError};

/// The root of a TSX tree: either a single element or a fragment.
#[derive(Debug, Clone)]
pub enum RootJsx {
    Element(Box<swc_ecma_ast::JSXElement>),
    Fragment(Box<swc_ecma_ast::JSXFragment>),
}

/// A parsed TSX module plus the `SourceMap` needed to localize errors.
pub struct Parsed {
    pub root: RootJsx,
    pub cm: Lrc<SourceMap>,
}

impl Parsed {
    /// Convert an swc byte `Span` into a 1-based `(line, col)` for diagnostics.
    pub fn locate(&self, span: swc_common::Span) -> (usize, usize) {
        let loc = self.cm.lookup_char_pos(span.lo());
        (loc.line, loc.col_display + 1)
    }
}

/// Parse `source`, wrapping a bare JSX expression as a default export so callers
/// can pass either a full component module or a plain `<Stack>…</Stack>` snippet.
pub fn parse(source: &str) -> Result<Parsed> {
    parse_inner(source)
        .or_else(|| {
            // Fall back to treating the whole source as a default-exported
            // expression, which lets `compile("<Stack/>")` work without ceremony.
            let wrapped = format!("export default ( {source} );");
            parse_inner(&wrapped)
        })
        .ok_or_else(|| TsxError::new("no JSX element or fragment found at the module root"))
}

/// Run the swc parser once and extract the first JSX root.
fn parse_inner(source: &str) -> Option<Parsed> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Anon.into(), source.to_string());
    let comments = SingleThreadedComments::default();
    let lexer = Lexer::new(
        Syntax::Es(EsSyntax {
            jsx: true,
            ..Default::default()
        }),
        EsVersion::Es2022,
        StringInput::from(&*fm),
        Some(&comments),
    );
    let mut parser = Parser::new_from(lexer);
    let module = parser.parse_module().ok()?;
    extract_root(&module).map(|root| Parsed { root, cm })
}

/// Find the first JSX tree in a module: a default-exported expression, a
/// default-exported component's `return`, or a top-level expression statement.
fn extract_root(module: &swc_ecma_ast::Module) -> Option<RootJsx> {
    for item in &module.body {
        match item {
            swc_ecma_ast::ModuleItem::ModuleDecl(swc_ecma_ast::ModuleDecl::ExportDefaultExpr(export)) => {
                if let Some(root) = expr_to_jsx(&export.expr) {
                    return Some(root);
                }
            }
            swc_ecma_ast::ModuleItem::ModuleDecl(swc_ecma_ast::ModuleDecl::ExportDefaultDecl(decl)) => {
                if let swc_ecma_ast::DefaultDecl::Fn(fn_expr) = &decl.decl {
                    if let Some(root) = fn_return_jsx(&fn_expr.function) {
                        return Some(root);
                    }
                }
            }
            swc_ecma_ast::ModuleItem::Stmt(stmt) => {
                if let Some(root) = stmt_to_jsx(stmt) {
                    return Some(root);
                }
            }
            _ => {}
        }
    }
    None
}

/// The JSX returned by the first `return` statement in a function body.
fn fn_return_jsx(function: &swc_ecma_ast::Function) -> Option<RootJsx> {
    let body = function.body.as_ref()?;
    for stmt in &body.stmts {
        if let swc_ecma_ast::Stmt::Return(ret) = stmt {
            if let Some(arg) = &ret.arg {
                return expr_to_jsx(arg);
            }
        }
    }
    None
}

/// A top-level expression statement: `;<Stack/>;` or `const __root = <Stack/>;`.
fn stmt_to_jsx(stmt: &swc_ecma_ast::Stmt) -> Option<RootJsx> {
    match stmt {
        swc_ecma_ast::Stmt::Expr(expr_stmt) => expr_to_jsx(&expr_stmt.expr),
        swc_ecma_ast::Stmt::Decl(swc_ecma_ast::Decl::Var(var)) => {
            for decl in &var.decls {
                if let Some(init) = &decl.init {
                    if let Some(root) = expr_to_jsx(init) {
                        return Some(root);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Unwrap parentheses and match a JSX element or fragment.
fn expr_to_jsx(expr: &swc_ecma_ast::Expr) -> Option<RootJsx> {
    match expr {
        swc_ecma_ast::Expr::Paren(paren) => expr_to_jsx(&paren.expr),
        // `Expr::JSXElement` holds a `Box<JSXElement>`; `Expr::JSXFragment` an
        // unboxed `JSXFragment`.
        swc_ecma_ast::Expr::JSXElement(el) => Some(RootJsx::Element(el.clone())),
        swc_ecma_ast::Expr::JSXFragment(frag) => Some(RootJsx::Fragment(Box::new(frag.clone()))),
        _ => None,
    }
}
