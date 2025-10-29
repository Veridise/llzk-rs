//! IR printer utility

use std::fmt::{Display, Formatter, Result as FmtResult, Write};

use crate::{
    backend::func::FuncIO,
    ir::{
        ResolvedIRCircuit,
        expr::{IRAexpr, IRBexpr},
        groups::GroupBody,
        stmt::IRStmt,
    },
};

#[derive(Debug, Copy, Clone)]
enum IRPrinterCapture<'a> {
    Circuit(&'a ResolvedIRCircuit),
    Group(&'a GroupBody<IRAexpr>),
    Stmt(&'a IRStmt<IRAexpr>),
    Bexpr(&'a IRBexpr<IRAexpr>),
    Aexpr(&'a IRAexpr),
}

/// Prints a human-friendly representation of the IR meant for debugging.
///
/// The structure of the output emitted by the printer is never considered stable and shouldn't be
/// relied upon as it may change unexpectedly. The purpose of the printer is to be a debugging aid
/// for inspecting the shape of the IR and not a serialization/deserialization format.
#[derive(Debug, Copy, Clone)]
pub struct IRPrinter<'a>(IRPrinterCapture<'a>);

impl<'a> IRPrinter<'a> {
    /// Creates a printer from a [`ResolvedIRCircuit`].
    pub fn from_circuit(c: &'a ResolvedIRCircuit) -> Self {
        Self(IRPrinterCapture::Circuit(c))
    }

    /// Creates a printer from a [`GroupBody`].
    pub fn from_group(c: &'a GroupBody<IRAexpr>) -> Self {
        Self(IRPrinterCapture::Group(c))
    }

    /// Creates a printer from a [`IRStmt`].
    pub fn from_stmt(c: &'a IRStmt<IRAexpr>) -> Self {
        Self(IRPrinterCapture::Stmt(c))
    }

    /// Creates a printer from a [`IRBexpr`].
    pub fn from_bexpr(c: &'a IRBexpr<IRAexpr>) -> Self {
        Self(IRPrinterCapture::Bexpr(c))
    }

    /// Creates a printer from a [`IRAexpr`].
    pub fn from_aexpr(c: &'a IRAexpr) -> Self {
        Self(IRPrinterCapture::Aexpr(c))
    }

    fn fmt_circuit(&self, circuit: &ResolvedIRCircuit, ctx: &mut IRPrinterCtx) -> FmtResult {
        self.list_nl("prime-number", ctx, |ctx| {
            write!(ctx, "{}", circuit.prime())
        })?;
        for group in circuit.groups() {
            ctx.nl()?;
            self.fmt_group(group, ctx)?;
        }
        Ok(())
    }

    fn fmt_group(&self, group: &GroupBody<IRAexpr>, ctx: &mut IRPrinterCtx) -> FmtResult {
        self.block("group", ctx, |ctx| {
            writeln!(
                ctx,
                "{} \"{}\" (inputs {}) (outputs {})",
                group.id(),
                group.name(),
                group.input_count(),
                group.output_count()
            )?;

            for callsite in group.callsites() {
                self.fmt_call(
                    callsite.name(),
                    callsite.inputs(),
                    callsite.output_vars(),
                    Some(callsite.callee_id()),
                    ctx,
                )?;
                ctx.nl()?;
            }

            for stmt in group.statements() {
                self.fmt_stmt(stmt, ctx)?;
                ctx.nl()?;
            }

            Ok(())
        })
    }

    fn fmt_call(
        &self,
        callee: &str,
        inputs: &[IRAexpr],
        outputs: &[FuncIO],
        id: Option<usize>,
        ctx: &mut IRPrinterCtx,
    ) -> FmtResult {
        self.block("call", ctx, |ctx| {
            if let Some(id) = id {
                write!(ctx, "{id} ")?;
            }
            writeln!(ctx, "\"{}\" ", callee)?;
            self.block("inputs", ctx, |ctx| {
                let do_nl = inputs.iter().any(|expr| self.aexpr_depth(expr) > 1);
                let mut is_first = true;
                for expr in inputs {
                    if do_nl && !is_first {
                        ctx.nl()?;
                    }
                    is_first = false;
                    self.fmt_aexpr(expr, ctx)?;
                }
                Ok(())
            })?;
            ctx.nl()?;
            self.list("outputs", ctx, |ctx| {
                for output in outputs {
                    self.fmt_func_io(output, ctx)?;
                }
                Ok(())
            })
        })
    }

    fn fmt_func_io(&self, func_io: &FuncIO, ctx: &mut IRPrinterCtx) -> FmtResult {
        match func_io {
            FuncIO::Arg(arg_no) => write!(ctx, "(input {arg_no})"),
            FuncIO::Field(field_id) => write!(ctx, "(output {field_id})"),
            FuncIO::Advice(cell_ref) => {
                write!(ctx, "(advice {cell_ref})")
            }
            FuncIO::Fixed(cell_ref) => write!(ctx, "(fixed {cell_ref})"),
            FuncIO::TableLookup(id, col, row, idx, region) => {
                write!(ctx, "(lookup {id} {col} {row} {idx} {region})")
            }
            FuncIO::CallOutput(call, idx) => write!(ctx, "(call-result {call} {idx})"),
            FuncIO::Temp(temp) => write!(ctx, "(temp {})", **temp),
        }
    }

    fn fmt_stmt(&self, stmt: &IRStmt<IRAexpr>, ctx: &mut IRPrinterCtx) -> FmtResult {
        match stmt {
            IRStmt::ConstraintCall(call) => {
                self.fmt_call(call.callee(), call.inputs(), call.outputs(), None, ctx)
            }
            IRStmt::Constraint(constraint) => {
                self.block(format!("assert/{}", constraint.op()).as_str(), ctx, |ctx| {
                    if self.aexpr_depth(constraint.lhs()) > 1 {
                        ctx.nl()?;
                    }
                    self.fmt_aexpr(constraint.lhs(), ctx)?;
                    if self.aexpr_depth(constraint.lhs()) > 1
                        || self.aexpr_depth(constraint.rhs()) > 1
                    {
                        ctx.nl()?;
                    }
                    self.fmt_aexpr(constraint.rhs(), ctx)
                })
            }
            IRStmt::Comment(comment) => {
                ctx.nl()?;
                writeln!(ctx, "; {}", comment.value())
            }
            IRStmt::AssumeDeterministic(assume_deterministic) => {
                self.list_nl("assume-deterministic", ctx, |ctx| {
                    self.fmt_func_io(&assume_deterministic.value(), ctx)
                })
            }
            IRStmt::Assert(assert) => {
                self.block("assert", ctx, |ctx| self.fmt_bexpr(assert.cond(), ctx))
            }
            IRStmt::Seq(seq) => {
                for stmt in seq.iter() {
                    self.fmt_stmt(stmt, ctx)?;
                }
                Ok(())
            }
            IRStmt::PostCond(post_cond) => self.block("post-cond", ctx, |ctx| {
                self.fmt_bexpr(post_cond.cond(), ctx)
            }),
        }
    }

    fn fmt_bexpr(&self, bexpr: &IRBexpr<IRAexpr>, ctx: &mut IRPrinterCtx) -> FmtResult {
        match bexpr {
            IRBexpr::True => write!(ctx, "(true)"),
            IRBexpr::False => write!(ctx, "(false)"),
            IRBexpr::Cmp(cmp_op, lhs, rhs) => {
                self.block(format!("{cmp_op}").as_str(), ctx, |ctx| {
                    if self.aexpr_depth(lhs) > 1 {
                        ctx.nl()?;
                    }
                    self.fmt_aexpr(lhs, ctx)?;
                    if self.aexpr_depth(&lhs) > 1 || self.aexpr_depth(&rhs) > 1 {
                        ctx.nl()?;
                    }
                    self.fmt_aexpr(rhs, ctx)
                })
            }
            IRBexpr::And(exprs) => self.block("&&", ctx, |ctx| {
                let do_nl = exprs.iter().any(|expr| self.bexpr_depth(expr) > 1);
                let mut is_first = true;
                for expr in exprs {
                    if do_nl && !is_first {
                        ctx.nl()?;
                    }
                    is_first = false;
                    self.fmt_bexpr(expr, ctx)?;
                }
                Ok(())
            }),
            IRBexpr::Or(exprs) => self.block("||", ctx, |ctx| {
                let do_nl = exprs.iter().any(|expr| self.bexpr_depth(expr) > 1);
                let mut is_first = true;
                for expr in exprs {
                    if do_nl && !is_first {
                        ctx.nl()?;
                    }
                    is_first = false;
                    self.fmt_bexpr(expr, ctx)?;
                }
                Ok(())
            }),
            IRBexpr::Not(expr) => self.block("!", ctx, |ctx| self.fmt_bexpr(&expr, ctx)),
            IRBexpr::Det(expr) => self.block("det", ctx, |ctx| self.fmt_aexpr(expr, ctx)),
            IRBexpr::Implies(lhs, rhs) => self.block("=>", ctx, |ctx| {
                if self.bexpr_depth(&lhs) > 1 {
                    ctx.nl()?;
                }
                self.fmt_bexpr(lhs, ctx)?;
                if self.bexpr_depth(&lhs) > 1 || self.bexpr_depth(&rhs) > 1 {
                    ctx.nl()?;
                }
                self.fmt_bexpr(rhs, ctx)
            }),
            IRBexpr::Iff(lhs, rhs) => self.block("<=>", ctx, |ctx| {
                if self.bexpr_depth(&lhs) > 1 {
                    ctx.nl()?;
                }
                self.fmt_bexpr(lhs, ctx)?;
                if self.bexpr_depth(&lhs) > 1 || self.bexpr_depth(&rhs) > 1 {
                    ctx.nl()?;
                }
                self.fmt_bexpr(rhs, ctx)
            }),
        }
    }

    fn fmt_aexpr(&self, aexpr: &IRAexpr, ctx: &mut IRPrinterCtx) -> FmtResult {
        match aexpr {
            IRAexpr::Constant(felt) => self.list("const", ctx, |ctx| write!(ctx, "{}", felt)),
            IRAexpr::IO(func_io) => self.fmt_func_io(func_io, ctx),
            IRAexpr::Challenge(challenge) => {
                self.list("challenge", ctx, |ctx| write!(ctx, "{}", challenge.index()))
            }
            IRAexpr::Negated(expr) => self.block("-", ctx, |ctx| self.fmt_aexpr(&expr, ctx)),
            IRAexpr::Sum(lhs, rhs) => self.block("+", ctx, |ctx| {
                let do_nl = self.aexpr_depth(&lhs) > 1 || self.aexpr_depth(&rhs) > 1;
                if self.aexpr_depth(lhs) > 1 {
                    ctx.nl()?;
                }
                self.fmt_aexpr(&lhs, ctx)?;
                if do_nl {
                    ctx.nl()?;
                } else {
                    write!(ctx, " ")?;
                }
                self.fmt_aexpr(&rhs, ctx)
            }),
            IRAexpr::Product(lhs, rhs) => self.block("*", ctx, |ctx| {
                let do_nl = self.aexpr_depth(&lhs) > 1 || self.aexpr_depth(&rhs) > 1;
                if self.aexpr_depth(lhs) > 1 {
                    ctx.nl()?;
                }
                self.fmt_aexpr(&lhs, ctx)?;
                if do_nl {
                    ctx.nl()?;
                } else {
                    write!(ctx, " ")?;
                }
                self.fmt_aexpr(&rhs, ctx)
            }),
        }
    }

    /// Returns the depth of the boolean expression.
    ///
    /// The depth is used for the heuristic used for deciding when to indentate or not.
    fn bexpr_depth(&self, bexpr: &IRBexpr<IRAexpr>) -> usize {
        match bexpr {
            IRBexpr::True | IRBexpr::False => 1,
            IRBexpr::Cmp(_, lhs, rhs) => {
                1 + std::cmp::max(self.aexpr_depth(lhs), self.aexpr_depth(rhs))
            }
            IRBexpr::And(exprs) | IRBexpr::Or(exprs) => {
                1 + exprs
                    .iter()
                    .map(|expr| self.bexpr_depth(expr))
                    .max()
                    .unwrap_or_default()
            }
            IRBexpr::Not(expr) => 1 + self.bexpr_depth(&expr),
            IRBexpr::Det(expr) => 1 + self.aexpr_depth(expr),
            IRBexpr::Implies(lhs, rhs) | IRBexpr::Iff(lhs, rhs) => {
                1 + std::cmp::max(self.bexpr_depth(&lhs), self.bexpr_depth(&rhs))
            }
        }
    }

    /// Returns the depth of the arithmetic expression.
    ///
    /// The depth is used for the heuristic used for deciding when to indentate or not.
    fn aexpr_depth(&self, aexpr: &IRAexpr) -> usize {
        match aexpr {
            IRAexpr::Constant(_) | IRAexpr::IO(_) | IRAexpr::Challenge(_) => 1,
            IRAexpr::Negated(expr) => 1 + self.aexpr_depth(&expr),
            IRAexpr::Sum(lhs, rhs) | IRAexpr::Product(lhs, rhs) => {
                1 + std::cmp::max(self.aexpr_depth(&lhs), self.aexpr_depth(&rhs))
            }
        }
    }

    fn block(
        &self,
        atom: &str,
        ctx: &mut IRPrinterCtx,
        body: impl FnOnce(&mut IRPrinterCtx) -> FmtResult,
    ) -> FmtResult {
        self.list(atom, ctx, |ctx| {
            ctx.push_indent(2 + atom.len());
            body(ctx)?;
            ctx.pop_indent();
            Ok(())
        })
    }

    fn list_nl(
        &self,
        atom: &str,
        ctx: &mut IRPrinterCtx,
        body: impl FnOnce(&mut IRPrinterCtx) -> FmtResult,
    ) -> FmtResult {
        self.list(atom, ctx, body)?;
        ctx.nl()
    }

    fn list(
        &self,
        atom: &str,
        ctx: &mut IRPrinterCtx,
        body: impl FnOnce(&mut IRPrinterCtx) -> FmtResult,
    ) -> FmtResult {
        write!(ctx, "(")?;
        if !atom.is_empty() {
            write!(ctx, "{atom} ")?;
        }
        body(ctx)?;
        write!(ctx, ")")
    }
}

impl Display for IRPrinter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut ctx = IRPrinterCtx::new(f);
        match self.0 {
            IRPrinterCapture::Circuit(circuit) => self.fmt_circuit(circuit, &mut ctx),
            IRPrinterCapture::Group(group) => self.fmt_group(group, &mut ctx),
            IRPrinterCapture::Stmt(stmt) => self.fmt_stmt(stmt, &mut ctx),
            IRPrinterCapture::Bexpr(bexpr) => self.fmt_bexpr(bexpr, &mut ctx),
            IRPrinterCapture::Aexpr(aexpr) => self.fmt_aexpr(aexpr, &mut ctx),
        }
    }
}

struct IRPrinterCtx<'a, 'f> {
    f: &'a mut Formatter<'f>,
    indent: Vec<usize>,
    indent_pending: bool,
}

impl<'a, 'f> IRPrinterCtx<'a, 'f> {
    fn new(f: &'a mut Formatter<'f>) -> Self {
        Self {
            f,
            indent: vec![],
            indent_pending: true,
        }
    }

    fn nl(&mut self) -> FmtResult {
        if !self.indent_pending {
            self.indent_pending = true;
            writeln!(self.f, "")?;
        }
        Ok(())
    }

    fn push_indent(&mut self, value: usize) {
        self.indent.push(value);
    }

    fn pop_indent(&mut self) {
        self.indent.pop();
    }

    fn do_indent(&mut self) -> FmtResult {
        if !self.indent_pending {
            return Ok(());
        }
        for indent in &self.indent {
            write!(self.f, "{}", " ".repeat(*indent))?;
        }
        self.indent_pending = false;
        Ok(())
    }
}

impl Write for IRPrinterCtx<'_, '_> {
    fn write_str(&mut self, s: &str) -> FmtResult {
        let ends_with_nl = s.ends_with('\n');
        let mut lines = s.lines().peekable();
        loop {
            let Some(line) = lines.next() else {
                self.indent_pending = ends_with_nl;
                return Ok(());
            };
            let not_done = lines.peek().is_some();
            self.do_indent()?;

            write!(self.f, "{}", line)?;
            if not_done || ends_with_nl {
                writeln!(self.f)?;
            }
        }
    }
}
