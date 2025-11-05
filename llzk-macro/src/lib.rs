#![doc = include_str!("../README.md")]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

use error::Error;
use parse::*;
use proc_macro::TokenStream;
use quote::quote;
use std::error::Error as StdError;
use syn::parse_macro_input;

mod error;
mod parse;
mod pass;

/// Creates functions for registering and creating conversion passes.
#[proc_macro]
pub fn conversion_passes(stream: TokenStream) -> TokenStream {
    let identifiers = parse_macro_input!(stream as IdentifierList);

    convert_result(pass::generate(identifiers.identifiers(), None, |name| {
        let prefixes = ["Conversion", "Convert", "ConversionPass", "Pass"];

        let name = prefixes
            .iter()
            .find_map(|prefix| name.strip_prefix(prefix))
            .ok_or_else(|| Error::failed_to_strip(prefixes, name))?;
        Ok(name.to_string())
    }))
}

/// Creates functions for registering and creating passes.
#[proc_macro]
pub fn passes(stream: TokenStream) -> TokenStream {
    let set = parse_macro_input!(stream as PassSet);

    convert_result(pass::generate(
        set.identifiers(),
        Some(set.prefix()),
        |name| {
            let name = name
                .strip_prefix(&set.prefix().value())
                .ok_or_else(|| Error::failed_to_strip([set.prefix().value()], name))?;
            Ok(name.to_string())
        },
    ))
}

/// Converts a [`Result::Err`] into a compilation error.
fn convert_result(result: Result<TokenStream, Box<dyn StdError>>) -> TokenStream {
    result.unwrap_or_else(|error| {
        let message = error.to_string();

        quote! { compile_error!(#message) }.into()
    })
}
