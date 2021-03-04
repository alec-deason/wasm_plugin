#![doc(html_root_url = "https://docs.rs/wasm_plugin_guest_derive/0.1.2")]
#![deny(missing_docs)]

//! This crate provides attribute macros used by [wasm_plugin_guest](https://crates.io/crates/wasm_plugin_guest)

use proc_macro::TokenStream;
extern crate proc_macro;
use quote::{format_ident, quote};
use syn;

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
            pub extern "C" fn #remote_name() -> i32 {
                wasm_plugin_guest::write_message(&#name())
            }
        }
    } else {
        let mut argument_types = quote!();
        let mut call = quote!();
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
        quote! {
            #[no_mangle]
            pub extern "C" fn #remote_name() -> i32 {
                let message:(#argument_types) = wasm_plugin_guest::read_message();

                wasm_plugin_guest::write_message(&#name(#call))
            }
        }
    };
    quote!(#gen #ast).into()
}
