extern crate proc_macro;

use heck::ShoutySnakeCase;
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

    // struct declaration transformation
    // capabilities
    // create
    // destroy
    // execute
    unimplemented!()
}
