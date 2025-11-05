//! Heavily commented example of creating IR representing a circuit for a division gadget.
//!
//! The gadget performs the division and constrains the dividend to be equal to the quotient times the divisor.
//!
//! Creates a single struct with two inputs and one output.

use std::error::Error as StdError;
use std::result::Result as StdResult;

// Commonly used types are re-exported in the prelude.
use llzk::{builder::OpBuilder, prelude::*};

type Result<T> = StdResult<T, Box<dyn StdError>>;

fn main() -> Result<()> {
    // The context preloads the LLZK dialects for convenience.
    let context = LlzkContext::new();
    // IR objects have a location associated to them. Usually a source location
    // but we won't bother with that in this case.
    let location = Location::unknown(&context);
    // LLZK top-level modules require some additional attributes.
    // This function creates a module preconfigured with these attributes.
    let module = llzk_module(location);

    // The entry point of the circuit is always a struct named `@Main`.
    // Operations can be created with factory methods with the same name as the op they create,
    // mimicking its mnemonic (struct.def in this case).
    let main_st = r#struct::def(location, "Main", &[], [])?;

    // The inputs of `@Main` must be of type !struct.type<@Signal>.
    // We need to create this struct to generate properly constructed IR.
    let signal_st: StructDefOpRef = module
        .body()
        .insert_operation(0, r#struct::helpers::define_signal_struct(&context)?.into())
        .try_into()?;

    // We store the output of the division in a data field.
    // Fields can have two extra annotations; column and public.
    // The public annotation makes the field an output of the circuit.
    let out_field = r#struct::field(
        location,
        "c",
        FeltType::new(&context),
        /*is_column=*/ false,
        /*is_public=*/ true,
    )?;
    let compute_fn = witness(&context, location, signal_st.r#type().into(), &out_field)?;
    let constrain_fn = constraints(&context, location, signal_st.r#type().into(), &out_field)?;

    main_st.body().append_operation(out_field.into());
    main_st.body().append_operation(compute_fn.into());
    main_st.body().append_operation(constrain_fn.into());

    // Now that we have filled out the struct we can add it to the module, verify it, and print it.
    module.body().append_operation(main_st.into());
    // For verifying and printing we need get a reference to the `builtin.module` op representing
    // the module.
    let module_op = module.as_operation();

    if module_op.verify() {
        println!("{module_op}")
    } else {
        eprintln!("Module failed to verify");
    }

    Ok(())
}

fn witness<'c>(
    // Context is the type used in melior to represent the MLIRContext.
    // A reference to a LlzkContext can be used as a reference to a Context.
    context: &'c Context,
    location: Location<'c>,
    signal_ty: Type<'c>,
    out_field: &FieldDefOp<'c>,
) -> Result<Operation<'c>> {
    // The inputs to the functions are public circuit inputs.
    let inputs = vec![(signal_ty, location); 2];
    let pub_attr = [PublicAttribute::named_attr_pair(context)];
    let main_ty = StructType::from_str(context, "Main");

    // The functions inside a struct need to have a particular structure. This helper creates the
    // `@compute` function with its proper structure.
    let compute_fn =
        r#struct::helpers::compute_fn(location, main_ty, &inputs, Some(&[&pub_attr, &pub_attr]))?;

    // Witness generation is represented by creating an instance of the containing struct, filling
    // its fields, and returning the value of the struct. The `compute_fn` helper
    // inserts a `struct.new` operation followed by a `function.return` operation to represent this.
    // The specific IR for our circuit needs to go in between these two operations.
    // We will insert it using the return op as reference so we need to get ahold of it and the
    // block that contains it.
    let (block, ret_op) = compute_fn
        .region(0)?
        .first_block()
        .and_then(|b| Some((b, b.terminator()?)))
        .unwrap();

    let builder = OpBuilder::new(context);

    // To get the inputs we get the arguments and then read the inner value of the signal struct
    // for performing the arithmetic.
    let a = block
        .insert_operation_before(
            ret_op,
            r#struct::readf(
                &builder,
                location,
                FeltType::new(context).into(),
                block.argument(0)?.into(),
                "reg",
            )?,
        )
        .result(0)?;
    let b = block
        .insert_operation_before(
            ret_op,
            r#struct::readf(
                &builder,
                location,
                FeltType::new(context).into(),
                block.argument(1)?.into(),
                "reg",
            )?,
        )
        .result(0)?;

    // The witness computes c = a / b
    let c = block
        .insert_operation_before(ret_op, felt::div(location, a.into(), b.into())?)
        .result(0)?;
    // The result needs to be written into the output field. For that we need to get the value
    // created by `struct.new` first.
    let self_value = block.first_operation().unwrap().result(0)?;
    // Then use the `struct.writef` operation to commit the value into the signal.
    block.insert_operation_before(
        ret_op,
        r#struct::writef(
            location,
            self_value.into(),
            out_field.field_name(),
            c.into(),
        )?,
    );

    Ok(compute_fn.into())
}

fn constraints<'c>(
    context: &'c Context,
    location: Location<'c>,
    signal_ty: Type<'c>,
    out_field: &FieldDefOp<'c>,
) -> Result<Operation<'c>> {
    // The inputs to the functions are public circuit inputs.
    let inputs = vec![(signal_ty, location); 2];
    let pub_attr = [PublicAttribute::named_attr_pair(context)];
    let main_ty = StructType::from_str(context, "Main");

    // The functions inside a struct need to have a particular structure. This helper creates the
    // `@constrain` function with its proper structure.
    let constrain_fn =
        r#struct::helpers::constrain_fn(location, main_ty, &inputs, Some(&[&pub_attr, &pub_attr]))?;

    // The constraint system is represented by a function that takes as argument an instance of
    // the parent struct as well as the same inputs the `@compute` function takes.
    // This function returns no values.
    // The `constrain_fn` helper inserts an empty `function.return` operation.
    //
    // Similar to how we generated the IR for `@compute` we need to put the IR before the
    // `function.return` operation.
    let (block, ret_op) = constrain_fn
        .region(0)?
        .first_block()
        .and_then(|b| Some((b, b.terminator()?)))
        .unwrap();

    let builder = OpBuilder::new(context);

    // We follow the same steps for obtaining the inputs but with the offsets increased by 1.
    let a = block
        .insert_operation_before(
            ret_op,
            r#struct::readf(
                &builder,
                location,
                FeltType::new(context).into(),
                block.argument(1)?.into(),
                "reg",
            )?,
        )
        .result(0)?;
    let b = block
        .insert_operation_before(
            ret_op,
            r#struct::readf(
                &builder,
                location,
                FeltType::new(context).into(),
                block.argument(2)?.into(),
                "reg",
            )?,
        )
        .result(0)?;
    // The instance that we are constraining is passed as the first argument.
    let self_value = block.argument(0)?;
    // And then read the witness output from the instance.
    let c = block
        .insert_operation_before(
            ret_op,
            r#struct::readf(
                &builder,
                location,
                FeltType::new(context).into(),
                self_value.into(),
                out_field.field_name(),
            )?,
        )
        .result(0)?;

    // The constraint is  c * b = a
    // We can use the `constrain.eq` operation for emitting equality constraints.
    let t = block
        .insert_operation_before(ret_op, felt::mul(location, c.into(), b.into())?)
        .result(0)?;
    block.insert_operation_before(ret_op, constrain::eq(location, t.into(), a.into()));

    Ok(constrain_fn.into())
}
