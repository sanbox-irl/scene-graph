#![doc = include_str!("../README.md")]
#![deny(rust_2018_idioms)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

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

/// The core structure of `scene-graph`. This forms a rose tree, similar to a geneological tree.
/// In this crate, we use geneological terms like `parent`, `child`, and `sibling` to describe node
/// relationships.
///
/// A scene graph is composed of nodes, which have children, which are other nodes.
/// Nodes additionally have siblings, which is determined by order of insertion into the graph.
///
/// You can traverse the SceneGraph by `iter`, to iterate downwards over the entire graph, or
/// `iter_on_node`, to iterate downward from a particular node. This crate has no ability to iterate
/// upwards, but you can use `get` to find a node's parent. If iterating upwards if useful to your
/// usecase, please file an issue. Additionally, there are mutable variants of all of these
/// iterators available.
#[derive(Debug)]
pub struct SceneGraph<T> {
    arena: Arena<Node<T>>,
    root: T,
    root_children: Option<Children>,
}

impl<T> SceneGraph<T> {
    /// Creates a new `SceneGraph`
    pub const fn new(root: T) -> Self {
        Self {
            arena: Arena::new(),
            root,
            root_children: None,
        }
    }

    /// Clears all nodes from `self`, leaving the `Root` in place. If you want to edit the root too,
    /// just make a new SceneGraph.
    ///
    /// Note: this method maintains the underlying container's size, so future attaches could have
    /// some performance gains.
    pub fn clear(&mut self) {
        self.arena.clear();
        self.root_children = None;
    }

    /// Returns the number of NON-ROOT nodes in the graph.
    pub fn len(&self) -> usize {
        self.arena.len()
    }

    /// Checks if the SceneGraph contains only the root.
    pub fn is_empty(&self) -> bool {
        self.root_children.is_none()
    }

    /// Attaches a node to the root node, returning a handle to it.
    ///
    /// This is a convenience method which will never fail.
    pub fn attach_at_root(&mut self, value: T) -> NodeIndex {
        self.attach(NodeIndex::Root, value).unwrap()
    }

    /// Attaches a node to another node, returning a handle to it.
    pub fn attach(&mut self, parent: NodeIndex, value: T) -> Result<NodeIndex, ParentNodeNotFound> {
        // push that node!
        let new_idx = self.arena.insert(Node::new(value, parent));
        self.place_node(parent, new_idx)?;

        Ok(NodeIndex::Branch(new_idx))
    }

    /// Attaches an entire scene graph to a place on this graph. The old root node will be at
    /// the returned NodeIndex.
    pub fn attach_graph_unsafe(
        &mut self,
        parent: NodeIndex,
        mut other_graph: SceneGraph<T>,
    ) -> Result<NodeIndex, ParentNodeNotFound> {
        let other_root = other_graph.root;
        let new_root_idx = self.attach(parent, other_root)?;

        let mut helper_map = std::collections::HashMap::new();
        helper_map.insert(NodeIndex::Root, new_root_idx);

        let detach_iter = SceneGraphDetachIter::new(&mut other_graph.arena, NodeIndex::Root, other_graph.root_children);

        for detached_node in detach_iter {
            let parent_place = helper_map.get(&detached_node.parent_idx).unwrap();
            let new_idx = self.attach(*parent_place, detached_node.node_value).unwrap();

            helper_map.insert(detached_node.node_idx, new_idx);
        }

        Ok(new_root_idx)
    }

