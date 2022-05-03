use crate::{Node, SceneGraph};

pub struct SceneGraphIter<'a, T> {
    sg: &'a SceneGraph<T>,
    stacks: Vec<StackState<'a, T>>,
}

impl<'a, T> SceneGraphIter<'a, T> {
    pub fn new(sg: &'a SceneGraph<T>) -> Self {
        let head_node = sg.arena.get(sg.root).expect("incorrect head");
        SceneGraphIter {
            sg,
            stacks: vec![StackState::new(head_node)],
        }
    }
}

impl<'a, T> Iterator for SceneGraphIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // if we're out of stack frames, we die here
            let stack_frame = self.stacks.last_mut()?;

            match stack_frame.node.children.get(stack_frame.child_idx) {
                Some(v) => {
                    stack_frame.child_idx += 1;
                    let output_node = self.sg.arena.get(*v).expect("we have an invalid node");

                    // add this to our stack frame idx
                    self.stacks.push(StackState {
                        node: output_node,
                        child_idx: 0,
                    });

                    return Some(&output_node.value);
                }
                None => {
                    // okay we're out of children, time to POP our stack frame
                    self.stacks.pop();
                }
            }
        }
    }
}

struct StackState<'a, T> {
    node: &'a Node<T>,
    child_idx: usize,
}

impl<'a, T> StackState<'a, T> {
    fn new(node: &'a Node<T>) -> Self {
        Self { node, child_idx: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Coord(u32);

    #[test]
    fn scene_graph_returns_nothing_on_empty() {
        let mut scene_graph = SceneGraph::new(Coord(0));

        assert!(scene_graph.iter_mut().next().is_none());
    }
}
