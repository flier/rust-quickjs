//! `qjs` is an experimental Rust binding for the QuickJS Javascript Engine
//!
//! # Examples
//!
//! `qjs` macro can evalute the Javascript code in an anonymouse context.
//!
//! ```
//! use qjs::qjs;
//!
//! let v: i32 = qjs!(1+2).unwrap().unwrap();
//!
//! assert_eq!(v, 3);
//! ```
//!
//! `qjs` macro can also convert a Javascript closure to a rust function.
//!
//! ```
//! use qjs::qjs;
//!
//! let f = qjs!{ (name: &str) -> String => { return "hello " + name; } };
//! let s: String = f("world").unwrap().unwrap();
//!
//! assert_eq!(s, "hello world");
//! ```
//!
//! Variable interpolation is done with `#var` (similar to `$var` in `macro_rules!` macros).
//! This grabs the var variable that is currently in scope and inserts it in that location in the output tokens.
//!
//! ```
//! use qjs::qjs;
//!
//! # let _ = pretty_env_logger::try_init();
//!
//! let f = |name| qjs!{ "hello " + #name };
//! let s: String = f("world").unwrap().unwrap();
//!
//! assert_eq!(s, "hello world");
//! ```
//!
//! The primitive types, including `bool`, `i32`, `i64`, `u64`, `f64`, `String` etc,
//! and other type which implements `NewValue` trait could be used in the variable interpolation.
//!
//! The function which parameters implements `ExtractValue` trait and output type implements `NewValue` trait
//! can also be used in the variable interpolation.
//!
//! ```
//! use qjs::qjs;
//!
//! fn hello(name: String) -> String {
//!     format!("hello {}", name)
//! }
//!
//! let hello: fn(String) -> String = hello;
//! //let s: String = qjs!{ #hello ("world") }.unwrap().unwrap();
//!
//! // assert_eq!(s, "hello world");
//! ```
#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate foreign_types;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate cstr;

pub use qjs_sys as ffi;

use proc_macro_hack::proc_macro_hack;
#[proc_macro_hack]
pub use qjs_derive::qjs;

#[macro_use]
mod macros;
mod arraybuf;
mod atom;
mod cfunc;
mod class;
mod context;
mod error;
mod eval;
mod func;
mod handle;
mod job;
mod module;
mod precompile;
mod prop;
mod runtime;
#[cfg(feature = "stdlib")]
mod stdlib;
mod userdata;
mod value;

pub use arraybuf::{ArrayBuffer, SharedArrayBuffer};
pub use atom::{Atom, NewAtom};
pub use cfunc::{CFunc, CFunction, UnsafeCFunction, UnsafeCFunctionData, UnsafeCFunctionMagic};
pub use class::{ClassDef, ClassId};
pub use context::{Builder as ContextBuilder, Context, ContextRef};
pub use error::ErrorKind;
pub use eval::{eval, load_file, Eval, Source};
pub use func::Args;
pub use handle::{Bindable, Local, Unbindable};
pub use job::JobFunc;
pub use module::{detect_module, ModuleDef, ModuleInitFunc, ModuleLoaderFunc, ModuleNormalizeFunc};
pub use precompile::{ReadObj, WriteObj};
pub use prop::{
    DefinePropertyGetSet, DefinePropertyValue, DeleteProperty, Descriptor as PropertyDescriptor,
    GetProperty, HasProperty, Names as PropertyNames, Prop, SetProperty,
};
pub use runtime::{Interrupt, InterruptHandler, MallocFunctions, MemoryUsage, Runtime, RuntimeRef};
pub use value::{
    ExtractValue, NewValue, Value, EXCEPTION, FALSE, NAN, NULL, TRUE, UNDEFINED, UNINITIALIZED,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

lazy_static! {
    pub static ref LONG_VERSION: String = format!(
        "{} (quickjs {}{})",
        VERSION,
        ffi::VERSION.trim(),
        if cfg!(feature = "bignum") {
            " with BigNum"
        } else {
            ""
        },
    );
}
