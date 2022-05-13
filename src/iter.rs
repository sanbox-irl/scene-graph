use crate::{Node, SceneGraph};

pub struct SceneGraphIter<'a, T> {
    sg: &'a SceneGraph<T>,
    stacks: Vec<StackState<'a, T>>,
}

impl<'a, T> SceneGraphIter<'a, T> {
    pub fn new(sg: &'a SceneGraph<T>) -> Self {
        let head_node = sg.get_root();
        SceneGraphIter {
            sg,
            stacks: vec![StackState::new(head_node)],
        }
    }
}

impl<'a, T: std::fmt::Debug> Iterator for SceneGraphIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            println!("stack frame = {:#?}", self.stacks);
            // if we're out of stack frames, we die here
            let stack_frame = self.stacks.last_mut()?;

            println!("num_children = {}", stack_frame.node.num_children);
            let current_idx = stack_frame.node.first_child + stack_frame.child_idx as usize;
            if stack_frame.child_idx == stack_frame.node.num_children {
                self.stacks.pop();
                continue;
            }
            stack_frame.child_idx += 1;

            let output = &self.sg.arena[current_idx];

            // add this to our stack frame idx
            self.stacks.push(StackState {
                node: output,
                child_idx: 0,
            });

            return Some(&output.value);
        }
    }
}

#[derive(Debug)]
struct StackState<'a, T> {
    node: &'a Node<T>,
    child_idx: u32,
}

impl<'a, T> StackState<'a, T> {
    fn new(node: &'a Node<T>) -> Self {
        Self { node, child_idx: 0 }
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
            Vec::from_iter(sg.iter().cloned()),
            vec!["First Child", "Second Child", "First Grandchild"]
        );
    }

    #[test]
    fn single_iteration() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = sg.root_idx();
        sg.attach(root_idx, "First Child").unwrap();

        assert_eq!(Vec::from_iter(sg.iter().cloned()), vec!["First Child",]);
    }
}
