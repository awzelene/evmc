#![recursion_limit = "128"]

extern crate proc_macro;

use heck::ShoutySnakeCase;
use heck::SnakeCase;
use heck::TitleCase;
use proc_macro::TokenStream;
use quote::quote;
use quote::quote_each_token;
use quote::ToTokens;
use syn::parse;
use syn::parse2;
use syn::parse_macro_input;
use syn::parse_str;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::Field;
use syn::Fields;
use syn::FieldsNamed;
use syn::Ident;
use syn::IntSuffix;
use syn::ItemStruct;
use syn::Lit;
use syn::LitInt;
use syn::LitStr;
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

    // Parse the attribute meta-items for the name and version. Verify param count.
    let meta = parse_macro_input!(args as MetaList);
    assert!(
        meta.nested.len() == 3,
        "Incorrect number of meta-items passed to evmc_declare_vm."
    );

    //TODO: Reduce code duplication here for accessing meta-args.
    // Extract a name from the first argument.
    let vm_name_stylized = match &meta.nested[0] {
        NestedMeta::Meta(m) => {
            // Try to form a string from the identifier if a meta-item was supplied.
            if let Meta::Word(id) = m {
                id.to_string()
            } else {
                panic!("Meta-item passed to evmc_declare_vm is not a valid identifier.")
            }
        }
        NestedMeta::Literal(l) => {
            // Try to extract a valid UTF-8 string if a literal was supplied.
            if let Lit::Str(s) = l {
                s.value()
            } else {
                panic!("Literal passed to evmc_declare_vm is not a valid UTF-8 string literal.")
            }
        }
    };

    // Extract a version string from the second argument.
    let vm_version_string = if let NestedMeta::Literal(l) = &meta.nested[1] {
        match l {
            Lit::Str(s) => s.value(),
            _ => {
                panic!("Argument 2 passed to evmc_declare_vm is not a valid UTF-8 string literal.")
            }
        }
    } else {
        panic!(
            "Argument 2 passed to evmc_declare_vm is of incorrect type. Expected a string literal."
        )
    };

    // Extract the capabilities string from the third argument and convert it to the appropriate
    // flag.
    // NOTE: We use the strings because attribute parameters cannot be integer literals and a meta-item cannot be used to
    // describe a version number.
    let vm_capabilities = if let NestedMeta::Literal(l) = &meta.nested[2] {
        match l {
            Lit::Str(s) => match s.value().as_str() {
                "evm1" => 0x1u32,
                "ewasm" => 0x1u32 << 1u32,
                _ => panic!("Invalid capabilities specifier. Use 'evm1' or 'ewasm'."),
            },
            _ => {
                panic!("Argument 3 passed to evmc_declare_vm is not a valid UTF-8 string literal.")
            }
        }
    } else {
        panic!(
            "Argument 3 passed to evmc_declare_vm is of incorrect type. Expected a string literal."
        )
    };

    // create
    // destroy
    // execute
    unimplemented!()
}

