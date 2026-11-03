//! The TSX application entry: `mount`, and the `window`/`app`/`platform` API.
//!
//! nsh lets a TSX module *drive* the runtime, not just hand back a tree: it
//! calls `mount(App)` at the top level and sets window/platform state through
//! small callable surfaces (`Window.setTitle`, `Menu.setApplicationMenu`, …).
//! This module is floem-tsx's compile-time take on that — there is no VM, so
//! the calls are read by the compiler and turned into the same things a Rust
//! author would write by hand:
//!
//! ```tsx
//! import { app, mount, platform, window } from "floom";
//!
//! app.setName("floom + tsx");
//! window.setSize(640, 420);
//! platform.require("ime", "warn");
//!
//! function App() { return <Stack>…</Stack>; }
//! mount(App);
//! ```
//!
//! - `mount(App)` names the root component (a function or arrow that returns
//!   JSX), or passes the tree inline (`mount(() => <Stack/>)`). An
//!   `export default` still works as a fallback.
//! - `window.setTitle/setSize/setResizable` become the [`AppConfig`].
//! - `platform.require(cap, policy)` becomes a `floom::platform::require` call
//!   emitted into `main` (see [`crate::app::emit_main`]).
//!
//! The `<App title size>` wrapper is still accepted — its attributes are the
//! same config — but the callable surface is the React/Vue-shaped way to do it,
//! and explicit `window.*` calls win over the wrapper's attributes.

use swc_common::{Span, Spanned};
use swc_ecma_ast as ast;

use crate::error::{Result, TsxError};
use crate::parser::{Parsed, RootJsx};

/// Build a span-located error from `Parsed`.
fn err_at(parsed: &Parsed, span: Span, message: impl Into<String>) -> TsxError {
    let (line, col) = parsed.locate(span);
    TsxError::new(message).at(line, col)
}

/// A capability the app needs, and what to do if the compiled-in backend lacks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    pub capability: String,
    pub policy: String,
}

/// Window/app configuration declared by a TSX application.
#[derive(Debug, Clone, PartialEq)]
pub struct AppConfig {
    pub title: String,
    pub width: f64,
    pub height: f64,
    /// Whether the window can be resized. Defaults to true so a desktop window
    /// is, unless the author says otherwise, a normal resizable one.
    pub resizable: bool,
    /// Capabilities the app gates on, in declaration order.
    pub requires: Vec<Requirement>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "floom".to_string(),
            width: 480.0,
            height: 320.0,
            resizable: true,
            requires: Vec::new(),
        }
    }
}

/// What the compiler resolved the entry to: the tree root and the config.
#[derive(Debug, Clone)]
pub struct Entry {
    pub root: Option<RootJsx>,
    pub config: AppConfig,
}

/// Read the entry surface (`mount`, `window.*`, `app.*`, `platform.require`)
/// out of a parsed module and resolve the root component.
pub fn resolve(parsed: &Parsed) -> Result<Entry> {
    let mut config = AppConfig::default();
    let mut mounted_name: Option<String> = None;
    let mut mounted_root: Option<RootJsx> = None;

    for item in &parsed.module.body {
        let Some(stmt) = as_expr_stmt(item) else {
            continue;
        };
        let ast::Expr::Call(call) = stmt else {
            continue;
        };
        let ast::Callee::Expr(callee) = &call.callee else {
            continue;
        };
        match callee.as_ref() {
            ast::Expr::Ident(id) if id.sym == "mount" => {
                let (name, root) = mount_target(call, parsed)?;
                mounted_name = name;
                mounted_root = root;
            }
            ast::Expr::Member(m) => {
                let Some(obj) = member_object_name(m) else {
                    continue;
                };
                apply_surface_call(&obj, call, &mut config, parsed)?;
            }
            _ => {}
        }
    }

    // Resolve the root: an inline `mount(() => <Stack/>)` wins, then a named
    // `mount(App)`, then the default-export / top-level JSX the parser found.
    let mut root: Option<RootJsx> = match mounted_root {
        Some(r) => Some(r),
        None => match mounted_name {
            Some(name) => Some(resolve_component(&name, parsed)?),
            None => parsed.root.clone(),
        },
    };

    // If the resolved root is an `<App>` wrapper, its attributes describe the
    // window; the wrapper is not a widget, so the real root is its first child.
    if let Some(RootJsx::Element(el)) = &root {
        if jsx_tag_name(&el.opening.name) == "App" {
            config = config_from_wrapper(el, config);
            root = app_inner_root(el);
        }
    }

    Ok(Entry { root, config })
}

