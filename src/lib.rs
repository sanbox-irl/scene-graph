use std::cmp::Eq;

mod iter;
// mod iter_mut;

pub use iter::SceneGraphIter;
// pub use iter_mut::SceneGraphIterMut;

pub struct SceneGraph<T> {
    arena: Vec<Node<T>>,
}

impl<T> SceneGraph<T> {
    /// We take a root node here, but we will never actually give this root node back
    /// in any iteration.
    pub fn new(root: T) -> Self {
        Self {
            arena: vec![Node::new(root)],
        }
    }

    /// Clears all nodes from `self`, leaving the `Root` in place. If you want to edit the root too,
    /// just make a new SceneGraph.
    pub fn clear(&mut self) {
        let root = self.arena.swap_remove(0);
        self.arena.clear();
        self.arena.push(Node::new(root.value));
    }

    /// Attaches a node to another node, returning a handle to it.
    #[cfg(not(feature = "deny_double"))]
    pub fn attach(&mut self, parent: NodeIndex, value: T) -> Result<NodeIndex, SceneGraphErr> {
        self.attach_inner(parent, value)
    }

    /// This is the inner attach!
    fn attach_inner(&mut self, parent: NodeIndex, value: T) -> Result<NodeIndex, SceneGraphErr> {
        let arena_len = self.arena.len();

        let new_node = Node::new(value);
        let parent = self
            .arena
            .get_mut(parent.0)
            .ok_or(SceneGraphErr::ParentNodeNotFound)?;

        let idx = if parent.num_children == 0 {
            parent.num_children += 1;
            parent.first_child = arena_len;

            self.arena.push(new_node);

            arena_len
        } else {
            let target_idx = parent.num_children as usize + parent.first_child as usize;
            parent.num_children += 1;
            self.arena.insert(target_idx, new_node);

            // now we need to increment *everyone*
            for node in self.arena.iter_mut() {
                if node.first_child >= target_idx {
                    node.first_child += 1;
                }
            }

            target_idx
        };

        Ok(NodeIndex(idx))
    }

    /// Removes a given node from the scene graph.
    ///
    /// ## Panics
    /// Panics if index is out of bounds.
    pub fn remove(&mut self, node_index: NodeIndex) -> Node<T> {
        let node = self.arena.remove(node_index.0);

        // decrement everyone
        // now we need to increment *everyone*
        for node in self.arena.iter_mut() {
            // check parent...
            if (node.first_child..(node.first_child + node.num_children as usize))
                .contains(&node_index.0)
            {
                node.num_children -= 1;
            } else if node.first_child > node_index.0 {
                node.first_child -= 1;
            }
        }

        node
    }

    pub const fn root_idx(&self) -> NodeIndex {
        NodeIndex(0)
    }

    /// Gets a given node based on `NodeIndex`.
    ///
    /// **Warning:** `NodeIndex` is not stable across `attach` and `remove`, and so,
    /// you should consider using `get_by_value` instead.
    pub fn get(&self, node_index: NodeIndex) -> Option<&Node<T>> {
        self.arena.get(node_index.0)
    }

    pub fn get_root(&self) -> &Node<T> {
        self.get(self.root_idx()).unwrap()
    }

    // /// Iterate mutably over the Scene Graph in a depth first traversal.
    // pub fn iter_mut(&mut self) -> SceneGraphIterMut<'_, T> {
    //     SceneGraphIterMut::new(self)
    // }

    /// Iterate immutably over the Scene Graph in a depth first traversal.
    pub fn iter(&self) -> SceneGraphIter<'_, T> {
        SceneGraphIter::new(self)
    }
}

impl<T: PartialEq> SceneGraph<T> {
    /// Gets the index of a given value of T, if it's in the map.
    pub fn get_index(&self, value: &T) -> Option<NodeIndex> {
        for (i, v) in self.arena.iter().enumerate() {
            if v.value.eq(value) {
                return Some(NodeIndex(i));
            }
        }

        None
    }

    /// Gets the node of a given value of T, if it's in the map.
    pub fn get_by_value(&self, value: &T) -> Option<&Node<T>> {
        let idx = self.get_index(value)?;

        self.get(idx)
    }

    /// Attaches a node to another node, returning a handle to it.
    #[cfg(feature = "deny_double")]
    pub fn attach(&mut self, parent: NodeIndex, value: T) -> Result<NodeIndex, SceneGraphErr> {
        for v in self.arena.iter() {
            if v.value.eq(&value) {
                return Err(SceneGraphErr::NodeAlreadyPresent);
            }
        }

        self.attach_inner(parent, value)
    }
}

