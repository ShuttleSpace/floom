//! Lower a parsed TSX tree into a `floom_ui_ir::UiDocument`.
//!
//! The mapping is deliberately closed: every TSX tag routes through
//! `NodeKind::parse`, every `onXxx` attribute through `EventName::parse`, and
//! only props/events the IR schema declares are emitted. Anything else is
//! rejected with a span-based diagnostic so an author gets a fixable error
//! rather than a silent placeholder.
//!
//! Signal-backed expressions (`{count()}`) lower to `PropValue::Ref(name)` and
//! the signal declaration (`createSignal(0)`) to a `Binding`; the
//! `floom-dsl` emitter turns those into real `floom_signal` calls.

use std::collections::HashMap;

use floom_ui_ir::{
    Binding, EventBinding, EventName, NodeKey, NodeKind, Prop, PropValue, Rgba, Slot, UiDocument,
    UiNode, ValueType,
};
use swc_common::Span;
use swc_ecma_ast as ast;

use crate::error::{Result, TsxError};
use crate::parser::{Parsed, RootJsx};

/// State threaded through a single `compile` call.
struct Lower<'a> {
    parsed: &'a Parsed,
    nodes: Vec<UiNode>,
    /// Bindings referenced by refs/handlers, keyed by name.
    value_bindings: HashMap<String, ValueType>,
    handler_bindings: HashMap<String, ()>,
    counter: usize,
}

impl<'a> Lower<'a> {
    fn next_key(&mut self) -> NodeKey {
        let key = format!("n{}", self.counter);
        self.counter += 1;
        NodeKey::new(key)
    }

    /// Localize an swc span to an error.
    fn err(&self, span: Span, message: impl Into<String>) -> TsxError {
        let (line, col) = self.parsed.locate(span);
        TsxError::new(message).at(line, col)
    }

    /// Lower a JSX element into a fresh node and return its key.
    fn element(&mut self, el: &ast::JSXElement) -> Result<NodeKey> {
        let key = self.next_key();
        let kind = self.tag_kind(el.opening.span, &el.opening.name)?;

        let mut props = Vec::new();
        let mut events = Vec::new();
        let mut class_tokens: Vec<String> = Vec::new();
        for attr in &el.opening.attrs {
            if let ast::JSXAttrOrSpread::JSXAttr(attr) = attr {
                self.attribute(&key, kind, attr, &mut props, &mut events, &mut class_tokens)?;
            }
        }
        // Resolve `class="text-primary"` tokens into style props after the
        // explicit attributes, so an explicit `foreground=` wins over a token.
        for token in class_tokens {
            if let Some((prop_name, value)) = resolve_class(&token) {
                if !props.iter().any(|p| p.name == prop_name) {
                    props.push(Prop::new(prop_name, value));
                }
            }
        }

        let children = self.children(&el.children)?;

        // `<Text>hello</Text>` is one Text widget with `label`, not a Text that
        // contains a child Text. Fold a sole text run into the `label` prop.
        let mut sole_text: Option<String> = None;
        if kind == NodeKind::Text && children.len() == 1 {
            if let Some(only) = self.nodes.iter().find(|n| n.key == children[0]) {
                sole_text = only
                    .props
                    .iter()
                    .find(|p| p.name == "label")
                    .and_then(|p| match &p.value {
                        PropValue::Text(t) => Some(t.clone()),
                        _ => None,
                    });
            }
        }

        let mut node = UiNode::new(key.clone(), kind).with_props(props);
        if !events.is_empty() {
            node = node.with_events(events);
        }
        match sole_text {
            Some(label) => {
                node.props.push(Prop::new("label", PropValue::Text(label)));
                let dropped = children[0].clone();
                self.nodes.retain(|n| n.key != dropped);
            }
            None if !children.is_empty() => {
                node = node.with_slot(Slot::default_slot(children));
            }
            _ => {}
        }
        self.nodes.push(node);
        Ok(key)
    }

