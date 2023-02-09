# scene-graph

![docs.rs](https://img.shields.io/docsrs/scene-graph)
![Crates.io](https://img.shields.io/crates/v/scene-graph)
![Crates.io](https://img.shields.io/crates/l/scene-graph)

This crate provides a Scene Graph structure, similar to the one used in engines like Unity or Unreal. It is fast, performant, and easy to manipulate.

## Quick Start

To install, add the following to your Cargo.toml:

```toml
scene-graph = "0.1.0"
```

or run:

```sh
cargo add scene-graph
```

Here's a basic `SceneGraph` example:

```rust
use scene_graph::SceneGraph;

fn main() {
    let mut sg: SceneGraph<&'static str> = SceneGraph::new("root");

    sg.attach_at_root("first child");
    // note that insertion order is honored.
    let second_child_handle = sg.attach_at_root("second child");

    // collect the nodes
    let nodes = Vec::from_iter(sg.iter().map(|(_parent, node)| *node));

    // note that the "root" is not seen in an `iter` operation.
    assert_eq!(nodes, ["first child", "second child"]);

    sg.attach(second_child_handle, "first grand-child").unwrap();
    sg.attach(second_child_handle, "second grand-child").unwrap();

    sg.attach_at_root("weird third way younger child");

    let nodes = Vec::from_iter(sg.iter().map(|(_parent, node)| *node));

    // note the iteration order -- because we `iter` depth first, we'll get the youngest child last.
    assert_eq!(
        nodes,
        [
            "first child",
            "second child",
            "first grand-child",
            "second grand-child",
            "weird third way younger child"
        ]
    );
}
```

SceneGraph's `iter` function returns a tuple of the parent's value and the current node's value in a *depth first* traversal. SceneGraph is designed, primarily, for trees of Transforms, and its `iter` is the best way to iterate over those transforms to resolve a scene graph of local transforms into world space transforms.

## Detaching Nodes

Nodes in a scene graph can be detached by calling `detach`, which will return a new `SceneGraph<T>` where the root is the node provided. A `SceneGraph<T>` can be attached to another `SceneGraph<T>` via `SceneGraph::attach_graph`. If that functionality isn't needed, users can instead use `remove` to simply remove the node and drop it entirely.

Detaching the children of a node without removing that node is simple as well -- `iter_detach` will return an iterator which detaches each descendent of the node.

## Comparison to `petgraph`

SceneGraph is similar to a `petgraph::stable_graph::StableGraph`, but has a few differences.

SceneGraph, on an M1 Mac, is slightly faster at iterating over nodes than `petgraph`s is, but with a tradeoff for creating nodes lagging a bit.

| benches |`scene-graph` | `petgraph` |
|---------|--------------|------------|
| adding and removing a node | `52 ns`  | `8.56 ns` |
| iter 50k nodes             | `217 µs` | `299.49 µs` |
| iter 64 nodes              | `311 ns` | `456.49 ns` |

However, this is not where `scene-graph`'s utility really shines -- `scene-graph` was written with the goal of quick iteration in mind, unlike `petgraph`, which is a general purpose graphing utility. For example, there is no simply equivalent in `petgraph` to the `iter` function.

```rs
// in `scene-graph`
for (parent, child) in sg.iter() {
    todo!();
}

// in `petgraph`
petgraph::visit::depth_first_search(&petgraph_sg, Some(root_idx), |event| match event {
    petgraph::visit::DfsEvent::Discover(_, _) => todo!(),
    petgraph::visit::DfsEvent::TreeEdge(_, _) => todo!(),
    petgraph::visit::DfsEvent::BackEdge(_, _) => todo!(),
    petgraph::visit::DfsEvent::CrossForwardEdge(_, _) => todo!(),
    petgraph::visit::DfsEvent::Finish(_, _) => todo!(),
});
```

However, `petgraph` offers many algorithms which `scene-graph` completely lacks. For example, finding the distance between two nodes in the graph is simple in `petgraph` and completely in users' hands in `scene-graph`.

## Dependencies

This crate depends on `thiserror` for convenience and `thunderdome` for its backing Arena allocator. Experimentation proved `thunderdome` to be both the easiest to work with and the fastest among options.

## MSRV

This crate has no MSRV yet. If it sees good adoption, an MSRV policy will be decided.

## License

Dual-licensed under MIT or APACHE 2.0.
