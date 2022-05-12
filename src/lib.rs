// mod iter;
// mod iter_mut;

// pub use iter::SceneGraphIter;
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

    /// Attaches a node to another node, returning a handle to it.
    pub fn attach(&mut self, parent: NodeIndex, value: T) -> Result<NodeIndex, SceneGraphErr> {
        let new_node = Node::new(value);
        let parent = self
            .arena
            .get_mut(parent.0)
            .ok_or(SceneGraphErr::ParentNodeNotFound)?;

        let idx = if parent.num_children == 0 {
            parent.num_children += 1;

            let new_node_idx = self.arena.len();
            self.arena.push(new_node);

            new_node_idx
        } else {
            let target_idx = parent.num_children as usize + parent.first_child as usize + 1;

            self.arena.insert(target_idx, new_node);

            target_idx
        };

        Ok(NodeIndex(idx))
    }

    pub fn get(&self, node_index: NodeIndex) -> Option<&Node<T>> {
        self.arena.get(node_index.0)
    }

    pub fn get_root(&self) -> &Node<T> {
        self.get(self.root_idx()).unwrap()
    }

    pub fn root_idx(&self) -> NodeIndex {
        NodeIndex(0)
    }

    // /// Iterate mutably over the Scene Graph in a depth first traversal.
    // pub fn iter_mut(&mut self) -> SceneGraphIterMut<'_, T> {
    //     SceneGraphIterMut::new(self)
    // }

    // /// Iterate immutably over the Scene Graph in a depth first traversal.
    // pub fn iter(&self) -> SceneGraphIter<'_, T> {
    //     SceneGraphIter::new(self)
    // }
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
    first_child: u32,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_values(sg: &SceneGraph<&'static str>) -> Vec<&'static str> {
        let mut out = vec![];
        for v in sg.arena.iter() {
            out.push(v.value);
        }

        out
    }

    #[test]
    fn attach_works() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = sg.root_idx();
        sg.attach(root_idx, "First Child").unwrap();
        let second_child = sg.attach(root_idx, "Second Child").unwrap();
        sg.attach(second_child, "First Grandchild").unwrap();

        assert_eq!(
            get_values(&sg),
            vec!["Root", "First Child", "Second Child", "First Grandchild"]
        );
    }
}
