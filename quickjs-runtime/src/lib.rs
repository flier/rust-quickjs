#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate foreign_types;

use quickjs_sys as ffi;

#[macro_use]
mod macros;
mod atom;
mod context;
mod error;
mod eval;
mod runtime;
mod value;

pub use atom::{Atom, NewAtom};
pub use context::{Context, ContextRef};
pub use error::ErrorKind;
pub use eval::Eval;
pub use ffi::JSMemoryUsage as MemoryUsage;
pub use runtime::{Runtime, RuntimeRef};
pub use value::{CStrBuf, NewValue, Value};