    /// Removes a given node from the scene graph, returning a new SceneGraph where the given
    /// node is now the *root*.
    ///
    /// Note: this always returns `None` when the node doesn't exist, or when the `node_index` is
    /// the Root.
    pub fn detach(&mut self, node_index: NodeIndex) -> Option<SceneGraph<T>> {
        let node_index = match node_index {
            NodeIndex::Root => return None,
            NodeIndex::Branch(idx) => idx,
        };

        let node = self.arena.remove(node_index)?;
        let mut new_sg = SceneGraph::new(node.value);

        let mut helper_map = std::collections::HashMap::new();
        helper_map.insert(NodeIndex::Branch(node_index), NodeIndex::Root);

        for detached_node in SceneGraphDetachIter::new(&mut self.arena, NodeIndex::Branch(node_index), node.children) {
            println!("detached_node = {:#?}", detached_node);

            let parent_place = match detached_node.parent_idx {
                NodeIndex::Root => NodeIndex::Root,
                NodeIndex::Branch(_) => *helper_map.get(&detached_node.parent_idx).unwrap(),
            };
            let new_idx = new_sg.attach(parent_place, detached_node.node_value).unwrap();

            helper_map.insert(detached_node.node_idx, new_idx);
        }

        self.fix_parent(node.next_sibling, node.last_sibling, node.parent, node_index);

        Some(new_sg)
    }

    /// Moves a node from one parent to another parent. If this operation returns `Err`, then
    /// nothing will have happened to the node.
    pub fn move_node(&mut self, moving_node_idx: NodeIndex, new_parent: NodeIndex) -> Result<(), NodeDoesNotExist> {
        let moving_node_idx = match moving_node_idx {
            NodeIndex::Root => return Err(NodeDoesNotExist),
            NodeIndex::Branch(idx) => {
                if !self.arena.contains(idx) {
                    return Err(NodeDoesNotExist);
                }

                idx
            }
        };

        if let NodeIndex::Branch(idx) = new_parent {
            if !self.arena.contains(idx) {
                return Err(NodeDoesNotExist);
            }
        }

        // okay, now we hot swap these bitches
        let moving_node = self.arena.get_mut(moving_node_idx).expect("we checked earlier");
        let old_parent = moving_node.parent;
        moving_node.parent = new_parent;

        let next_sibling = moving_node.next_sibling;
        moving_node.next_sibling = None;
        let last_sibling = moving_node.last_sibling;

        // now let's fix our old dad
        self.fix_parent(next_sibling, last_sibling, old_parent, moving_node_idx);

        // place it!
        self.place_node(new_parent, moving_node_idx)
            .expect("we checked earlier");

        Ok(())
    }

    /// Removes a node *without* returning anything. This can save a few allocations.
    pub fn remove(&mut self, node_index: NodeIndex) {
        let index = match node_index {
            NodeIndex::Root => panic!("you cannot remove the root"),
            NodeIndex::Branch(index) => index,
        };

        let Some(node) = self.arena.remove(index) else { return };

        // detach em all!
        for _v in SceneGraphDetachIter::new(&mut self.arena, node_index, node.children) {}

        self.fix_parent(node.next_sibling, node.last_sibling, node.parent, index);
    }

    /// Returns `true` is the given `node_index` is valid.
    pub fn contains(&self, node_index: NodeIndex) -> bool {
        match node_index {
            NodeIndex::Root => true,
            NodeIndex::Branch(idx) => self.arena.contains(idx),
        }
    }

    /// Gets a given node based on `NodeIndex`. Note that the `Root` always returns `None`,
    /// as it is not a true node. Use `get_children` to generically get children.
    pub fn get(&self, node_index: NodeIndex) -> Option<&Node<T>> {
        match node_index {
            NodeIndex::Root => None,
            NodeIndex::Branch(idx) => self.arena.get(idx),
        }
    }

    /// Gets a given node based on `NodeIndex`. Note that the `Root` always returns `None`,
    /// as it is not a true node. Use `get_children` to generically get children.
    pub fn get_mut(&mut self, node_index: NodeIndex) -> Option<&mut Node<T>> {
        match node_index {
            NodeIndex::Root => None,
            NodeIndex::Branch(idx) => self.arena.get_mut(idx),
        }
    }

    /// Gets the root node's value.
    pub fn root(&self) -> &T {
        &self.root
    }

