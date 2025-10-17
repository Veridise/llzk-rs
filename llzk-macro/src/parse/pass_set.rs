//! Type representing a set of passes. Based on [melior]'s.
//!
//! [melior]: https://github.com/mlir-rs/melior/blob/main/macro/src/parse/pass_set.rs.

use super::IdentifierList;
use proc_macro2::Ident;
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    LitStr, Result, Token,
};

/// Struct representing the small DSL used by the [`crate::passes`] macro.
///
/// Accepts a literal string (the name of the pass family), followed by a comma and then
/// a bracketed list of identifiers (the names of the passes).
pub struct PassSet {
    prefix: LitStr,
    identifiers: IdentifierList,
}

impl PassSet {
    pub const fn prefix(&self) -> &LitStr {
        &self.prefix
    }

    pub fn identifiers(&self) -> &[Ident] {
        self.identifiers.identifiers()
    }
}

impl Parse for PassSet {
    fn parse(input: ParseStream) -> Result<Self> {
        let prefix = input.parse()?;
        <Token![,]>::parse(input)?;

        Ok(Self {
            prefix,
            identifiers: {
                let content;
                bracketed!(content in input);
                content.parse::<IdentifierList>()?
            },
        })
    }
}
