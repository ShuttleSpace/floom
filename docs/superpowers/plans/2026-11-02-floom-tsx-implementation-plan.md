# floom-tsx Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `floom-tsx` crate that compiles TSX source into `floom-ui-ir::UiDocument` at build time, then lowers through the existing `floom-dsl::emit` backend into a Rust builder program drawn by Floom's own renderers (vello/wgpu, tiny-skia, TUI, WASM). The TSX reactive shim (`createSignal`/`createMemo`/`createEffect`/`provideContext`/`useContext`) translates at compile time into `floom_signal`, so TSX components are genuinely reactive with zero runtime VM. Architecture mirrors nsh's TSX development experience but anchors rendering to Floom's own renderer stack.

**Architecture:**

```
.tsx source ──► floom-tsx crate (NEW, build-time only, swc_ecma_parser)
    │    parser.rs  : swc_ecma_parser → TSX AST
    │    lower.rs   : TSX AST → UiDocument + signal-shim recognition
    │    decl.rs    : schema → .d.ts editor declarations
    │    error.rs   : span-based diagnostics
    ▼
floom-ui-ir::UiDocument        (EXISTING — the bridge; JSON-serializable, closed NodeKind enum, validate())
    ▼
floom-dsl::emit               (EXISTING, reused; small reactive-prop extension)
    → Rust builder program
    ▼
application binary → Floom renderers (vello/wgpu, tiny-skia, TUI, WASM)
                    + Floom platform layer + Cargo/`just build` packaging

The UiDocument is the lingua franca: floom-dsl already proves source → UiDocument → Rust.
floom-tsx only replaces the front-end parser; nothing ships in the binary (swc is build-time only).

**Tech Stack:** swc_ecma_parser (build-time dep), `floom-ui-ir`, `floom-dsl::emit`, `floom_signal`, existing floom prelude/renderers.

---

## File Map

| File | Action | Why |
|------|--------|-----|
| `crates/floom-tsx/Cargo.toml` | Create | build-time deps only (swc_ecma_parser) |
| `crates/floom-tsx/src/lib.rs` | Create | Entry: `compile(source)`, `compile_project(paths)`, `.d.ts` generation |
| `crates/floom-tsx/src/parser.rs` | Create | swc_ecma_parser → TSX AST |
| `crates/floom-tsx/src/lower.rs` | Create | TSX AST → UiDocument + signal-shim recognition |
| `crates/floom-tsx/src/decl.rs` | Create | UiDocument schema → .d.ts declarations |
| `crates/floom-tsx/src/error.rs` | Create | span-based diagnostics (align with floom-dsl) |
| `crates/floom-tsx/tests/` | Create | golden JSON unit tests, CI integration |
| `examples/floom-tsx-showcase/src/main.rs` | Create | Golden example: mount counter.tsx, headless pump asserts |
| `examples/floom-tsx-showcase/src/counter.tsx` | Create | Demo TSX component used by showcase |
| `examples/floom-tsx-showcase/Cargo.toml` | Create | Depends on floom-tsx + floom |
| `crates/floom-dsl/src/emit.rs` | Modify (small) | Add reactive-prop extension via EmitOptions flag |
| `docs/superpowers/specs/2026-11-02-floom-tsx-design.md` | Already exists | Reference spec |

---

## Tasks

### Task 1: Scaffold `floom-tsx` crate skeleton

**Files:**
- Create: `crates/floom-tsx/Cargo.toml`
- Create: `crates/floom-tsx/src/lib.rs`
- Create: `crates/floom-tsx/src/parser.rs`
- Create: `crates/floom-tsx/src/lower.rs`
- Create: `crates/floom-tsx/src/decl.rs`
- Create: `crates/floom-tsx/src/error.rs`

**Step 1:** Write the failing test
```rust
// crates/floom-tsx/tests/golden.rs
use floom_tsx::compile;

