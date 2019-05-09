#![recursion_limit = "128"]

extern crate proc_macro;

use heck::ShoutySnakeCase;
use heck::SnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::spanned::Spanned;
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
        meta.nested.len() == 1,
        "Incorrect number of meta-items passed to evmc_declare_vm."
    );

    // Extract the name and capabilities from the attribute's key-value item.
    let (vm_name_stylized, vm_capabilities) = match &meta.nested[0] {
        NestedMeta::Meta(m) => {
            // Try to form a string from the identifier if a meta-item was supplied.
            if let Meta::NameValue(pair) = m {
                let name = pair.ident.to_string();
                let capabilities = match pair.lit {
                    Lit::Str(ref s) => match s.value().as_str() {
                        "evm1" => 0x1u32,
                        "ewasm" => 0x1u32 << 1u32,
                        _ => panic!("Invalid capabilities specifier. Use 'evm1' or 'ewasm'."),
                    },
                    _ => panic!("Argument 1 is not a valid string literal."),
                };
                (name, capabilities)
            } else {
                panic!("Argument passed to evmc_declare_vm is not a name-value item.")
            }
        }
        _ => panic!(
            "Argument passed to evmc_declare_vm is of incorrect type. Expected a name-value item."
        ),
    };

    // Get the VM version from the crate version.
    let vm_version_string = env!("CARGO_PKG_VERSION").to_string();

    // Get all the tokens from the respective helpers.
    let static_data_tokens =
        build_static_data(&vm_name_stylized, &vm_name_allcaps, &vm_version_string);
    let capabilities_tokens = build_capabilities_fn(&vm_name_lowercase, vm_capabilities);
    let create_tokens = build_create_fn(&vm_name_lowercase, &vm_name_allcaps, &vm_type_name);
    let destroy_tokens = build_destroy_fn(&vm_name_lowercase, &vm_type_name);
    let execute_tokens = build_execute_fn(&vm_name_lowercase, &vm_type_name);

    let quoted = quote! {
        #input
        #static_data_tokens
        #capabilities_tokens
        #create_tokens
        #destroy_tokens
        #execute_tokens
    };

    quoted.into()
}

fn build_execute_fn(name_lowercase: &String, type_name: &String) -> proc_macro2::TokenStream {
    let fn_name_string = format!("{}_execute", name_lowercase);
    let fn_name_ident = Ident::new(&fn_name_string, name_lowercase.span());
    let type_name_ident = Ident::new(type_name, type_name.span());

    quote! {
        extern "C" fn #fn_name_ident(
            instance: *mut ::evmc_sys::evmc_instance,
            context: *mut ::evmc_sys::evmc_context,
            rev: ::evmc_sys::evmc_revision,
            msg: *const ::evmc_sys::evmc_message,
            code: *const u8,
            code_size: usize
        ) -> ::evmc_sys::evmc_result
        {
            assert!(!msg.is_null());
            assert!(!context.is_null());
            assert!(!instance.is_null());
            assert!(!code.is_null());

            let execution_context = ::evmc_vm::ExecutionContext::new(
                msg.as_ref().expect("EVMC message is null"),
                context.as_mut().expect("EVMC context is null")
            );

            let code_ref: &[u8] = unsafe {
                ::std::slice::from_raw_parts(code, code_size);
            }

            let container = EvmcContainer::from_ffi_pointer::<#type_name_ident>(instance);

            let result = container.execute(code_ref, &execution_context);

            EvmcContainer::into_ffi_pointer(container);

            result.into()
        }
    }
}

