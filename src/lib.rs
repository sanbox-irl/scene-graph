mod iter;
mod iter_mut;

pub use iter::SceneGraphIter;
pub use iter_mut::SceneGraphIterMut;

use thunderdome::{Arena, Index};

pub struct SceneGraph<T> {
    arena: Arena<Node<T>>,
    root: Index,
}

impl<T> SceneGraph<T> {
    /// We take a root node here, but we will never actually give this root node back
    /// in any iteration.
    pub fn new(root: T) -> Self {
        let mut arena = Arena::new();
        let root = arena.insert(Node::new(root));

        Self { arena, root }
    }

    /// Iterate mutably over the Scene Graph in a depth first traversal.
    pub fn iter_mut(&mut self) -> SceneGraphIterMut<'_, T> {
        SceneGraphIterMut::new(self)
    }

    /// Iterate immutably over the Scene Graph in a depth first traversal.
    pub fn iter(&self) -> SceneGraphIter<'_, T> {
        SceneGraphIter::new(self)
    }
}

impl<'a, T> IntoIterator for &'a SceneGraph<T> {
    type Item = &'a T;

    type IntoIter = SceneGraphIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut SceneGraph<T> {
    type Item = &'a mut T;

    type IntoIter = SceneGraphIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub struct Node<T> {
    value: T,
    children: Vec<Index>,
}

impl<T> Node<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            children: vec![],
        }
    }
}