    /// Resolve a JSX tag name to a `NodeKind`, mapping `floom.Button` member
    /// expressions onto the bare kind and rejecting unknown tags.
    fn tag_kind(&self, span: Span, name: &ast::JSXElementName) -> Result<NodeKind> {
        let ident = match name {
            ast::JSXElementName::Ident(id) => id.sym.as_str().to_string(),
            ast::JSXElementName::JSXMemberExpr(member) => member.prop.sym.as_str().to_string(),
            ast::JSXElementName::JSXNamespacedName(ns) => ns.name.sym.as_str().to_string(),
        };
        NodeKind::parse(&ident).ok_or_else(|| {
            self.err(
                span,
                format!("unknown component `<{ident}>`; not a Floom widget"),
            )
        })
    }

    /// Lower one attribute into a prop or an event, or record a `class` token.
    fn attribute(
        &mut self,
        node_key: &NodeKey,
        kind: NodeKind,
        attr: &ast::JSXAttr,
        props: &mut Vec<Prop>,
        events: &mut Vec<EventBinding>,
        class_tokens: &mut Vec<String>,
    ) -> Result<()> {
        let name = match &attr.name {
            ast::JSXAttrName::Ident(id) => id.sym.as_str().to_string(),
            ast::JSXAttrName::JSXNamespacedName(ns) => ns.name.sym.as_str().to_string(),
        };

        if name == "class" {
            if let Some(values) = self.attr_string_value(attr)? {
                class_tokens.extend(values.split_whitespace().map(str::to_string));
            }
            return Ok(());
        }

        if let Some(event) = name
            .strip_prefix("on")
            .and_then(|e| EventName::parse(&e.to_lowercase()))
        {
            let handler = self.handler_name(attr)?;
            events.push(EventBinding::new(event, handler));
            return Ok(());
        }

        // Property: derive its value, defaulting to the literal `true` for a
        // bare attribute (`<Checkbox checked />`).
        let value = match &attr.value {
            None => PropValue::Bool(true),
            Some(v) => self.attr_value(node_key, kind, &name, attr.span, v)?,
        };
        props.push(Prop::new(name, value));
        Ok(())
    }

    /// The handler name for an `onXxx` attribute: an identifier reference like
    /// `onClick={onPress}` becomes the binding `onPress`.
    fn handler_name(&mut self, attr: &ast::JSXAttr) -> Result<String> {
        match &attr.value {
            Some(ast::JSXAttrValue::JSXExprContainer(container)) => {
                if let ast::JSXExpr::Expr(expr) = &container.expr {
                    if let ast::Expr::Ident(id) = &**expr {
                        let name = id.sym.as_str().to_string();
                        self.handler_bindings.insert(name.clone(), ());
                        return Ok(name);
                    }
                }
                Err(self.err(
                    container.span,
                    "event handlers must be a function reference (e.g. onClick={onPress})",
                ))
            }
            _ => Err(self.err(
                attr.span,
                "event handlers need a `={...}` expression",
            )),
        }
    }

    /// A string literal attribute value, if the attribute is a string literal.
    fn attr_string_value(&self, attr: &ast::JSXAttr) -> Result<Option<String>> {
        match &attr.value {
            // `Str.value` is a `Wtf8Atom`, whose `as_str` returns `Option<&str>`
            // (WTF-8 need not be valid UTF-8). TSX literals are UTF-8, so a
            // missing value is empty rather than an error.
            Some(ast::JSXAttrValue::Str(s)) => {
                Ok(Some(s.value.as_str().unwrap_or_default().to_string()))
            }
            _ => Ok(None),
        }
    }

    /// Lower a property's `={...}` or literal value, coercing literals to the
    /// type the schema declares for this prop (e.g. `axis="vertical"` is an
    /// `Ident`, `background="#f00"` is a `Color`).
    fn attr_value(
        &mut self,
        node_key: &NodeKey,
        kind: NodeKind,
        name: &str,
        attr_span: Span,
        value: &ast::JSXAttrValue,
    ) -> Result<PropValue> {
        let spec_ty = floom_ui_ir::prop_spec(kind, name).map(|spec| spec.ty);
        match value {
            ast::JSXAttrValue::Str(s) => self.string_value(s, spec_ty),
            ast::JSXAttrValue::JSXExprContainer(container) => {
                if let ast::JSXExpr::Expr(expr) = &container.expr {
                    self.expr_value(node_key, kind, name, expr, container.span)
                } else {
                    Err(self.err(container.span, "empty JSX expression in attribute"))
                }
            }
            _ => Err(self.err(attr_span, "unsupported attribute value")),
        }
    }

