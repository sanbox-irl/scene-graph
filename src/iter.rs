use crate::{Node, SceneGraph};

pub struct SceneGraphIter<'a, T> {
    sg: &'a SceneGraph<T>,
    stacks: Vec<StackState<'a, T>>,
}

impl<'a, T> SceneGraphIter<'a, T> {
    pub(crate) fn new(sg: &'a SceneGraph<T>, root_node: &'a Node<T>) -> Self {
        let mut stacks = Vec::new();
        if let Some(first_child) = root_node.children.map(|v| v.first) {
            stacks.push(StackState::new(root_node, &sg.arena[first_child]));
        };
        SceneGraphIter { sg, stacks }
    }
}

impl<'a, T> Iterator for SceneGraphIter<'a, T> {
    type Item = (&'a T, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        // if we're out of stack frames, we die here
        let stack_frame = self.stacks.pop()?;

        // if there's a sibling, push it onto the to do list!
        if let Some(next_sibling) = stack_frame.current_child.next_sibling {
            self.stacks.push(StackState::new(
                stack_frame.parent,
                &self.sg.arena[next_sibling],
            ));
        }

        if let Some(first_child) = stack_frame.current_child.children.map(|v| v.first) {
            let new_stack = StackState::new(stack_frame.current_child, &self.sg.arena[first_child]);
            self.stacks.push(new_stack);
        }

        Some((&stack_frame.parent.value, &stack_frame.current_child.value))
    }
}

struct StackState<'a, T> {
    parent: &'a Node<T>,
    current_child: &'a Node<T>,
}

impl<'a, T> std::fmt::Debug for StackState<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StackState")
            .field("parent", &self.parent)
            .field("current_child", &self.current_child)
            .finish()
    }
}

impl<'a, T> StackState<'a, T> {
    fn new(parent: &'a Node<T>, first_child: &'a Node<T>) -> Self {
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
        let scene_graph = SceneGraph::new("Root");

        assert!(scene_graph.iter().next().is_none());
    }

    #[test]
    fn normal_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = sg.root_idx();
        sg.attach(root_idx, "First Child").unwrap();

        let second_child = sg.attach(root_idx, "Second Child").unwrap();
        sg.attach(second_child, "First Grandchild").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter().map(|(_parent, value)| value).cloned()),
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
            Vec::from_iter(sg.iter().map(|(_parent, value)| value).cloned()),
            vec!["First Child", "Second Child"]
        );
    }

    #[test]
    fn single_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = sg.root_idx();
        sg.attach(root_idx, "First Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter().map(|(_parent, value)| value).cloned()),
            vec!["First Child",]
        );
    }
}
