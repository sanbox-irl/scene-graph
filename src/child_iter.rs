use thunderdome::Index;

use crate::{Node, NodeIndex, SceneGraph};

pub struct SceneGraphChildIter<'a, T> {
    sg: &'a SceneGraph<T>,
    current_node: Option<(Index, &'a Node<T>)>,
}

impl<'a, T> SceneGraphChildIter<'a, T> {
    pub(crate) fn new(sg: &'a SceneGraph<T>, parent_node: &'a Node<T>) -> Self {
        SceneGraphChildIter {
            sg,
            current_node: parent_node
                .children
                .map(|v| (v.first, sg.arena.get(v.first).unwrap())),
        }
    }

    pub fn with_node(self) -> SceneGraphChildIterWithNode<'a, T> {
        SceneGraphChildIterWithNode {
            sg: self.sg,
            current_node: self.current_node,
        }
    }
}

impl<'a, T> Iterator for SceneGraphChildIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let yield_me = self.current_node?;

        self.current_node = yield_me
            .1
            .next_sibling
            .map(|v| (v, self.sg.arena.get(v).unwrap()));

        Some(&yield_me.1.value)
    }
}

pub struct SceneGraphChildIterWithNode<'a, T> {
    sg: &'a SceneGraph<T>,
    current_node: Option<(Index, &'a Node<T>)>,
}

impl<'a, T> Iterator for SceneGraphChildIterWithNode<'a, T> {
    type Item = (NodeIndex, &'a Node<T>);

    fn next(&mut self) -> Option<Self::Item> {
        let yield_me = self.current_node?;

        self.current_node = yield_me
            .1
            .next_sibling
            .map(|v| (v, self.sg.arena.get(v).unwrap()));

        Some((NodeIndex(yield_me.0), yield_me.1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_graph_returns_nothing_on_empty_iteration() {
        let scene_graph = SceneGraph::new("Root");

        assert!(scene_graph
            .iter_children(scene_graph.root_idx())
            .unwrap()
            .next()
            .is_none());
    }

    #[test]
    fn normal_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = sg.root_idx();
        let fg = sg.attach(root_idx, "First Child").unwrap();
        sg.attach(fg, "First Grandchild").unwrap();
        sg.attach(fg, "Second Grandchild").unwrap();
        sg.attach(fg, "Third Grandchild").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_children(fg).unwrap().cloned()),
            vec!["First Grandchild", "Second Grandchild", "Third Grandchild"]
        );
    }
}