    /// Lower a string-literal attribute value, coercing to `spec_ty` when known.
    fn string_value(&self, s: &ast::Str, spec_ty: Option<ValueType>) -> Result<PropValue> {
        let text = s.value.as_str().unwrap_or_default().to_string();
        match spec_ty {
            Some(ValueType::Color) => Rgba::from_hex(&text)
                .map(PropValue::Color)
                .ok_or_else(|| TsxError::new(format!("invalid color literal `{text}`"))),
            // Domain-typed props (`axis`, `variant`, …) arrive as string
            // literals but are identifiers in the IR.
            Some(ValueType::Ident) => Ok(PropValue::Ident(text)),
            _ => Ok(PropValue::Text(text)),
        }
    }

    /// Lower a `={expr}` attribute value. Identifiers become signal refs;
    /// `createSignal`/`createMemo` calls are recognized as signal sources.
    fn expr_value(
        &mut self,
        node_key: &NodeKey,
        kind: NodeKind,
        name: &str,
        expr: &ast::Expr,
        span: Span,
    ) -> Result<PropValue> {
        match expr {
            ast::Expr::Ident(id) => {
                let sig = id.sym.as_str().to_string();
                let ty = floom_ui_ir::prop_spec(kind, name)
                    .map(|spec| spec.ty)
                    .unwrap_or(ValueType::Text);
                self.value_bindings.insert(sig.clone(), ty);
                Ok(PropValue::Ref(sig))
            }
            ast::Expr::Call(call) => self.call_value(node_key, kind, name, call),
            ast::Expr::Lit(lit) => {
                let spec_ty = floom_ui_ir::prop_spec(kind, name).map(|spec| spec.ty);
                self.lit_value(lit, spec_ty)
            }
            _ => Err(self.err(
                span,
                "only literal values, signal refs, and signal calls are supported here",
            )),
        }
    }

    /// Lower a literal expression (`{16}`, `{"hi"}`, `{true}`), coercing to the
    /// schema's declared type so a `Number` seat never receives an `Int` (or
    /// vice versa).
    fn lit_value(&self, lit: &ast::Lit, spec_ty: Option<ValueType>) -> Result<PropValue> {
        match lit {
            ast::Lit::Str(s) => self.string_value(s, spec_ty),
            ast::Lit::Bool(b) => Ok(PropValue::Bool(b.value)),
            ast::Lit::Num(n) => {
                let f = n.value;
                match spec_ty {
                    Some(ValueType::Int) => Ok(PropValue::Int(f as i64)),
                    Some(ValueType::Number) => Ok(PropValue::Number(f)),
                    _ if f.fract() == 0.0 => Ok(PropValue::Int(f as i64)),
                    _ => Ok(PropValue::Number(f)),
                }
            }
            _ => Err(TsxError::new("unsupported literal in attribute")),
        }
    }

    /// Recognize a signal reference in an attribute value.
    ///
    /// `count()` — a zero-argument call to a bare identifier — is how the TSX
    /// shim reads a signal, so it lowers to `PropValue::Ref("count")`. A
    /// `createSignal`/`createMemo` call in attribute position is an error: the
    /// signal must be bound with `const [name] = ...` at module scope.
    fn call_value(
        &mut self,
        _node_key: &NodeKey,
        kind: NodeKind,
        name: &str,
        call: &ast::CallExpr,
    ) -> Result<PropValue> {
        if let ast::Callee::Expr(e) = &call.callee {
            if let ast::Expr::Ident(id) = e.as_ref() {
                let callee = id.sym.as_str().to_string();
                if callee == "createSignal" || callee == "createMemo" {
                    return Err(self.err(
                        call.span,
                        format!("{callee} must be bound with `const [name] = ...` at module scope"),
                    ));
                }
                if call.args.is_empty() {
                    let ty = floom_ui_ir::prop_spec(kind, name)
                        .map(|spec| spec.ty)
                        .unwrap_or(ValueType::Text);
                    self.value_bindings.insert(callee.clone(), ty);
                    return Ok(PropValue::Ref(callee));
                }
            }
        }
        Err(self.err(call.span, "unsupported call in attribute"))
    }

