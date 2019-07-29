extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro]
pub fn js(input: TokenStream) -> TokenStream {
    qjs_derive_support::js(proc_macro2::TokenStream::from(input))
        .unwrap()
        .into()
}
