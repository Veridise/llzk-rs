use std::{
    any::Any,
    fmt,
    iter::{Product, Sum},
    marker::PhantomData,
    ops::{Add, AddAssign, Deref, DerefMut, Mul, MulAssign, Neg, Sub, SubAssign},
    rc::Rc,
    sync::{Mutex, MutexGuard},
};

use subtle::{Choice, ConditionallySelectable, ConstantTimeEq, CtOption};

use crate::{
    arena::{BumpArena, Index},
    halo2::{Field, PrimeField, Value},
};

pub mod lift;

/// IR for operations that occur in the main circuit.
pub enum CircuitStmt<T> {
    ConstraintCall(String, Vec<Value<T>>, Vec<Value<T>>),
    EqConstraint(Value<T>, Value<T>),
}

pub type CircuitStmts<T> = Vec<CircuitStmt<T>>;
