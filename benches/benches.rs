use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scene_graph::*;

pub fn criterion_benchmark(c: &mut Criterion) {
    let input_node: Vec<_> = (0..50_000).map(|v| format!("Node_{}", v)).collect();
    let mut sg = SceneGraph::new("Root");
    // c.bench_function("add 1st node", |b| {
    //     b.iter(|| {
    //         sg.attach(sg.root_idx(), "single boy").unwrap();
    //     })
    // });

    for v in input_node.iter() {
        sg.attach(sg.root_idx(), v).unwrap();
    }

    // c.bench_function("add/remove 50000th node", |b| {
    //     b.iter(|| {
    //         let idx = sg.attach(sg.root_idx(), "Finality").unwrap();
    //         sg.remove(idx);
    //     })
    // });

    // sg.clear();
    // for v in input_node.iter() {
    //     sg.attach(sg.root_idx(), v).unwrap();
    // }

    c.bench_function("iter 50k", |b| {
        b.iter(|| {
            for v in sg.iter() {
                black_box(v);
            }
        })
    });

    // sg.clear();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
