use std::cmp::Eq;
use thunderdome::{Arena, Index};

mod child_iter;
mod detatch_iter;
mod iter;
mod iter_mut;

pub use child_iter::SceneGraphChildIter;
pub use detatch_iter::{DetachedNode, SceneGraphDetachIter};
pub use iter::SceneGraphIter;
pub use iter_mut::SceneGraphIterMut;

#[derive(Debug)]
pub struct SceneGraph<T> {
    arena: Arena<Node<T>>,
    root_idx: Index,
}

impl<T> SceneGraph<T> {
    /// We take a root node here, but we will never actually give this root node back
    /// in any iteration.
    pub fn new(root: T) -> Self {
        let mut arena = Arena::new();
        let root_index = arena.insert(Node::new(root, None));
        Self {
            arena,
            root_idx: root_index,
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
        self.root_idx = self.arena.insert(Node::new(root.value, None));
    }

    /// Checks if the SceneGraph contains only the root.
    pub fn is_empty(&self) -> bool {
        self.arena.len() == 1
    }

    /// Attaches a node to the root node, returning a handle to it.
    ///
    /// This is a convenience method which will never fail.
    pub fn attach_at_root(&mut self, value: T) -> NodeIndex {
        self.attach(self.root_idx(), value).unwrap()
    }

    /// Attaches a node to another node, returning a handle to it.
    pub fn attach(&mut self, parent: NodeIndex, value: T) -> Result<NodeIndex, SceneGraphErr> {
        // push that node!
        let new_idx = self.arena.insert(Node::new(value, Some(parent.0)));

        self.place_node(parent.0, new_idx)?;

        Ok(NodeIndex(new_idx))
    }

    /// Attaches an entire scene graph to a place on this graph. The old root node will be at
    /// the returned NodeIndex.
    pub fn attach_graph(
        &mut self,
        parent: NodeIndex,
        mut other_graph: SceneGraph<T>,
    ) -> Result<NodeIndex, SceneGraphErr> {
        let other_root = other_graph.arena.remove(other_graph.root_idx).unwrap();
        let new_root_idx = self.attach(parent, other_root.value)?;

        let mut helper_map = std::collections::HashMap::new();
        helper_map.insert(other_graph.root_idx(), new_root_idx.0);

        for detached_node in other_graph.iter_detach() {
            let parent_place = helper_map.get(&detached_node.parent_idx).unwrap();
            let new_idx = self
                .attach(NodeIndex(*parent_place), detached_node.node_value)
                .unwrap();

            helper_map.insert(detached_node.node_idx, new_idx.0);
        }

        Ok(new_root_idx)
    }

    /// Removes a given node from the scene graph, returning a new SceneGraph where the given
    /// node is now the *root*.
    pub fn detach(&mut self, node_index: NodeIndex) -> Option<SceneGraph<T>> {
        if node_index.0 == self.root_idx {
            return None;
        }

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

        self.fix_parent(
            node.next_sibling,
            node.last_sibling,
            node.parent.unwrap(),
            node_index.0,
        );

        Some(new_sg)
    }

    /// Moves a node from one parent to another parent. If this operation returns `Err`, then
    /// nothing will have happened to the node.
    pub fn move_node(
        &mut self,
        moving_node_idx: NodeIndex,
        new_parent: NodeIndex,
    ) -> Result<(), SceneGraphErr> {
        if moving_node_idx.0 == self.root_idx {
            return Err(SceneGraphErr::CannotRemoveRoot);
        }

        if !self.arena.contains(moving_node_idx.0) || !self.arena.contains(new_parent.0) {
            return Err(SceneGraphErr::NodeDoesNotExist);
        }

        // okay, now we hot swap these bitches
        let moving_node = self
            .arena
            .get_mut(moving_node_idx.0)
            .expect("we checked earlier");
        let old_parent = moving_node.parent.unwrap();
        moving_node.parent = Some(new_parent.0);

        let next_sibling = moving_node.next_sibling;
        moving_node.next_sibling = None;
        let last_sibling = moving_node.last_sibling;

        // now let's fix our old dad
        self.fix_parent(next_sibling, last_sibling, old_parent, moving_node_idx.0);

        // place it!
        self.place_node(new_parent.0, moving_node_idx.0)
            .expect("we checked earlier");

        Ok(())
    }

    /// Removes a node *without* returning anything. This can save a few allocations.
    pub fn remove(&mut self, node_index: NodeIndex) {
        if node_index.0 == self.root_idx {
            return;
        }

        let node = self.arena.remove(node_index.0);
        let node = if let Some(node) = node { node } else { return };

        // detach em all!
        for _v in SceneGraphDetachIter::new(self, node_index, node.children) {}

        self.fix_parent(
            node.next_sibling,
            node.last_sibling,
            node.parent.unwrap(),
            node_index.0,
        );
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
    /// This operation is O1 over the number of nodes in the SceneGraph.
    /// Note: this returns `None` for the Root.
    pub fn parent(&self, node_index: NodeIndex) -> Option<NodeIndex> {
        self.get(node_index)?.parent.map(NodeIndex)
    }

    // /// Iterate mutably over the Scene Graph in a depth first traversal.
    // pub fn iter_mut(&mut self) -> SceneGraphIterMut<'_, T> {
    //     SceneGraphIterMut::new(self)
    // }

    /// Iterate immutably over the Scene Graph in a depth first traversal.
    pub fn iter(&self) -> SceneGraphIter<'_, T> {
        SceneGraphIter::new(self, self.get_root())
    }

    /// Iterate mutably over the Scene Graph in a depth first traversal.
    pub fn iter_mut(&mut self) -> SceneGraphIterMut<'_, T> {
        SceneGraphIterMut::new(self, self.root_idx)
    }

