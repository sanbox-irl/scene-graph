use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scene_graph::*;

pub fn criterion_benchmark(c: &mut Criterion) {
    let input_node: Vec<_> = (0..50_000).map(|v| format!("Node_{}", v)).collect();
    let mut sg = SceneGraph::new("Root");

    let mut petgraph_sg = petgraph::stable_graph::StableGraph::new();
    let root_idx = petgraph_sg.add_node("root");

    let mut group = c.benchmark_group("add/remove one node");
    group.bench_function("scene-graph", |b| {
        b.iter(|| {
            let idx = sg.attach_at_root("single boy");
            sg.remove(idx);
        })
    });
    group.bench_function("petgraph", |b| {
        b.iter(|| {
            let new_node = petgraph_sg.add_node("single boy");
            petgraph_sg.add_edge(root_idx, new_node, ());

            petgraph_sg.remove_node(new_node).unwrap();
        })
    });
    group.finish();

    sg.clear();
    for v in input_node.iter() {
        sg.attach_at_root(v);
    }

    petgraph_sg.clear();
    let root_idx = petgraph_sg.add_node("root");

    let mut group = c.benchmark_group("add/remove 50000th node");

    group.bench_function("scene-graph", |b| {
        b.iter(|| {
            let idx = sg.attach_at_root("Finality");
            sg.remove(idx);
        })
    });
    group.bench_function("petgraph", |b| {
        b.iter(|| {
            let new_node = petgraph_sg.add_node("Finality");
            petgraph_sg.add_edge(root_idx, new_node, ());

            petgraph_sg.remove_node(new_node).unwrap();
        })
    });
    group.finish();

    sg.clear();
    for v in input_node.iter() {
        sg.attach_at_root(v);
    }

    petgraph_sg.clear();
    let root_idx = petgraph_sg.add_node("root");

    for v in input_node.iter() {
        let new_node = petgraph_sg.add_node(v);
        petgraph_sg.add_edge(root_idx, new_node, ());
    }

    let mut group = c.benchmark_group("iter 50k");
    group.bench_function("scene_graph", |b| {
        b.iter(|| {
            for v in sg.iter() {
                black_box(v);
            }
        })
    });
    group.bench_function("petgraph", |b| {
        b.iter(|| {
            petgraph::visit::depth_first_search(&petgraph_sg, Some(root_idx), |event| match event {
                petgraph::visit::DfsEvent::Discover(_, _) => todo!(),
                petgraph::visit::DfsEvent::TreeEdge(_, _) => todo!(),
                petgraph::visit::DfsEvent::BackEdge(_, _) => todo!(),
                petgraph::visit::DfsEvent::CrossForwardEdge(_, _) => todo!(),
                petgraph::visit::DfsEvent::Finish(_, _) => todo!(),
            });
        })
    });
    group.finish();

    sg.clear();
    for v in input_node.iter().take(64) {
        sg.attach_at_root(v);
    }

    petgraph_sg.clear();
    let root_idx = petgraph_sg.add_node("root");

    for v in input_node.iter().take(64) {
        let new_node = petgraph_sg.add_node(v);
        petgraph_sg.add_edge(root_idx, new_node, ());
    }

    let mut group = c.benchmark_group("iter 64");

    group.bench_function("scene-graph", |b| {
        b.iter(|| {
            for v in sg.iter() {
                black_box(v);
            }
        })
    });
    group.bench_function("petgraph", |b| {
        b.iter(|| {
            petgraph::visit::depth_first_search(&petgraph_sg, Some(root_idx), |event| {
                black_box(event);
            });
        })
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
