use std::cmp::Eq;
use thunderdome::{Arena, Index};

mod detatch_iter;
mod iter;
// mod iter_mut;

pub use detatch_iter::{DetachedNode, SceneGraphDetachIter};
pub use iter::SceneGraphIter;
// pub use iter_mut::SceneGraphIterMut;

#[derive(Debug)]
pub struct SceneGraph<T> {
    root_idx: Index,
    arena: Arena<Node<T>>,
}

impl<T> SceneGraph<T> {
    /// We take a root node here, but we will never actually give this root node back
    /// in any iteration.
    pub fn new(root: T) -> Self {
        let mut arena = Arena::new();
        let root_index = arena.insert(Node::new(root));
        Self {
            root_idx: root_index,
            arena,
        }
    }

    /// Clears all nodes from `self`, leaving the `Root` in place. If you want to edit the root too,
    /// just make a new SceneGraph.
    ///
    /// Note: this method maintains the underlying container's size, so future attaches could have
    /// some performance gains.
    pub fn clear(&mut self) {
        let root = self.arena.remove(self.root_idx).unwrap();
        self.arena.clear();
        self.root_idx = self.arena.insert(Node::new(root.value));
    }

    /// Checks if the SceneGraph contains only the root.
    pub fn is_empty(&self) -> bool {
        self.arena.len() == 1
    }

    /// Attaches a node to another node, returning a handle to it.
    pub fn attach(&mut self, parent: NodeIndex, value: T) -> Result<NodeIndex, SceneGraphErr> {
        // push that node!
        let new_idx = self.arena.insert(Node::new(value));

        let parent = self
            .arena
            .get_mut(parent.0)
            .ok_or(SceneGraphErr::ParentNodeNotFound)?;

        // fix the parent's last child
        match &mut parent.children {
            Some(children) => {
                let old_last = children.last;
                children.last = new_idx;

                let mut last_sibling = &mut self.arena[old_last];
                last_sibling.next_sibling = Some(new_idx);
            }
            None => {
                parent.children = Some(Children {
                    first: new_idx,
                    last: new_idx,
                });
            }
        };

        Ok(NodeIndex(new_idx))
    }

    /// Removes a given node from the scene graph, returning a new SceneGraph where the given
    /// node is now the *root*.
    pub fn detach(&mut self, node_index: NodeIndex) -> Option<SceneGraph<T>> {
        let node = self.arena.remove(node_index.0)?;
        let mut new_sg = SceneGraph::new(node.value);

        let mut helper_map = std::collections::HashMap::new();
        helper_map.insert(node_index, new_sg.root_idx);

        for detached_node in SceneGraphDetachIter::new(self, node_index, node.children) {
            let parent_place = helper_map.get(&detached_node.parent_idx).unwrap();
            let new_idx = new_sg
                .attach(NodeIndex(*parent_place), detached_node.node_value)
                .unwrap();

            helper_map.insert(detached_node.node_idx, new_idx.0);
        }

        // fix up the parent if it was the first child...
        let parent_index = self.parent(node_index).unwrap();
        let mut parent_children = self.arena[parent_index.0].children.unwrap();

        if parent_children.first == parent_children.last && parent_children.first == node_index.0 {
            self.arena[parent_index.0].children = None;
        } else {
            // extremely hard to follow the logic of this unwrap here, but if this branch is taken,
            // then we're *never* the last child, which means we have a sibling.
            if parent_children.first == node_index.0 {
                parent_children.first = node.next_sibling.unwrap();
            }

            // fix up the next children...
            let mut last_valid_child = parent_children.first;
            loop {
                let sibling = self.arena.get_mut(last_valid_child).unwrap();
                if sibling.next_sibling == Some(node_index.0) {
                    sibling.next_sibling = node.next_sibling;

                    // pop over our deletion or break
                    match node.next_sibling {
                        Some(next_sibling) => {
                            last_valid_child = next_sibling;
                        }
                        None => {
                            break;
                        }
                    }
                }

                if sibling.next_sibling.is_none() {
                    break;
                }

                last_valid_child = sibling.next_sibling.unwrap();
            }

            if parent_children.last == node_index.0 {
                parent_children.last = last_valid_child;
            }

            // finally, dump our updated parent children back
            self.arena[parent_index.0].children = Some(parent_children);
        }

        Some(new_sg)
    }

    pub fn root_idx(&self) -> NodeIndex {
        NodeIndex(self.root_idx)
    }

    /// Gets a given node based on `NodeIndex`.
    pub fn get(&self, node_index: NodeIndex) -> Option<&Node<T>> {
        self.arena.get(node_index.0)
    }

    pub fn root(&self) -> &T {
        &self.get_root().value
    }

    pub fn root_mut(&mut self) -> &mut T {
        &mut self.arena.get_mut(self.root_idx).unwrap().value
    }

    pub fn get_root(&self) -> &Node<T> {
        self.get(self.root_idx()).unwrap()
    }

    /// Returns the parent NodeIndex of a given Node.
    ///
    /// This operation is O(N) over the number of nodes in the SceneGraph.
    pub fn parent(&self, node_index: NodeIndex) -> Option<NodeIndex> {
        for (i, v) in self.arena.iter() {
            if let Some(children) = v.children {
                let mut child = children.first;
                loop {
                    if child == node_index.0 {
                        return Some(NodeIndex(i));
                    } else if child == children.last {
                        break;
                    } else {
                        // we know it has a next sibling always...
                        child = self.arena[child].next_sibling.unwrap();
                    }
                }
            }
        }

        None
    }

