extern crate proc_macro;

use std::sync::Once;

use proc_macro::TokenStream;
use proc_macro_hack::proc_macro_hack;

#[proc_macro_hack]
pub fn qjs(input: TokenStream) -> TokenStream {
    LOG_INIT.call_once(log_init);

    qjs_derive_support::qjs(proc_macro2::TokenStream::from(input))
        .unwrap()
        .into()
}

const ERROR: usize = 0;
const WARN: usize = 1;
const INFO: usize = 2;
const DEBUG: usize = 3;
const TRACE: usize = 4;

static LOG_INIT: Once = Once::new();

fn log_init() {
    stderrlog::new()
        .color(stderrlog::ColorChoice::Never)
        .verbosity(
            std::env::var("RUST_LOG")
                .ok()
                .and_then(|s| {
                    s.split(",")
                        .flat_map(|s| match s.to_lowercase().trim() {
                            "error" => Some(ERROR),
                            "warn" => Some(WARN),
                            "info" => Some(INFO),
                            "debug" => Some(DEBUG),
                            "trace" => Some(TRACE),
                            _ => None,
                        })
                        .next()
                })
                .unwrap_or(TRACE),
        )
        .init()
        .unwrap();
}
