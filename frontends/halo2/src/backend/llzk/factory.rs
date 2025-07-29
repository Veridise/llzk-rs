use std::iter;

use llzk::{
    dialect::{
        felt::FeltType,
        function::{self, FuncDefOpLike as _},
        r#struct::{self, FieldDefOp, StructDefOp, StructType},
    },
    error::Error,
};
use melior::{
    ir::{attribute::FlatSymbolRefAttribute, r#type::FunctionType, Location, Operation, Type},
    Context,
};

fn struct_def_op_location<'c>(context: &'c Context, name: &str, index: usize) -> Location<'c> {
    Location::new(context, format!("struct {}", name).as_str(), index, 0)
}

fn create_field<'c>(
    context: &'c Context,
    header: &str,
    name: &str,
    public: bool,
) -> Result<FieldDefOp<'c>, Error> {
    let field_name = FlatSymbolRefAttribute::new(context, name);
    let filename = format!("struct {} | field {}", header, name);
    let loc = Location::new(context, &filename, 0, 0);

    r#struct::field(loc, field_name, FeltType::new(context), false, public)
}

fn struct_type<'c>(context: &'c Context, name: &str) -> Type<'c> {
    StructType::from_str(context, name).into()
}

pub fn create_struct<'c>(
    context: &'c Context,
    struct_name: &str,
    idx: usize,
    advice_inputs: usize,
    instance_inputs: usize,
    advice_outputs: usize,
    instance_outputs: usize,
) -> Result<StructDefOp<'c>, Error> {
    let loc = struct_def_op_location(context, struct_name, idx);
    let fields = iter::zip(0..advice_inputs, iter::repeat(("in", false)))
        .chain(iter::zip(0..advice_outputs, iter::repeat(("out", false))))
        .chain(iter::zip(0..instance_outputs, iter::repeat(("out", true))))
        .map(|(idx, (kind, public))| (format!("{kind}_{idx}"), public))
        .map(|(name, public)| -> Result<Operation<'c>, Error> {
            create_field(context, struct_name, &name, public).map(Into::into)
        });

    let func_args = [struct_type(context, struct_name)]
        .into_iter()
        .chain(iter::repeat_with(|| FeltType::new(context).into()).take(instance_inputs))
        .collect::<Vec<_>>();

    let constrain = function::def(
        loc,
        "constrain",
        FunctionType::new(context, &func_args, &[]),
        &[],
        None,
    )
    .inspect(|f| f.set_allow_constraint_attr(true));

    r#struct::def(
        loc,
        FlatSymbolRefAttribute::new(context, struct_name),
        &[],
        fields.chain([constrain.map(Into::into)]),
    )
}