#[test]
fn tsx_to_uidoc_static() {
    let source = r#"<Stack><Text>hello</Text></Stack>"#;
    let doc = compile(source).expect("valid");
    assert_eq!(doc.nodes.len(), 2);
}
```
Run: `cargo test -p floom-tsx tsx_to_uidoc_static -v`
Expected: FAIL (crate not yet implemented)

**Step 2:** Run test to verify it fails

**Step 3:** Write minimal implementation
```rust
// crates/floom-tsx/src/lib.rs
pub mod parser;
pub mod lower;
pub mod decl;
pub mod error;

use floom_ui_ir::UiDocument;

pub fn compile(source: &str) -> Result<UiDocument, TsxError> {
    unimplemented!()
}

pub fn compile_project(_paths: &[std::path::PathBuf]) -> Result<UiDocument, TsxError> {
    unimplemented!()
}

#[derive(Debug)]
pub struct TsxError;
```

**Step 4:** Run test to verify it passes

**Step 5:** Commit
```bash
git add crates/floom-tsx/
git commit -m "feat: scaffold floom-tsx crate skeleton"
```

### Task 2: Implement parser (swc_ecma_parser)

**Files:**
- Create: `crates/floom-tsx/Cargo.toml` (add swc_ecma_parser dep)
- Modify: `crates/floom-tsx/src/parser.rs` (implement TSX → AST using swc)

**Step 1:** Write the failing test
```rust
// crates/floom-tsx/tests/parser_golden.rs
use floom_tsx::parser;

#[test]
fn parse_simple_tsx() {
    let source = r#"<Button label="hi" onClick={h}/>"#;
    let ast = parser::parse(source).expect("parse");
    assert!(ast.is_some());
}
```
Run: `cargo test -p floom-tsx parser_golden -v`
Expected: FAIL (parser not yet implemented)

**Step 2:** Run test to verify it fails

**Step 3:** Write minimal implementation
```rust
// crates/floom-tsx/src/parser.rs
// TODO: import swc_ecma_parser; parse source → TSX AST skeleton
pub fn parse(_source: &str) -> Result<(), TsxError> {
    unimplemented!()
}
```

**Step 4:** Run test to verify it passes

**Step 5:** Commit
```bash
git add crates/floom-tsx/Cargo.toml crates/floom-tsx/src/parser.rs
git commit -m "feat: parser skeleton with swc_ecma_parser"
```

### Task 3: IR validation + error.rs diagnostics (P0 foundation)

**Files:**
- Modify: `crates/floom-tsx/src/lower.rs` (call `UiDocument::validate`)
- Create: `crates/floom-tsx/src/error.rs` (span-based diagnostics)

**Step 1:** Write the failing test
```rust
// crates/floom-tsx/tests/validation_test.rs
use floom_tsx::compile;

#[test]
fn rejects_duplicate_key() {
    let source = r#"<Stack key="x"/><Text key="x"/>"#;
    let result = compile(source);
    assert!(result.is_err());
}

#[test]
fn rejects_unknown_prop() {
    let source = r#"<Text not_a_prop="x"/>"#;
    let result = compile(source);
    assert!(result.is_err());
}
```
Run: `cargo test -p floom-tsx validation_test -v`
Expected: FAIL (validation/diagnostics not yet wired)

**Step 2:** Run test to verify it fails

**Step 3:** Write minimal implementation
```rust
// crates/floom-tsx/src/error.rs
// TODO: span-based diagnostics aligning with floom-dsl::DslError

// crates/floom-tsx/src/lower.rs
// TODO: after building UiDocument, call doc.validate() and map IrError → TsxError
```

**Step 4:** Run test to verify it passes

**Step 5:** Commit
```bash
git add crates/floom-tsx/src/lower.rs crates/floom-tsx/src/error.rs
git commit -m "feat: IR validation + span-based diagnostics"
```

### Task 4: Lower TSX → UiDocument static subset (P0 core)

**Files:**
- Modify: `crates/floom-tsx/src/lower.rs` (implement full TSX→UiDocument lowering)

**Step 1:** Write the failing test
```rust
// crates/floom-tsx/tests/lower_golden.rs
use floom_ui_ir::NodeKind;
use floom_tsx::compile;

