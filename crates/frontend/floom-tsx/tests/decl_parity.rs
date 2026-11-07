// SPDX-License-Identifier: MIT
//! Parser-parity contract for the generated declarations (Task 3).

use floom_dsl::ui_ir::{EventName, NodeKind};
use floom_tsx::generate_dts;

/// Extract `IntrinsicElements { ... }` block内的 `Key: Floom.XxxProps;` 的 Key。
fn intrinsic_tags(dts: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    for line in dts.lines() {
        if line.trim_start().starts_with("interface IntrinsicElements") {
            in_block = true;
            continue;
        }
        if in_block {
            let t = line.trim();
            if t == "}" {
                break;
            }
            if let Some((key, _)) = t.split_once(':') {
                out.push(key.trim().to_string());
            }
        }
    }
    out
}

/// 抽取某个 `interface XxxProps { ... }` 块内的 `name?: type;` 字段名。
fn field_names(dts: &str, interface: &str) -> Vec<String> {
    let marker = format!("export interface {interface}");
    let mut out = Vec::new();
    let mut depth: i32 = 0;
    let mut in_iface = false;
    for line in dts.lines() {
        let t = line.trim_start();
        if t.starts_with(&marker) {
            in_iface = true;
            depth = 0;
            continue;
        }
        if in_iface {
            depth += t.matches('{').count() as i32 - t.matches('}').count() as i32;
            if let Some((name, _)) = t.split_once(':') {
                let name = name.trim().trim_end_matches('?');
                if !name.is_empty()
                    && name
                        .chars()
                        .next()
                        .map(|c| c.is_alphabetic())
                        .unwrap_or(false)
                {
                    out.push(name.to_string());
                }
            }
            if depth <= 0 {
                break;
            }
        }
    }
    out
}

/// 抽取所有 `export interface XxxProps` 的名字。
fn interface_names(dts: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in dts.lines() {
        let t = line.trim_start();
        // 生成的接口行带 `export ` 前缀：`  export interface XxxProps {`
        let rest = t
            .strip_prefix("export interface ")
            .or_else(|| t.strip_prefix("interface "));
        if let Some(rest) = rest
            && let Some(name) = rest.split_whitespace().next()
            && name.ends_with("Props")
        {
            out.push(name.to_string());
        }
    }
    out
}

/// `decl.rs::kind_ts_name` 的镜像（原生 → Web/Video/Map/TextInput，其余 PascalCase）。
fn ts_name(kind: NodeKind) -> String {
    use floom_dsl::ui_ir::NativeKind;
    use floom_dsl::ui_ir::NodeKind;
    match kind {
        NodeKind::Native(NativeKind::Web) => "Web".to_string(),
        NodeKind::Native(NativeKind::Video) => "Video".to_string(),
        NodeKind::Native(NativeKind::Map) => "Map".to_string(),
        NodeKind::Native(NativeKind::TextInput) => "TextInput".to_string(),
        other => other
            .name()
            .split('_')
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            })
            .collect(),
    }
}

#[test]
fn parity_tag_roundtrip() {
    let dts = generate_dts();
    let tags = intrinsic_tags(&dts);
    // spec 不变量：IntrinsicElements 覆盖每个 NodeKind（+ `<App>` 包装根）。
    assert_eq!(
        tags.len(),
        NodeKind::ALL.len() + 1,
        "IntrinsicElements must cover every NodeKind plus <App>"
    );
    for tag in &tags {
        if tag == "App" {
            continue;
        }
        assert!(
            NodeKind::parse(&tag.to_lowercase()).is_some(),
            "declared tag <{tag}> is not accepted by the parser"
        );
    }
}

#[test]
fn parity_prop_accepted() {
    use floom_dsl::ui_ir::prop_spec;
    let dts = generate_dts();
    for kind in NodeKind::ALL {
        let iface = format!("{}Props", ts_name(*kind));
        for field in field_names(&dts, &iface) {
            if field == "class" || field == "children" || field.starts_with("on") {
                continue;
            }
            assert!(
                prop_spec(*kind, &field).is_some(),
                "{kind:?} prop {field} is declared but not accepted by the parser"
            );
        }
    }
}

