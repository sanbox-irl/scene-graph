use thunderdome::Index;

use crate::{Node, NodeIndex, SceneGraph};

/// A mutable iterator over the children of a node in a [SceneGraph].
/// See [SceneGraph::iter_mut] for more information.
pub struct SceneGraphIterMut<'a, T> {
    sg: &'a mut SceneGraph<T>,
    stacks: Vec<StackState>,
}

impl<'a, T> SceneGraphIterMut<'a, T> {
    pub(crate) fn new(sg: &'a mut SceneGraph<T>, root_node_idx: NodeIndex) -> Self {
        let mut stacks = Vec::new();

        if let Some(first_child) = sg.get_children(root_node_idx).map(|v| v.first) {
            stacks.push(StackState::new(root_node_idx, first_child));
        };
        SceneGraphIterMut { sg, stacks }
    }
}

impl<'a, T> Iterator for SceneGraphIterMut<'a, T> {
    type Item = (&'a mut T, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        // if we're out of stack frames, we die here
        let stack_frame = self.stacks.pop()?;

        let (parent, current_child) = match stack_frame.parent {
            NodeIndex::Root => {
                let parent = &mut self.sg.root;

                let child = self.sg.arena.get_mut(stack_frame.current_child).unwrap();

                (parent, child)
            }
            NodeIndex::Branch(idx) => {
                let (parent, current_child) =
                    self.sg.arena.get2_mut(idx, stack_frame.current_child);

                (&mut parent.unwrap().value, current_child.unwrap())
            }
        };

        // safety:  this is a lifetime extension, which i know is valid because get2_mut
        // panics when we pass in two of the same things, and this iterator requires `&mut SG`
        // to call `next`.
        let (parent, current_child): (&mut T, &mut Node<T>) =
            unsafe { (&mut *(parent as *mut _), &mut *(current_child as *mut _)) };

        // if there's a sibling, push it onto the to do list!
        if let Some(next_sibling) = current_child.next_sibling {
            self.stacks
                .push(StackState::new(stack_frame.parent, next_sibling));
        }

        if let Some(first_child) = current_child.children.map(|v| v.first) {
            self.stacks.push(StackState::new(
                NodeIndex::Branch(stack_frame.current_child),
                first_child,
            ));
        }

        Some((parent, &mut current_child.value))
    }
}

#[derive(Debug)]
struct StackState {
    parent: NodeIndex,
    current_child: Index,
}

impl StackState {
    fn new(parent: NodeIndex, first_child: Index) -> Self {
        Self {
            parent,
            current_child: first_child,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_graph_returns_nothing_on_empty_iteration() {
        let mut scene_graph = SceneGraph::new("Root");

        assert!(scene_graph.iter_mut().next().is_none());
    }

    #[test]
    fn normal_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
        sg.attach(root_idx, "First Child").unwrap();

        let second_child = sg.attach(root_idx, "Second Child").unwrap();
        sg.attach(second_child, "First Grandchild").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_mut().map(|(_parent, value)| &*value).copied()),
            vec!["First Child", "Second Child", "First Grandchild"]
        );
    }

    #[test]
    fn stagger_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
        let child = sg.attach(root_idx, "First Child").unwrap();
        sg.attach(child, "Second Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_mut().map(|(_parent, value)| &*value).copied()),
            vec!["First Child", "Second Child"]
        );
    }

    #[test]
    fn single_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
        sg.attach(root_idx, "First Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_mut().map(|(_parent, value)| &*value).copied()),
            vec!["First Child",]
        );
    }
}
