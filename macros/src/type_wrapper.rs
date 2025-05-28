use anyhow::{anyhow, bail, Result};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_str, Data, DeriveInput, Field, Fields, Ident, Type};

pub fn find_raw_field_inner<F: Clone, It, EqTypeFn, IsAnnotatedFn>(
    it: It,
    eq_type: EqTypeFn,
    is_annotated: IsAnnotatedFn,
) -> Result<F>
where
    It: Iterator<Item = F>,
    EqTypeFn: FnMut(&F) -> bool,
    IsAnnotatedFn: FnMut(&F) -> bool,
{
    let mlir_type_fields: Vec<_> = it.filter(eq_type).collect();
    if mlir_type_fields.is_empty() {
        bail!("Struct does not have any MlirType field")
    }

    if mlir_type_fields.len() == 1 {
        return Ok(mlir_type_fields[0].clone());
    }

    let annotated_fields: Vec<_> = mlir_type_fields.into_iter().filter(is_annotated).collect();

    if annotated_fields.is_empty() {
        bail!("Ambigouous fields. Mark one of the fields with #[raw]");
    }
    if annotated_fields.len() > 1 {
        bail!("Ambigouous fields. Only one field can be annotated with #[raw]");
    }
    Ok(annotated_fields[0].clone())
}

enum FieldLookup {
    Named(Ident),
    Unnamed,
}

impl FieldLookup {
    pub fn quote_create(&self, type_name: &Ident, raw: &Ident) -> TokenStream {
        match self {
            FieldLookup::Named(field) => quote! {
                #type_name { #field: #raw }
            },
            FieldLookup::Unnamed => quote! {
                #type_name(#raw)
            },
        }
    }

    pub fn quote_access(&self) -> TokenStream {
        match self {
            FieldLookup::Named(field) => quote! {
                self.#field
            },

            FieldLookup::Unnamed => quote! {
                self.0
            },
        }
    }
}

impl TryFrom<Result<&Field>> for FieldLookup {
    type Error = anyhow::Error;

    fn try_from(value: Result<&Field>) -> std::result::Result<Self, Self::Error> {
        Ok(FieldLookup::Named(
            value?
                .ident
                .clone()
                .ok_or(anyhow!("Missing field name in named struct"))?,
        ))
    }
}

impl TryFrom<Result<(usize, &Field)>> for FieldLookup {
    type Error = anyhow::Error;

    fn try_from(_value: Result<(usize, &Field)>) -> std::result::Result<Self, Self::Error> {
        Ok(FieldLookup::Unnamed)
    }
}

fn find_raw_field(fields: &Fields) -> Result<FieldLookup> {
    let mlir_type = parse_str::<Type>("mlir_sys::MlirType")?;
    let is_mlir_type = |f: &&Field| f.ty == mlir_type;

    let is_annotated = |f: &&Field| f.attrs.iter().any(|a| a.path().is_ident("raw"));
    match fields {
        Fields::Named(fields) => {
            let result = find_raw_field_inner(fields.named.iter(), is_mlir_type, is_annotated);
            Ok(FieldLookup::Named(
                result?
                    .ident
                    .clone()
                    .ok_or(anyhow!("Missing field name in named struct"))?,
            ))
        }
        Fields::Unnamed(fields) => {
            find_raw_field_inner(fields.unnamed.iter(), is_mlir_type, is_annotated)
                .map(|_| FieldLookup::Unnamed)
        }
        Fields::Unit => bail!("Struct does not have any fields"),
    }
}

pub fn derive_type_wrapper_impl(input: DeriveInput) -> Result<TokenStream> {
    println!("input = {input:?}");
    let type_name = input.ident;
    let raw = parse_str::<Ident>("raw")?;

    if let Data::Struct(data) = input.data {
        let field = find_raw_field(&data.fields)?;
        let struct_creation = field.quote_create(&type_name, &raw);
        let struct_access = field.quote_access();
        let check_fn = format_ident!("llzkTypeIsA{}", type_name);
        return Ok(quote! {
                                   impl<'c> llzk::util::FromRaw<mlir_sys::MlirType> for #type_name<'c> {
                                       unsafe fn from_raw(t: mlir_sys::MlirType) -> Self {
                                            #struct_creation
                                       }
                                   }

                       impl<'c> melior::ir::r#type::TypeLike<'c> for #type_name<'c> {
                           fn to_raw(&self) -> mlir_sys::MlirType {
                               #struct_access
                           }
                       }

                       impl<'c> TryFrom<melior::ir::r#type::Type<'c>> for #type_name<'c> {
                           type Error = melior::Error;

                           fn try_from(t: melior::ir::r#type::Type<'c>) -> Result<Self, Self::Error> {
                               if unsafe { llzk_sys::#check_fn(t.to_raw()) } {
                                   Ok(unsafe { Self::from_raw(t.to_raw()) })
                               } else {
                                   Err(Self::Error::TypeExpected("llzk struct", t.to_string()))
                               }
                           }
                       }

                       impl<'c> std::fmt::Display for #type_name<'c> {
                           fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {

                let mut data = (formatter, Ok(()));

                       unsafe {
                           mlir_sys::mlirTypePrint(
                               self.to_raw(),
                               Some(llzk::utils::print_callback),
                               &mut data as *mut _ as *mut c_void,
                           );
                       }

                       data.1
                           }
                       }

                       impl<'c> Into<melior::ir::r#type::Type<'c>> for #type_name<'c> {
                           fn into(self) -> melior::ir::r#type::Type<'c> {
                               unsafe { melior::ir::r#type::Type::from_raw(self.to_raw()) }
                           }
                       }

        //impl<T: melior::ir::r#type::TypeLike> std::cmp::PartialEq<Rhs=T> for #type_name<'_> {
        //
        //    fn eq(&self, other: &Self) -> bool {
        //        unsafe { mlirTypeEqual(self.to_raw(), other.to_raw()) }
        //    }
        //}

        impl Eq for Type<'_> {}
                               });
    }
    Ok(quote! {})
}
