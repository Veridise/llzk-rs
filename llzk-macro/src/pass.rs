//! Macro for emitting create and register functions for a set of passes.

use crate::error::Error;
use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use std::error::Error as StdError;
use syn::LitStr;

const CREATE_FUNCTION_PREFIX: &str = "mlirCreate";

/// General implementation for the pass related macros.
///
/// Generates creation and registration functions for each identifier (the name of the CAPI create
/// function) and, if given, a registration function for the whole family. Requires a callback
/// that extracts the name of the actual pass from the name.
pub fn generate(
    names: &[Ident],
    family: Option<&LitStr>,
    extract_pass_name: impl Fn(&str) -> Result<String, Error>,
) -> Result<TokenStream, Box<dyn StdError>> {
    let mut stream = TokenStream::new();

    for name in names {
        let foreign_name = name.to_string();
        let foreign_name = foreign_name
            .strip_prefix(CREATE_FUNCTION_PREFIX)
            .ok_or_else(|| Error::failed_to_strip([CREATE_FUNCTION_PREFIX], &foreign_name))?;
        let pass_name = extract_pass_name(foreign_name)?;

        let function_name = create_function_name("create", &pass_name, name.span());
        let document = format!(" Creates a `{pass_name}` pass.");

        stream.extend(TokenStream::from(quote! {
            #[doc = #document]
            pub fn #function_name() -> melior::pass::Pass {
                unsafe { melior::pass::Pass::from_raw_fn(llzk_sys::#name) }
            }
        }));

        let foreign_function_name =
            Ident::new(&("mlirRegister".to_owned() + foreign_name), name.span());
        let function_name = create_function_name("register", &pass_name, name.span());
        let document = format!(" Registers a `{pass_name}` pass.");

        stream.extend(TokenStream::from(quote! {
            #[doc = #document]
            pub fn #function_name() {
                unsafe { llzk_sys::#foreign_function_name() }
            }
        }));
    }

    if let Some(family) = family {
        let family_name = family.value();
        let family_pretty = family_name.to_case(Case::Sentence).replace("Llzk", "LLZK");
        let document = format!(" Registers all {family_pretty} passes.");
        let foreign_function_name =
            Ident::new(&format!("mlirRegister{family_name}Passes"), family.span());
        let function_name = Ident::new(
            &format!("register_{}_passes", family_name.to_case(Case::Snake)),
            family.span(),
        );
        stream.extend(TokenStream::from(quote! {
            #[doc = #document]
            pub fn #function_name() {
                unsafe { llzk_sys::#foreign_function_name() }
            }
        }))
    }

    Ok(stream)
}

fn create_function_name(prefix: &str, pass_name: &str, span: Span) -> Ident {
    Ident::new(
        &format!("{}_{}", prefix, &pass_name.to_case(Case::Snake)),
        span,
    )
}
