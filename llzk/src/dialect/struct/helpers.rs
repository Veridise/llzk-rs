//! Convenience functions for creating common operation patterns.

use melior::{
    Context,
    ir::{
        Attribute, Block, BlockLike as _, Identifier, Location, RegionLike as _, Type,
        operation::OperationLike as _, r#type::FunctionType,
    },
};

use crate::{
    attributes::NamedAttribute,
    builder::OpBuilder,
    dialect::{
        constrain,
        function::{self, FuncDefOp},
    },
    error::Error,
    prelude::{FeltType, FuncDefOpLike as _, StructDefOp},
};

use super::r#type::StructType;

/// Creates an empty `@compute` function with the configuration expected by `struct.def`.
pub fn compute_fn<'c>(
    loc: Location<'c>,
    struct_type: StructType<'c>,
    inputs: &[(Type<'c>, Location<'c>)],
    arg_attrs: Option<&[Vec<NamedAttribute<'c>>]>,
) -> Result<FuncDefOp<'c>, Error> {
    let context = loc.context();
    let input_types: Vec<Type<'c>> = inputs.iter().map(|(t, _)| *t).collect();
    function::def(
        loc,
        "compute",
        FunctionType::new(
            unsafe { context.to_ref() },
            &input_types,
            &[struct_type.into()],
        ),
        &[],
        arg_attrs,
    )
    .and_then(|f| {
        let block = Block::new(inputs);
        let new_struct = block.append_operation(super::new(loc, struct_type));
        block.append_operation(function::r#return(loc, &[new_struct.result(0)?.into()]));
        f.set_allow_witness_attr(true);
        f.region(0)?.append_block(block);
        Ok(f)
    })
}

/// Creates an empty `@constrain` function with the configuration expected by `struct.def`.
///
/// If `arg_attrs` is `Some` it must have `inputs.len() + 1` elements and element #0 is the
/// argument attributes of the self argument.
pub fn constrain_fn<'c>(
    loc: Location<'c>,
    struct_type: StructType<'c>,
    inputs: &[(Type<'c>, Location<'c>)],
    arg_attrs: Option<&[Vec<NamedAttribute<'c>>]>,
) -> Result<FuncDefOp<'c>, Error> {
    let context = loc.context();
    let mut input_types: Vec<Type<'c>> = Vec::with_capacity(inputs.len() + 1);
    input_types.push(struct_type.into());
    input_types.extend(inputs.iter().map(|(t, _)| *t));
    let mut all_inputs = vec![(struct_type.into(), loc)];
    all_inputs.extend(inputs);
    let all_arg_attrs = arg_attrs.map(|original| {
        let mut result: Vec<Vec<(Identifier<'_>, Attribute<'_>)>> = vec![vec![]];
        result.extend(original.iter().cloned());
        result
    });
    function::def(
        loc,
        "constrain",
        FunctionType::new(unsafe { context.to_ref() }, &input_types, &[]),
        &[],
        all_arg_attrs.as_deref(),
    )
    .and_then(|f| {
        let block = Block::new(&all_inputs);
        block.append_operation(function::r#return(loc, &[]));
        f.set_allow_constraint_attr(true);
        f.region(0)?.append_block(block);
        Ok(f)
    })
}

/// Creates the `@Signal` struct.
///
/// The `@Main` struct's inputs must be of this type or arrays of this type.
pub fn define_signal_struct<'c>(context: &'c Context) -> Result<StructDefOp<'c>, Error> {
    let loc = Location::new(context, "Signal struct", 0, 0);
    let typ = StructType::from_str(context, "Signal");
    let reg = "reg";
    super::def(loc, "Signal", &[], {
        [
            super::field(loc, reg, FeltType::new(context), false, true).map(Into::into),
            compute_fn(loc, typ, &[(FeltType::new(context).into(), loc)], None)
                .and_then(|compute| {
                    let block = compute
                        .region(0)?
                        .first_block()
                        .ok_or(Error::BlockExpected(0))?;
                    let fst = block.first_operation().ok_or(Error::EmptyBlock)?;
                    if fst.name() != Identifier::new(context, "struct.new") {
                        return Err(Error::OperationExpected(
                            "struct.new",
                            fst.name().as_string_ref().as_str()?.to_owned(),
                        ));
                    }
                    block.insert_operation_after(
                        fst,
                        super::writef(loc, fst.result(0)?.into(), reg, block.argument(0)?.into())?,
                    );
                    Ok(compute)
                })
                .map(Into::into),
            constrain_fn(loc, typ, &[(FeltType::new(context).into(), loc)], None)
                .and_then(|constrain| {
                    let block = constrain
                        .region(0)?
                        .first_block()
                        .ok_or(Error::BlockExpected(0))?;
                    let fst = block.first_operation().ok_or(Error::EmptyBlock)?;
                    if fst.name() != Identifier::new(context, "function.return") {
                        return Err(Error::OperationExpected(
                            "function.return",
                            fst.name().as_string_ref().as_str()?.to_owned(),
                        ));
                    }
                    let reg = block.insert_operation_before(
                        fst,
                        super::readf(
                            &OpBuilder::new(context),
                            loc,
                            FeltType::new(context).into(),
                            block.argument(0)?.into(),
                            "reg",
                        )?,
                    );
                    block.insert_operation_after(
                        reg,
                        constrain::eq(loc, reg.result(0)?.into(), block.argument(1)?.into()),
                    );
                    Ok(constrain)
                })
                .map(Into::into),
        ]
    })
}
