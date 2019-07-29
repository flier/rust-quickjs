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

#[allow(unused_imports)]
#[macro_use]
extern crate qjs_derive;
#[doc(hidden)]
pub use qjs_derive::*;

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
pub use eval::{eval, Eval, EvalBinary, Source};
pub use func::Args;
pub use handle::{Bindable, Local, Unbindable};
pub use job::JobFunc;
pub use module::{ModuleDef, ModuleInitFunc, ModuleLoaderFunc, ModuleNormalizeFunc};
pub use precompile::{ReadObj, WriteObj};
pub use prop::{
    DefinePropertyGetSet, DefinePropertyValue, DeleteProperty, GetProperty, HasProperty, Prop,
    SetProperty,
};
pub use runtime::{Interrupt, InterruptHandler, MallocFunctions, MemoryUsage, Runtime, RuntimeRef};
pub use value::{ExtractValue, NewValue, Value};

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
