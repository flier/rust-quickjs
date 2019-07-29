extern crate proc_macro;

use proc_macro::TokenStream;

#[proc_macro]
pub fn js_with_context(input: TokenStream) -> TokenStream {
    let mut vars = vec![];
    let mut interpolated = qjs_derive_support::interpolate(proc_macro2::TokenStream::from(input), &mut vars).unwrap();

    interpolated.into()
}
