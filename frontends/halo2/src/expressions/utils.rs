use crate::halo2::{Expression, Field, Rotation};

struct FDebug<F: Field>(F);

impl<F: Field> std::fmt::Debug for FDebug<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == F::ZERO {
            write!(f, "0")
        } else if self.0 == F::ONE {
            write!(f, "1")
        } else if self.0 == -F::ONE {
            write!(f, "-1")
        } else {
            write!(f, "{:?}", self.0)
        }
    }
}

struct RotationDebug(Rotation);

impl std::fmt::Debug for RotationDebug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 .0 != 0 {
            write!(f, "@{}", self.0 .0)
        } else {
            write!(f, "")
        }
    }
}

pub struct ExprDebug<'a, F>(pub &'a Expression<F>);

impl<'a, F: Field> std::fmt::Debug for ExprDebug<'a, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn fmt_query(
            f: &mut std::fmt::Formatter<'_>,
            typ: &str,
            idx: usize,
            r: Rotation,
        ) -> std::fmt::Result {
            let r = RotationDebug(r);
            write!(f, "{}{}{:?}", typ, idx, r)
        }

        fn fmt_binop<F: Field>(
            f: &mut std::fmt::Formatter<'_>,
            typ: &str,
            lhs: &Box<Expression<F>>,
            rhs: &Box<Expression<F>>,
        ) -> std::fmt::Result {
            write!(
                f,
                "{typ}({:?}, {:?})",
                ExprDebug(lhs.as_ref()),
                ExprDebug(rhs.as_ref())
            )
        }
        match &self.0 {
            Expression::Constant(ff) => write!(f, "{:?}", FDebug(*ff)),
            Expression::Selector(selector) => write!(f, "s{}", selector.index()),
            Expression::Fixed(q) => fmt_query(f, "f", q.column_index(), q.rotation()),

            Expression::Advice(q) => fmt_query(f, "a", q.column_index(), q.rotation()),
            Expression::Instance(q) => fmt_query(f, "i", q.column_index(), q.rotation()),
            Expression::Challenge(challenge) => write!(f, "c{}", challenge.index()),
            Expression::Negated(expression) => {
                write!(f, "Negated({:?})", ExprDebug(expression.as_ref()))
            }
            Expression::Sum(lhs, rhs) => fmt_binop(f, "Sum", lhs, rhs),
            Expression::Product(lhs, rhs) => fmt_binop(f, "Product", lhs, rhs),
            Expression::Scaled(lhs, rhs) => write!(
                f,
                "Scaled({:?}, {:?})",
                ExprDebug(lhs.as_ref()),
                FDebug(*rhs)
            ),
        }
    }
}
