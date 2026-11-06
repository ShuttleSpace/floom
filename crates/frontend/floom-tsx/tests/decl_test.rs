// SPDX-License-Identifier: MIT
//! `.d.ts` generation from the IR schema (Task 8).

use floom_tsx::generate_dts;
use floom_dsl::ui_ir::{all_prop_names, events_for, prop_spec};

#[test]
fn decl_exposes_module_and_jsx_namespaces() {
    let dts = generate_dts();
    assert!(dts.contains("declare module \"floom\""));
    assert!(dts.contains("declare namespace JSX"));
    assert!(dts.contains("interface IntrinsicElements"));
}

#[test]
fn decl_covers_every_kind_and_prop() {
    use floom_dsl::ui_ir::{NodeKind};
    let dts = generate_dts();

    // 每个组件都有 XxxProps 接口与 IntrinsicElements 映射。
    for kind in NodeKind::ALL {
        let ts = match kind {
            NodeKind::Native(n) => format!("{:?}", n), // 见下：原生用 Web/Video/Map/TextInput
            _ => kind.name().split('_').map(|s| {
                let mut c = s.chars();
                match c.next() {
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            }).collect::<String>(),
        };
        assert!(dts.contains(&format!("{}Props ", ts)), "missing {ts}Props");
        assert!(dts.contains(&format!("{ts}: Floom.{ts}Props")), "missing Intrinsics {ts}");
        // 每个 schema 属性都出现。
        for name in all_prop_names() {
            if prop_spec(*kind, name).is_some() {
                assert!(dts.contains(&format!("{name}?:")),
                    "{kind:?} should declare prop {name}");
            }
        }
        // 每个事件都出现。
        for ev in events_for(*kind) {
            assert!(dts.contains(&format!("on{}?:", ev.name().to_string().chars()
                .next().unwrap().to_uppercase().collect::<String>()
                + &ev.name()[1..])), "{kind:?} should declare {ev:?}");
        }
    }
    // 颜色类型收窄、domain 联合、App 包装器、shim 函数、entry 句柄。
    assert!(dts.contains("export type Color = `#${string}`"), "color must be narrowed to #hex");
    assert!(dts.contains("\"horizontal\" | \"vertical\""));
    assert!(dts.contains("interface AppProps"));
    assert!(dts.contains("export function createSignal<T>"));
    assert!(dts.contains("export const platform"));
}