/// Takes an identifier and struct definition, builds an evmc_create_* function for FFI.
fn build_create_fn(
    name_lowercase: &String,
    name_caps: &String,
    type_name: &String,
) -> proc_macro2::TokenStream {
    let fn_name = format!("evmc_create_{}", name_lowercase);
    let fn_ident = Ident::new(&fn_name, name_lowercase.span());
    let type_ident = Ident::new(type_name, type_name.span());

    // TODO: reduce code duplication here.
    let capabilities_fn_string = format!("{}_get_capabilities", name_lowercase);
    let capabilities_fn_ident = Ident::new(&capabilities_fn_string, name_lowercase.span());
    let destroy_fn_string = format!("{}_destroy", name_lowercase);
    let destroy_fn_ident = Ident::new(&destroy_fn_string, name_lowercase.span());
    let static_name_string = format!("{}_NAME", name_caps);
    let static_version_string = format!("{}_VERSION", name_caps);
    let static_name_ident = Ident::new(&static_name_string, name_caps.span());
    let static_version_ident = Ident::new(&static_version_string, name_caps.span());
    let execute_fn_string = format!("{}_execute", name_lowercase);
    let execute_fn_ident = Ident::new(&execute_fn_string, name_lowercase.span());

    quote! {
        #[no_mangle]
        extern "C" fn #fn_ident() -> *const ::evmc_sys::evmc_instance {
            let new_instance = ::evmc_sys::evmc_instance {
                abi_version: ::evmc_sys::EVMC_ABI_VERSION as i32,
                destroy: Some(#destroy_fn_ident),
                execute: Some(#execute_fn_ident),
                get_capabilities: Some(#capabilities_fn_ident),
                set_option: None,
                set_tracer: None,
                name: ::std::ffi::CString::new(#static_name_ident).expect("Failed to build VM name string").into_raw() as *const i8,
                version: ::std::ffi::CString::new(#static_version_ident).expect("Failed to build VM version string").into_raw() as *const i8,
            };

            EvmcContainer::into_ffi_pointer(Box::new(EvmcContainer::new::<#type_ident>(new_instance)))
        }
    }
}

/// Builds a callback to dispose of the VM instance
fn build_destroy_fn(name_lowercase: &String, type_name: &String) -> proc_macro2::TokenStream {
    let fn_ident_string = format!("{}_destroy", name_lowercase);
    let fn_ident = Ident::new(&fn_ident_string, name_lowercase.span());
    let type_ident = Ident::new(type_name, type_name.span());

    quote! {
        extern "C" fn #fn_ident(instance: *mut ::evmc_sys::evmc_instance) {
            EvmcContainer::from_ffi_pointer::<#type_ident>(instance);
        }
    }
}

/// Takes a capabilities flag and builds the evmc_get_capabilities callback.
fn build_capabilities_fn(name_lowercase: &String, capabilities: u32) -> proc_macro2::TokenStream {
    // Could avoid using a special name and just use get_capabilities.
    let concatenated = format!("{}_get_capabilities", name_lowercase);
    let capabilities_fn_ident = Ident::new(&concatenated, name_lowercase.span());
    let capabilities_literal =
        LitInt::new(capabilities as u64, IntSuffix::U32, capabilities.span());

    quote! {
        unsafe extern "C" fn #capabilities_fn_ident(instance: *mut ::evmc_sys::evmc_instance) -> ::evmc_sys::evmc_capabilities_flagset {
            #capabilities_literal
        }
    }
}

/// Generate tokens for the static data associated with an EVMC VM.
fn build_static_data(
    name_stylized: &String,
    name_allcaps: &String,
    version: &String,
) -> proc_macro2::TokenStream {
    // Stitch together the VM name and the suffix _NAME
    let concatenated_name = format!("{}_NAME", name_allcaps);
    let concatenated_version = format!("{}_VERSION", name_allcaps);
    let static_name_ident = Ident::new(&concatenated_name, name_allcaps.span());
    let static_version_ident = Ident::new(&concatenated_version, name_allcaps.span());

    // Turn the stylized VM name and version into string literals.
    // FIXME: Not sure if the span of name.as_str() is the same as that of name.
    let stylized_name_literal = LitStr::new(name_stylized.as_str(), name_stylized.as_str().span());
    let version_literal = LitStr::new(version, version.span());

    quote! {
        static #static_name_ident: &'static str = #stylized_name_literal;
        static #static_version_ident: &'static str = #version_literal;
    }
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
