use crate::{Children, NodeIndex, SceneGraph};

/// An iterator over only the immediate children of a node in a [SceneGraph].
/// See [SceneGraph::iter_children] for more information.
pub struct SceneGraphChildIter<'a, T> {
    sg: &'a SceneGraph<T>,
    current_node: Option<thunderdome::Index>,
}

impl<'a, T> SceneGraphChildIter<'a, T> {
    pub(crate) fn new(sg: &'a SceneGraph<T>, root_index: NodeIndex) -> Self {
        let children = match root_index {
            NodeIndex::Root => sg.root_children.as_ref(),
            NodeIndex::Branch(idx) => sg.arena[idx].children.as_ref(),
        };

        Self::with_children(sg, children)
    }

    pub(crate) fn with_children(sg: &'a SceneGraph<T>, children: Option<&'a Children>) -> Self {
        SceneGraphChildIter {
            sg,
            current_node: children.map(|v| v.first),
        }
    }
}

impl<'a, T> Iterator for SceneGraphChildIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let yield_me = self.sg.arena.get(self.current_node?).unwrap();
        self.current_node = yield_me.next_sibling;

        Some(&yield_me.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_graph_returns_nothing_on_empty_iteration() {
        let scene_graph = SceneGraph::new("Root");

        assert!(
            scene_graph
                .iter_direct_children(NodeIndex::Root)
                .unwrap()
                .next()
                .is_none()
        );
    }

    #[test]
    fn normal_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
        let fg = sg.attach(root_idx, "First Child").unwrap();
        sg.attach(fg, "First Grandchild").unwrap();
        sg.attach(fg, "Second Grandchild").unwrap();
        sg.attach(fg, "Third Grandchild").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_direct_children(fg).unwrap().cloned()),
            vec!["First Grandchild", "Second Grandchild", "Third Grandchild"]
        );
    }
}
