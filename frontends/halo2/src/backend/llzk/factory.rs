use std::{borrow::Cow, iter};

use llzk::prelude::*;

use melior::{
    ir::{
        Attribute, Identifier, Location, Operation, Type,
    },
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

impl StructIO {
    fn fields(&self) -> impl IntoIterator<Item = Field> {
        self.public_outputs().chain(self.private_outputs())
    }

    fn private_outputs(&self) -> impl Iterator<Item = Field> {
        iter::repeat(("out", false))
            .map(Field::from)
            .zip((0..self.private_outputs).map(|n| n + self.public_outputs))
            .map(|(f, n)| f.renamed(|name| format!("{name}_{n}")))
    }

    fn public_outputs(&self) -> impl Iterator<Item = Field> {
        iter::repeat(("out", true))
            .map(Field::from)
            .zip(0..self.public_outputs)
            .map(|(f, n)| f.renamed(|name| format!("{name}_{n}")))
    }

    pub fn public_inputs<'c>(
        &self,
        ctx: &'c Context,
        is_main: bool,
    ) -> impl IntoIterator<Item = Type<'c>> {
        iter::repeat_with(move || {
            if is_main {
                StructType::from_str(ctx, "Signal").into()
            } else {
                FeltType::new(ctx).into()
            }
        })
        .take(self.public_inputs)
    }

    pub fn private_inputs<'c>(
        &self,
        ctx: &'c Context,
        is_main: bool,
    ) -> impl IntoIterator<Item = Type<'c>> {
        iter::repeat_with(move || {
            if is_main {
                StructType::from_str(ctx, "Signal").into()
            } else {
                FeltType::new(ctx).into()
            }
        })
        .take(self.private_inputs)
    }

    pub fn args<'c>(
        &self,
        ctx: &'c Context,
        is_main: bool,
        struct_name: &str,
    ) -> Vec<(Type<'c>, Location<'c>)> {
        self.public_inputs(ctx, is_main)
            .into_iter()
            .enumerate()
            .map(|(n, typ)| {
                (
                    typ,
                    Location::new(
                        ctx,
                        format!("struct {struct_name} | public inputs").as_str(),
                        n,
                        0,
                    ),
                )
            })
            .chain(
                self.private_inputs(ctx, is_main)
                    .into_iter()
                    .enumerate()
                    .map(|(n, typ)| {
                        (
                            typ,
                            Location::new(
                                ctx,
                                format!("struct {struct_name} | private inputs").as_str(),
                                n,
                                0,
                            ),
                        )
                    }),
            )
            .collect::<Vec<_>>()
    }

    pub fn arg_count(&self) -> usize {
        self.public_inputs + self.private_inputs
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
            private_inputs: inputs,
            public_inputs: 0,
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
    is_main: bool,
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

    let func_args = io.args(context, is_main, struct_name);

    log::debug!("Creating function with arguments: {func_args:?}");

    let pub_attr = [(
        Identifier::new(context, "llzk.pub"),
        PublicAttribute::new(context).into(),
    )];
    let no_attr = [];
    let compute_arg_attrs = (0..func_args.len())
        .map(|n| {
            if n < io.public_inputs {
                &pub_attr as &[(Identifier<'c>, Attribute<'c>)]
            } else {
                &no_attr
            }
        })
        .collect::<Vec<_>>();

    let constrain_arg_attrs = std::iter::once(&no_attr as &[(Identifier<'c>, Attribute<'c>)])
        .chain((1..=func_args.len()).map(|n| {
            if n <= io.public_inputs {
                &pub_attr as &[(Identifier<'c>, Attribute<'c>)]
            } else {
                &no_attr
            }
        }))
        .collect::<Vec<_>>();

    let funcs = [
        r#struct::helpers::compute_fn(
            loc,
            StructType::from_str(context, struct_name),
            &func_args,
            Some(&compute_arg_attrs),
        )
        .map(Into::into),
        r#struct::helpers::constrain_fn(
            loc,
            StructType::from_str(context, struct_name),
            &func_args,
            Some(&constrain_arg_attrs),
        )
        .map(Into::into),
    ];

    log::debug!("Creating constraint op");
    r#struct::def(loc, struct_name, &[], fields.chain(funcs))
}
