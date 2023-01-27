use crate::{Children, Node, NodeIndex};
use std::collections::VecDeque;

/// An iterator over the children of a node in a [SceneGraph].
/// See [SceneGraph::iter_detach] and [SceneGraph::iter_detach_all] for more information.
pub struct SceneGraphDetachIter<'a, T> {
    sg: &'a mut thunderdome::Arena<Node<T>>,
    stacks: VecDeque<StackState<T>>,
}

impl<'a, T> SceneGraphDetachIter<'a, T> {
    pub(crate) fn new(
        sg: &'a mut thunderdome::Arena<Node<T>>,
        head_index: NodeIndex,
        children: Option<Children>,
    ) -> Self {
        let mut stacks = VecDeque::new();
        if let Some(children) = children {
            stacks.push_front(StackState::new(
                head_index,
                sg.remove(children.first).unwrap(),
                NodeIndex::Branch(children.first),
            ));
        };
        SceneGraphDetachIter { sg, stacks }
    }
}

impl<'a, T> Iterator for SceneGraphDetachIter<'a, T> {
    type Item = DetachedNode<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // if we're out of stack frames, we die here
        let stack_frame = self.stacks.pop_front()?;

        // if there's a sibling, push it onto the to do list!
        if let Some(next_sibling) = stack_frame.current_child.next_sibling {
            self.stacks.push_front(StackState::new(
                stack_frame.parent,
                self.sg.remove(next_sibling).unwrap(),
                NodeIndex::Branch(next_sibling),
            ));
        }

        // if there's a child, push it on the list first
        if let Some(children) = stack_frame.current_child.children {
            let new_stack = StackState::new(
                stack_frame.current_child_idx,
                self.sg.remove(children.first).unwrap(),
                NodeIndex::Branch(children.first),
            );
            self.stacks.push_front(new_stack);
        }

        Some(DetachedNode {
            parent_idx: stack_frame.parent,
            node_idx: stack_frame.current_child_idx,
            node_value: stack_frame.current_child.value,
        })
    }
}

/// A detached node from a scene graph.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct DetachedNode<T> {
    /// The original parent idx, which may or may not still be in the scene graph itself.
    pub parent_idx: NodeIndex,
    /// Its old node_index.
    pub node_idx: NodeIndex,
    /// The value of the node.
    pub node_value: T,
}

impl<T> std::fmt::Debug for DetachedNode<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DetachedNode")
            .field("parent_idx", &self.parent_idx)
            .field("node_idx", &self.node_idx)
            .finish_non_exhaustive()
    }
}

struct StackState<T> {
    parent: NodeIndex,
    current_child: Node<T>,
    current_child_idx: NodeIndex,
}

impl<T> std::fmt::Debug for StackState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StackState")
            .field("parent", &self.parent)
            .field("current_child", &self.current_child)
            .field("current_child_idx", &self.current_child_idx)
            .finish()
    }
}

impl<T> StackState<T> {
    fn new(parent: NodeIndex, first_child: Node<T>, first_child_idx: NodeIndex) -> Self {
        Self {
            parent,
            current_child: first_child,
            current_child_idx: first_child_idx,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::SceneGraph;

    use super::*;

    #[test]
    fn detach_handles_empty() {
        let mut scene_graph = SceneGraph::new("Root");

        assert!(scene_graph.iter_detach_all().next().is_none());
    }

    #[test]
    fn detach_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
        sg.attach(root_idx, "First Child").unwrap();

        let second_child = sg.attach(root_idx, "Second Child").unwrap();
        sg.attach(second_child, "First Grandchild").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_detach_all().map(|d_v| d_v.node_value)),
            vec!["First Child", "Second Child", "First Grandchild"]
        );

        assert!(sg.is_empty());
    }

    #[test]
    fn stagger_detach_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
        let child = sg.attach(root_idx, "First Child").unwrap();
        sg.attach(child, "Second Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_detach_all().map(|value| value.node_value)),
            vec!["First Child", "Second Child"]
        );
        assert!(sg.is_empty());
    }

    #[test]
    fn single_detach_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
        sg.attach(root_idx, "First Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_detach_all().map(|value| value.node_value)),
            vec!["First Child",]
        );
        assert!(sg.is_empty());
    }

    #[test]
    fn child_detach_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
        sg.attach(root_idx, "First Child").unwrap();

        let second_child = sg.attach(root_idx, "Second Child").unwrap();
        sg.attach(second_child, "First Grandchild").unwrap();
        sg.attach(second_child, "Second Grandchild").unwrap();
        sg.attach(second_child, "Third Grandchild").unwrap();
        sg.attach(second_child, "Fourth Grandchild").unwrap();

        assert_eq!(
            Vec::from_iter(
                sg.iter_detach_children(second_child)
                    .unwrap()
                    .map(|d_v| d_v.node_value)
            ),
            vec![
                "First Grandchild",
                "Second Grandchild",
                "Third Grandchild",
                "Fourth Grandchild"
            ]
        );

        assert!(!sg.is_empty());
    }
}
