use crate::halo2::{Any, Column, Field, Fixed};
use std::collections::BTreeSet;

/// Possible nodes in the graph.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EqConstraintArg<F: Field> {
    /// Constant finite field element.
    Const(F),
    /// A cell of any type.
    Any(Column<Any>, usize),
    /// A fixed cell.
    Fixed(Column<Fixed>, usize),
}

impl<F: Field> EqConstraintArg<F> {
    /// Creates an storage version adding the finite field value if not found
    fn to_sto(self, storage: &mut Vec<F>) -> EqConstraintArgSto {
        match self {
            EqConstraintArg::Const(f) => EqConstraintArgSto::Const({
                // Find an existing index for the value or create.
                match storage.iter().position(|value| *value == f) {
                    Some(idx) => idx,
                    None => {
                        let idx = storage.len();
                        storage.push(f);
                        idx
                    }
                }
            }),
            EqConstraintArg::Any(column, row) => EqConstraintArgSto::Any(column, row),
            EqConstraintArg::Fixed(column, row) => EqConstraintArgSto::Fixed(column, row),
        }
    }

    /// Creates an storage version, failing if the finite field value was not found
    fn to_sto_ro(self, storage: &Vec<F>) -> Option<EqConstraintArgSto> {
        Some(match self {
            EqConstraintArg::Const(f) => {
                EqConstraintArgSto::Const(storage.iter().position(|value| *value == f)?)
            }
            EqConstraintArg::Any(column, row) => EqConstraintArgSto::Any(column, row),
            EqConstraintArg::Fixed(column, row) => EqConstraintArgSto::Fixed(column, row),
        })
    }
}

/// Possible nodes in the graph, storage form.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum EqConstraintArgSto {
    /// Index to a finite field constant value.
    Const(usize),
    /// A cell of any type.
    Any(Column<Any>, usize),
    /// A fixed cell.
    Fixed(Column<Fixed>, usize),
}

impl EqConstraintArgSto {
    pub fn from_sto<F: Field>(self, storage: &[F]) -> EqConstraintArg<F> {
        match self {
            EqConstraintArgSto::Const(idx) => EqConstraintArg::Const(storage[idx]),
            EqConstraintArgSto::Any(column, row) => EqConstraintArg::Any(column, row),
            EqConstraintArgSto::Fixed(col, row) => EqConstraintArg::Fixed(col, row),
        }
    }
}

/// Represents a copy constraint in between cells or in between a fixed cell and a constant value.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd)]
pub enum EqConstraint<F: Field> {
    /// Edge between two cells of any type.
    AnyToAny(Column<Any>, usize, Column<Any>, usize),
    /// Edge between a fixed cell and a constant value.
    FixedToConst(Column<Fixed>, usize, F),
}

impl<F: Field> EqConstraint<F> {
    pub fn any_to_any(from: Column<Any>, from_row: usize, to: Column<Any>, to_row: usize) -> Self {
        Self::AnyToAny(from, from_row, to, to_row)
    }

    pub fn fixed_to_const(from: Column<Fixed>, row: usize, f: F) -> Self {
        Self::FixedToConst(from, row, f)
    }

    pub fn from(&self) -> EqConstraintArg<F> {
        match *self {
            EqConstraint::AnyToAny(col, row, _, _) => EqConstraintArg::Any(col, row),
            EqConstraint::FixedToConst(col, row, _) => EqConstraintArg::Fixed(col, row),
        }
    }

    pub fn to(&self) -> EqConstraintArg<F> {
        match *self {
            EqConstraint::AnyToAny(_, _, col, row) => EqConstraintArg::Any(col, row),
            EqConstraint::FixedToConst(_, _, f) => EqConstraintArg::Const(f),
        }
    }

    pub fn vertices(&self) -> (EqConstraintArg<F>, EqConstraintArg<F>) {
        (self.from(), self.to())
    }
}

impl<F: Field> From<(EqConstraintArg<F>, EqConstraintArg<F>)> for EqConstraint<F> {
    fn from(value: (EqConstraintArg<F>, EqConstraintArg<F>)) -> Self {
        match value {
            (EqConstraintArg::Const(f), EqConstraintArg::Fixed(col, row)) => {
                Self::FixedToConst(col, row, f)
            }
            (EqConstraintArg::Any(from, from_row), EqConstraintArg::Any(to, to_row)) => {
                Self::AnyToAny(from, from_row, to, to_row)
            }
            (EqConstraintArg::Fixed(col, row), EqConstraintArg::Const(f)) => {
                Self::FixedToConst(col, row, f)
            }
            _ => unreachable!(),
        }
    }
}

/// Graph of equality constraints between cells and finite field values.
pub struct EqConstraintGraph<F> {
    edges: BTreeSet<(EqConstraintArgSto, EqConstraintArgSto)>,
    vertices: BTreeSet<EqConstraintArgSto>,
    /// Finite field elements are stored here because they do not implement the required traits to
    /// be used in BTreeSet.
    ff_storage: Vec<F>,
}

impl<F: Field> EqConstraintGraph<F> {
    /// Adds a new edge to the graph
    pub fn add(&mut self, edge: EqConstraint<F>) {
        let (from, to) = edge.vertices();
        let from = from.to_sto(&mut self.ff_storage);
        let to = to.to_sto(&mut self.ff_storage);
        assert!(from != to, "Self loops are not allowed!");
        if !self.contains(&edge) {
            self.vertices.insert(from);
            self.vertices.insert(to);
            self.edges.insert((from, to));
        }
    }

    /// Returns true if the graph contains the edge.
    pub fn contains(&self, edge: &EqConstraint<F>) -> bool {
        /// Is easier to implement with the ? operator.
        fn inner<F: Field>(
            from: EqConstraintArg<F>,
            to: EqConstraintArg<F>,
            sto: &Vec<F>,
            edges: &BTreeSet<(EqConstraintArgSto, EqConstraintArgSto)>,
        ) -> Option<bool> {
            let from = from.to_sto_ro(sto)?;
            let to = to.to_sto_ro(sto)?;

            Some(edges.contains(&(from, to)) || edges.contains(&(to, from)))
        }

        let (from, to) = edge.vertices();
        inner(from, to, &self.ff_storage, &self.edges).unwrap_or_default()
    }

    /// Returns an iterator of the nodes in the graph.
    pub fn edges(&self) -> Vec<EqConstraint<F>> {
        self.edges
            .iter()
            .copied()
            .map(move |(f, t)| (f.from_sto(&self.ff_storage), t.from_sto(&self.ff_storage)).into())
            .collect()
    }

    pub fn vertices(&self) -> Vec<EqConstraintArg<F>> {
        self.vertices
            .iter()
            .map(|v| v.from_sto(&self.ff_storage))
            .collect()
    }
}

impl<F: Field> Default for EqConstraintGraph<F> {
    fn default() -> Self {
        Self {
            edges: Default::default(),
            vertices: Default::default(),
            ff_storage: Default::default(),
        }
    }
}
