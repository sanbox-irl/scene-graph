use crate::{Children, Node, NodeIndex, SceneGraph};
use std::collections::VecDeque;

pub struct SceneGraphDetachIter<'a, T> {
    sg: &'a mut SceneGraph<T>,
    stacks: VecDeque<StackState<T>>,
}

pub struct DetachedNode<T> {
    pub parent_idx: NodeIndex,
    pub node_idx: NodeIndex,
    pub node_value: T,
}

impl<'a, T> SceneGraphDetachIter<'a, T> {
    pub(crate) fn new(
        sg: &'a mut SceneGraph<T>,
        head_index: NodeIndex,
        children: Option<Children>,
    ) -> Self {
        let mut stacks = VecDeque::new();
        if let Some(children) = children {
            stacks.push_front(StackState::new(
                head_index,
                sg.arena.remove(children.first).unwrap(),
                NodeIndex(children.first),
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
                self.sg.arena.remove(next_sibling).unwrap(),
                NodeIndex(next_sibling),
            ));
        }

        // if there's a child, push it on the list first
        if let Some(children) = stack_frame.current_child.children {
            let new_stack = StackState::new(
                stack_frame.current_child_idx,
                self.sg.arena.remove(children.first).unwrap(),
                NodeIndex(children.first),
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
    use super::*;

    #[test]
    fn detach_handles_empty() {
        let mut scene_graph = SceneGraph::new("Root");

        assert!(scene_graph.iter_detach().next().is_none());
    }

    #[test]
    fn detach_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = sg.root_idx();
        sg.attach(root_idx, "First Child").unwrap();

        let second_child = sg.attach(root_idx, "Second Child").unwrap();
        sg.attach(second_child, "First Grandchild").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_detach().map(|d_v| d_v.node_value)),
            vec!["First Child", "Second Child", "First Grandchild"]
        );

        assert!(sg.is_empty());
    }

    #[test]
    fn stagger_detach_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = sg.root_idx();
        let child = sg.attach(root_idx, "First Child").unwrap();
        sg.attach(child, "Second Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_detach().map(|value| value.node_value)),
            vec!["First Child", "Second Child"]
        );
        assert!(sg.is_empty());
    }

    #[test]
    fn single_detach_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = sg.root_idx();
        sg.attach(root_idx, "First Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_detach().map(|value| value.node_value)),
            vec!["First Child",]
        );
        assert!(sg.is_empty());
    }
}
