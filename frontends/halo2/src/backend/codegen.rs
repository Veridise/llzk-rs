use super::lowering::lowerable::LowerableStmt;
use super::lowering::ExprLowering as _;
use super::{func::FuncIO, lowering::Lowering};
use crate::io::AllCircuitIO;
use crate::ir::expr::IRAexpr;
use crate::ir::{IRCircuit, IRCtx};
use crate::{expressions::ScopedExpression, halo2::Field, ir::stmt::IRStmt};
use anyhow::Result;

pub mod lookup;
//pub mod queue;
pub mod strats;

pub trait Codegen<'c: 's, 's>: Sized + 's {
    type FuncOutput: Lowering;
    type Output;
    //type F: Field + Clone;
    type State: 'c;

    fn initialize(state: &'s Self::State) -> Self;

    fn define_function(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
    ) -> Result<Self::FuncOutput>;

    fn define_function_with_body<FN, L, I>(
        &self,
        name: &str,
        inputs: usize,
        outputs: usize,
        f: FN,
    ) -> Result<()>
    where
        FN: FnOnce(&Self::FuncOutput, &[FuncIO], &[FuncIO]) -> Result<I>,
        I: IntoIterator<Item = L>,
        L: LowerableStmt,
    {
        let func = self.define_function(name, inputs, outputs)?;
        let inputs = func.lower_function_inputs(0..inputs);
        let outputs = func.lower_function_outputs(0..outputs);
        let stmts = f(&func, &inputs, &outputs)?;
        for stmt in stmts {
            stmt.lower(&func)?;
        }
        self.on_scope_end(func)
    }

    fn define_main_function(
        &self, /*, syn: &CircuitSynthesis<Self::F>*/
        io: AllCircuitIO,
    ) -> Result<Self::FuncOutput>;

    fn define_main_function_with_body<L>(
        &self,
        io: AllCircuitIO,
        stmts: impl IntoIterator<Item = L>,
    ) -> Result<()>
    where
        L: LowerableStmt,
    {
        let main = self.define_main_function(io)?;
        log::debug!("Defined main function");
        for stmt in stmts {
            stmt.lower(&main)?;
        }
        log::debug!("Lowered function body");
        self.on_scope_end(main)
    }

    fn on_scope_end(&self, _: Self::FuncOutput) -> Result<()> {
        Ok(())
    }

    fn generate_output(self) -> Result<Self::Output>
    where
        Self::Output: 'c;
}

//pub trait CodegenQueue<'c: 's, 's>: Codegen<'c, 's> {
//    fn event_receiver(
//        state: &'s Self::State,
//    ) -> impl EventReceiver<Message = BackendMessages<Self::F>> + Clone {
//        CodegenEventReceiver::new(Self::initialize(state))
//    }
//
//    fn enqueue_stmts(
//        &self,
//        region: crate::halo2::RegionIndex,
//        stmts: Vec<IRStmt<Expression<Self::F>>>,
//    ) -> Result<()>;
//}

pub trait CodegenStrategy: Default {
    fn codegen<'c: 'st, 's, 'st, C>(
        &self,
        codegen: &C,
        ctx: &IRCtx,
        ir: &IRCircuit<IRAexpr>,
        //lookups: &dyn LookupCallbacks<C::F>,
        //gate_cbs: &dyn GateCallbacks<C::F>,
        //injector: &mut dyn crate::IRInjectCallback<C::F>,
    ) -> Result<()>
    where
        C: Codegen<'c, 'st>;
    //Row<'s, 's, C::F>: ResolversProvider<C::F> + 's,
    //RegionRow<'s, 's, 's, C::F>: ResolversProvider<C::F> + 's;
}

//pub struct CodegenEventReceiver<'c: 's, 's, C> {
//    codegen: Rc<C>,
//    _marker: PhantomData<(&'s (), &'c ())>,
//}

//impl<C> Clone for CodegenEventReceiver<'_, '_, C> {
//    fn clone(&self) -> Self {
//        Self {
//            codegen: self.codegen.clone(),
//            _marker: Default::default(),
//        }
//    }
//}
//
//impl<C> CodegenEventReceiver<'_, '_, C> {
//    pub fn new(codegen: C) -> Self {
//        Self {
//            codegen: Rc::new(codegen),
//            _marker: Default::default(),
//        }
//    }
//}

//impl<'c: 's, 's, C> EventReceiver for CodegenEventReceiver<'c, 's, C>
//where
//    C: CodegenQueue<'c, 's>,
//{
//    type Message = BackendMessages<C::F>;
//
//    fn accept(&self, msg: Self::Message) -> Result<<Self::Message as Message>::Response> {
//        match msg {
//            BackendMessages::EmitStmts(msg) => self
//                .codegen
//                .enqueue_stmts(msg.0, msg.1)
//                .map(BackendResponse::EmitStmts),
//        }
//    }
//}

//pub fn lower_injected_ir<'s, F: Field>(
//    ir: IRStmt<(usize, Expression<F>)>,
//    region: crate::synthesis::regions::RegionData<'s>,
//    advice_io: &'s CircuitIO<Advice>,
//    instance_io: &'s CircuitIO<Instance>,
//    fqr: &'s dyn FixedQueryResolver<F>,
//) -> Result<IRStmt<IRAexpr<F>>> {
//    ir.map(&|(row, expr)| {
//        ScopedExpression::new(
//            expr,
//            RegionRow::new(region, row, advice_io, instance_io, fqr),
//        )
//    })
//    .try_map(&IRAexpr::try_from)
//}

/// If the given statement is not empty prepends a comment
/// with contextual information.
#[inline]
fn prepend_comment<'a, F: Field>(
    stmt: IRStmt<ScopedExpression<'a, 'a, F>>,
    comment: impl FnOnce() -> IRStmt<ScopedExpression<'a, 'a, F>>,
) -> IRStmt<ScopedExpression<'a, 'a, F>> {
    if stmt.is_empty() {
        return stmt;
    }
    [comment(), stmt].into_iter().collect()
}
//
///// Converts scoped expressions into concrete arith expressions, disconecting the statements from
///// the lifetime of the scope.
//#[inline]
//fn scoped_exprs_to_aexpr<'a, F: Field>(
//    stmts: Vec<IRStmt<ScopedExpression<'a, 'a, F>>>,
//) -> Result<IRStmt<IRAexpr<F>>> {
//    stmts
//        .into_iter()
//        .map(|stmt| stmt.try_map(&IRAexpr::try_from))
//        .collect()
//}