    /// Gets the root node's value mutably.
    pub fn root_mut(&mut self) -> &mut T {
        &mut self.root
    }

    /// Returns the parent NodeIndex of a given Node.
    ///
    /// This operation is O1 over the number of nodes in the SceneGraph.
    /// Note: this returns `None` for the Root.
    pub fn parent(&self, node_index: NodeIndex) -> Option<NodeIndex> {
        self.get(node_index).map(|v| v.parent)
    }

    /// Iterate mutably over the Scene Graph in a depth first traversal.
    pub fn iter_mut(&mut self) -> SceneGraphIterMut<'_, T> {
        SceneGraphIterMut::new(self, NodeIndex::Root)
    }

    /// Iterate immutably over the Scene Graph in a depth first traversal.
    pub fn iter(&self) -> SceneGraphIter<'_, T> {
        self.iter_from_node(NodeIndex::Root).unwrap()
    }

    /// Iterate immutably over the Scene Graph out of order. This is useful for speed.
    pub fn iter_out_of_order(&self) -> impl Iterator<Item = (NodeIndex, &T)> {
        self.arena.iter().map(|(k, v)| (NodeIndex::Branch(k), &v.value))
    }

    /// Iterate immutably over the Scene Graph in a depth first traversal.
    pub fn iter_from_node(&self, node_index: NodeIndex) -> Result<SceneGraphIter<'_, T>, NodeDoesNotExist> {
        let (parent_value, children) = match node_index {
            NodeIndex::Root => (&self.root, self.root_children.as_ref()),
            NodeIndex::Branch(idx) => {
                let node = self.arena.get(idx).ok_or(NodeDoesNotExist)?;

                (&node.value, node.children.as_ref())
            }
        };

        Ok(SceneGraphIter::new(self, parent_value, children))
    }

    /// Iterate immutably over the Scene Graph in a depth first traversal.
    pub fn iter_mut_from_node(&mut self, node_index: NodeIndex) -> Result<SceneGraphIterMut<'_, T>, NodeDoesNotExist> {
        match node_index {
            NodeIndex::Root => {}
            NodeIndex::Branch(idx) => {
                if !self.arena.contains(idx) {
                    return Err(NodeDoesNotExist);
                }
            }
        };

        Ok(SceneGraphIterMut::new(self, node_index))
    }

    /// Iterate while detaching over the Scene Graph in a depth first traversal.
    ///
    /// Note: the `root` will never be detached.
    pub fn iter_detach_from_root(&mut self) -> SceneGraphDetachIter<'_, T> {
        SceneGraphDetachIter::new(&mut self.arena, NodeIndex::Root, self.root_children.take())
    }

    /// Iterate while detaching over the Scene Graph in a depth first traversal.
    /// This leaves the `node_index` given, but removes all the children.
    ///
    /// Note: the `root` will never be detached.
    pub fn iter_detach(&mut self, node_index: NodeIndex) -> Result<SceneGraphDetachIter<'_, T>, NodeDoesNotExist> {
        let children = match node_index {
            NodeIndex::Root => self.root_children.take(),
            NodeIndex::Branch(br) => match self.arena.get_mut(br) {
                Some(v) => v.children.take(),
                None => return Err(NodeDoesNotExist),
            },
        };

        Ok(SceneGraphDetachIter::new(&mut self.arena, node_index, children))
    }

    /// Iterate directly over only the *direct* children of `parent_index`.
    ///
    /// For example, given a graph:
    /// ROOT:
    ///     A
    ///         B
    ///         C
    ///             D
    /// using [iter_direct_children] and passing in the `parent_index` for `A` will only yield `B`
    /// and `C`, *not* `D`. For that kind of depth first traversal, using `iter_on_node`.
    ///
    /// [iter_direct_children]: [Self::iter_direct_children]
    pub fn iter_direct_children(
        &self,
        parent_index: NodeIndex,
    ) -> Result<SceneGraphChildIter<'_, T>, NodeDoesNotExist> {
        if let NodeIndex::Branch(idx) = parent_index {
            self.arena.get(idx).ok_or(NodeDoesNotExist)?;
        }

        Ok(SceneGraphChildIter::new(self, parent_index))
    }

    /// Places a node as part of moving or attaching it.
    fn place_node(&mut self, new_parent: NodeIndex, node_to_place: Index) -> Result<(), ParentNodeNotFound> {
        // okay, now we gotta ATTACH ourselves back, without being monsters about it
        let parent_children = match new_parent {
            NodeIndex::Root => &mut self.root_children,
            NodeIndex::Branch(idx) => &mut self.arena.get_mut(idx).ok_or(ParentNodeNotFound)?.children,
        };

        // slap ourselves in here
        match parent_children.as_mut() {
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
                *parent_children = Some(Children {
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
        removed_parent: NodeIndex,
        removed_idx: Index,
    ) {
        // fix up the parent if it was the first child...

        let mut parent_children = match removed_parent {
            NodeIndex::Root => self.root_children.unwrap(),
            NodeIndex::Branch(idx) => self.arena[idx].children.unwrap(),
        };

        if parent_children.first == parent_children.last && parent_children.first == removed_idx {
            match removed_parent {
                NodeIndex::Root => self.root_children = None,
                NodeIndex::Branch(idx) => self.arena[idx].children = None,
            };
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
            match removed_parent {
                NodeIndex::Root => self.root_children = Some(parent_children),
                NodeIndex::Branch(idx) => self.arena[idx].children = Some(parent_children),
            };
        }
    }
}

impl<'a, T> IntoIterator for &'a SceneGraph<T> {
    type Item = (&'a T, &'a T);

    type IntoIter = SceneGraphIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut SceneGraph<T> {
    type Item = (&'a mut T, &'a mut T);

    type IntoIter = SceneGraphIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// A wrapper around the values given to the SceneGraph. This struct includes the data on the
/// relationships to other nodes, in addition to the value placed at the node.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Node<T> {
    /// The value contained within the node.
    pub value: T,
    parent: NodeIndex,
    children: Option<Children>,
    last_sibling: Option<Index>,
    next_sibling: Option<Index>,
}

impl<T> Node<T> {
    fn new(value: T, parent: NodeIndex) -> Self {
        Self {
            value,
            parent,
            last_sibling: None,
            next_sibling: None,
            children: None,
        }
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
    /// Note: passing in a SceneGraph of a different kind than this node belongs to (but of the same
    /// type) will create logic errors or panics.
    pub fn iter_children<'a>(&'a self, sg: &'a SceneGraph<T>) -> SceneGraphChildIter<'a, T> {
        SceneGraphChildIter::with_children(sg, self.children.as_ref())
    }

    /// Returns the index of the parent.
    pub fn parent(&self) -> NodeIndex {
        self.parent
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
struct Children {
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

/// A node index into the SceneGraph.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum NodeIndex {
    /// Signifies that the index corresponds to the root of the graph.
    Root,

    /// Signifies a non-root node.
    Branch(thunderdome::Index),
}

impl NodeIndex {
    /// Returns `true` if the node index is [`Root`].
    ///
    /// [`Root`]: NodeIndex::Root
    #[must_use]
    pub fn is_root(&self) -> bool {
        matches!(self, Self::Root)
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("parent node not found")]
/// The parent node requested was not found.
pub struct ParentNodeNotFound;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("node does not exist")]
/// The node does not exist.
pub struct NodeDoesNotExist;

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
    fn basic_attach() {
        let mut sg = SceneGraph::new("Root");
        let root_idx = NodeIndex::Root;
        sg.attach(root_idx, "First Child").unwrap();
        let second_child = sg.attach(root_idx, "Second Child").unwrap();
        sg.attach(second_child, "First Grandchild").unwrap();

        assert_eq!(get_values(&sg), vec!["First Child", "Second Child", "First Grandchild"]);
    }

    #[test]
    fn attach_internals() {
        let mut sg = SceneGraph::new("Root");

        assert_eq!(sg.root_children, None);

        let root_idx = NodeIndex::Root;

        let first_idx = sg.attach(root_idx, "First Child").unwrap();

        // assert_eq!(sg.get_root().num_children, 1);
        assert_eq!(NodeIndex::Branch(sg.root_children.unwrap().first), first_idx);
        assert_eq!(NodeIndex::Branch(sg.root_children.unwrap().last), first_idx);

        let second_idx = sg.attach(root_idx, "Second Child").unwrap();

        assert_eq!(NodeIndex::Branch(sg.root_children.unwrap().first), first_idx);
        assert_eq!(NodeIndex::Branch(sg.root_children.unwrap().last), second_idx);

        assert_eq!(
            sg.get(first_idx).unwrap().next_sibling.map(NodeIndex::Branch),
            Some(second_idx)
        );
        assert_eq!(sg.get(first_idx).unwrap().last_sibling, None);

        assert_eq!(sg.get(second_idx).unwrap().next_sibling, None);
        assert_eq!(
            sg.get(second_idx).unwrap().last_sibling.map(NodeIndex::Branch),
            Some(first_idx)
        );
    }

    #[test]
    fn detach_basic() {
        let mut sg = SceneGraph::new("Root");
        let first_child = sg.attach_at_root("First Child");
        let second_child = sg.attach_at_root("Second Child");
        let third_child = sg.attach_at_root("Third Child");

        let second_child = sg.detach(second_child).unwrap();
        assert_eq!(*second_child.root(), "Second Child");

        assert_eq!(NodeIndex::Branch(sg.root_children.unwrap().first), first_child);
        assert_eq!(NodeIndex::Branch(sg.root_children.unwrap().last), third_child);

        assert_eq!(sg.get(first_child).unwrap().last_sibling, None);
        assert_eq!(
            sg.get(first_child).unwrap().next_sibling.map(NodeIndex::Branch),
            Some(third_child)
        );

        assert_eq!(
            sg.get(third_child).unwrap().last_sibling.map(NodeIndex::Branch),
            Some(first_child)
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
        assert_eq!(*third_child_tree.root(), "Third Child");
    }

    #[test]
    fn move_node() {
        let mut sg = SceneGraph::new("Root");
        let fg = sg.attach(NodeIndex::Root, "First Child").unwrap();
        sg.attach(fg, "First Grandchild").unwrap();
        sg.attach(fg, "Second Grandchild").unwrap();
        sg.attach(fg, "Third Grandchild").unwrap();
        let second_child = sg.attach(NodeIndex::Root, "Second Child").unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_direct_children(fg).unwrap().cloned()),
            vec!["First Grandchild", "Second Grandchild", "Third Grandchild",]
        );

        sg.move_node(fg, second_child).unwrap();

        assert_eq!(
            Vec::from_iter(sg.iter_direct_children(NodeIndex::Root).unwrap().cloned()),
            vec!["Second Child",]
        );

        assert_eq!(
            Vec::from_iter(sg.iter_direct_children(fg).unwrap().cloned()),
            vec!["First Grandchild", "Second Grandchild", "Third Grandchild",]
        );

        assert_eq!(
            Vec::from_iter(sg.iter_direct_children(second_child).unwrap().cloned()),
            vec!["First Child",]
        );
    }

    #[test]
    fn clear_works() {
        let input_node: Vec<_> = (0..50_000).map(|v| format!("Node_{}", v)).collect();
        let mut sg = SceneGraph::new("Root");

        for v in input_node.iter() {
            sg.attach_at_root(v);
        }

        sg.clear();

        assert_eq!(sg.len(), 0);
        assert!(sg.is_empty());
        assert!(sg.root_children.is_none());
        assert!(sg.arena.is_empty());
    }
}
