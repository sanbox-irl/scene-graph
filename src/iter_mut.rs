use thunderdome::Index;

use crate::{Node, SceneGraph};

pub struct SceneGraphIterMut<'a, T> {
    sg: &'a mut SceneGraph<T>,
    stacks: Vec<StackState>,
}

impl<'a, T> SceneGraphIterMut<'a, T> {
    pub(crate) fn new(sg: &'a mut SceneGraph<T>, root_node_idx: Index) -> Self {
        let mut stacks = Vec::new();
        if let Some(first_child) = sg.arena[root_node_idx].children.map(|v| v.first) {
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

        let (parent, child) = self
            .sg
            .arena
            .get2_mut(stack_frame.parent, stack_frame.current_child);

        // safety:  this is a lifetime extension, which i know is valid because get2_mut
        // panics when we pass in two of the same things, and this iterator requires `&mut SG`
        // to call `next`.
        let (parent, current_child): (&mut Node<T>, &mut Node<T>) = unsafe {
            let parent = parent.unwrap();
            let current_child = child.unwrap();

            (&mut *(parent as *mut _), &mut *(current_child as *mut _))
        };

        // if there's a sibling, push it onto the to do list!
        if let Some(next_sibling) = current_child.next_sibling {
            self.stacks
                .push(StackState::new(stack_frame.parent, next_sibling));
        }

        if let Some(first_child) = current_child.children.map(|v| v.first) {
            self.stacks
                .push(StackState::new(stack_frame.current_child, first_child));
        }

        Some((&mut parent.value, &mut current_child.value))
    }
}

#[derive(Debug)]
struct StackState {
    parent: Index,
    current_child: Index,
}

impl StackState {
    fn new(parent: Index, first_child: Index) -> Self {
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
        let root_idx = sg.root_idx();
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
        let root_idx = sg.root_idx();
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
        let root_idx = sg.root_idx();
        sg.attach(root_idx, "First Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_mut().map(|(_parent, value)| &*value).copied()),
            vec!["First Child",]
        );
    }
}