    /// Iterate immutably over the Scene Graph in a depth first traversal.
    pub fn iter_on_node(
        &self,
        node_index: NodeIndex,
    ) -> Result<SceneGraphIter<'_, T>, SceneGraphErr> {
        let node = self
            .arena
            .get(node_index.0)
            .ok_or(SceneGraphErr::NodeDoesNotExist)?;

        Ok(SceneGraphIter::new(self, node))
    }

    /// Iterate while detaching over the Scene Graph in a depth first traversal.
    ///
    /// Note: the `root` will never be detached.
    pub fn iter_detach(&mut self) -> SceneGraphDetachIter<'_, T> {
        SceneGraphDetachIter::new(self, NodeIndex(self.root_idx), self.get_root().children)
    }

    /// Iterate directly over only the *direct* children of `parent_index`.
    ///
    /// For example, given a graph:
    /// ROOT:
    ///     A
    ///         B
    ///         C
    ///             D
    /// using `iter_children` and passing in the `parent_index` for `A` will only yield `B`
    /// and `C`, *not* `D`. For that kind of depth first traversal, using `iter_on_node`.
    pub fn iter_children(
        &self,
        parent_index: NodeIndex,
    ) -> Result<SceneGraphChildIter<'_, T>, SceneGraphErr> {
        let node = self
            .arena
            .get(parent_index.0)
            .ok_or(SceneGraphErr::NodeDoesNotExist)?;

        Ok(SceneGraphChildIter::new(self, node))
    }

    /// Places a node as part of moving or attaching it.
    fn place_node(&mut self, new_parent: Index, node_to_place: Index) -> Result<(), SceneGraphErr> {
        // okay, now we gotta ATTACH ourselves back, without being monsters about it
        let parent = self
            .arena
            .get_mut(new_parent)
            .ok_or(SceneGraphErr::ParentNodeNotFound)?;

        // slap ourselves in here
        match &mut parent.children {
            Some(children) => {
                let old_last = children.last;
                children.last = node_to_place;

                let mut last_sibling = &mut self.arena[old_last];
                last_sibling.next_sibling = Some(node_to_place);

                // fix this up too
                self.arena[node_to_place].last_sibling = Some(old_last);
            }
            None => {
                // this is the easy case
                parent.children = Some(Children {
                    first: node_to_place,
                    last: node_to_place,
                });
            }
        };

        Ok(())
    }

    /// Fixes a parent with a removed child.
    fn fix_parent(
        &mut self,
        removed_next_sibling: Option<Index>,
        removed_last_sibling: Option<Index>,
        removed_parent: Index,
        removed_idx: Index,
    ) {
        // fix up the parent if it was the first child...
        let mut parent_children = self.arena[removed_parent].children.unwrap();

        if parent_children.first == parent_children.last && parent_children.first == removed_idx {
            self.arena[removed_parent].children = None;
        } else {
            // extremely hard to follow the logic of this unwrap here, but if this branch is taken,
            // then we're *never* the last child, which means we have a sibling.
            if parent_children.first == removed_idx {
                parent_children.first = removed_next_sibling.unwrap();
            }

            if parent_children.last == removed_idx {
                parent_children.last = removed_last_sibling.unwrap();
            }

            if let Some(last_sibling) = removed_last_sibling {
                let last_sibling = self.arena.get_mut(last_sibling).unwrap();
                last_sibling.next_sibling = removed_next_sibling;
            }

            if let Some(next_sibling) = removed_next_sibling {
                let next_sibling = self.arena.get_mut(next_sibling).unwrap();
                next_sibling.last_sibling = removed_last_sibling;
            }

            // finally, dump our updated parent children back
            self.arena[removed_parent].children = Some(parent_children);
        }
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

impl<T> std::ops::Index<NodeIndex> for SceneGraph<T> {
    type Output = T;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self.arena[index.0].value
    }
}

