// SPDX-License-Identifier: MIT
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use floom_layout::{FlexDirection, LayoutEngine, LayoutStyle};
use floom_types::{Length, Size};

fn incremental_subtree(c: &mut Criterion) {
    let mut eng = LayoutEngine::new();
    let mut children = Vec::new();
    for _ in 0..64 {
        children.push(eng.new_leaf(LayoutStyle {
            width: Some(Length::Px(10.0)),
            height: Some(Length::Px(10.0)),
            ..Default::default()
        }));
    }
    let root = eng.new_with_children(
        LayoutStyle {
            flex_direction: FlexDirection::Row,
            width: Some(Length::Px(800.0)),
            height: Some(Length::Px(40.0)),
            ..Default::default()
        },
        &children,
    );
    eng.compute(root, Size::new(800.0, 40.0));

    c.bench_function("relayout_64_flex_row", |b| {
        b.iter(|| {
            eng.set_style(
                children[0],
                LayoutStyle {
                    width: Some(Length::Px(12.0)),
                    height: Some(Length::Px(10.0)),
                    ..Default::default()
                },
            );
            eng.compute(root, Size::new(800.0, 40.0));
            black_box(eng.layout(root));
        })
    });
}

criterion_group!(benches, incremental_subtree);
criterion_main!(benches);
