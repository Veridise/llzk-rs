//! Comparison IR operator.

/// Comparison operators between arithmetic expressions.
#[derive(Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
pub enum CmpOp {
    /// Equality
    Eq,
    /// Less than
    Lt,
    /// Less of equal than
    Le,
    /// Greater than
    Gt,
    /// Greater or equal than
    Ge,
    /// Not equal
    Ne,
}

impl std::fmt::Display for CmpOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CmpOp::Eq => "==",
                CmpOp::Lt => "<",
                CmpOp::Le => "<=",
                CmpOp::Gt => ">",
                CmpOp::Ge => ">=",
                CmpOp::Ne => "!=",
            }
        )
    }
}
