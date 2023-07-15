use std::vec;

use convertible_definitions::dart::*;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(DartConvertible, attributes(dart_convertible))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let is_struct = match input.data {
        syn::Data::Struct(_) => true,
        _ => false,
    };

    // TODO: enums are also supported but not implemented yet
    if !is_struct {
        panic!("Only structs are supported");
    }

    let decorators: Vec<String> = vec!["@JsonSerializable()".into()];

    // let collect the fields of the struct
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = input.data
    {
        named
    } else {
        panic!("Only structs with named fields are supported");
    };

    let fields: Vec<DartField> = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap().to_string();
            // Only Normal fields and Vec fields are supported for now
            // Optional fields are supported by default

            todo!();
        })
        .collect();

    let expanded = quote! {
        impl convertible::definitions::DartConvertible for #name {
            fn to_dart() -> &'static str {
                r"
                @JsonSerializable()
                I am a dummy
                "
            }
        }
    };

    TokenStream::from(expanded)
}
