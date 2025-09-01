use std::{any::type_name, borrow::Borrow, marker::PhantomData, ops::Deref};

use crate::halo2::{Advice, Any, ColumnType, Fixed, Instance};

/// Denotes the intended role of a particular cell in the circuit.
/// When defining the following rules apply:
///  - Any instance cell marked with either Input or Output is
///    marked as a public input or output. The type system disallows marking an instance cell with
///    the intermediate role. An unmarked instance cell is defaulted to Input.
///  - Any advice cell marked as Input or Output is marked as a
///    private input or output. Any unmarked cell is defaulted to
///    Intermediate.
///  - Fixed cells cannot be marked with a role and the type system wont allow creating a role
///  for them.
#[derive(Debug, Copy, Clone)]
pub struct CellRole<C: ColumnType> {
    role: Roles,
    _marker: PhantomData<C>,
}

impl<C: ColumnType> CellRole<C> {
    fn new(role: Roles) -> Self {
        Self {
            role,
            _marker: Default::default(),
        }
    }

    pub fn into_cells(self, r: impl IntoIterator<Item = usize>) -> Vec<(usize, Self)> {
        r.into_iter().map(|n| (n, self)).collect()
    }
}

impl<C: ColumnType> PartialEq<Roles> for CellRole<C> {
    fn eq(&self, other: &Roles) -> bool {
        self.role == *other
    }
}

impl<C: ColumnType> Deref for CellRole<C> {
    type Target = Roles;

    fn deref(&self) -> &Self::Target {
        &self.role
    }
}

impl<C: ColumnType> AsRef<Roles> for CellRole<C> {
    fn as_ref(&self) -> &Roles {
        &self.role
    }
}

impl<C: ColumnType> Borrow<Roles> for CellRole<C> {
    fn borrow(&self) -> &Roles {
        &self.role
    }
}

impl<C: SupportsInput> CellRole<C> {
    /// Creates an input role.
    pub fn input() -> Self {
        Self::new(Roles::Input)
    }

    /// Creates a list of cells with the input role.
    pub fn inputs(r: impl IntoIterator<Item = usize>) -> Vec<(usize, Self)> {
        Self::input().into_cells(r)
    }
}

impl<C: SupportsOutput> CellRole<C> {
    /// Creates an output role.
    pub fn output() -> Self {
        Self::new(Roles::Output)
    }

    /// Creates a list of cells with the output role.
    pub fn outputs(r: impl IntoIterator<Item = usize>) -> Vec<(usize, Self)> {
        Self::output().into_cells(r)
    }
}

impl<C: SupportsIntermediate> CellRole<C> {
    /// Creates an intermediate role.
    pub fn intermediate() -> Self {
        Self::new(Roles::Intermediate)
    }

    /// Creates a list of cells with the intermediate role.
    pub fn intermediates(r: impl IntoIterator<Item = usize>) -> Vec<(usize, Self)> {
        Self::intermediate().into_cells(r)
    }
}

impl Default for CellRole<Advice> {
    /// Creates an intermediate role, the default for advice cells.
    fn default() -> Self {
        Self::intermediate()
    }
}

impl Default for CellRole<Instance> {
    /// Creates an input role, the default for instance cells.
    fn default() -> Self {
        Self::input()
    }
}

impl TryFrom<CellRole<Any>> for CellRole<Instance> {
    type Error = RoleError<Instance>;

    fn try_from(value: CellRole<Any>) -> Result<Self, Self::Error> {
        match value.role {
            Roles::Input => Ok(Self::input()),
            Roles::Output => Ok(Self::output()),
            _ => Err(Self::Error::unsupported(value.role)),
        }
    }
}

impl TryFrom<CellRole<Any>> for CellRole<Advice> {
    type Error = RoleError<Advice>;

    fn try_from(value: CellRole<Any>) -> Result<Self, Self::Error> {
        Ok(Self::new(value.role))
    }
}

/// The possible roles for a cell in the circuit.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Roles {
    Input,
    Output,
    Intermediate,
}

pub trait SupportsInput: ColumnType {}
pub trait SupportsOutput: ColumnType {}
pub trait SupportsIntermediate: ColumnType {}

macro_rules! supports {
    ($tr:ident, $($types:ty),+ $(,)?) => {
        $(
            impl $tr for $types {}
        )*
    };
}

supports!(SupportsInput, Instance, Advice, Any);
supports!(SupportsOutput, Instance, Advice, Any);
supports!(SupportsIntermediate, Advice, Any);

#[derive(Debug)]
struct UnsupportedRoleError<C: std::fmt::Debug> {
    role: Roles,
    _marker: PhantomData<C>,
}

/// Error related to cell roles.
#[derive(Debug)]
pub enum RoleError<C: std::fmt::Debug> {
    Unsupported(UnsupportedRoleError<C>),
}

impl<C: std::fmt::Debug> RoleError<C> {
    pub fn unsupported(role: Roles) -> Self {
        Self::Unsupported(UnsupportedRoleError {
            role,
            _marker: PhantomData,
        })
    }
}

impl<C: std::fmt::Debug> std::error::Error for RoleError<C> {}

impl<C: std::fmt::Debug> std::fmt::Display for RoleError<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleError::Unsupported(unsupported_role_error) => write!(
                f,
                "Role {:?} is not supported for column type {}",
                unsupported_role_error.role,
                type_name::<C>()
            ),
        }
    }
}
