use crate::{Node, SceneGraph};

pub struct SceneGraphChildIter<'a, T> {
    sg: &'a SceneGraph<T>,
    current_node: Option<&'a Node<T>>,
}

impl<'a, T> SceneGraphChildIter<'a, T> {
    pub(crate) fn new(sg: &'a SceneGraph<T>, parent_node: &'a Node<T>) -> Self {
        SceneGraphChildIter {
            sg,
            current_node: parent_node.children.map(|v| sg.arena.get(v.first).unwrap()),
        }
    }
}

impl<'a, T> Iterator for SceneGraphChildIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let yield_me = self.current_node?;

        self.current_node = yield_me.next_sibling.map(|v| self.sg.arena.get(v).unwrap());

        Some(&yield_me.value)
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
