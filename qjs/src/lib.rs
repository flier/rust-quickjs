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
mod precompile;
mod prop;
mod runtime;
mod userdata;
mod value;

pub use arraybuf::{ArrayBuffer, SharedArrayBuffer};
pub use atom::NewAtom;
pub use cfunc::{CFunc, CFunction, UnsafeCFunction, UnsafeCFunctionData, UnsafeCFunctionMagic};
pub use class::{ClassDef, ClassId};
pub use context::{Builder, Context, ContextRef};
pub use error::ErrorKind;
pub use eval::Eval;
pub use ffi::JSMemoryUsage as MemoryUsage;
pub use func::Args;
pub use handle::{Bindable, Local, Unbindable};
pub use job::JobFunc;
pub use precompile::{ReadObj, WriteObj};
pub use prop::{
    DefinePropertyGetSet, DefinePropertyValue, DeleteProperty, GetProperty, HasProperty, Prop,
    SetProperty,
};
pub use runtime::{Interrupt, InterruptHandler, Runtime, RuntimeRef};
pub use value::{NewValue, Value};