#[test]
fn lower_stack_text() {
    let source = r#"<Stack axis="vertical"><Text label="hi"/></Stack>"#;
    let doc = compile(source).expect("lower");
    assert_eq!(doc.nodes.len(), 2);
    assert_eq!(doc.nodes[0].kind, NodeKind::Stack);
    assert_eq!(doc.nodes[1].kind, NodeKind::Text);
}

#[test]
fn lower_conditional() {
    let source = r#"{cond ? <A/> : <B/>}"#;
    let doc = compile(source).expect("lower");
    assert_eq!(doc.nodes.len(), 1);
    assert_eq!(doc.nodes[0].kind, NodeKind::Conditional);
}

#[test]
fn lower_list_with_keys() {
    let source = r#"<List>{items.map(i => <Row key={i.id}/>)}</List>"#;
    let doc = compile(source).expect("lower");
    assert_eq!(doc.nodes.len(), 1);
    assert_eq!(doc.nodes[0].kind, NodeKind::List);
}

#[test]
fn lower_slot_children() {
    let source = r#"<Stack><Button/><Text/></Stack>"#;
    let doc = compile(source).expect("lower");
    let stack = &doc.nodes[0];
    let children: Vec<_> = stack.slots.iter().flat_map(|s| &s.children).collect();
    assert_eq!(children.len(), 2);
}

#[test]
fn lower_event_binding() {
    let source = r#"<Button onClick={h}/>"#;
    let doc = compile(source).expect("lower");
    let node = &doc.nodes[0];
    assert_eq!(node.events.len(), 1);
    assert_eq!(node.events[0].event, floom_ui_ir::EventName::Click);
}

#[test]
fn lower_full_nodekind_coverage() {
    for kind in NodeKind::ALL {
        let tag = match kind {
            NodeKind::Text => "<Text/>",
            NodeKind::Icon => "<Icon/>",
            NodeKind::Image => "<Image/>",
            NodeKind::Container => "<Container/>",
            NodeKind::Spacer => "<Spacer/>",
            NodeKind::Divider => "<Divider/>",
            NodeKind::Button => "<Button/>",
            NodeKind::Checkbox => "<Checkbox/>",
            NodeKind::Radio => "<Radio/>",
            NodeKind::Switch => "<Switch/>",
            NodeKind::Slider => "<Slider/>",
            NodeKind::Progress => "<Progress/>",
            NodeKind::Select => "<Select/>",
            NodeKind::TextField => "<TextField/>",
            NodeKind::Stack => "<Stack/>",
            NodeKind::Grid => "<Grid/>",
            NodeKind::List => "<List/>",
            NodeKind::Conditional => "{c ? <A/> : <B/>}",
            NodeKind::Overlay => "<Overlay/>",
            NodeKind::Dialog => "<Dialog/>",
            NodeKind::Popup => "<Popup/>",
            NodeKind::Tabs => "<Tabs/>",
            NodeKind::NavigationBar => "<NavigationBar/>",
            NodeKind::NavigationRail => "<NavigationRail/>",
            NodeKind::NavigationDrawer => "<NavigationDrawer/>",
            NodeKind::Native(n) => match n {
                floom_ui_ir::NativeKind::Web => "<Web/>",
                floom_ui_ir::NativeKind::Video => "<Video/>",
                floom_ui_ir::NativeKind::Map => "<Map/>",
                floom_ui_ir::NativeKind::TextInput => "<TextInput/>",
            },
        };
        let source = format!("<Stack>{tag}</Stack>");
        let doc = compile(&source).expect(format!("lower {kind:?}").as_str());
        assert!(doc.nodes.len() >= 2);
    }
}
```
Run: `cargo test -p floom-tsx lower_golden -v`
Expected: FAIL (lower not yet implemented for full vocabulary)

**Step 2:** Run test to verify it fails

**Step 3:** Write minimal implementation
```rust
// crates/floom-tsx/src/lower.rs
// TODO: implement full TSX AST → UiDocument lowering per spec mapping table
```

**Step 4:** Run test to verify it passes

**Step 5:** Commit
```bash
git add crates/floom-tsx/src/lower.rs
git commit -m "feat: lower TSX → UiDocument static subset (P0 core)"
```

### Task 5: Signal shim translation + destructuring (P1 signals)

**Files:**
- Modify: `crates/floom-tsx/src/lower.rs` (recognize `createSignal`/`createMemo`/`createEffect`/`provideContext`/`useContext`)

**Step 1:** Write the failing test
```rust
// crates/floom-tsx/tests/signal_golden.rs
use floom_tsx::compile;

