use std::{cell::RefCell, ops::Deref};

use melior::{
    ir::{
        attribute::IntegerAttribute, Attribute, BlockLike, BlockRef, Location, Operation,
        OperationRef, Type,
    },
    Context, ContextRef,
};

pub struct Builder<'c> {
    context: ContextRef<'c>,
}

impl<'c> Builder<'c> {
    pub fn new(context: ContextRef<'c>) -> Self {
        Self { context }
    }

    pub fn from_ref(context: &'c Context) -> Self {
        Self::new(unsafe { ContextRef::from_raw(context.to_raw()) })
    }

    pub fn context(&self) -> ContextRef<'c> {
        self.context
    }

    pub fn context_ref(&self) -> &'c Context {
        unsafe { self.context.to_ref() }
    }

    pub fn unknown_loc(&self) -> Location {
        Location::unknown(self.context_ref())
    }

    pub fn index_type(&self) -> Type {
        return Type::index(self.context_ref());
    }

    pub fn index_attr(&self, val: i64) -> Attribute {
        IntegerAttribute::new(self.index_type(), val).into()
    }
}

pub struct OpBuilder<'c, 'a> {
    inner: Builder<'c>,
    block: BlockRef<'c, 'a>,
    insertion_point: RefCell<Option<OperationRef<'c, 'a>>>,
}

impl<'c, 'a> OpBuilder<'c, 'a> {
    pub fn at_block_begin<B: BlockLike<'c, 'a>>(ctx: ContextRef<'c>, block: B) -> Self {
        Self {
            inner: Builder::new(ctx),
            block: unsafe { BlockRef::from_raw(block.to_raw()) },
            insertion_point: block.first_operation().into(),
        }
    }

    pub fn insert<F: FnOnce(&'c Self, Location<'c>) -> Operation<'c>>(
        &'c self,
        loc: Location<'c>,
        f: F,
    ) -> OperationRef<'c, 'a> {
        let op = f(self, loc);

        let mut point = self.insertion_point.borrow_mut();
        *point = Some(if let Some(p) = *point {
            self.block.insert_operation_after(p, op)
        } else {
            self.block.append_operation(op)
        });

        return point.unwrap();
    }
}

impl<'c, 'a> Deref for OpBuilder<'c, 'a> {
    type Target = Builder<'c>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
