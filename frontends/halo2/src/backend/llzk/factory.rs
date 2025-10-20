use llzk::{attributes::NamedAttribute, prelude::*};

use melior::{
    Context,
    ir::{Identifier, Location, Operation, Type},
};

fn struct_def_op_location<'c>(context: &'c Context, name: &str, index: usize) -> Location<'c> {
    Location::new(context, format!("struct {name}").as_str(), index, 0)
}

pub struct StructIO {
    private_inputs: usize,
    public_inputs: usize,
    private_outputs: usize,
    public_outputs: usize,
}

impl StructIO {
    fn fields<'c>(
        &self,
        context: &'c Context,
        header: &str,
    ) -> impl Iterator<Item = Result<FieldDefOp<'c>, LlzkError>> {
        let public_filename = format!("struct {header} | public outputs");
        let private_filename = format!("struct {header} | private outputs");
        std::iter::repeat_n(true, self.public_outputs)
            .enumerate()
            .chain(std::iter::repeat_n(false, self.private_outputs).enumerate())
            .map(move |(n, public)| {
                let filename = if public {
                    &public_filename
                } else {
                    &private_filename
                };
                (public, Location::new(context, filename, n, 0))
            })
            .enumerate()
            .map(|(n, (public, loc))| {
                let name = format!("out_{n}");
                r#struct::field(loc, &name, FeltType::new(context), false, public)
            })
    }

    pub fn args<'c>(
        &self,
        ctx: &'c Context,
        is_main: bool,
        struct_name: &str,
    ) -> Vec<(Type<'c>, Location<'c>)> {
        let public_filename = format!("struct {struct_name} | public inputs");
        let private_filename = format!("struct {struct_name} | private inputs");
        let public_locs = std::iter::repeat(&public_filename)
            .enumerate()
            .take(self.public_inputs);
        let private_locs = std::iter::repeat(&private_filename)
            .enumerate()
            .take(self.private_inputs);
        let locs = public_locs
            .chain(private_locs)
            .map(|(n, filename)| Location::new(ctx, filename, n, 0));

        let ty: Type<'c> = if is_main {
            StructType::from_str(ctx, "Signal").into()
        } else {
            FeltType::new(ctx).into()
        };
        let types = std::iter::repeat(ty).take(self.public_inputs + self.private_inputs);

        std::iter::zip(types, locs).collect()
    }

    /// Returns the list of argument attributes for the struct's functions.
    ///
    /// Element #0 is the attribute for the self argument in `@constrain`. if using the output of
    /// this method to create the `@compute` function read the slice from element #1 `[1..]`.
    pub fn arg_attrs<'c>(&self, ctx: &'c Context) -> Vec<Vec<NamedAttribute<'c>>> {
        let pub_attr = (
            Identifier::new(ctx, "llzk.pub"),
            PublicAttribute::new(ctx).into(),
        );
        std::iter::once(vec![])
            .chain(std::iter::repeat_n(vec![pub_attr], self.public_inputs))
            .chain(std::iter::repeat_n(vec![], self.private_inputs))
            .collect()
    }

    pub fn from_io(advice: &crate::io::AdviceIO, instance: &crate::io::InstanceIO) -> Self {
        Self {
            private_inputs: advice.inputs().len(),
            public_inputs: instance.inputs().len(),
            private_outputs: advice.outputs().len(),
            public_outputs: instance.outputs().len(),
        }
    }

    pub fn from_io_count(inputs: usize, outputs: usize) -> Self {
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
        .fields(context, struct_name)
        .map(|r| r.map(Operation::from));

    let func_args = io.args(context, is_main, struct_name);
    let arg_attrs = io.arg_attrs(context);

    log::debug!("Creating function with arguments: {func_args:?}");

    let funcs = [
        r#struct::helpers::compute_fn(
            loc,
            StructType::from_str(context, struct_name),
            &func_args,
            Some(&arg_attrs[1..]),
        )
        .map(Operation::from),
        r#struct::helpers::constrain_fn(
            loc,
            StructType::from_str(context, struct_name),
            &func_args,
            Some(&arg_attrs),
        )
        .map(Operation::from),
    ];

    r#struct::def(loc, struct_name, &[], fields.chain(funcs))
}