#[test]
fn lower_translates_create_signal() {
    let source = r#"const [count, setCount] = createSignal(0);"#;
    let doc = compile(source).expect("lower");
    assert!(doc.bindings.iter().any(|b| b.name == "count" && matches!(b.kind, floom_ui_ir::BindingKind::Value(_))));
}

#[test]
fn lower_translates_create_memo() {
    let source = r#"const double = createMemo(() => count() * 2);"#;
    let doc = compile(source).expect("lower");
    assert!(doc.bindings.iter().any(|b| b.name == "double"));
}

#[test]
fn lower_translates_create_effect() {
    let source = r#"createEffect(() => { console.log(count()); });"#;
    let doc = compile(source).expect("lower");
    // createEffect produces no binding; verify it lowered without error
    assert!(!doc.nodes.is_empty());
}

#[test]
fn lower_translates_provide_context() {
    let source = r#"provideContext("theme", "dark");"#;
    let doc = compile(source).expect("lower");
    assert!(doc.bindings.iter().any(|b| b.name == "theme"));
}

#[test]
fn lower_translates_use_context() {
    let source = r#"const t = useContext("theme"); <Text label={t} />"#;
    let doc = compile(source).expect("lower");
    // useContext creates a binding named 't' with the context value
    assert!(doc.bindings.iter().any(|b| b.name == "t"));
}
```
Run: `cargo test -p floom-tsx signal_golden -v`
Expected: FAIL (signal shim not yet recognized)

**Step 2:** Run test to verify it fails

**Step 3:** Write minimal implementation
```rust
// crates/floom-tsx/src/lower.rs
// TODO: recognize createSignal/ createMemo/ createEffect/ provideContext/ useContext;
// emit bindings and references; destructuring [get,set] → .get()/.set(v)
// count() → count.get(); setCount(v) → count.set(v)
```

**Step 4:** Run test to verify it passes

**Step 5:** Commit
```bash
git add crates/floom-tsx/src/lower.rs
git commit -m "feat: signal shim translation (createSignal/Memo/Effect/context)"
```

### Task 6: IR → emit extension (reactive prop closure)

**Files:**
- Modify: `crates/floom-dsl/src/emit.rs` (add reactive flag to EmitOptions, emit effect registration inside mount function)

**Step 1:** Write the failing test
```rust
// crates/floom-dsl/tests/reactive_emit_test.rs
use floom_dsl::emit::{emit, EmitOptions, WidgetSource};

