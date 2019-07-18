#[macro_use]
extern crate log;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate foreign_types;
#[macro_use]
extern crate cstr;

use quickjs_sys as ffi;

#[macro_use]
mod macros;
mod atom;
mod context;
mod error;
mod eval;
mod handle;
mod prop;
mod runtime;
mod value;

pub use atom::{Atom, NewAtom};
pub use context::{Context, ContextRef};
pub use error::ErrorKind;
pub use eval::Eval;
pub use ffi::JSMemoryUsage as MemoryUsage;
pub use handle::Local;
pub use runtime::{Runtime, RuntimeRef};
pub use value::{CStrBuf, NewValue, Value, FALSE, TRUE};