    // /// Iterate mutably over the Scene Graph in a depth first traversal.
    // pub fn iter_mut(&mut self) -> SceneGraphIterMut<'_, T> {
    //     SceneGraphIterMut::new(self)
    // }

    /// Iterate immutably over the Scene Graph in a depth first traversal.
    pub fn iter(&self) -> SceneGraphIter<'_, T> {
        SceneGraphIter::new(self, self.get_root())
    }

    /// Iterate while detaching over the Scene Graph in a depth first traversal.
    ///
    /// Note: the `root` will never be detached.
    pub fn iter_detach(&mut self) -> SceneGraphDetachIter<'_, T> {
        SceneGraphDetachIter::new(self, NodeIndex(self.root_idx), self.get_root().children)
    }
}

impl<T: PartialEq> SceneGraph<T> {
    /// Gets the index of a given value of T, if it's in the map.
    pub fn get_index(&self, value: &T) -> Option<NodeIndex> {
        for (i, v) in self.arena.iter() {
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
}

impl<'a, T> IntoIterator for &'a SceneGraph<T> {
    type Item = (&'a T, &'a T);

    type IntoIter = SceneGraphIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// impl<'a, T> IntoIterator for &'a mut SceneGraph<T> {
//     type Item = &'a mut T;

//     type IntoIter = SceneGraphIterMut<'a, T>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.iter_mut()
//     }
// }

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Node<T> {
    pub value: T,
    children: Option<Children>,
    next_sibling: Option<Index>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Children {
    first: Index,
    last: Index,
}

impl<T> std::fmt::Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("children", &self.children)
            .field("next_sibling", &self.next_sibling)
            .finish()
    }
}

impl<T> Node<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            next_sibling: None,
            children: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
#[repr(transparent)]
pub struct NodeIndex(thunderdome::Index);

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
        for (_, v) in sg.iter() {
            out.push(*v);
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

        println!("Tree looks like {:#?}", sg);

        assert_eq!(
            get_values(&sg),
            vec!["First Child", "Second Child", "First Grandchild"]
        );
    }

    #[test]
    fn attach_internals() {
        let mut sg = SceneGraph::new("Root");

        assert_eq!(sg.get_root().children, None);

        let root_idx = sg.root_idx();

        let first_idx = sg.attach(root_idx, "First Child").unwrap();

        // assert_eq!(sg.get_root().num_children, 1);
        assert_eq!(sg.get_root().children.unwrap().first, first_idx.0);
        assert_eq!(sg.get_root().children.unwrap().last, first_idx.0);

        let second_idx = sg.attach(root_idx, "Second Child").unwrap();

        assert_eq!(sg.get_root().children.unwrap().first, first_idx.0);
        assert_eq!(sg.get_root().children.unwrap().last, second_idx.0);
    }

    // #[test]
    // fn attach_bump() {
    //     let mut sg = SceneGraph::new("Root");
    //     let root_idx = sg.root_idx();
    //     let first_child = sg.attach(root_idx, "First Child").unwrap();
    //     let idx = sg.attach(first_child, "First Grandchild").unwrap();

    //     assert_eq!(idx.0.slot(), 2);
    //     sg.attach(root_idx, "Second Child").unwrap();
    //     let new_idx = sg.get_index(&"First Grandchild").unwrap();

    //     assert_ne!(idx, new_idx);
    // }

    // #[test]
    // fn attach_bump_internals() {
    //     let mut sg = SceneGraph::new("Root");
    //     let first_child = sg.attach(sg.root_idx(), "First Child").unwrap();
    //     let idx = sg.attach(first_child, "First Grandchild").unwrap();

    //     assert_eq!(idx.0, 2);
    //     assert_eq!(
    //         sg.get(first_child).unwrap().first_child,
    //         sg.get_index(&"First Grandchild").unwrap().0
    //     );

    //     sg.attach(sg.root_idx(), "Second Child").unwrap();
    //     assert_eq!(
    //         sg.get(first_child).unwrap().first_child,
    //         sg.get_index(&"First Grandchild").unwrap().0
    //     );
    // }

    #[test]
    fn detach_basic() {
        let mut sg = SceneGraph::new("Root");
        sg.attach(sg.root_idx(), "First Child").unwrap();
        let second_child = sg.attach(sg.root_idx(), "Second Child").unwrap();
        let third_child = sg.attach(sg.root_idx(), "Third Child").unwrap();

        let second_child = sg.detach(second_child).unwrap();
        assert_eq!(second_child.get_root().value, "Second Child");

        assert_eq!(get_values(&sg), vec!["First Child", "Third Child"]);

        let g = sg.attach(third_child, "First Grandchild").unwrap();
        sg.attach(g, "Second Grandchild").unwrap();
        let g_3 = sg.attach(g, "Third Grandchild").unwrap();
        sg.attach(g_3, "First Greatgrandchild").unwrap();

        let third_child_tree = sg.detach(third_child).unwrap();
        assert_eq!(get_values(&sg), vec!["First Child"]);
        assert_eq!(
            get_values(&third_child_tree),
            vec![
                "First Grandchild",
                "Second Grandchild",
                "Third Grandchild",
                "First Greatgrandchild"
            ]
        );
        assert_eq!(third_child_tree.get_root().value, "Third Child");
    }
}
