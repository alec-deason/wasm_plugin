#![doc(html_root_url = "https://docs.rs/wasm_plugin_guest_derive/0.1.1")]
#![deny(missing_docs)]

//! This crate provides attribute macros used by [wasm_plugin_guest](https://crates.io/crates/wasm_plugin_guest)

use proc_macro::TokenStream;
extern crate proc_macro;
use syn;
use quote::{quote, format_ident};

/// Builds an extern function which will handle serializing and
/// deserializing of arguments and return values of the function it is applied
/// to. The function must take a single deserializable argument and return
/// a serializable value.
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
    let gen;
    if ast.sig.inputs.is_empty() {
        gen = quote! {
            #[no_mangle]
            pub extern "C" fn #remote_name() -> i32 {
                wasm_plugin_guest::write_message(&#name())
            }
            #ast
        };
    } else {
        gen = quote! {
            #[no_mangle]
            pub extern "C" fn #remote_name() -> i32 {
                let message = wasm_plugin_guest::read_message();

                wasm_plugin_guest::write_message(&#name(message))
            }
            #ast
        };
    }
    gen.into()
}
