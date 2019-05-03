extern crate proc_macro;

use heck::ShoutySnakeCase;
use heck::SnakeCase;
use heck::TitleCase;
use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::ItemStruct;
use syn::Lit;
use syn::Meta;
use syn::MetaList;
use syn::NestedMeta;

#[proc_macro_attribute]
pub fn evmc_raw(args: TokenStream, item: TokenStream) -> TokenStream {
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
            panic!("More than one meta-item supplied to evmc_raw")
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
                        panic!("Meta-item passed to evmc_raw is not a valid identifier")
                    }
                }
                NestedMeta::Literal(l) => {
                    // Try to extract a valid UTF-8 string if a literal was supplied.
                    if let Lit::Str(s) = l {
                        s.value()
                    } else {
                        panic!("Literal passed to evmc_raw is not a valid UTF-8 string literal")
                    }
                }
            }
        }
    } else {
        vm_type_name.to_title_case()
    };

    // struct declaration transformation
    // capabilities
    // create
    // destroy
    // execute
    unimplemented!()
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