/// Takes an identifier and struct definition, builds an evmc_create_* function for FFI.
fn build_create_fn(
    name_lowercase: &String,
    name_caps: &String,
    type_name: &String,
    vm_definition: &ItemStruct,
) -> TokenStream {
    let fn_name = format!("evmc_create_{}", name_lowercase);
    let fn_ident = Ident::new(&fn_name, name_lowercase.span());
    let type_ident = Ident::new(type_name, type_name.span());

    // TODO: reduce code duplication here.
    let capabilities_fn_string = format!("{}_get_capabilities", name_lowercase);
    let capabilities_fn_ident = Ident::new(&capabilities_fn_string, name_lowercase.span());
    let static_name_string = format!("{}_NAME", name_caps);
    let static_version_string = format!("{}_VERSION", name_caps);
    let static_name_ident = Ident::new(&static_name_string, name_caps.span());
    let static_version_ident = Ident::new(&static_version_string, name_caps.span());

    // TODO: set_option
    // TODO: tracer fn?
    // TODO: auto-initialize user defined params with default trait
    let quoted = quote! {
        #[no_mangle]
        extern "C" fn #fn_ident() -> *const ::evmc_sys::evmc_instance {
            let new_instance = ::evmc_sys::evmc_instance {
                abi_version: ::evmc_sys::EVMC_ABI_VERSION as i32,
                destroy: Some(/*some destroy fn*/),
                execute: Some(/*some execution fn*/),
                get_capabilities: Some(#capabilities_fn_ident),
                set_option: None,
                set_tracer: None,
                name: ::std::ffi::CString::new(#static_name_ident).expect("Failed to build VM name string").into_raw() as *const i8,
                version: ::std::ffi::CString::new(#static_version_ident).expect("Failed to build VM version string").into_raw() as *const i8,
            };

            __EvmcContainer::new(new_instance).into_ffi_pointer() as *const ::evmc_sys::evmc_instance
        }
    };

    unimplemented!()
}

/// Builds a callback to dispose of the VM instance
fn build_destroy_fn(name_lowercase: &String, type_name: &String) -> TokenStream {
    let fn_ident_string = format!("{}_destroy", name_lowercase);
    let fn_ident = Ident::new(&fn_ident_string, name_lowercase.span());
    let type_ident = Ident::new(type_name, type_name.span());

    let quoted = quote! {
        extern "C" fn #fn_ident(instance: *mut ::evmc_sys::evmc_instance) {
            Box::new(__EvmcContainer<#type_ident>::from_ffi_pointer(instance));
        }
    };

    quoted.into()
}

/// Takes a capabilities flag and builds the evmc_get_capabilities callback.
fn build_capabilities_fn(
    name_lowercase: &String,
    type_name: &String,
    capabilities: u32,
) -> TokenStream {
    // Could avoid using a special name and just use get_capabilities.
    let concatenated = format!("{}_get_capabilities", name_lowercase);
    let capabilities_fn_ident = Ident::new(&concatenated, name_lowercase.span());
    let capabilities_literal =
        LitInt::new(capabilities as u64, IntSuffix::U32, capabilities.span());

    let quoted = quote! {
        unsafe extern "C" fn #capabilities_fn_ident(instance: *mut ::evmc_sys::evmc_instance) -> ::evmc_sys::evmc_capabilities_flagset {
            #capabilities_literal
        }
    };
    // Convert to the old-school token stream, since this will be combined with other generated
    // streams to form a full EVMC impl
    quoted.into()
}

/// Generate tokens for the static data associated with an EVMC VM.
fn build_static_data(
    name_stylized: &String,
    name_allcaps: &String,
    version: &String,
) -> TokenStream {
    // Stitch together the VM name and the suffix _NAME
    let concatenated_name = format!("{}_NAME", name_allcaps);
    let concatenated_version = format!("{}_VERSION", name_allcaps);
    let static_name_ident = Ident::new(&concatenated_name, name_allcaps.span());
    let static_version_ident = Ident::new(&concatenated_version, name_allcaps.span());

    // Turn the stylized VM name and version into string literals.
    // FIXME: Not sure if the span of name.as_str() is the same as that of name.
    let stylized_name_literal = LitStr::new(name_stylized.as_str(), name_stylized.as_str().span());
    let version_literal = LitStr::new(version, version.span());

    let quoted = quote! {
        static #static_name_ident: &'static str = #stylized_name_literal;
        static #static_version_ident: &'static str = #version_literal;
    };
    // Convert to the old-school token stream, since this will be combined with other generated
    // streams to form a full EVMC impl
    quoted.into()
}

/// Generates a definition and impl for a struct which contains the EVMC instance needed by FFI,
/// and the user-defined VM.
fn build_vm_container() -> TokenStream {
    let quoted = quote! {
        struct __EvmcContainer<T: ::evmc_vm::EvmcVm + Sized> {
            instance: ::evmc_sys::evmc_instance,
            vm: T,
        }

        impl<T: ::evmc_vm::EvmcVm + Sized>  __EvmcContainer<T> {
            pub fn new(_instance: ::evmc_sys::evmc_instance) -> Self {
                T {
                    instance: _instance,
                    vm: T::init(),
                }
            }

            pub unsafe fn from_ffi_pointer(instance: *mut ::evmc_sys::evmc_instance) -> Self {
                if let Some(container) = (instance as *mut __EvmcContainer).as_ref() {
                    let ret = container.clone();
                    Box::from_raw(instance);
                    ret
                } else {
                    panic!("instance is null");
                }
            }

            pub unsafe fn into_ffi_pointer(mut self) -> *mut ::evmc_sys::evmc_instance {
                Box::into_raw(Box::new(self)) as *mut ::evmc_sys::evmc_instance
            }

            pub fn execute(&self, code: &[u8], context: &::evmc_vm::ExecutionContext) -> ::evmc_vm::ExecutionResult {
                self.vm.execute(code, context)
            }
        }
    };

    quoted.into()
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
