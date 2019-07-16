#[macro_use]
extern crate log;
#[macro_use]
extern crate foreign_types;

use quickjs_sys as ffi;

#[macro_use]
mod macros;
mod context;
mod runtime;

pub use context::{Context, ContextRef};
pub use ffi::JSMemoryUsage as MemoryUsage;
pub use runtime::{Runtime, RuntimeRef};