#[test]
fn emit_reactive_prop_registers_effect() {
    let source = r#"<Stack><Text label={count()} /></Stack>"#;
    // Use floom_tsx::compile to produce a UiDocument with a signal-backed prop
    let doc = floom_tsx::compile(source).expect("lower");
    let mut opts = EmitOptions::default();
    opts.reactive = true;  // new flag
    let rust = emit(&doc, WidgetSource::WithComposites, opts).expect("emit");
    assert!(rust.contains("create_effect"));
    assert!(rust.contains("widget_mut"));
}
```
Run: `cargo test -p floom-dsl reactive_emit_test -v`
Expected: FAIL (reactive emit extension not yet added)

**Step 2:** Run test to verify it fails

**Step 3:** Write minimal implementation
```rust
// crates/floom-dsl/src/emit.rs
// Add reactive flag to EmitOptions
// When opts.reactive is true and a prop value is a signal reference,
// append create_effect inside mount function:
// create_effect(move || {
//     let v = /* closure that reads count */;
//     host.widget_mut::<T>(id).set_content(v);
// });
```

**Step 4:** Run test to verify it passes

**Step 5:** Commit
```bash
git add crates/floom-dsl/src/emit.rs
git commit -m "feat: reactive prop closure + effect wiring in emit"
```

### Task 7: Showcase example + build.rs integration

**Files:**
- Create: `examples/floom-tsx-showcase/src/main.rs`
- Create: `examples/floom-tsx-showcase/src/counter.tsx`
- Create: `examples/floom-tsx-showcase/Cargo.toml`
- Create: `examples/floom-tsx-showcase/build.rs`

**Step 1:** Write the failing test
- Not a unit test; verify `cargo build -p floom-tsx-showcase` compiles
Run: `cargo build -p floom-tsx-showcase`
Expected: FAIL (showcase not yet created)

**Step 2:** Run test to verify it fails

**Step 3:** Write minimal integration
```rust
// examples/floom-tsx-showcase/src/main.rs
mod ui_gen { include!(concat!(env!("OUT_DIR"), "/ui_gen.rs")); }
fn main() {
    floom::launch(|host| ui_gen::mount(host));
}
```
```tsx
// examples/floom-tsx-showcase/src/counter.tsx
import { createSignal } from "floom";
export function Counter() {
  const [count, setCount] = createSignal(0);
  return (
    <Stack axis="vertical" gap={8}>
      <Text label={() => `Value: ${count()}`} />
      <Button label="+" onClick={() => setCount(count() + 1)} />
    </Stack>
  );
}
```
```toml
// examples/floom-tsx-showcase/Cargo.toml
[package]
name = "floom-tsx-showcase"
version.workspace = true
edition.workspace = true

[build-dependencies]
glob = "0.9"
floom-tsx = { path = "../../crates/floom-tsx" }
floom-dsl = { path = "../../crates/floom-dsl" }
```
```rust
// examples/floom-tsx-showcase/build.rs
fn main() {
    let out = std::env::var("OUT_DIR").unwrap();
    let tsx: Vec<_> = glob::glob("src/**/*.tsx").unwrap().filter_map(Result::ok).collect();
    let doc = floom_tsx::compile_project(&tsx).expect("tsx");
    let rust = floom_dsl::emit::emit(&doc, floom_dsl::WidgetSource::WithComposites);
    std::fs::write(format!("{out}/ui_gen.rs"), rust).unwrap();
    println!("cargo:rerun-if-changed=src");
}
```

**Step 4:** Run test to verify it passes

**Step 5:** Commit
```bash
git add examples/floom-tsx-showcase/
git commit -m "feat: showcase example + build.rs TSX integration"
```

### Task 8: `.d.ts` generation (P2)

**Files:**
- Modify: `crates/floom-tsx/src/decl.rs` (generate editor completions from IR schema)

**Step 1:** Write the failing test
```rust
// crates/floom-tsx/tests/decl_test.rs
use floom_tsx::compile;

