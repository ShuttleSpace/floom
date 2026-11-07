// SPDX-License-Identifier: MIT
//! The `floom create --style tsx` starter is a real app: the parser must accept
//! it, or the scaffold would hand every new project a file that does not build.

const APP_TSX: &str = include_str!("../../../app/floom-cli/src/templates/app_tsx.tsx");

#[test]
fn create_tsx_starter_parses() {
    let source = APP_TSX.replace("{{TITLE}}", "demo");
    floom_tsx::compile(&source).expect("the scaffolded app.tsx must parse");
}
