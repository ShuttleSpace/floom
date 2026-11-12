// SPDX-License-Identifier: MIT
//! Lower a parsed TSX tree into a `floom_dsl::ui_ir::UiDocument`.
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

use std::collections::{HashMap, HashSet};

use floom_dsl::ui_ir::{
    Binding, EventBinding, EventName, NodeKey, NodeKind, Prop, PropValue, Rgba, Slot, UiDocument,
    UiNode, ValueType,
};
use swc_common::{Span, Spanned};
use swc_ecma_ast as ast;

use crate::error::{Result, TsxError};
use crate::module::ModuleExport;
use crate::parser::{Parsed, RootJsx};

/// State threaded through a single `compile` call.
struct Lower<'a> {
    parsed: &'a Parsed,
    nodes: Vec<UiNode>,
    /// Bindings referenced by refs/handlers, keyed by name.
    value_bindings: HashMap<String, ValueType>,
    handler_bindings: HashMap<String, ()>,
    /// Names of every declared signal (the `count` of `createSignal`), so an
    /// inline handler can rewrite `count()` to `count.get()`.
    signals: HashSet<String>,
    /// Setter name → signal name (`setCount` → `count`), so an inline handler can
    /// rewrite `setCount(x)` to `count.set(x)`.
    setters: HashMap<String, String>,
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
            if let Some((prop_name, value)) = resolve_class(&token)
                && !props.iter().any(|p| p.name == prop_name)
            {
                props.push(Prop::new(prop_name, value));
            }
        }

        let children = self.children(&el.children)?;

        // `<Text>hello</Text>` is one Text widget with `label`, not a Text that
        // contains a child Text. Fold a sole text run into the `label` prop.
        let mut sole_text: Option<String> = None;
        if kind == NodeKind::Text
            && children.len() == 1
            && let Some(only) = self.nodes.iter().find(|n| n.key == children[0])
        {
            sole_text =
                only.props
                    .iter()
                    .find(|p| p.name == "label")
                    .and_then(|p| match &p.value {
                        PropValue::Text(t) => Some(t.clone()),
                        _ => None,
                    });
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
            events.push(self.handler_binding(attr, event)?);
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

    /// The handler for an `onXxx` attribute. Two shapes are accepted:
    ///
    /// - a function reference `onClick={onPress}` → a *reference* binding named
    ///   `onPress` (resolved later by the IR/document);
    /// - an inline arrow `onClick={() => setCount(count() + 1)}` → the arrow body
    ///   is translated to Rust and carried as an *inline* body on the binding.
    fn handler_binding(&mut self, attr: &ast::JSXAttr, event: EventName) -> Result<EventBinding> {
        match &attr.value {
            Some(ast::JSXAttrValue::JSXExprContainer(container)) => {
                if let ast::JSXExpr::Expr(expr) = &container.expr {
                    match expr.as_ref() {
                        ast::Expr::Ident(id) => {
                            let name = id.sym.as_str().to_string();
                            self.handler_bindings.insert(name.clone(), ());
                            return Ok(EventBinding::new(event, name));
                        }
                        ast::Expr::Arrow(arrow) => {
                            let body = self.translate_handler(arrow)?;
                            return Ok(EventBinding::inline(event, body));
                        }
                        _ => {}
                    }
                }
                Err(self.err(
                    container.span,
                    "event handlers must be a function reference (e.g. onClick={onPress}) or an inline arrow (e.g. onClick={() => setCount(count() + 1)})",
                ))
            }
            _ => Err(self.err(attr.span, "event handlers need a `={...}` expression")),
        }
    }

    /// Translate an inline handler arrow into the Rust fragment for its body.
    fn translate_handler(&self, arrow: &ast::ArrowExpr) -> Result<String> {
        // The emitter's event setters take at most one payload argument and
        // name it `_v` (see `handler_closure("_v", …)` in `floom-dsl`). An
        // inline arrow that declares `(v) => …` therefore has to use that same
        // name in its body, or the emitted closure references an undefined
        // binding. Zero-argument arrows are left alone.
        let renamed = match arrow.params.len() {
            0 => None,
            1 => match arrow.params.first() {
                Some(pat) => Some(pat_ident_name(pat).ok_or_else(|| {
                    self.err(
                        arrow.span,
                        "inline event handlers take a plain identifier parameter (e.g. `(value) => …`)",
                    )
                })?),
                None => None,
            },
            _ => {
                return Err(self.err(
                    arrow.span,
                    "inline event handlers take at most one parameter (e.g. `(value) => …`)",
                ))
            }
        };
        let span = arrow.span;
        let body = match arrow.body.as_ref() {
            ast::BlockStmtOrExpr::Expr(e) => self.translate_expr(e)?,
            ast::BlockStmtOrExpr::BlockStmt(block) => {
                let mut out = String::new();
                for stmt in &block.stmts {
                    match stmt {
                        ast::Stmt::Expr(es) => {
                            out.push_str(&self.translate_expr(&es.expr)?);
                            out.push(';');
                        }
                        ast::Stmt::Return(r) => {
                            out.push_str("return ");
                            if let Some(arg) = &r.arg {
                                out.push_str(&self.translate_expr(arg)?);
                            }
                            out.push(';');
                        }
                        _ => return Err(self.err(span, "unsupported statement in inline handler")),
                    }
                }
                out
            }
        };
        // `to_snake` is applied to identifiers, so a camelCase parameter
        // (`newValue`) must be renamed in its snake form.
        Ok(match renamed {
            Some(param) => replace_ident(&body, &to_snake(&param), "_v"),
            None => body,
        })
    }

    /// Translate a JS expression into a Rust fragment, rewriting signal reads and
    /// setter calls. The supported subset is deliberately closed: anything beyond
    /// identifiers, literals, calls, binary/unary ops, member access, parens, and
    /// arrows is rejected with a span error rather than silently mis-emitted.
    fn translate_expr(&self, expr: &ast::Expr) -> Result<String> {
        match expr {
            ast::Expr::Ident(id) => Ok(to_snake(id.sym.as_str())),
            ast::Expr::Lit(lit) => Ok(lit_to_rust(lit)),
            ast::Expr::Paren(p) => Ok(format!("({})", self.translate_expr(&p.expr)?)),
            ast::Expr::Call(call) => self.translate_call(call),
            ast::Expr::Bin(bin) => Ok(format!(
                "{} {} {}",
                self.translate_expr(&bin.left)?,
                bin_op(bin.op),
                self.translate_expr(&bin.right)?
            )),
            ast::Expr::Unary(u) => Ok(format!(
                "{}{}",
                unary_op(u.op),
                self.translate_expr(&u.arg)?
            )),
            ast::Expr::Member(m) => {
                let obj = self.translate_expr(&m.obj)?;
                let prop = match &m.prop {
                    ast::MemberProp::Ident(i) => to_snake(i.sym.as_str()),
                    _ => return Err(self.err(m.span, "unsupported computed member access")),
                };
                Ok(format!("{obj}.{prop}"))
            }
            ast::Expr::Cond(cond) => Ok(format!(
                "if {} {{ {} }} else {{ {} }}",
                self.translate_expr(&cond.test)?,
                self.translate_expr(&cond.cons)?,
                self.translate_expr(&cond.alt)?
            )),
            ast::Expr::Arrow(arrow) => self.translate_handler(arrow),
            _ => Err(self.err(expr.span(), "unsupported expression in inline handler")),
        }
    }

    /// Translate a call, rewriting signal reads and setters.
    fn translate_call(&self, call: &ast::CallExpr) -> Result<String> {
        let callee = match &call.callee {
            ast::Callee::Expr(e) => e,
            _ => return Err(self.err(call.span, "unsupported call callee")),
        };
        let name = match callee.as_ref() {
            ast::Expr::Ident(id) => id.sym.as_str().to_string(),
            _ => return Err(self.err(call.span, "unsupported call callee")),
        };
        let args = call
            .args
            .iter()
            .map(|a| self.translate_expr(&a.expr))
            .collect::<Result<Vec<_>>>()?;
        let joined = args.join(", ");
        // `setCount(x)` → `count.set(x)` (setter name resolves to its signal).
        if let Some(signal) = self.setters.get(&name) {
            return Ok(format!("{signal}.set({joined})"));
        }
        // `count()` → `count.get()` (a zero-arg call to a known signal).
        if args.is_empty() && self.signals.contains(&name) {
            return Ok(format!("{name}.get()"));
        }
        Ok(format!("{}({})", to_snake(&name), joined))
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
        let spec_ty = floom_dsl::ui_ir::prop_spec(kind, name).map(|spec| spec.ty);
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
                let ty = floom_dsl::ui_ir::prop_spec(kind, name)
                    .map(|spec| spec.ty)
                    .unwrap_or(ValueType::Text);
                self.value_bindings.insert(sig.clone(), ty);
                Ok(PropValue::Ref(sig))
            }
            ast::Expr::Call(call) => self.call_value(node_key, kind, name, call),
            ast::Expr::Lit(lit) => {
                let spec_ty = floom_dsl::ui_ir::prop_spec(kind, name).map(|spec| spec.ty);
                self.lit_value(lit, spec_ty)
            }
            // A computed expression (`checked() ? "on" : "off"`, `count() * 2`, …):
            // it owns no binding, so it becomes a `PropValue::Computed` carrying the
            // Rust that reproduces it. The emitter inlines it at every read site,
            // which recomputes it from the signals it names each frame — the same
            // pull-based reactivity the framework already uses for `Ref` reads.
            ast::Expr::Cond(_) | ast::Expr::Bin(_) | ast::Expr::Unary(_)
            | ast::Expr::Member(_) | ast::Expr::Paren(_) => {
                let rust = self.translate_expr(expr)?;
                Ok(PropValue::Computed(rust))
            }
            _ => Err(self.err(
                span,
                "only literal values, signal refs, signal calls, and computed expressions are supported here",
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
        if let ast::Callee::Expr(e) = &call.callee
            && let ast::Expr::Ident(id) = e.as_ref()
        {
            let callee = id.sym.as_str().to_string();
            if matches!(
                ModuleExport::from_name(&callee),
                Some(ModuleExport::CreateSignal) | Some(ModuleExport::CreateMemo)
            ) {
                return Err(self.err(
                    call.span,
                    format!("{callee} must be bound with `const [name] = ...` at module scope"),
                ));
            }
            if call.args.is_empty() {
                let ty = floom_dsl::ui_ir::prop_spec(kind, name)
                    .map(|spec| spec.ty)
                    .unwrap_or(ValueType::Text);
                self.value_bindings.insert(callee.clone(), ty);
                return Ok(PropValue::Ref(callee));
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
                        let node = UiNode::new(key.clone(), NodeKind::Text)
                            .with_props([Prop::new("label", PropValue::Text(trimmed.to_string()))]);
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
                        return Err(
                            self.err(cond.span, "the `?:` then-branch must be a JSX element")
                        );
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
                        return Err(self.err(call.span, ".map body must be an expression"));
                    }
                }
            }
            _ => return Err(self.err(call.span, ".map expects an arrow returning JSX")),
        };
        let child_key = self.element(template)?;
        let key = self.next_key();
        let list =
            UiNode::new(key.clone(), NodeKind::List).with_slot(Slot::default_slot(vec![child_key]));
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
    let entry = crate::entry::resolve(parsed)?;
    lower_with_root(parsed, &entry.root)
}

