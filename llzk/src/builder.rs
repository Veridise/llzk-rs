use std::marker::PhantomData;

use llzk_sys::{
    MlirOpBuilder, mlirOpBuilderCreate, mlirOpBuilderDestroy, mlirOpBuilderGetContext,
    mlirOpBuilderGetInsertionBlock, mlirOpBuilderGetInsertionPoint,
    mlirOpBuilderSetInsertionPointToStart,
};
use melior::{
    Context, ContextRef,
    ir::{BlockLike, BlockRef, Location, Operation, OperationRef},
};

pub trait OpBuilderLike<'c> {
    fn to_raw(&self) -> MlirOpBuilder;

    fn context(&self) -> ContextRef<'c> {
        unsafe { ContextRef::from_raw(mlirOpBuilderGetContext(self.to_raw())) }
    }

    fn set_insertion_point_at_start<'a, B: BlockLike<'c, 'a>>(&self, block: B) {
        unsafe {
            mlirOpBuilderSetInsertionPointToStart(self.to_raw(), block.to_raw());
        }
    }

    fn insertion_block<'a>(&self) -> BlockRef<'c, 'a> {
        unsafe { BlockRef::from_raw(mlirOpBuilderGetInsertionBlock(self.to_raw())) }
    }

    fn insertion_point<'a>(&self) -> OperationRef<'c, 'a> {
        unsafe { OperationRef::from_raw(mlirOpBuilderGetInsertionPoint(self.to_raw())) }
    }

    fn insert<'a, F: FnOnce(ContextRef<'c>, Location<'c>) -> Operation<'c>>(
        &'c self,
        loc: Location<'c>,
        f: F,
    ) -> OperationRef<'c, 'a> {
        let op = f(self.context(), loc);
        self.insertion_block()
            .insert_operation_after(self.insertion_point(), op)
    }
}

#[derive(Debug)]
pub struct OpBuilder<'c> {
    raw: MlirOpBuilder,
    _context: PhantomData<&'c Context>,
}

impl<'c> OpBuilder<'c> {
    pub fn new(context: &'c Context) -> Self {
        unsafe {
            let ctx = context.to_raw();
            Self {
                raw: mlirOpBuilderCreate(ctx),
                _context: Default::default(),
            }
        }
    }

    pub fn from_raw(raw: MlirOpBuilder) -> Self {
        Self {
            raw,
            _context: Default::default(),
        }
    }

    pub fn at_block_begin<'a, B: BlockLike<'c, 'a>>(ctx: &'c Context, block: B) -> Self {
        let b = Self::new(ctx);
        b.set_insertion_point_at_start(block);
        b
    }
}

impl<'c> OpBuilderLike<'c> for OpBuilder<'c> {
    fn to_raw(&self) -> MlirOpBuilder {
        self.raw
    }
}

impl Drop for OpBuilder<'_> {
    fn drop(&mut self) {
        unsafe { mlirOpBuilderDestroy(self.raw) }
    }
}

#[derive(Debug)]
pub struct OpBuilderRef<'c, 'a> {
    raw: MlirOpBuilder,
    _reference: PhantomData<&'a OpBuilder<'c>>,
}

impl<'c, 'a> OpBuilderRef<'c, 'a> {
    pub fn from_raw(raw: MlirOpBuilder) -> Self {
        Self {
            raw,
            _reference: Default::default(),
        }
    }
}

impl<'c> OpBuilderLike<'c> for OpBuilderRef<'c, '_> {
    fn to_raw(&self) -> MlirOpBuilder {
        self.raw
    }
}