#[test]
fn parity_event_accepted() {
    let dts = generate_dts();
    // 遍历所有 XxxProps 接口，抽取 onXxx 字段，strip "on" 后 EventName::parse 必须成功。
    for iface in interface_names(&dts) {
        for field in field_names(&dts, &iface) {
            if let Some(name) = field.strip_prefix("on") {
                let ev = name
                    .chars()
                    .next()
                    .unwrap()
                    .to_lowercase()
                    .collect::<String>()
                    + &name[1..];
                assert!(
                    EventName::parse(&ev).is_some(),
                    "declared handler on{name} does not resolve to a known event"
                );
            }
        }
    }
}

#[test]
fn parity_entry_accepted() {
    // 调用每个 entry 句柄的 TSX 能被 entry::resolve 接受；<App ...> 包装器也接受。
    let src = r#"
        import { app, mount, platform, window } from "floom";
        app.setName("x");
        window.setTitle("x"); window.setSize(640, 480); window.setSize("640x480");
        window.setResizable(true);
        platform.require("ime", "warn");
        function App() { return <App title="t" size="1x1"><Text label="x" /></App>; }
        mount(App);
    "#;
    floom_tsx::compile(src).expect("entry API must parse");
}

#[test]
fn parity_shims_compile() {
    let src = r#"
        import { createSignal, createMemo, createEffect, provideContext, useContext, mount } from "floom";
        const [n, setN] = createSignal(0);
        const d = createMemo(() => 1);
        createEffect(() => {});
        provideContext("t", "v");
        const u = useContext("t");
        function App() { return <Text label={n() ? "on" : "off"} />; }
        mount(App);
    "#;
    floom_tsx::compile(src).expect("reactive shims must parse");
}

#[test]
fn decl_exposes_reactive_type_aliases() {
    // 这些 TS 类型别名由 .d.ts 暴露给编辑器；解析器是 ES-only（不接受 TS 注解），
    // 故只验证声明文本包含它们，不通过 parser 编译验证。
    let dts = generate_dts();
    assert!(dts.contains("export type Component = () => JSX.Element"));
    assert!(dts.contains("export interface Accessor<T>"));
    assert!(dts.contains("export interface Setter<T>"));
    assert!(dts.contains("export type Color = `#${string}`"));
}

#[test]
fn parity_full_document_roundtrip() {
    // 用从生成文本里抽出的每个 tag 拼出一段 TSX，整体编译必须成功。
    // 每个组件用最简 <Tag />（schema 属性都是可选的），容器无子节点也可解析；
    // 属性/事件的逐个接受已由 parity_prop_accepted / parity_event_accepted 覆盖。
    let dts = generate_dts();
    let mut body = String::new();
    for tag in intrinsic_tags(&dts) {
        if tag == "App" {
            continue;
        }
        body += &format!("<{tag} />");
    }
    let src = format!(
        "import {{ mount }} from \"floom\";\nfunction App() {{ return <Stack>{body}</Stack>; }}\nmount(App);"
    );
    floom_tsx::compile(&src).expect("every declared component must be parseable");
}

#[test]
fn parity_encouraged_syntax() {
    // 解析器是 ES+JSX（不接受 TS 类型注解），因此只验证 .d.ts 鼓励的*运行时*用法
    // 能被 parser 接受：解构 createSignal、函数组件、mount。TS 注解形式的覆盖见
    // decl_exposes_reactive_type_aliases（声明文本存在性）。
    let src = r#"
        import { createSignal, mount } from "floom";
        const [n, setN] = createSignal(0);
        function App() { return <Stack />; }
        mount(App);
    "#;
    floom_tsx::compile(src).expect("encouraged runtime usage must parse");
}