/// Lower a parsed TSX module and surface the app/window config.
///
/// Resolution of the entry (`mount`, the `<App>` wrapper, `window.*` calls) lives
/// in [`entry::resolve`](crate::entry::resolve); this lowers the resolved root into a document.
pub fn lower_with_app(parsed: &Parsed) -> Result<(UiDocument, crate::entry::AppConfig)> {
    let entry = crate::entry::resolve(parsed)?;
    let doc = lower_with_root(parsed, &entry.root)?;
    Ok((doc, entry.config))
}

/// Lower a resolved root (`mount` target, default export, or `<App>` child) into
/// a validated `UiDocument`. Entry resolution and window config live upstream in
/// [`entry::resolve`](crate::entry::resolve); this only turns JSX into nodes and bindings.
pub fn lower_with_root(parsed: &Parsed, root_jsx: &Option<RootJsx>) -> Result<UiDocument> {
    // Module-level signal/context declarations are the authoritative bindings;
    // JSX-derived refs/handlers reference them and must not be duplicated.
    let module_signals = collect_module_signals(&parsed.module);
    let module_bindings = module_signals.bindings.clone();

    let mut lower = Lower {
        parsed,
        nodes: Vec::new(),
        value_bindings: HashMap::new(),
        handler_bindings: HashMap::new(),
        signals: module_signals.signals,
        setters: module_signals.setters,
        counter: 0,
    };
    let root = match root_jsx {
        Some(RootJsx::Element(el)) => lower.element(el)?,
        Some(RootJsx::Fragment(frag)) => {
            let key = lower.next_key();
            let children = lower.children(&frag.children)?;
            let node =
                UiNode::new(key.clone(), NodeKind::Stack).with_slot(Slot::default_slot(children));
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
        bindings
            .entry(name.clone())
            .or_insert_with(|| Binding::handler(name));
    }
    let bindings: Vec<Binding> = bindings.into_values().collect();

    let doc = UiDocument {
        version: floom_dsl::ui_ir::IrVersion::CURRENT,
        root,
        bindings,
        nodes: lower.nodes,
    };
    doc.validate()
        .map_err(|e| TsxError::new(format!("IR validation failed: {e}")))?;
    Ok(doc)
}

/// The signal bookkeeping collected from a module.
#[derive(Default)]
struct ModuleSignals {
    /// All bindings (signal/context declarations) the module exposes.
    bindings: Vec<Binding>,
    /// Every declared signal name (`count` of `const [count, …] = createSignal`).
    signals: HashSet<String>,
    /// Setter name → signal name (`setCount` → `count`).
    setters: HashMap<String, String>,
}

/// Recognize signal/context declarations anywhere in a module and turn them into
/// `Binding`s plus the signal/setter maps the inline handler translator needs.
///
/// This is the compile-time half of the reactive shim: a TSX `createSignal`
/// becomes a `floom_signal` value binding (and records its setter), `provideContext`
/// / `useContext` become context bindings, and `createEffect` is recognized but
/// emits no binding (its body runs as an effect at runtime).
///
/// Scanning is **recursive into function bodies** (e.g. `function App() { const
/// [x, setX] = createSignal(0) }`), not just the module's top level — TSX authors
/// declare signals inside components, so a top-level-only scan would miss them and
/// the translator would fail to rewrite `setX(...)` / `x()`.
fn collect_module_signals(module: &ast::Module) -> ModuleSignals {
    let mut out = ModuleSignals::default();
    for item in &module.body {
        collect_module_item(item, &mut out);
    }
    out
}

/// Recurse into a module item.
fn collect_module_item(item: &ast::ModuleItem, out: &mut ModuleSignals) {
    if let ast::ModuleItem::Stmt(stmt) = item {
        collect_stmt(stmt, out)
    }
}

/// Recurse into a statement, descending into block/function bodies where signals
/// are typically declared.
fn collect_stmt(stmt: &ast::Stmt, out: &mut ModuleSignals) {
    match stmt {
        ast::Stmt::Decl(ast::Decl::Var(var)) => {
            for decl in &var.decls {
                if let Some(b) = var_decl_signal(decl) {
                    out.bindings.push(b.clone());
                    out.signals.insert(b.name.clone());
                    if let Some((signal, setter)) = var_decl_signal_setter(decl) {
                        out.setters.insert(setter, signal);
                    }
                }
            }
        }
        ast::Stmt::Decl(ast::Decl::Fn(f)) => collect_fn(&f.function, out),
        ast::Stmt::Expr(expr_stmt) => {
            if let Some(b) = expr_stmt_signal(&expr_stmt.expr) {
                out.bindings.push(b.clone());
                out.signals.insert(b.name.clone());
            }
            collect_expr(&expr_stmt.expr, out);
        }
        ast::Stmt::Block(block) => collect_block(block, out),
        _ => {}
    }
}

fn collect_block(block: &ast::BlockStmt, out: &mut ModuleSignals) {
    for stmt in &block.stmts {
        collect_stmt(stmt, out);
    }
}

fn collect_fn(f: &ast::Function, out: &mut ModuleSignals) {
    if let Some(body) = &f.body {
        collect_block(body, out);
    }
}

fn collect_expr(expr: &ast::Expr, out: &mut ModuleSignals) {
    match expr {
        ast::Expr::Fn(f) => collect_fn(&f.function, out),
        ast::Expr::Arrow(arrow) => {
            if let ast::BlockStmtOrExpr::BlockStmt(block) = arrow.body.as_ref() {
                collect_block(block, out);
            }
        }
        ast::Expr::Call(call) => {
            if let ast::Callee::Expr(e) = &call.callee {
                collect_expr(e, out);
            }
            for arg in &call.args {
                collect_expr(&arg.expr, out);
            }
        }
        _ => {}
    }
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
    match ModuleExport::from_name(&callee) {
        Some(ModuleExport::CreateSignal) | Some(ModuleExport::CreateMemo) => {
            // `[count, setCount]` or `double`.
            let name = array_first_name(&decl.name).or_else(|| pat_ident_name(&decl.name))?;
            let first = call.args.first();
            let ty = first
                .map(|a| lit_type(a.expr.as_ref()))
                .unwrap_or(ValueType::Text);
            // A signal's initial value is part of its observable behaviour, so
            // carry it through to the emitter. Only literals are representable;
            // a computed initial (`createSignal(f())`) falls back to the type
            // default, which is the same compromise the old code made for all.
            let init = first.and_then(|a| expr_lit_value(a.expr.as_ref(), ty));
            Some(Binding::value_with_init(name, ty, init))
        }
        Some(ModuleExport::UseContext) => {
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
    match ModuleExport::from_name(&callee) {
        // Recognized but emits no binding: its body runs as an effect.
        Some(ModuleExport::CreateEffect) => None,
        Some(ModuleExport::ProvideContext) => {
            let name = {
                let a = call.args.first()?;
                match a.expr.as_ref() {
                    ast::Expr::Lit(ast::Lit::Str(s)) => {
                        s.value.as_str().unwrap_or_default().to_string()
                    }
                    _ => return None,
                }
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
    if let ast::Callee::Expr(e) = &call.callee
        && let ast::Expr::Ident(id) = e.as_ref()
    {
        return Some(id.sym.as_str().to_string());
    }
    None
}

/// The identifier bound by `[a, b]` destructuring, or `None`.
fn array_first_name(pat: &ast::Pat) -> Option<String> {
    if let ast::Pat::Array(arr) = pat
        && let Some(Some(ast::Pat::Ident(id))) = arr.elems.first()
    {
        return Some(id.id.sym.as_str().to_string());
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

/// Replace `from` with `to` where it appears as a whole Rust identifier in
/// `body`.
///
/// A textual replacement would rewrite `count` inside `discount`, so the scan
/// only accepts a match whose neighbours are not identifier characters. This is
/// enough for the closed inline-handler grammar: the body is a generated Rust
/// fragment, not arbitrary user text.
fn replace_ident(body: &str, from: &str, to: &str) -> String {
    if from.is_empty() || from == to {
        return body.to_string();
    }
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(pos) = rest.find(from) {
        let before = rest[..pos].chars().next_back();
        let after = rest[pos + from.len()..].chars().next();
        let boundary_before = before.is_none_or(|c| !is_ident_char(c));
        let boundary_after = after.is_none_or(|c| !is_ident_char(c));
        if boundary_before && boundary_after {
            out.push_str(&rest[..pos]);
            out.push_str(to);
        } else {
            out.push_str(&rest[..pos + from.len()]);
        }
        rest = &rest[pos + from.len()..];
    }
    out.push_str(rest);
    out
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `const [count, setCount] = createSignal(0)` → `("count", "setCount")`.
///
/// Returns the signal and setter names so the inline handler translator can rewrite
/// `setCount(x)` to `count.set(x)`. Only array patterns from `createSignal` /
/// `createMemo` qualify.
fn var_decl_signal_setter(decl: &ast::VarDeclarator) -> Option<(String, String)> {
    let init = decl.init.as_ref()?;
    let call = match init.as_ref() {
        ast::Expr::Call(c) => c,
        _ => return None,
    };
    let callee = call_name(call)?;
    if !matches!(
        ModuleExport::from_name(&callee),
        Some(ModuleExport::CreateSignal) | Some(ModuleExport::CreateMemo)
    ) {
        return None;
    }
    let ast::Pat::Array(arr) = &decl.name else {
        return None;
    };
    let signal = match arr.elems.first() {
        Some(Some(ast::Pat::Ident(id))) => id.id.sym.as_str().to_string(),
        _ => return None,
    };
    let setter = match arr.elems.get(1) {
        Some(Some(ast::Pat::Ident(id))) => id.id.sym.as_str().to_string(),
        _ => return None,
    };
    Some((signal, setter))
}

/// Convert a JS identifier to Rust `snake_case` (`setCount` → `set_count`).
fn to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        if c.is_uppercase() {
            if let Some(prev) = out.chars().last()
                && (prev.is_lowercase() || prev.is_ascii_digit())
            {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Render a JS literal as a Rust literal fragment.
fn lit_to_rust(lit: &ast::Lit) -> String {
    match lit {
        ast::Lit::Num(n) => {
            if n.value.fract() == 0.0 {
                format!("{:.0}", n.value)
            } else {
                n.value.to_string()
            }
        }
        // String signals are emitted as `SyncBaseSignal<String>`, so a literal
        // assigned through `setX(...)` must be a `String`, not a `&str`.
        ast::Lit::Str(s) => format!("\"{}\".to_string()", s.value.as_str().unwrap_or_default()),
        ast::Lit::Bool(b) => b.value.to_string(),
        _ => String::new(),
    }
}

/// Map a JS binary operator to its Rust spelling (`===` → `==`, `!==` → `!=`).
fn bin_op(op: ast::BinaryOp) -> &'static str {
    match op {
        ast::BinaryOp::EqEqEq => "==",
        ast::BinaryOp::NotEqEq => "!=",
        ast::BinaryOp::EqEq => "==",
        ast::BinaryOp::NotEq => "!=",
        ast::BinaryOp::Lt => "<",
        ast::BinaryOp::LtEq => "<=",
        ast::BinaryOp::Gt => ">",
        ast::BinaryOp::GtEq => ">=",
        ast::BinaryOp::Add => "+",
        ast::BinaryOp::Sub => "-",
        ast::BinaryOp::Mul => "*",
        ast::BinaryOp::Div => "/",
        ast::BinaryOp::Mod => "%",
        ast::BinaryOp::LogicalAnd => "&&",
        ast::BinaryOp::LogicalOr => "||",
        _ => "?",
    }
}

/// Map a JS unary operator to its Rust spelling.
fn unary_op(op: ast::UnaryOp) -> &'static str {
    match op {
        ast::UnaryOp::Bang => "!",
        ast::UnaryOp::Minus => "-",
        ast::UnaryOp::Plus => "+",
        _ => "",
    }
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
///
/// Numeric literals are inferred as `Number`, not `Int`, even when they have no
/// fractional part: a signal's type is decided by how it is *written* (a setter
/// call from a slider yields `f64`), and the two numeric seats a widget reads —
/// [`Slider`](crate::emit) and [`Progress`](crate::emit) — both want `f64`. An
/// `Int` signal would force every write site to cast.
fn lit_type(expr: &ast::Expr) -> ValueType {
    match expr {
        ast::Expr::Lit(ast::Lit::Num(_)) => ValueType::Number,
        ast::Expr::Lit(ast::Lit::Str(_)) => ValueType::Text,
        ast::Expr::Lit(ast::Lit::Bool(_)) => ValueType::Bool,
        _ => ValueType::Text,
    }
}

/// Turn a signal's initial expression into a `PropValue` when it is a literal
/// representable in the IR. `None` means "no literal", so the emitter seeds
/// the type's default instead of inventing a value.
fn expr_lit_value(expr: &ast::Expr, ty: ValueType) -> Option<PropValue> {
    match (expr, ty) {
        (ast::Expr::Lit(ast::Lit::Str(s)), ValueType::Text | ValueType::Ident) => Some(
            PropValue::Text(s.value.as_str().unwrap_or_default().to_string()),
        ),
        (ast::Expr::Lit(ast::Lit::Bool(b)), ValueType::Bool) => Some(PropValue::Bool(b.value)),
        (ast::Expr::Lit(ast::Lit::Num(n)), ValueType::Int) => Some(PropValue::Int(n.value as i64)),
        (ast::Expr::Lit(ast::Lit::Num(n)), _) => Some(PropValue::Number(n.value)),
        _ => None,
    }
}
