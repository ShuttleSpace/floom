//! Turn TSX source into an swc AST and pull out the root JSX tree.
//!
//! The parser is deliberately thin: it hands the heavy lifting to
//! `swc_ecma_parser`, which is a production-grade, build-time-only dependency
//! (it never ships in the application binary). Lowering — turning the JSX into a
//! `floom_ui_ir::UiDocument` — lives in `lower.rs`.

use swc_common::comments::SingleThreadedComments;
use swc_common::{FileName, SourceMap, Spanned, sync::Lrc};
use swc_ecma_ast::EsVersion;
use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax, lexer::Lexer};

use crate::error::{Result, TsxError};

/// The root of a TSX tree: either a single element or a fragment.
#[derive(Debug, Clone)]
pub enum RootJsx {
    Element(Box<swc_ecma_ast::JSXElement>),
    Fragment(Box<swc_ecma_ast::JSXFragment>),
}

/// A parsed TSX module plus the `SourceMap` needed to localize errors.
///
/// `root` is `None` when the module holds only signal/context declarations and
/// no JSX — `compile` then emits a synthetic root so the document still
/// validates.
pub struct Parsed {
    pub root: Option<RootJsx>,
    pub module: swc_ecma_ast::Module,
    pub cm: Lrc<SourceMap>,
}

impl Parsed {
    /// Convert an swc byte `Span` into a 1-based `(line, col)` for diagnostics.
    pub fn locate(&self, span: swc_common::Span) -> (usize, usize) {
        let loc = self.cm.lookup_char_pos(span.lo());
        (loc.line, loc.col_display + 1)
    }
}

/// Parse `source`. A bare JSX expression is wrapped as a default export so
/// callers can pass either a full component module or a plain
/// `<Stack>…</Stack>` snippet. Modules with only signal declarations and no JSX
/// are valid too (`root` will be `None`).
pub fn parse(source: &str) -> Result<Parsed> {
    match parse_inner(source) {
        Ok(parsed) => Ok(parsed),
        // A bare JSX expression is not a valid statement on its own; retry as a
        // default export, which is how `compile("<Stack/>")` is meant to be used.
        Err(_) => {
            let wrapped = format!("export default ( {source} );");
            parse_inner(&wrapped)
        }
    }
}

/// Run the swc parser once and extract the first JSX root (if any).
fn parse_inner(source: &str) -> Result<Parsed> {
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
    let result = parser.parse_module();
    // `parse_module` reports the *first* error it hit, but swc's recovery can
    // re-interpret a perfectly good `import` as a dynamic-import expression
    // when a later item fails to parse, so the returned error can point at the
    // import line instead of the real syntax error. `take_errors` holds the
    // recovered diagnostics; taking the one furthest into the source surfaces
    // the line the author actually has to fix.
    if let Err(original) = result {
        let errors = parser.take_errors();
        let best = errors
            .into_iter()
            .chain(std::iter::once(original))
            .max_by_key(|e| e.span().lo())
            .expect("a parse failure carries an error");
        let loc = cm.lookup_char_pos(best.span().lo());
        // swc sometimes re-interprets a valid top-level `import` as a
        // dynamic-import expression when it fails to parse a later item, so the
        // fatal error lands on the import line. Surface the real message, but
        // note that the cause may be elsewhere when the flagged token is an
        // `import`.
        let base = format!("{:?}", best.kind());
        let hint = if matches!(
            best.kind(),
            swc_ecma_parser::error::SyntaxError::InvalidImport
        ) {
            " — note: this may be swc's mis-recovery; check for an unclosed JSX tag, an unbalanced brace, or a malformed attribute elsewhere"
        } else {
            ""
        };
        return Err(TsxError::new(format!("TSX parse error: {base}{hint}"))
            .at(loc.line, loc.col_display + 1));
    }
    let module = result.map_err(|err| {
        let loc = cm.lookup_char_pos(err.span().lo());
        TsxError::new(format!("TSX parse error: {err:?}")).at(loc.line, loc.col_display + 1)
    })?;
    let root = extract_root(&module);
    Ok(Parsed { root, module, cm })
}

/// Find the first JSX tree in a module: a default-exported expression, a
/// default-exported component's `return`, or a top-level expression statement.
fn extract_root(module: &swc_ecma_ast::Module) -> Option<RootJsx> {
    for item in &module.body {
        match item {
            swc_ecma_ast::ModuleItem::ModuleDecl(swc_ecma_ast::ModuleDecl::ExportDefaultExpr(
                export,
            )) => {
                if let Some(root) = expr_to_jsx(&export.expr) {
                    return Some(root);
                }
            }
            swc_ecma_ast::ModuleItem::ModuleDecl(swc_ecma_ast::ModuleDecl::ExportDefaultDecl(
                decl,
            )) => {
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