// impl<'a, T> IntoIterator for &'a SceneGraph<T> {
//     type Item = &'a T;

//     type IntoIter = SceneGraphIter<'a, T>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.iter()
//     }
// }

// impl<'a, T> IntoIterator for &'a mut SceneGraph<T> {
//     type Item = &'a mut T;

//     type IntoIter = SceneGraphIterMut<'a, T>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.iter_mut()
//     }
// }

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Node<T> {
    pub value: T,
    first_child: usize,
    num_children: u32,
}

impl<T> Node<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            first_child: 0,
            num_children: 0,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct NodeIndex(usize);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum SceneGraphErr {
    #[error("parent node not found")]
    ParentNodeNotFound,

    #[error("not cannot be attachd because it is already present")]
    NodeAlreadyPresent,

    #[error("scene graph root cannot be removed")]
    CannotRemoveRoot,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_values(sg: &SceneGraph<&'static str>) -> Vec<&'static str> {
        let mut out = vec![];
        for v in sg.iter() {
            out.push(*v);
        }

        out
    }

    fn get_arena_values(sg: &SceneGraph<&'static str>) -> Vec<&'static str> {
        let mut out = vec![];
        for v in sg.arena.iter() {
            out.push(v.value);
        }

        out
    }

    #[test]
    fn cannot_double_attach() {
        let mut sg = SceneGraph::new("Root");
        sg.attach(sg.root_idx(), "Bad").unwrap();
        sg.attach(sg.root_idx(), "Oh No!").unwrap();
    }

    #[test]
    fn basic_attach() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = sg.root_idx();
        sg.attach(root_idx, "First Child").unwrap();
        let second_child = sg.attach(root_idx, "Second Child").unwrap();
        sg.attach(second_child, "First Grandchild").unwrap();

        assert_eq!(
            get_values(&sg),
            vec!["First Child", "Second Child", "First Grandchild"]
        );
    }

    #[test]
    fn attach_internals() {
        let mut sg = SceneGraph::new("Root");

        assert_eq!(sg.get_root().num_children, 0);
        assert_eq!(sg.get_root().first_child, 0);

        let root_idx = sg.root_idx();

        let first_idx = sg.attach(root_idx, "First Child").unwrap();

        assert_eq!(sg.get_root().num_children, 1);
        assert_eq!(sg.get_root().first_child, first_idx.0);

        sg.attach(root_idx, "Second Child").unwrap();

        assert_eq!(sg.get_root().num_children, 2);
        assert_eq!(sg.get_root().first_child, first_idx.0);
    }

    #[test]
    fn attach_bump() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = sg.root_idx();
        let first_child = sg.attach(root_idx, "First Child").unwrap();
        let idx = sg.attach(first_child, "First Grandchild").unwrap();

        assert_eq!(idx.0, 2);
        sg.attach(root_idx, "Second Child").unwrap();
        let new_idx = sg.get_index(&"First Grandchild").unwrap();

        assert_ne!(idx, new_idx);
    }

    #[test]
    fn attach_bump_internals() {
        let mut sg = SceneGraph::new("Root");
        let first_child = sg.attach(sg.root_idx(), "First Child").unwrap();
        let idx = sg.attach(first_child, "First Grandchild").unwrap();

        assert_eq!(idx.0, 2);
        assert_eq!(
            sg.get(first_child).unwrap().first_child,
            sg.get_index(&"First Grandchild").unwrap().0
        );

        sg.attach(sg.root_idx(), "Second Child").unwrap();
        assert_eq!(
            sg.get(first_child).unwrap().first_child,
            sg.get_index(&"First Grandchild").unwrap().0
        );
    }

    #[test]
    fn remove() {
        let mut sg = SceneGraph::new("Root");
        sg.attach(sg.root_idx(), "First Child").unwrap();
        let second_child = sg.attach(sg.root_idx(), "Second Child").unwrap();
        sg.attach(sg.root_idx(), "Third Child").unwrap();

        sg.remove(second_child);

        assert_eq!(get_values(&sg), vec!["First Child", "Third Child"]);

        let third_child = sg.get_index(&"Third Child").unwrap();
        sg.attach(third_child, "First Grandchild").unwrap();
        let third_child = sg.get_index(&"Third Child").unwrap();
        sg.remove(third_child);
        assert_eq!(get_values(&sg), vec!["First Child"]);
        assert_eq!(
            get_arena_values(&sg),
            vec!["Root", "First Child", "First Grandchild"]
        );
    }
}
