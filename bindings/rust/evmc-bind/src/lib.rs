extern crate proc_macro;

use heck::ShoutySnakeCase;
use heck::SnakeCase;
use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::ItemStruct;

#[proc_macro_attribute]
pub fn evmc_raw(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // First, try to parse the input token stream into an AST node representing a struct
    // declaration.
    let input: ItemStruct = parse_macro_input!(item as ItemStruct);

    // Extract the identifier of the struct from the AST node.
    let vm_type_name: String = input.ident.to_string();

    // Get the name in shouty snake case for the statically defined VM data.
    let vm_name_allcaps: String = vm_type_name.to_shouty_snake_case();
    
    // Get the name in snake case and strip the underscores for the symbol name.
    let vm_name_lowercase: String = vm_type_name.to_snake_case().chars().filter(|c| *c != '_').collect();

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
