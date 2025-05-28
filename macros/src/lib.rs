mod type_wrapper;
use syn::{parse_macro_input, DeriveInput};

use proc_macro::TokenStream;

#[proc_macro_derive(TypeWrapper, attributes(raw))]
pub fn derive_type_wrapper(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    type_wrapper::derive_type_wrapper_impl(input)
        .expect("Type wrapper macro expansion error")
        .into()
}