/// The root JSX returned by the named component, or an error if it cannot be
/// found or does not return JSX.
fn resolve_component(name: &str, parsed: &Parsed) -> Result<RootJsx> {
    for item in &parsed.module.body {
        match item {
            ast::ModuleItem::ModuleDecl(ast::ModuleDecl::ExportDecl(decl)) => {
                if let Some(jsx) = component_return(&decl.decl, name) {
                    return Ok(jsx);
                }
            }
            ast::ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Fn(f))) => {
                if f.ident.sym == name {
                    if let Some(jsx) = fn_return_jsx(&f.function) {
                        return Ok(jsx);
                    }
                }
            }
            ast::ModuleItem::Stmt(ast::Stmt::Decl(ast::Decl::Var(var))) => {
                for d in &var.decls {
                    if let ast::Pat::Ident(binding) = &d.name {
                        if binding.id.sym == name {
                            if let Some(init) = &d.init {
                                if let Some(jsx) = arrow_return_jsx(init) {
                                    return Ok(jsx);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Err(TsxError::new(format!(
        "mount({name}): no component returning JSX named `{name}` was found in this module"
    )))
}

/// JSX returned by a function or arrow declaration named `name`.
fn component_return(decl: &ast::Decl, name: &str) -> Option<RootJsx> {
    match decl {
        ast::Decl::Fn(f) if f.ident.sym == name => fn_return_jsx(&f.function),
        _ => None,
    }
}

/// The JSX returned by a `() => <Stack/>` or `() => { return <Stack/>; }` init.
fn arrow_return_jsx(init: &ast::Expr) -> Option<RootJsx> {
    match init {
        ast::Expr::Arrow(arrow) => match arrow.body.as_ref() {
            ast::BlockStmtOrExpr::Expr(e) => expr_to_jsx(e),
            ast::BlockStmtOrExpr::BlockStmt(block) => block_return_jsx(block),
        },
        ast::Expr::Fn(fn_expr) => fn_return_jsx(&fn_expr.function),
        _ => expr_to_jsx(init),
    }
}

fn block_return_jsx(block: &ast::BlockStmt) -> Option<RootJsx> {
    for stmt in &block.stmts {
        if let ast::Stmt::Return(ret) = stmt {
            if let Some(arg) = &ret.arg {
                return expr_to_jsx(arg);
            }
        }
    }
    None
}

fn fn_return_jsx(function: &ast::Function) -> Option<RootJsx> {
    let body = function.body.as_ref()?;
    block_return_jsx(body)
}

fn expr_to_jsx(expr: &ast::Expr) -> Option<RootJsx> {
    match expr {
        ast::Expr::Paren(p) => expr_to_jsx(&p.expr),
        ast::Expr::JSXElement(el) => Some(RootJsx::Element(el.clone())),
        ast::Expr::JSXFragment(frag) => Some(RootJsx::Fragment(Box::new(frag.clone()))),
        _ => None,
    }
}

/// Resolve a `mount(...)` argument to either a component name (resolved later)
/// or an inline JSX tree (an arrow/function whose body is JSX).
fn mount_target(
    call: &ast::CallExpr,
    parsed: &Parsed,
) -> Result<(Option<String>, Option<RootJsx>)> {
    let Some(arg) = call.args.first() else {
        return Err(err_at(
            parsed,
            call.span,
            "mount() needs a component or JSX",
        ));
    };
    let arg_expr = arg.expr.as_ref();
    // `mount(<Stack/>)` — the argument is the tree itself.
    if let Some(jsx) = expr_to_jsx(arg_expr) {
        return Ok((None, Some(jsx)));
    }
    // `mount(() => <Stack/>)` / `mount(function () { return <Stack/> })`.
    if let Some(jsx) = arrow_return_jsx(arg_expr) {
        return Ok((None, Some(jsx)));
    }
    match arg_expr {
        ast::Expr::Ident(id) => Ok((Some(id.sym.to_string()), None)),
        other => Err(err_at(
            parsed,
            other.span(),
            "mount() takes a component name or inline JSX (e.g. `mount(App)`)",
        )),
    }
}

/// Apply one top-level `window.*` / `app.*` / `platform.require` call.
fn apply_surface_call(
    object: &str,
    call: &ast::CallExpr,
    config: &mut AppConfig,
    parsed: &Parsed,
) -> Result<()> {
    match object {
        "window" => apply_window(call, config, parsed),
        "app" => apply_app(call, config, parsed),
        "platform" => apply_platform(call, config, parsed),
        // Not part of the floem surface; leave it to the app's own runtime.
        _ => Ok(()),
    }
}

fn apply_window(call: &ast::CallExpr, config: &mut AppConfig, parsed: &Parsed) -> Result<()> {
    let Some(method) = member_method(call) else {
        return Ok(());
    };
    match method.as_str() {
        "setTitle" => {
            if let Some(s) = string_arg(call, 0) {
                config.title = s;
            }
        }
        "setSize" => {
            // `window.setSize(640, 420)` or `window.setSize("640x420")`.
            if let Some(s) = string_arg(call, 0) {
                parse_size(&s, config);
            } else if let (Some(w), Some(h)) = (number_arg(call, 0), number_arg(call, 1)) {
                config.width = w;
                config.height = h;
            }
        }
        "setResizable" => {
            if let Some(b) = bool_arg(call, 0) {
                config.resizable = b;
            }
        }
        m => {
            return Err(err_at(
                parsed,
                call.span,
                format!(
                    "`window.{m}` is not part of the floem TSX surface (known: setTitle, setSize, setResizable)"
                ),
            ));
        }
    }
    Ok(())
}

fn apply_app(call: &ast::CallExpr, config: &mut AppConfig, _parsed: &Parsed) -> Result<()> {
    let Some(method) = member_method(call) else {
        return Ok(());
    };
    match method.as_str() {
        // `app.setName` is the default window title; `window.setTitle` overrides.
        "setName" => {
            if let Some(s) = string_arg(call, 0) {
                config.title = s;
            }
        }
        m => {
            return Err(TsxError::new(format!(
                "`app.{m}` is not part of the floem TSX surface (known: setName)"
            )));
        }
    }
    Ok(())
}

fn apply_platform(call: &ast::CallExpr, config: &mut AppConfig, parsed: &Parsed) -> Result<()> {
    let Some(method) = member_method(call) else {
        return Ok(());
    };
    if method != "require" {
        return Err(TsxError::new(format!(
            "`platform.{method}` is not part of the floem TSX surface (known: require)"
        )));
    }
    let cap = string_arg(call, 0)
        .ok_or_else(|| TsxError::new(String::from("platform.require needs a capability string")))?;
    let policy = string_arg(call, 1)
        .ok_or_else(|| TsxError::new(String::from("platform.require needs a policy string")))?;
    validate_capability(&cap, call.span, parsed)?;
    validate_policy(&policy, call.span, parsed)?;
    config.requires.push(Requirement {
        capability: cap,
        policy,
    });
    Ok(())
}

const CAPABILITIES: &[&str] = &[
    "windowing",
    "clipboard",
    "ime",
    "accessibility",
    "webgpu",
    "software_render",
    "touch",
    "native_view",
];

fn validate_capability(cap: &str, span: Span, parsed: &Parsed) -> Result<()> {
    if CAPABILITIES.contains(&cap) {
        Ok(())
    } else {
        Err(err_at(
            parsed,
            span,
            format!(
                "`{cap}` is not a known capability (one of: {})",
                CAPABILITIES.join(", ")
            ),
        ))
    }
}

fn validate_policy(policy: &str, span: Span, parsed: &Parsed) -> Result<()> {
    if matches!(policy, "warn" | "fail" | "exit") {
        Ok(())
    } else {
        Err(err_at(
            parsed,
            span,
            format!("policy must be `warn`, `fail` or `exit`, got `{policy}`"),
        ))
    }
}

// --- helpers -----------------------------------------------------------------

fn as_expr_stmt(item: &ast::ModuleItem) -> Option<&ast::Expr> {
    if let ast::ModuleItem::Stmt(ast::Stmt::Expr(expr_stmt)) = item {
        Some(&expr_stmt.expr)
    } else {
        None
    }
}

fn member_object_name(m: &ast::MemberExpr) -> Option<String> {
    match m.obj.as_ref() {
        ast::Expr::Ident(id) => Some(id.sym.to_string()),
        _ => None,
    }
}

/// The method name of `obj.method()` once the callee is known to be a member.
fn member_method(call: &ast::CallExpr) -> Option<String> {
    let ast::Callee::Expr(callee) = &call.callee else {
        return None;
    };
    if let ast::Expr::Member(m) = callee.as_ref() {
        if let ast::MemberProp::Ident(id) = &m.prop {
            return Some(id.sym.to_string());
        }
    }
    None
}

fn string_arg(call: &ast::CallExpr, i: usize) -> Option<String> {
    let arg = call.args.get(i)?;
    match arg.expr.as_ref() {
        ast::Expr::Lit(ast::Lit::Str(s)) => Some(s.value.as_str().unwrap_or_default().to_string()),
        _ => None,
    }
}

fn number_arg(call: &ast::CallExpr, i: usize) -> Option<f64> {
    let arg = call.args.get(i)?;
    match arg.expr.as_ref() {
        ast::Expr::Lit(ast::Lit::Num(n)) => Some(n.value),
        _ => None,
    }
}

fn bool_arg(call: &ast::CallExpr, i: usize) -> Option<bool> {
    let arg = call.args.get(i)?;
    match arg.expr.as_ref() {
        ast::Expr::Lit(ast::Lit::Bool(b)) => Some(b.value),
        _ => None,
    }
}

fn jsx_tag_name(name: &ast::JSXElementName) -> String {
    match name {
        ast::JSXElementName::Ident(id) => id.sym.as_str().to_string(),
        ast::JSXElementName::JSXMemberExpr(member) => member.prop.sym.as_str().to_string(),
        ast::JSXElementName::JSXNamespacedName(ns) => ns.name.sym.as_str().to_string(),
    }
}

/// The real root element inside an `<App>` wrapper (its first child element).
fn app_inner_root(el: &ast::JSXElement) -> Option<RootJsx> {
    for child in &el.children {
        if let ast::JSXElementChild::JSXElement(inner) = child {
            return Some(RootJsx::Element(inner.clone()));
        }
    }
    None
}

fn jsx_attr_name(attr: &ast::JSXAttr) -> String {
    match &attr.name {
        ast::JSXAttrName::Ident(id) => id.sym.as_str().to_string(),
        ast::JSXAttrName::JSXNamespacedName(ns) => ns.name.sym.as_str().to_string(),
    }
}

fn attr_string(attr: &ast::JSXAttr) -> Option<String> {
    match &attr.value {
        Some(ast::JSXAttrValue::Str(s)) => Some(s.value.as_str().unwrap_or_default().to_string()),
        Some(ast::JSXAttrValue::JSXExprContainer(c)) => {
            if let ast::JSXExpr::Expr(e) = &c.expr {
                if let ast::Expr::Lit(ast::Lit::Str(s)) = e.as_ref() {
                    Some(s.value.as_str().unwrap_or_default().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Config from an `<App title="…" size="WxH" width=… height=…>` wrapper, applied
/// on top of an existing config (so explicit `window.*` calls still win).
fn config_from_wrapper(el: &ast::JSXElement, mut config: AppConfig) -> AppConfig {
    for attr in &el.opening.attrs {
        if let ast::JSXAttrOrSpread::JSXAttr(attr) = attr {
            match jsx_attr_name(attr) {
                name if name == "title" => {
                    if let Some(v) = attr_string(attr) {
                        config.title = v;
                    }
                }
                name if name == "size" => {
                    if let Some(v) = attr_string(attr) {
                        parse_size(&v, &mut config);
                    }
                }
                name if name == "width" => {
                    if let Some(v) = attr_string(attr) {
                        config.width = v.parse().unwrap_or(config.width);
                    }
                }
                name if name == "height" => {
                    if let Some(v) = attr_string(attr) {
                        config.height = v.parse().unwrap_or(config.height);
                    }
                }
                _ => {}
            }
        }
    }
    config
}

fn parse_size(s: &str, app: &mut AppConfig) {
    if let Some((w, h)) = s.split_once('x') {
        if let (Ok(w), Ok(h)) = (w.trim().parse::<f64>(), h.trim().parse::<f64>()) {
            app.width = w;
            app.height = h;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(source: &str) -> Entry {
        let parsed = crate::parser::parse(source).expect("parse");
        crate::entry::resolve(&parsed).expect("resolve entry")
    }

    #[test]
    fn mount_names_the_root_component() {
        let entry =
            resolve("function App() { return <Stack><Text label=\"x\" /></Stack>; }\nmount(App);");
        let root = entry.root.expect("root");
        let RootJsx::Element(el) = root else {
            panic!("root should be a component -> element");
        };
        assert_eq!(jsx_tag_name(&el.opening.name), "Stack");
    }

    #[test]
    fn mount_accepts_an_inline_jsx_tree() {
        let entry = resolve("mount(() => <Stack />);");
        let root = entry.root.expect("root");
        let RootJsx::Element(el) = root else {
            panic!("root should be the inline element");
        };
        assert_eq!(jsx_tag_name(&el.opening.name), "Stack");
    }

    #[test]
    fn mount_unknown_component_is_an_error() {
        let parsed = crate::parser::parse("mount(Missing);").expect("parse");
        let err = crate::entry::resolve(&parsed).expect_err("should fail");
        assert!(err.message.contains("Missing"), "{err:?}");
    }

    #[test]
    fn window_calls_become_the_config() {
        let entry = resolve(
            "function App() { return <Stack />; }\nwindow.setTitle(\"Counter\");\nwindow.setSize(640, 420);\nwindow.setResizable(false);\nmount(App);",
        );
        assert_eq!(entry.config.title, "Counter");
        assert_eq!(entry.config.width, 640.0);
        assert_eq!(entry.config.height, 420.0);
        assert!(!entry.config.resizable);
    }

    #[test]
    fn app_set_name_is_the_default_title_but_window_wins() {
        let with_app = resolve(
            "function App() { return <Stack />; }\napp.setName(\"from-app\");\nmount(App);",
        );
        assert_eq!(with_app.config.title, "from-app");

        let with_both = resolve(
            "function App() { return <Stack />; }\napp.setName(\"from-app\");\nwindow.setTitle(\"from-window\");\nmount(App);",
        );
        assert_eq!(with_both.config.title, "from-window");
    }

    #[test]
    fn platform_require_is_recorded_in_declaration_order() {
        let entry = resolve(
            "function App() { return <Stack />; }\nplatform.require(\"ime\", \"warn\");\nplatform.require(\"clipboard\", \"fail\");\nmount(App);",
        );
        assert_eq!(
            entry.config.requires,
            vec![
                Requirement {
                    capability: "ime".into(),
                    policy: "warn".into()
                },
                Requirement {
                    capability: "clipboard".into(),
                    policy: "fail".into()
                },
            ]
        );
    }

    #[test]
    fn unknown_window_method_is_rejected() {
        let parsed = crate::parser::parse(
            "function App() { return <Stack />; }\nwindow.frobnicate(1);\nmount(App);",
        )
        .expect("parse");
        let err = crate::entry::resolve(&parsed).expect_err("unknown method");
        assert!(err.message.contains("window.frobnicate"), "{err:?}");
    }

    #[test]
    fn unknown_capability_or_policy_is_rejected() {
        let p1 = crate::parser::parse(
            "function App() { return <Stack />; }\nplatform.require(\"telepathy\", \"warn\");\nmount(App);",
        )
        .expect("parse");
        assert!(crate::entry::resolve(&p1)
            .expect_err("cap")
            .message
            .contains("telepathy"));

        let p2 = crate::parser::parse(
            "function App() { return <Stack />; }\nplatform.require(\"ime\", \"maybe\");\nmount(App);",
        )
        .expect("parse");
        assert!(crate::entry::resolve(&p2)
            .expect_err("policy")
            .message
            .contains("policy"));
    }

    #[test]
    fn the_app_wrapper_still_works_as_before() {
        let entry = resolve(
            "export default function App() {\n  return <App title=\"Wrapped\" size=\"300x200\"><Stack /></App>;\n}",
        );
        assert_eq!(entry.config.title, "Wrapped");
        assert_eq!(entry.config.width, 300.0);
        assert_eq!(entry.config.height, 200.0);
    }
}
