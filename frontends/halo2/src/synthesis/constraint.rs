use crate::halo2::{Any, Column};
use std::{
    collections::{hash_set::Iter, HashSet},
    hash::Hash,
};

type Node = (Column<Any>, usize);

#[derive(Hash)]
pub struct EqConstraint {
    lhs: Node,
    rhs: Node,
}

impl EqConstraint {
    pub fn new(from: Column<Any>, from_row: usize, to: Column<Any>, to_row: usize) -> Self {
        Self {
            lhs: (from, from_row),
            rhs: (to, to_row),
        }
    }
}

impl From<(Column<Any>, usize, Column<Any>, usize)> for EqConstraint {
    fn from(value: (Column<Any>, usize, Column<Any>, usize)) -> Self {
        Self::new(value.0, value.1, value.2, value.3)
    }
}

impl GraphEdge for EqConstraint {
    type Node = Node;

    fn from(&self) -> &Self::Node {
        &self.lhs
    }

    fn to(&self) -> &Self::Node {
        &self.rhs
    }
}

pub trait GraphEdge {
    type Node: Eq + Hash + Copy;

    fn from(&self) -> &Self::Node;
    fn to(&self) -> &Self::Node;
}

pub struct Graph<E: GraphEdge> {
    edges: HashSet<(E::Node, E::Node)>,
}

impl<E: GraphEdge> Graph<E> {
    pub fn add<I>(&mut self, edge: I)
    where
        I: Into<E>,
    {
        let edge = edge.into();
        if !self.contains(&edge) {
            self.edges.insert((*edge.from(), *edge.to()));
        }
    }

    pub fn contains(&self, edge: &E) -> bool {
        self.contains_helper(edge) || self.contains_helper(edge)
    }

    fn contains_helper(&self, edge: &E) -> bool {
        self.edges.contains(&(*edge.from(), *edge.to()))
    }

    pub fn iter(&self) -> Iter<'_, (E::Node, E::Node)> {
        self.edges.iter()
    }
}

impl<'a, E: GraphEdge> IntoIterator for &'a Graph<E> {
    type Item = &'a (E::Node, E::Node);

    type IntoIter = Iter<'a, (E::Node, E::Node)>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<E: GraphEdge + Copy> IntoIterator for Graph<E> {
    type Item = (E::Node, E::Node);

    type IntoIter = <HashSet<(E::Node, E::Node)> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.edges.into_iter()
    }
}

impl<E: GraphEdge> Default for Graph<E> {
    fn default() -> Self {
        Self {
            edges: Default::default(),
        }
    }
}
