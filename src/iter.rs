use crate::{Children, Node, SceneGraph};

pub struct SceneGraphIter<'a, T> {
    sg: &'a SceneGraph<T>,
    stacks: Vec<StackState<'a, T>>,
}

impl<'a, T> SceneGraphIter<'a, T> {
    pub(crate) fn new(
        sg: &'a SceneGraph<T>,
        root_value: &'a T,
        root_children: Option<&'a Children>,
    ) -> Self {
        let mut stacks = Vec::new();
        if let Some(first_child) = root_children.map(|v| v.first) {
            stacks.push(StackState::new(root_value, &sg.arena[first_child]));
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
                stack_frame.parent_value,
                &self.sg.arena[next_sibling],
            ));
        }

        if let Some(first_child) = stack_frame.current_child.children.map(|v| v.first) {
            self.stacks.push(StackState::new(
                &stack_frame.current_child.value,
                &self.sg.arena[first_child],
            ));
        }

        Some((stack_frame.parent_value, &stack_frame.current_child.value))
    }
}

#[derive(Debug)]
struct StackState<'a, T> {
    parent_value: &'a T,
    current_child: &'a Node<T>,
}

impl<'a, T> StackState<'a, T> {
    fn new(parent: &'a T, first_child: &'a Node<T>) -> Self {
        Self {
            parent_value: parent,
            current_child: first_child,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::NodeIndex;

    use super::*;

    #[test]
    fn scene_graph_returns_nothing_on_empty_iteration() {
        let scene_graph = SceneGraph::new("Root");

        assert!(scene_graph.iter().next().is_none());
    }

    #[test]
    fn normal_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
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
        let root_idx = NodeIndex::Root;
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
        let root_idx = NodeIndex::Root;
        sg.attach(root_idx, "First Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter().map(|(_parent, value)| value).cloned()),
            vec!["First Child",]
        );
    }
}
