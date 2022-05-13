use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scene_graph::*;

pub fn criterion_benchmark(c: &mut Criterion) {
    let input_node: Vec<_> = (0..50_000).map(|v| format!("Node_{}", v)).collect();
    let mut sg = SceneGraph::new("Root");
    c.bench_function("add 50000", |b| {
        b.iter(|| {
            for v in input_node.iter() {
                sg.attach(sg.root_idx(), v).unwrap();
            }

            sg.clear();
        })
    });

    c.bench_function("iter 50k", |b| {
        b.iter(|| {
            for v in sg.iter() {
                black_box(v);
            }
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