    /// Lower JSX children into a flat list of child node keys, turning text runs
    /// into `Text` nodes and expression containers into conditional/list nodes.
    fn children(&mut self, children: &[ast::JSXElementChild]) -> Result<Vec<NodeKey>> {
        let mut keys = Vec::new();
        for child in children {
            match child {
                ast::JSXElementChild::JSXText(text) => {
                    let trimmed = text.value.as_str().trim();
                    if !trimmed.is_empty() {
                        let key = self.next_key();
                        let node = UiNode::new(key.clone(), NodeKind::Text).with_props([Prop::new(
                            "label",
                            PropValue::Text(trimmed.to_string()),
                        )]);
                        self.nodes.push(node);
                        keys.push(key);
                    }
                }
                ast::JSXElementChild::JSXElement(el) => keys.push(self.element(el)?),
                ast::JSXElementChild::JSXFragment(frag) => {
                    for inner in &frag.children {
                        if let ast::JSXElementChild::JSXElement(el) = inner {
                            keys.push(self.element(el)?);
                        }
                    }
                }
                ast::JSXElementChild::JSXExprContainer(container) => {
                    if let ast::JSXExpr::Expr(expr) = &container.expr {
                        // An empty `{}` is an `Invalid` expression; skip it so
                        // whitespace/comment placeholders don't become errors.
                        if matches!(expr.as_ref(), ast::Expr::Invalid(_)) {
                            continue;
                        }
                        if let Some(key) = self.expr_child(expr, container.span)? {
                            keys.push(key);
                        }
                    }
                }
                ast::JSXElementChild::JSXSpreadChild(spread) => {
                    return Err(self.err(spread.span, "spread children are not supported"));
                }
            }
        }
        Ok(keys)
    }

    /// Lower an expression-child (`cond ? a : b`, `items.map(i => <Row/>)`).
    fn expr_child(&mut self, expr: &ast::Expr, span: Span) -> Result<Option<NodeKey>> {
        match expr {
            ast::Expr::Cond(cond) => {
                let key = self.next_key();
                let then_key = match cond.cons.as_ref() {
                    ast::Expr::JSXElement(el) => self.element(el)?,
                    _ => {
                        return Err(self.err(
                            cond.span,
                            "the `?:` then-branch must be a JSX element",
                        ))
                    }
                };
                let when = self.condition_ref(&cond.test, cond.span)?;
                let node = UiNode::new(key.clone(), NodeKind::Conditional)
                    .with_props([Prop::new("when", when)])
                    .with_slot(Slot::default_slot(vec![then_key]));
                self.nodes.push(node);
                Ok(Some(key))
            }
            ast::Expr::Call(call) => self.map_call(call),
            _ => Err(self.err(
                span,
                "unsupported expression child (only ?: and .map are supported)",
            )),
        }
    }

