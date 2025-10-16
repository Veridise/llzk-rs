use std::{borrow::Cow, iter};

use llzk::prelude::*;

use melior::{
    ir::{r#type::FunctionType, Location, Operation, Type},
    Context,
};

fn struct_def_op_location<'c>(context: &'c Context, name: &str, index: usize) -> Location<'c> {
    Location::new(context, format!("struct {name}").as_str(), index, 0)
}

fn create_field<'c>(
    context: &'c Context,
    header: &str,
    name: &str,
    public: bool,
) -> Result<FieldDefOp<'c>, LlzkError> {
    let filename = format!("struct {header} | field {name}");
    let loc = Location::new(context, &filename, 0, 0);

    r#struct::field(loc, name, FeltType::new(context), false, public)
}

fn struct_type<'c>(context: &'c Context, name: &str) -> Type<'c> {
    StructType::from_str(context, name).into()
}

struct Field {
    name: Cow<'static, str>,
    public: bool,
}

impl From<(&'static str, bool)> for Field {
    fn from(value: (&'static str, bool)) -> Self {
        Self {
            name: Cow::Borrowed(value.0),
            public: value.1,
        }
    }
}

impl Field {
    pub fn renamed(mut self, f: impl FnOnce(&str) -> String) -> Self {
        let new = f(self.name.as_ref());
        self.name = Cow::Owned(new);
        self
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn is_public(&self) -> bool {
        self.public
    }
}

pub struct StructIO {
    private_inputs: usize,
    public_inputs: usize,
    private_outputs: usize,
    public_outputs: usize,
}

macro_rules! field_iter {
    ($name:ident, $base:expr) => {
        fn $name(&self) -> impl Iterator<Item = Field> {
            iter::repeat($base)
                .map(Field::from)
                .zip(0..self.$name)
                .map(|(f, n)| f.renamed(|name| format!("{name}_{n}")))
        }
    };
}

impl StructIO {
    fn fields(&self) -> impl IntoIterator<Item = Field> {
        self.private_inputs()
            .chain(self.private_outputs())
            .chain(self.public_outputs())
    }

    field_iter!(private_inputs, ("in", false));
    field_iter!(private_outputs, ("out", false));
    field_iter!(public_outputs, ("out", true));

    pub fn public_inputs<'c>(&self, ctx: &'c Context) -> impl IntoIterator<Item = Type<'c>> {
        iter::repeat_with(|| FeltType::new(ctx).into()).take(self.public_inputs)
    }

    pub fn new_from_io(advice: &crate::io::AdviceIO, instance: &crate::io::InstanceIO) -> Self {
        Self {
            private_inputs: advice.inputs().len(),
            public_inputs: instance.inputs().len(),
            private_outputs: advice.outputs().len(),
            public_outputs: instance.outputs().len(),
        }
    }

    pub fn new_from_io_count(inputs: usize, outputs: usize) -> Self {
        Self {
            private_inputs: 0,
            public_inputs: inputs,
            private_outputs: outputs,
            public_outputs: 0,
        }
    }
}

pub fn create_struct<'c>(
    context: &'c Context,
    struct_name: &str,
    idx: usize,
    io: StructIO,
) -> Result<StructDefOp<'c>, LlzkError> {
    log::debug!("context = {context:?}");
    let loc = struct_def_op_location(context, struct_name, idx);
    log::debug!("Struct location: {loc:?}");
    let fields = io
        .fields()
        .into_iter()
        .map(|field| -> Result<Operation<'c>, LlzkError> {
            create_field(context, struct_name, field.name(), field.is_public()).map(Into::into)
        });

    let func_args = [struct_type(context, struct_name)]
        .into_iter()
        .chain(io.public_inputs(context))
        .collect::<Vec<_>>();

    log::debug!("Creating function with arguments: {func_args:?}");
    let constrain = function::def(
        loc,
        "constrain",
        FunctionType::new(context, &func_args, &[]),
        &[],
        None,
    )
    .inspect(|f| f.set_allow_constraint_attr(true));

    log::debug!("Creating constraint op");
    r#struct::def(
        loc,
        struct_name,
        &[],
        fields.chain([constrain.map(Into::into)]),
    )
}
