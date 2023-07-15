use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(DartConvertible, attributes(dart_convertible))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    let expanded = quote! {
        impl convertible::DartConvertible for #name {
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
