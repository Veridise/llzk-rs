//! Types and traits for working with operation builders.

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

/// Defines the general functionality of a builder.
pub trait OpBuilderLike<'c> {
    /// Returns the raw representation of the builder.
    fn to_raw(&self) -> MlirOpBuilder;

    /// Returns a reference to the context associated with the builder.
    fn context(&self) -> ContextRef<'c> {
        unsafe { ContextRef::from_raw(mlirOpBuilderGetContext(self.to_raw())) }
    }

    /// Sets the insertion point to the start of the given block.
    fn set_insertion_point_at_start<'a, B: BlockLike<'c, 'a>>(&self, block: B) {
        unsafe {
            mlirOpBuilderSetInsertionPointToStart(self.to_raw(), block.to_raw());
        }
    }

    /// Returns a reference to the block where the builder will insert operations.
    fn insertion_block<'a>(&self) -> BlockRef<'c, 'a> {
        unsafe { BlockRef::from_raw(mlirOpBuilderGetInsertionBlock(self.to_raw())) }
    }

    /// Returns a reference to the operation where the builder will insert operations after.
    fn insertion_point<'a>(&self) -> OperationRef<'c, 'a> {
        unsafe { OperationRef::from_raw(mlirOpBuilderGetInsertionPoint(self.to_raw())) }
    }

    /// Inserts the operation produced by the closure and returns a reference to it.
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

/// An owned operation builder.
#[derive(Debug)]
pub struct OpBuilder<'c> {
    raw: MlirOpBuilder,
    _context: PhantomData<&'c Context>,
}

impl<'c> OpBuilder<'c> {
    /// Creates a new operation builder.
    pub fn new(context: &'c Context) -> Self {
        unsafe {
            let ctx = context.to_raw();
            Self {
                raw: mlirOpBuilderCreate(ctx),
                _context: Default::default(),
            }
        }
    }

    /// Creates an operation builder from its raw representation.
    ///
    /// # Safety
    ///
    /// The reference must be valid.
    pub fn from_raw(raw: MlirOpBuilder) -> Self {
        Self {
            raw,
            _context: Default::default(),
        }
    }

    /// Creates a new operation builder with the given block as its insertion point.
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

/// Reference to an operation builder.
#[derive(Debug)]
pub struct OpBuilderRef<'c, 'a> {
    raw: MlirOpBuilder,
    _reference: PhantomData<&'a OpBuilder<'c>>,
}

impl<'c, 'a> OpBuilderRef<'c, 'a> {
    /// Creates an operation builder reference from its raw representation.
    ///
    /// # Safety
    ///
    /// The reference must be valid.
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