    /// Lower `items.map(i => <Row key={i.id}/>)` into a `List` node whose child
    /// is the template element. The list is static-structured; the emitter
    /// fills it per item at runtime.
    fn map_call(&mut self, call: &ast::CallExpr) -> Result<Option<NodeKey>> {
        if call.args.len() != 1 {
            return Err(self.err(call.span, "expected `.map(fn)` with one argument"));
        }
        let arg = &call.args[0];
        let template = match arg.expr.as_ref() {
            ast::Expr::Arrow(arrow) => {
                // The arrow's single param becomes the item binding name.
                let _item = arrow
                    .params
                    .first()
                    .and_then(|p| match p {
                        ast::Pat::Ident(id) => Some(id.id.sym.as_str().to_string()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "item".to_string());
                match arrow.body.as_ref() {
                    ast::BlockStmtOrExpr::Expr(e) => match e.as_ref() {
                        ast::Expr::JSXElement(el) => el,
                        _ => return Err(self.err(call.span, ".map body must return a JSX element")),
                    },
                    ast::BlockStmtOrExpr::BlockStmt(_) => {
                        return Err(self.err(call.span, ".map body must be an expression"))
                    }
                }
            }
            _ => return Err(self.err(call.span, ".map expects an arrow returning JSX")),
        };
        let child_key = self.element(template)?;
        let key = self.next_key();
        let list = UiNode::new(key.clone(), NodeKind::List)
            .with_slot(Slot::default_slot(vec![child_key]));
        self.nodes.push(list);
        Ok(Some(key))
    }

    /// Turn a condition expression into a `when` prop value. A bare identifier is
    /// a boolean signal ref; anything else is rejected for now.
    fn condition_ref(&mut self, test: &ast::Expr, span: Span) -> Result<PropValue> {
        match test {
            ast::Expr::Ident(id) => {
                let name = id.sym.as_str().to_string();
                self.value_bindings.insert(name.clone(), ValueType::Bool);
                Ok(PropValue::Ref(name))
            }
            _ => Err(self.err(
                span,
                "conditional condition must be a boolean signal reference",
            )),
        }
    }
}

/// Lower a parsed TSX module into a validated `UiDocument`.
pub fn lower(parsed: &Parsed) -> Result<UiDocument> {
    // Module-level signal/context declarations are the authoritative bindings;
    // JSX-derived refs/handlers reference them and must not be duplicated.
    let module_bindings = collect_module_signals(&parsed.module);

    let mut lower = Lower {
        parsed,
        nodes: Vec::new(),
        value_bindings: HashMap::new(),
        handler_bindings: HashMap::new(),
        counter: 0,
    };
    let root = match &parsed.root {
        Some(RootJsx::Element(el)) => lower.element(el)?,
        Some(RootJsx::Fragment(frag)) => {
            let key = lower.next_key();
            let children = lower.children(&frag.children)?;
            let node = UiNode::new(key.clone(), NodeKind::Stack).with_slot(Slot::default_slot(children));
            lower.nodes.push(node);
            key
        }
        // No JSX root (signal-only module): emit a synthetic root so the
        // document still validates.
        None => {
            let key = lower.next_key();
            lower.nodes.push(UiNode::new(key.clone(), NodeKind::Stack));
            key
        }
    };

    let mut bindings: HashMap<String, Binding> = module_bindings
        .into_iter()
        .map(|b| (b.name.clone(), b))
        .collect();
    // JSX-derived value refs: only add if no module binding already claims the
    // name (the module binding is the source of truth for its type).
    for (name, ty) in lower.value_bindings.drain() {
        bindings
            .entry(name.clone())
            .or_insert_with(|| Binding::value(name, ty));
    }
    for name in lower.handler_bindings.keys().cloned() {
        bindings.entry(name.clone()).or_insert_with(|| Binding::handler(name));
    }
    let bindings: Vec<Binding> = bindings.into_values().collect();

    let doc = UiDocument {
        version: floom_ui_ir::IrVersion::CURRENT,
        root,
        bindings,
        nodes: lower.nodes,
    };
    doc.validate()
        .map_err(|e| TsxError::new(format!("IR validation failed: {e}")))?;
    Ok(doc)
}

/// Recognize module-level signal/context declarations and turn them into
/// `Binding`s. This is the compile-time half of the reactive shim: a TSX
/// `createSignal` becomes a `floom_signal` value binding, `provideContext` /
/// `useContext` become context bindings, and `createEffect` is recognized but
/// emits no binding (its body runs as an effect at runtime).
fn collect_module_signals(module: &ast::Module) -> Vec<Binding> {
    let mut out = Vec::new();
    for item in &module.body {
        match item {
            ast::ModuleItem::Stmt(stmt) => match stmt {
                ast::Stmt::Decl(ast::Decl::Var(var)) => {
                    for decl in &var.decls {
                        if let Some(b) = var_decl_signal(decl) {
                            out.push(b);
                        }
                    }
                }
                ast::Stmt::Expr(expr_stmt) => {
                    if let Some(b) = expr_stmt_signal(&expr_stmt.expr) {
                        out.push(b);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    out
}

/// `const [count, setCount] = createSignal(0)` / `createMemo(..)` /
/// `const t = useContext("theme")`.
fn var_decl_signal(decl: &ast::VarDeclarator) -> Option<Binding> {
    let init = decl.init.as_ref()?;
    let call = match init.as_ref() {
        ast::Expr::Call(c) => c,
        _ => return None,
    };
    let callee = call_name(call)?;
    match callee.as_str() {
        "createSignal" | "createMemo" => {
            // `[count, setCount]` or `double`.
            let name = array_first_name(&decl.name).or_else(|| pat_ident_name(&decl.name))?;
            let ty = call
                .args
                .first()
                .map(|a| lit_type(a.expr.as_ref()))
                .unwrap_or(ValueType::Text);
            Some(Binding::value(name, ty))
        }
        "useContext" => {
            let name = pat_ident_name(&decl.name)?;
            Some(Binding::value(name, ValueType::Text))
        }
        _ => None,
    }
}

/// `createEffect(() => …)` (no binding) and `provideContext("theme", "dark")`.
fn expr_stmt_signal(expr: &ast::Expr) -> Option<Binding> {
    let call = match expr {
        ast::Expr::Call(c) => c,
        _ => return None,
    };
    let callee = call_name(call)?;
    match callee.as_str() {
        // Recognized but emits no binding: its body runs as an effect.
        "createEffect" => None,
        "provideContext" => {
            let name = match call.args.first() {
                Some(a) => match a.expr.as_ref() {
                    ast::Expr::Lit(ast::Lit::Str(s)) => {
                        s.value.as_str().unwrap_or_default().to_string()
                    }
                    _ => return None,
                },
                None => return None,
            };
            let ty = call
                .args
                .get(1)
                .map(|a| lit_type(a.expr.as_ref()))
                .unwrap_or(ValueType::Text);
            Some(Binding::value(name, ty))
        }
        _ => None,
    }
}

/// The callee name of a call expression, if it is a bare identifier.
fn call_name(call: &ast::CallExpr) -> Option<String> {
    if let ast::Callee::Expr(e) = &call.callee {
        if let ast::Expr::Ident(id) = e.as_ref() {
            return Some(id.sym.as_str().to_string());
        }
    }
    None
}

/// The identifier bound by `[a, b]` destructuring, or `None`.
fn array_first_name(pat: &ast::Pat) -> Option<String> {
    if let ast::Pat::Array(arr) = pat {
        if let Some(Some(ast::Pat::Ident(id))) = arr.elems.first() {
            return Some(id.id.sym.as_str().to_string());
        }
    }
    None
}

/// The identifier bound by a plain `const x = …` pattern, or `None`.
fn pat_ident_name(pat: &ast::Pat) -> Option<String> {
    if let ast::Pat::Ident(id) = pat {
        return Some(id.id.sym.as_str().to_string());
    }
    None
}

/// Resolve a `class="…"` design token to an IR style property.
///
/// A deliberately small built-in table: the real token vocabulary belongs to a
/// design system, not the compiler. Unrecognized tokens are ignored (the IR has
/// no place to carry a raw class name). `foreground`/`background` are declared
/// style colours the IR already validates.
fn resolve_class(token: &str) -> Option<(&'static str, PropValue)> {
    let color = |r, g, b| PropValue::Color(Rgba::opaque(r, g, b));
    match token {
        "text-primary" => Some(("foreground", color(26, 115, 232))),
        "text-muted" => Some(("foreground", color(95, 99, 104))),
        "text-inverse" => Some(("foreground", color(255, 255, 255))),
        "bg-surface" => Some(("background", color(255, 255, 255))),
        "bg-primary" => Some(("background", color(26, 115, 232))),
        _ => None,
    }
}

/// Infer a `ValueType` from a literal initial expression (default `Text`).
fn lit_type(expr: &ast::Expr) -> ValueType {
    match expr {
        ast::Expr::Lit(ast::Lit::Num(n)) => {
            if n.value.fract() == 0.0 {
                ValueType::Int
            } else {
                ValueType::Number
            }
        }
        ast::Expr::Lit(ast::Lit::Str(_)) => ValueType::Text,
        ast::Expr::Lit(ast::Lit::Bool(_)) => ValueType::Bool,
        _ => ValueType::Text,
    }
}
