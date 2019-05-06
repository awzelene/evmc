#![recursion_limit = "128"]

extern crate proc_macro;

use proc_macro::TokenStream;
use heck::ShoutySnakeCase;
use heck::SnakeCase;
use heck::TitleCase;
use quote::quote;
use quote::quote_each_token;
use quote::ToTokens;
use syn::parse_macro_input;
use syn::parse;
use syn::parse2;
use syn::parse_str;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::Field;
use syn::Fields;
use syn::FieldsNamed;
use syn::ItemStruct;
use syn::Lit;
use syn::Meta;
use syn::MetaList;
use syn::NestedMeta;

#[proc_macro_attribute]
pub fn evmc_declare_vm(args: TokenStream, item: TokenStream) -> TokenStream {
    // First, try to parse the input token stream into an AST node representing a struct
    // declaration.
    let input: ItemStruct = parse_macro_input!(item as ItemStruct);

    // Extract the identifier of the struct from the AST node.
    let vm_type_name: String = input.ident.to_string();

    // Get the name in shouty snake case for the statically defined VM data.
    let vm_name_allcaps: String = vm_type_name.to_shouty_snake_case();

    // Get the name in snake case and strip the underscores for the symbol name.
    let vm_name_lowercase: String = vm_type_name
        .to_snake_case()
        .chars()
        .filter(|c| *c != '_')
        .collect();

    // The stylized VM name can optionally be included as an argument for the attribute. If it is
    // not provided, the stylized name defaults to the name of the VM struct in title case.
    let vm_name_stylized: String = if !args.is_empty() {
        let meta = parse_macro_input!(args as MetaList);

        // If we have more than one argument, throw a compile error. Otherwise, extract the item
        // and try to form a valid stylized name from it.
        if meta.nested.len() != 1 {
            panic!("More than one meta-item supplied to evmc_declare_vm")
        } else {
            match meta
                .nested
                .first()
                .expect("Meta-item list missing a first element.")
                .into_value()
            {
                NestedMeta::Meta(m) => {
                    // Try to form a string from the identifier if a meta-item was supplied.
                    if let Meta::Word(id) = m {
                        id.to_string()
                    } else {
                        panic!("Meta-item passed to evmc_declare_vm is not a valid identifier")
                    }
                }
                NestedMeta::Literal(l) => {
                    // Try to extract a valid UTF-8 string if a literal was supplied.
                    if let Lit::Str(s) = l {
                        s.value()
                    } else {
                        panic!(
                            "Literal passed to evmc_declare_vm is not a valid UTF-8 string literal"
                        )
                    }
                }
            }
        }
    } else {
        vm_type_name.to_title_case()
    };
    
    // Add all the EVMC fields to the struct definition so we can pass it around FFI.
    let new_struct = instance_redeclare(input);

    // struct declaration transformation
    // capabilities
    // create
    // destroy
    // execute
    unimplemented!()
}

/// Take a struct definition and prepend its list of fields with those of ffi::evmc_instance, so
/// that it can be unsafely casted correctly when passed across FFI.
fn instance_redeclare(mut input: ItemStruct) -> ItemStruct {
    // Extract the fields and determine the "style" of the struct.
    match input.fields {
        // If the struct is normal with named fields, prepend the fields list and finish.
        Fields::Named(ref mut user_fields) => {
            // Get the required EVMC fields
            let mut new_fields = evmc_instance_fields().named;

            // Push the user-defined struct fields on top of the EVMC fields.
            for field in user_fields.named.iter() {
                new_fields.push(field.clone());
            }

            (*user_fields).named = new_fields;
        },

        // If the struct is a unit struct, convert to a named struct.
        // TODO: support unit structs
        Fields::Unit => panic!("Unit structs not supported yet."),

        // Tuples are not FFI-safe, so panic if encountered.
        Fields::Unnamed(_) => panic!("Tuple structs are not supported as they are not FFI-safe."),
    };
    
    // Slightly hacky way to auto-apply the repr(C) attr.
    // TODO: figure out if there is any weird behavior when the user specifies repr(C) on their
    // own.
    let ret_tokens = quote! {
        #[repr(C)]
        #input
    };

    parse2(ret_tokens).expect("Failed to re-parse struct item when attaching repr(C) attribute.")
}

/// Get the fields of evmc_instance in AST form as a punctuated list.
fn evmc_instance_fields() -> FieldsNamed {
    // FIXME: Make this version independent.
    // Parse the fields of evmc_instance and return them as AST nodes
    let instance_fields: FieldsNamed = parse_str(
        "{
            pub abi_version: ::std::os::raw::c_int,
            pub name: *const ::std::os::raw::c_char,
            pub version: *const ::std::os::raw::c_char,
            pub destroy: ::evmc_sys::evmc_destroy_fn,
            pub execute: ::evmc_sys::evmc_execute_fn,
            pub get_capabilities: ::evmc_sys::evmc_get_capabilities_fn,
            pub set_tracer: ::evmc_sys::evmc_set_tracer_fn,
            pub pub set_option: ::evmc_sys::evmc_set_option_fn,
        }",
    )
    .expect("Could not parse EVMC instance fields");

    instance_fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camel_to_lower() {
        let a = String::from("FooBarBaz");
        let b = a.to_snake_case();
        assert_eq!(b, "foo_bar_baz");
        let c: String = b.chars().filter(|c| *c != '_').collect();
        assert_eq!(c, String::from("foobarbaz"));
    }
}
