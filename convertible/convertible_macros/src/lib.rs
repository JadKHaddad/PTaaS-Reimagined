// use std::vec;

//use convertible_definitions::dart::*;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(Debug)]
struct FieldNameAndType {
    name: String,
    ty: String,
    optional: bool,
}

fn extract_type<'a>(ty: &'a syn::Type, types: &[&str]) -> Option<&'a syn::Type> {
    if let syn::Type::Path(syn::TypePath { qself: None, path }) = ty {
        let segments_str = &path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join(":");

        let option_segment = types
            .iter()
            .find(|s| segments_str == *s)
            .and_then(|_| path.segments.last());

        let inner_type = option_segment
            .and_then(|path_seg| match &path_seg.arguments {
                syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    args,
                    ..
                }) => args.first(),
                _ => None,
            })
            .and_then(|generic_arg| match generic_arg {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => None,
            });
        return inner_type;
    }
    None
}

fn is_simple_segment(segment: &syn::PathSegment) -> bool {
    let simple_types = [
        "String", "bool", "u8", "i8", "u16", "i16", "u32", "i32", "u64", "i64", "u128", "i128",
        "usize", "isize",
    ];
    let not_simple_types = [
        "Vec",
        "std::vec::Vec",
        "core::vec::Vec",
        "LinkedList",
        "std::collections::LinkedList",
        "core::collections::LinkedList",
        "BinaryHeap",
        "std::collections::BinaryHeap",
        "core::collections::BinaryHeap",
        "HashSet",
        "std::collections::HashSet",
        "core::collections::HashSet",
        "BTreeSet",
        "std::collections::BTreeSet",
        "core::collections::BTreeSet",
        "HashMap",
        "std::collections::HashMap",
        "core::collections::HashMap",
        "BTreeMap",
        "std::collections::BTreeMap",
        "core::collections::BTreeMap",
        "std::option::Option",
        "core::option::Option",
        "Option",
    ];

    let segment_ident = segment.ident.to_string();
    //simple_types.contains(&segment_ident.as_str())
    !not_simple_types.contains(&segment_ident.as_str())
}

fn is_simple_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(path) => {
            // Check if the type is a simple path
            let segments = &path.path.segments;
            segments.len() == 1 && is_simple_segment(&segments[0])
        }
        _ => false,
    }
}

#[proc_macro_derive(DartConvertible, attributes(dart_convertible))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    println!("struct_name: {}", struct_name);

    let mut is_struct = false;
    let mut is_enum = false;

    match input.data {
        syn::Data::Struct(_) => is_struct = true,
        syn::Data::Enum(_) => is_enum = true,
        _ => panic!("Only structs and enums are supported"),
    };

    println!("is_struct: {}", is_struct);
    println!("is_enum: {}", is_enum);

    // let is_struct = match input.data {
    //     syn::Data::Struct(_) => true,
    //     _ => false,
    // };

    // // TODO: enums are also supported but not implemented yet
    // if !is_struct {
    //     panic!("Only structs are supported");
    // }

    // let decorators: Vec<String> = vec!["@JsonSerializable()".into()];
    if is_struct {
        // lets collect the fields of the struct
        let fields = if let syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
            ..
        }) = input.data
        {
            named
        } else {
            panic!("Only structs with named fields are supported");
        };

        // now lets extract the name and type of each field
        let fields: Vec<FieldNameAndType> = fields
            .iter()
            .map(|field| {
                let field_name = field.ident.as_ref().unwrap().to_string();
                // Only Normal fields and Vec fields are supported for now
                // Optional fields are supported by default
                println!("field_name: {}", field_name);

                if is_simple_type(&field.ty) {
                    return FieldNameAndType {
                        name: field_name,
                        ty: field.ident.as_ref().unwrap().to_string(),
                        optional: false,
                    };
                }

                // see if its an optional field
                if let Some(inner_type) = extract_type(
                    &field.ty,
                    &["Option", "std:option:Option", "core:option:Option"],
                ) {
                    if !is_simple_type(inner_type) {
                        panic!("Only simple types are supported for now");
                    }

                    // TODO: inside the option we will allow our simple types or vec but not other option for example

                    return FieldNameAndType {
                        name: field_name,
                        ty: field.ident.as_ref().unwrap().to_string(),
                        optional: true,
                    };
                };

                // see if its a Vec field
                if let Some(inner_type) = extract_type(
                    &field.ty,
                    &[
                        "Vec",
                        "std:vec:Vec",
                        "core:vec:Vec",
                        "std:vec:vec",
                        "core:vec:vec",
                    ],
                ) {
                    if !is_simple_type(inner_type) {
                        panic!("Only simple types are supported for now");
                    }

                    return FieldNameAndType {
                        name: field_name,
                        ty: field.ident.as_ref().unwrap().to_string(),
                        optional: false,
                    };
                };

                panic!(
                    "Only simple types and Vec fields are supported for now [{}]",
                    field_name
                );
            })
            .collect();

        println!("fields: {:?}", fields);
    }

    // let fields: Vec<DartField> = fields
    //     .iter()
    //     .map(|field| {
    //         let field_name = field.ident.as_ref().unwrap().to_string();
    //         // Only Normal fields and Vec fields are supported for now
    //         // Optional fields are supported by default

    //         todo!();
    //     })
    //     .collect();

    let expanded = quote! {
        impl convertible::definitions::DartConvertible for #struct_name {
            fn to_dart() -> &'static str {
                r"
                @JsonSerializable()
                I am a dummy
                "
            }
        }
    };

    expanded.into()
}