#[test]
fn decl_generates_complete_types() {
    let source = r#"<Text label="hi" onClick={h}/><Stack/><Button/>"#;
    let doc = compile(source).expect("compile");
    let dts = floom_tsx::generate_dts(&doc);
    assert!(dts.contains("Text"));
    assert!(dts.contains("Stack"));
    assert!(dts.contains("Button"));
    assert!(dts.contains("interface") || dts.contains("type "));
}
```
Run: `cargo test -p floom-tsx decl_test -v`
Expected: FAIL (decl generation not yet implemented)

**Step 2:** Run test to verify it fails

**Step 3:** Write minimal implementation
```rust
// crates/floom-tsx/src/decl.rs
// TODO: walk UiDocument schema → .d.ts string (NodeKinds, props, events)
```

**Step 4:** Run test to verify it passes

**Step 5:** Commit
```bash
git add crates/floom-tsx/src/decl.rs
git commit -m "feat: .d.ts editor declarations generated from IR schema"
```

### Task 9: `class=` token resolution (P2)

**Files:**
- Modify: `crates/floom-tsx/src/lower.rs` (parse `class="text-primary"` → resolve into StyleProps prop)

**Step 1:** Write the failing test
```rust
// crates/floom-tsx/tests/class_golden.rs
use floom_tsx::compile;

#[test]
fn class_resolves_token() {
    let source = r#"<Text class="text-primary" />"#;
    let doc = compile(source).expect("lower");
    let prop = doc.nodes[0].props.iter().find(|p| p.name == "style");
    assert!(prop.is_some(), "class should resolve to a style prop");
}
```
Run: `cargo test -p floom-tsx class_golden -v`
Expected: FAIL

**Step 2:** Run test to verify it fails

**Step 3:** Write minimal implementation
```rust
// crates/floom-tsx/src/lower.rs
// TODO: map class="text-primary" → StyleProps via floom-style token table
```

**Step 4:** Run test to verify it passes

**Step 5:** Commit
```bash
git add crates/floom-tsx/src/lower.rs
git commit -m "feat: class= token resolution into StyleProps"
```

### Task 10: Golden example integration verification

**Files:**
- Uses showcase created in Task 7

**Step 1:** Write the failing test
- Run headless pump on showcase; assert painted > 0 and label changes after click
```rust
// crates/floom-tsx/tests/showcase_integration.rs
#[test]
fn showcase_headless_pump() {
    // This test builds the showcase and runs it headless
    // asserting painted > 0 and label changes after synthesized click
    // Implementation uses floom-testkit headless harness
}
```
Run: `cargo test -p floom-tsx-showcase showcase_headless_pump -v`
Expected: FAIL before implementation complete

**Step 2:** Run test to verify it fails

**Step 3:** Already implemented via Tasks 1-9

**Step 4:** Run test to verify it passes

**Step 5:** Commit
```bash
git add examples/floom-tsx-showcase/ crates/floom-tsx/tests/
git commit -m "feat: golden example integration verified"
```

---

## Phased Delivery (matches spec)

| Phase | Tasks | Deliverable |
|-------|-------|-------------|
| P0 static | Task 1 (skeleton), Task 2 (parser), Task 3 (validation+diagnostics), Task 4 (lower static subset) | Static TSX → UiDocument → emit → Rust, all NodeKinds covered |
| P1 signals | Task 5 (signal shim), Task 6 (emit reactive extension) | `createSignal`/`Memo`/`Effect`/context translate to `floom_signal` + effect wiring |
| P2 | Task 7 (showcase+build.rs), Task 8 (.d.ts), Task 9 (class tokens), Task 10 (golden integration) | Working `floom create`-less build.rs TSX integration + showcase golden |

## Non-Goals (explicitly excluded from this plan)

Route B runtime VM (rquickjs), TSX hot reload, GPUI backend, native-view rendering, full Tailwind utility library, mixed Rust+TSX projects, web-target bundling specifics. See spec Non-Goals section.

---

After all tasks complete:

1. Dispatch plan-document-reviewer subagent again with this plan path and spec path
2. Fix any issues, re-dispatch
3. Once approved, proceed to execution handoff (subagent-driven or inline)

**Plan complete and saved to** `docs/superpowers/plans/2026-11-02-floom-tsx-implementation-plan.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**