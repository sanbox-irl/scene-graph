use crate::{Node, SceneGraph};

pub struct SceneGraphIterMut<'a, T> {
    sg: &'a mut SceneGraph<T>,
    stacks: Vec<StackStateMut<T>>,
}

impl<'a, T> SceneGraphIterMut<'a, T> {
    pub fn new(sg: &'a mut SceneGraph<T>) -> Self {
        let head_node = sg.arena.get_mut(sg.root).expect("incorrect head");
        let head_node = head_node as *mut _;
        SceneGraphIterMut {
            sg,
            stacks: vec![StackStateMut::new(head_node)],
        }
    }
}

impl<'a, T> Iterator for SceneGraphIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // if we're out of stack frames, we die here
            let stack_frame = self.stacks.last_mut()?;

            let parent_node = unsafe { &mut *stack_frame.node };

            match parent_node.children.get_mut(stack_frame.child_idx) {
                Some(v) => {
                    stack_frame.child_idx += 1;
                    let output_node = self.sg.arena.get_mut(*v).expect("we have an invalid node");
                    let output_node: *mut Node<T> = output_node as *mut _;

                    // add this to our stack frame idx
                    self.stacks.push(StackStateMut {
                        node: output_node,
                        child_idx: 0,
                    });

                    // SAFETY: we do this literally just to obfuscate this to the borrow chcker,
                    // and extend the lifetime. Our promise is to never give out the same
                    // node twice in an iteration run, so we will not have an issue of having two
                    // mutable ptrs to the same data
                    let output_node: &mut Node<T> = unsafe { &mut *output_node };

                    return Some(&mut output_node.value);
                }
                None => {
                    // okay we're out of children, time to POP our stack frame
                    self.stacks.pop();
                }
            }
        }
    }
}

struct StackStateMut<T> {
    node: *mut Node<T>,
    child_idx: usize,
}

impl<T> StackStateMut<T> {
    fn new(node: *mut Node<T>) -> Self {
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
