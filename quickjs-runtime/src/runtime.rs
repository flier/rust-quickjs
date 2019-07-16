use std::mem;

use foreign_types::{ForeignType, ForeignTypeRef};

use crate::ffi;

foreign_type! {
    pub type Runtime : Send {
        type CType = ffi::JSRuntime;

        fn drop = ffi::JS_FreeRuntime;
    }
}

impl_foreign_type!(Runtime, RuntimeRef);

impl Default for Runtime {
    fn default() -> Self {
        Runtime::new()
    }
}

impl Runtime {
    /// Construct a new `Runtime`.
    pub fn new() -> Self {
        unsafe { Runtime::from_ptr(ffi::JS_NewRuntime()) }
    }
}

impl RuntimeRef {
    /// Set a global memory allocation limit to a given `Runtime`.
    pub fn set_memory_limit(&self, limit: usize) -> &Self {
        unsafe {
            ffi::JS_SetMemoryLimit(self.as_ptr(), limit);
        }
        self
    }

    /// Set the GC threshold to a given `Runtime`.
    pub fn set_gc_threshold(&self, gc_threshold: usize) -> &Self {
        unsafe {
            ffi::JS_SetGCThreshold(self.as_ptr(), gc_threshold);
        }
        self
    }

    /// Force to run GC to a given `Runtime`.
    pub fn run_gc(&self) {
        unsafe { ffi::JS_RunGC(self.as_ptr()) }
    }

    pub fn memory_usage(&self) -> ffi::JSMemoryUsage {
        unsafe {
            let mut usage: ffi::JSMemoryUsage = mem::zeroed();

            ffi::JS_ComputeMemoryUsage(self.as_ptr(), &mut usage);

            usage
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Context;

    use super::*;

    #[test]
    fn runtime() {
        let rt = Runtime::new();

        let usage = rt.memory_usage();

        assert!(usage.memory_used_size > 0);

        let ctxt = Context::new(&rt);

        assert_eq!(&rt, ctxt.runtime());

        rt.run_gc();
    }
}