impl<T> std::ops::IndexMut<NodeIndex> for SceneGraph<T> {
    fn index_mut(&mut self, index: NodeIndex) -> &mut Self::Output {
        &mut self.arena[index.0].value
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Node<T> {
    pub value: T,
    parent: Option<Index>,
    children: Option<Children>,
    last_sibling: Option<Index>,
    next_sibling: Option<Index>,
}

impl<T> Node<T> {
    /// Returns true if the node has is the root node or not.
    ///
    /// Note that this is the inverse of `has_parent`.
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// Returns true if the node has a parent.
    ///
    /// Note that this is the inverse of `is_root`.
    pub fn has_parent(&self) -> bool {
        self.parent.is_some()
    }

    /// Returns true if this node has children.
    pub fn has_children(&self) -> bool {
        self.children.is_some()
    }

    /// Iterate directly over only the *direct* children of `parent_index`.
    ///
    /// For example, given a graph:
    /// ROOT:
    ///     A
    ///         B
    ///         C
    ///             D
    /// using `iter_children` and passing in the `parent_index` for `A` will only yield `B`
    /// and `C`, *not* `D`. For that kind of depth first traversal, using `iter_on_node`.
    ///
    /// Note: passing in a SceneGraph of a different kind than this node belongs to (but of the same type)
    /// will create logic errors or panics.
    pub fn iter_children<'a>(&'a self, sg: &'a SceneGraph<T>) -> SceneGraphChildIter<'a, T> {
        SceneGraphChildIter::new(sg, self)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Children {
    first: Index,
    last: Index,
}

impl<T> std::fmt::Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("parent", &self.parent)
            .field("children", &self.children)
            .field("next_sibling", &self.next_sibling)
            .finish()
    }
}

impl<T> Node<T> {
    pub fn new(value: T, parent: Option<Index>) -> Self {
        Self {
            value,
            parent,
            last_sibling: None,
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

    #[error("node does not exist")]
    NodeDoesNotExist,

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

        assert_eq!(sg.get(first_idx).unwrap().next_sibling, Some(second_idx.0));
        assert_eq!(sg.get(first_idx).unwrap().last_sibling, None);

        assert_eq!(sg.get(second_idx).unwrap().next_sibling, None);
        assert_eq!(sg.get(second_idx).unwrap().last_sibling, Some(first_idx.0));
    }

    #[test]
    fn detach_basic() {
        let mut sg = SceneGraph::new("Root");
        let first_child = sg.attach(sg.root_idx(), "First Child").unwrap();
        let second_child = sg.attach(sg.root_idx(), "Second Child").unwrap();
        let third_child = sg.attach(sg.root_idx(), "Third Child").unwrap();

        let second_child = sg.detach(second_child).unwrap();
        assert_eq!(second_child.get_root().value, "Second Child");

        assert_eq!(
            sg.get(sg.root_idx()).unwrap().children.unwrap().first,
            first_child.0
        );
        assert_eq!(
            sg.get(sg.root_idx()).unwrap().children.unwrap().last,
            third_child.0
        );

        assert_eq!(sg.get(first_child).unwrap().last_sibling, None);
        assert_eq!(
            sg.get(first_child).unwrap().next_sibling,
            Some(third_child.0)
        );

        assert_eq!(
            sg.get(third_child).unwrap().last_sibling,
            Some(first_child.0)
        );
        assert_eq!(sg.get(third_child).unwrap().next_sibling, None);

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

    #[test]
    fn move_node() {
        let mut sg = SceneGraph::new("Root");
        let fg = sg.attach(sg.root_idx(), "First Child").unwrap();
        sg.attach(fg, "First Grandchild").unwrap();
        sg.attach(fg, "Second Grandchild").unwrap();
        sg.attach(fg, "Third Grandchild").unwrap();
        let second_child = sg.attach(sg.root_idx(), "Second Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_children(fg).unwrap().cloned()),
            vec!["First Grandchild", "Second Grandchild", "Third Grandchild",]
        );

        sg.move_node(fg, second_child).unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_children(sg.root_idx()).unwrap().cloned()),
            vec!["Second Child",]
        );

        assert_eq!(
            Vec::from_iter(sg.iter_children(fg).unwrap().cloned()),
            vec!["First Grandchild", "Second Grandchild", "Third Grandchild",]
        );

        assert_eq!(
            Vec::from_iter(sg.iter_children(second_child).unwrap().cloned()),
            vec!["First Child",]
        );
    }
}
