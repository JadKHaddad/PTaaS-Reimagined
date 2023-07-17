use convert_case::{Case, Casing};
use convertible_definitions::dart::*;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput};

fn create_serde_dart_class(fields: Vec<DartField>, class_name: String) -> DartClass {
    let constructor_parameters = DartParameters::Named(
        fields
            .iter()
            .map(|field| NamedDartParameter {
                required: true,
                parameter: DartParameter::ConstructorParameter(DartConstructorParameter {
                    name: field.name.clone(),
                }),
            })
            .collect(),
    );

    let constructor = DartConstructor::OneLiner(DartOnelineConstructor {
        name: class_name.clone(),
        parameters: constructor_parameters,
    });

    let factory_body = MethodBody::OneLiner(OnelineMethodBody {
        name: format!("_${}FromJson", class_name),
        parameters: vec![String::from("json")],
    });

    let factory_params =
        DartParameters::Positional(vec![DartParameter::MethodParameter(DartMethodParameter {
            name: String::from("json"),
            type_: DartType::Map(String::from("String"), String::from("dynamic")),
        })]);

    let factory = DartConstructor::Factory(DartFactoryConstructor::OneLiner(
        DartOnelineFactoryConstructor {
            class_name: class_name.clone(),
            name: String::from("fromJson"),
            parameters: factory_params,
            body: factory_body,
        },
    ));

    let to_json_method_params = DartParameters::Positional(vec![]);

    let to_json_method_body = MethodBody::OneLiner(OnelineMethodBody {
        name: format!("_${}ToJson", class_name),
        parameters: vec![String::from("this")],
    });

    let to_json_method = DartMethod::OneLiner(DartOnelineMethod {
        name: String::from("toJson"),
        type_: DartType::Map(String::from("String"), String::from("dynamic")),
        parameters: to_json_method_params,
        body: to_json_method_body,
    });

    DartClass {
        decorators: vec![String::from("@JsonSerializable()")],
        name: class_name,
        fields,
        constructors: vec![constructor, factory],
        methods: vec![to_json_method],
    }
}

/// Checks if the type is a wrapper type like Option or Vec
/// and returns the inner type.
/// If the type is not a wrapper type, it returns None.
/// For Option: ["Option", "std:option:Option", "core:option:Option"].
/// For Vec: ["Vec", "std:vec:Vec", "core:vec:Vec"].
fn extract_type_if_exists<'a>(ty: &'a syn::Type, types: &[&str]) -> Option<&'a syn::Type> {
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

    let mut is_struct = false;
    let mut _is_enum = false;

    match input.data {
        syn::Data::Struct(_) => is_struct = true,
        syn::Data::Enum(_) => _is_enum = true,
        _ => panic!("Only structs and enums are supported"),
    };

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
        let dart_fields: Vec<DartField> = fields
            .iter()
            .map(|field| {
                // TODO: rework!
                let field_name = field
                    .ident
                    .as_ref()
                    .expect("Field name not found")
                    .to_string();

                // Only Normal fields and Vec fields are supported for now
                // Optional fields are supported by default

                let mut ty = &field.ty.clone();
                let mut optional = false;

                // see if its an optional field
                if let Some(inner_type) = extract_type_if_exists(
                    ty,
                    &["Option", "std:option:Option", "core:option:Option"],
                ) {
                    optional = true;
                    ty = inner_type;
                }

                // this is a simple field, just take it
                if is_simple_type(ty) {
                    let ty_string = ty.to_token_stream().to_string();
                    return DartField {
                        keywords: vec![String::from("final")],
                        name: field_name.to_case(Case::Camel),
                        type_: DartType::Primitive(rust_primitive_to_dart_primitive(&ty_string)),
                        optional,
                    };
                }

                // see if its a Vec field
                if let Some(inner_type) = extract_type_if_exists(
                    ty,
                    &[
                        "Vec",
                        "std:vec:Vec",
                        "core:vec:Vec",
                        "std:vec:vec",
                        "core:vec:vec",
                    ],
                ) {
                    // now this is a Vec. lets check the inner type!
                    if !is_simple_type(inner_type) {
                        panic!(
                            "[{}] Only simple types are supported inside a Vec",
                            field_name
                        );
                    }

                    let ty_string = inner_type.to_token_stream().to_string();
                    return DartField {
                        keywords: vec![String::from("final")],
                        name: field_name.to_case(Case::Camel),
                        type_: DartType::List(rust_primitive_to_dart_primitive(&ty_string)),
                        optional,
                    };
                };

                panic!(
                    "[{}] Only simple types and Vec fields are supported",
                    field_name
                );
            })
            .collect();

        // println!("dart_fields: {:#?}", dart_fields);

        let dart_code = create_serde_dart_class(dart_fields, struct_name.to_string()).to_string();

        let expanded = quote! {
            impl convertible::definitions::DartConvertible for #struct_name {
                fn to_dart() -> &'static str {
                    #dart_code
                }
            }
        };

        return expanded.into();
    }

    let expanded = quote! {
        impl convertible::definitions::DartConvertible for #struct_name {
            fn to_dart() -> &'static str {
                r"
                Not implemented yet
                "
            }
        }
    };

    expanded.into()
}
