#![doc(html_root_url = "https://docs.rs/wasm_plugin_guest_derive/0.1.5")]
#![deny(missing_docs)]

//! This crate provides attribute macros used by [wasm_plugin_guest](https://crates.io/crates/wasm_plugin_guest)

use proc_macro::TokenStream;
extern crate proc_macro;
use quote::{format_ident, quote};

/// Builds an extern function which will handle serializing and
/// deserializing of arguments and return values of the function it is applied
/// to. The function must take only deserializable arguments and return
/// a serializable result.
///
/// The name of the exported function will be mangled to
/// `wasm_plugin_exported__ORIGINAL_NAME` The exported function is only
/// intended to be used by [wasm_plugin_host](https://crates.io/crates/wasm_plugin_host)
#[proc_macro_attribute]
pub fn export_function(_args: TokenStream, input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::ItemFn);

    impl_function_export(&ast)
}

fn impl_function_export(ast: &syn::ItemFn) -> TokenStream {
    let name = &ast.sig.ident;
    let remote_name = format_ident!("wasm_plugin_exported__{}", name);
    let gen = if ast.sig.inputs.is_empty() {
        quote! {
            #[no_mangle]
            pub extern "C" fn #remote_name() -> u64 {
                let (ptr, len) = wasm_plugin_guest::write_message(&#name());
                let mut fat = wasm_plugin_guest::FatPointer(0);
                fat.set_ptr(ptr as u32);
                fat.set_len(len as u32);
                fat.0
            }
        }
    } else {
        let mut argument_types = quote!();
        let mut call = quote!();
        if ast.sig.inputs.len() == 1 {
            if let syn::FnArg::Typed(t) = &ast.sig.inputs[0] {
                let ty = &t.ty;
                argument_types = quote!(#ty);
            } else {
                panic!();
            }
            call = quote!(message);
        } else {
            for (i, arg) in ast.sig.inputs.iter().enumerate() {
                let i = syn::Index::from(i);
                call = quote!(#call message.#i,);
                if let syn::FnArg::Typed(t) = arg {
                    let ty = &t.ty;
                    argument_types = quote!(#argument_types #ty,);
                } else {
                    panic!();
                }
            }
            argument_types = quote! { (#argument_types) };
        }
        quote! {
            #[no_mangle]
            pub extern "C" fn #remote_name(ptr: u32, len: u32) -> u64 {
                let message:#argument_types = wasm_plugin_guest::read_message(ptr as usize, len as usize);

                let (ptr, len) = wasm_plugin_guest::write_message(&#name(#call));
                let mut fat = wasm_plugin_guest::FatPointer(0);
                fat.set_ptr(ptr as u32);
                fat.set_len(len as u32);
                fat.0
            }
        }
    };
    quote!(#gen #ast).into()
}

struct FnImports {
    functions: Vec<syn::Signature>,
}

impl syn::parse::Parse for FnImports {
    fn parse(input: syn::parse::ParseStream) -> syn::parse::Result<Self> {
        let mut functions = vec![];
        while let Ok(f) = input.parse::<syn::Signature>() {
            functions.push(f);
            input.parse::<syn::Token![;]>()?;
        }
        Ok(FnImports { functions })
    }
}

/// Import functions from the host program. The function's arguments an return
/// type must all be serializable. Several functions can be imported at once
/// by listing their signatures seperated by `;`
///
/// ```rust
/// import_functions! {
///     fn my_function();
///     fn my_other_function(s: String) -> Vec<u8>;
/// }
/// ```
/// The macro creates a safe wrapper function using the given name which can
/// be called in the plugin code. The actual imported function, which normal
/// code will never need to access, will have a mangled name:
/// `wasm_plugin_imported__ORIGINAL_NAME` and is only intended to be called by
/// by host code using [wasm_plugin_host](https://crates.io/crates/wasm_plugin_host)
#[proc_macro]
pub fn import_functions(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as FnImports);
    impl_import_functions(&ast)
}

fn impl_import_functions(ast: &FnImports) -> TokenStream {
    let mut remote_fns = quote!();
    let mut local_fns = quote!();
    for f in ast.functions.iter().cloned() {
        let remote_name = format_ident!("wasm_plugin_imported__{}", f.ident);
        let gen = if f.inputs.is_empty() {
            match &f.output {
                syn::ReturnType::Default => {
                    quote! {
                        #f {
                            unsafe {
                                #remote_name();
                            }
                        }
                    }
                }
                syn::ReturnType::Type(_, ty) => {
                    quote! {
                        #f {
                            let fat_ptr = unsafe {
                                #remote_name()
                            };
                            let fat_ptr = wasm_plugin_guest::FatPointer(fat_ptr);
                            let message:(#ty) = wasm_plugin_guest::read_message(fat_ptr.ptr() as usize, fat_ptr.len() as usize);
                            message
                        }
                    }
                }
            }
        } else {
            let mut message = quote!();
            if f.inputs.len() == 1 {
                if let syn::FnArg::Typed(syn::PatType { pat: p, .. }) = &f.inputs[0] {
                    if let syn::Pat::Ident(i) = p.as_ref() {
                        message = quote!(#i);
                    } else {
                        unimplemented!("unsupported argument type");
                    }
                } else {
                    unimplemented!("unsupported argument type");
                }
            } else {
                for item in &f.inputs {
                    if let syn::FnArg::Typed(syn::PatType { pat: p, .. }) = item {
                        if let syn::Pat::Ident(i) = p.as_ref() {
                            message = quote!(#i,);
                        } else {
                            unimplemented!("unsupported argument type");
                        }
                    } else {
                        unimplemented!("unsupported argument type");
                    }
                }
                message = quote!((#message));
            }
            match &f.output {
                syn::ReturnType::Default => {
                    quote! {
                        #f {
                            let (ptr, len) = wasm_plugin_guest::write_message(&#message);
                            unsafe {
                                #remote_name(ptr as u32, len as u32);
                            }
                        }
                    }
                }
                syn::ReturnType::Type(_, ty) => {
                    quote! {
                        #f {
                            let (ptr, len) = wasm_plugin_guest::write_message(&(#message));
                            let fat_ptr = unsafe {
                                #remote_name(ptr as u32, len as u32)
                            };
                            let fat_ptr = wasm_plugin_guest::FatPointer(fat_ptr);
                            let message:(#ty) = wasm_plugin_guest::read_message(fat_ptr.ptr() as usize, fat_ptr.len() as usize);
                            message
                        }
                    }
                }
            }
        };
        local_fns = quote! {
            #local_fns
            #gen
        };
        let gen = if f.inputs.is_empty() {
            match &f.output {
                syn::ReturnType::Default => {
                    quote! {
                        fn #remote_name();
                    }
                }
                syn::ReturnType::Type(_, _) => {
                    quote! {
                        fn #remote_name() -> u64;
                    }
                }
            }
        } else {
            match &f.output {
                syn::ReturnType::Default => {
                    quote! {
                        fn #remote_name(ptr: u32, len: u32);
                    }
                }
                syn::ReturnType::Type(_, _) => {
                    quote! {
                        fn #remote_name(ptr: u32, len: u32) -> u64;
                    }
                }
            }
        };
        remote_fns = quote!(#remote_fns #gen);
    }
    let exports = quote! {
        #local_fns
        extern "C" {
            #remote_fns
        }
    };
    exports.into()
}
