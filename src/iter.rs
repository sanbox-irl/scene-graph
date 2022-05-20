use crate::{Node, SceneGraph};

pub struct SceneGraphIter<'a, T> {
    sg: &'a SceneGraph<T>,
    stacks: Vec<StackState<'a, T>>,
}

impl<'a, T> SceneGraphIter<'a, T> {
    pub fn new(sg: &'a SceneGraph<T>) -> Self {
        let head_node = sg.get_root();
        let first_child = sg.arena.get(head_node.first_child);
        SceneGraphIter {
            sg,
            stacks: vec![StackState::new(head_node, first_child)],
        }
    }
}

impl<'a, T> Iterator for SceneGraphIter<'a, T> {
    type Item = (&'a T, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // if we're out of stack frames, we die here
            let stack_frame = self.stacks.last_mut()?;

            match &mut stack_frame.current_child {
                Some(child) => {
                    let next_sibling = child.next_sibling;
                    // okay time to change the child
                    let child = std::mem::replace(
                        &mut stack_frame.current_child,
                        self.sg.arena.get(next_sibling),
                    )
                    .unwrap();

                    return Some((&stack_frame.parent.value, &child.value));
                }
                None => {
                    // we're done here.
                    self.stacks.pop();
                    continue;
                }
            }
        }
    }
}

#[derive(Debug)]
struct StackState<'a, T> {
    parent: &'a Node<T>,
    current_child: Option<&'a Node<T>>,
}

impl<'a, T> StackState<'a, T> {
    fn new(parent: &'a Node<T>, first_child: Option<&'a Node<T>>) -> Self {
        Self {
            parent,
            current_child: first_child,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Coord(u32);

    #[test]
    fn scene_graph_returns_nothing_on_empty() {
        let scene_graph = SceneGraph::new(Coord(0));

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
