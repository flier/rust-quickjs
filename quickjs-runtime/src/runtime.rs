use std::mem;

use foreign_types::{ForeignType, ForeignTypeRef};

use crate::ffi;

const NO_LIMIT: isize = -1;

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
    pub fn set_memory_limit(&self, limit: Option<usize>) -> &Self {
        trace!("{:?} set memory limit to {:?}", self, limit);

        unsafe {
            ffi::JS_SetMemoryLimit(self.as_ptr(), limit.unwrap_or(NO_LIMIT as usize));
        }
        self
    }

    /// Set the GC threshold to a given `Runtime`.
    pub fn set_gc_threshold(&self, gc_threshold: usize) -> &Self {
        trace!("{:?} set GC threshold to {}", self, gc_threshold);

        unsafe {
            ffi::JS_SetGCThreshold(self.as_ptr(), gc_threshold);
        }
        self
    }

    /// Force to run GC to a given `Runtime`.
    pub fn run_gc(&self) {
        trace!("{:?} run GC", self);

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
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();

        let usage = rt.memory_usage();
        debug!("{:#?}", usage);
        assert!(usage.memory_used_size > 0);

        let ctxt = Context::new(&rt);

        assert_eq!(&rt, ctxt.runtime());
        let usage2 = rt.memory_usage();
        assert!(usage2.memory_used_size > usage.memory_used_size);

        mem::drop(ctxt);

        let usage3 = rt.memory_usage();
        assert!(usage3.memory_used_size > usage.memory_used_size);

        rt.run_gc();

        let usage4 = rt.memory_usage();
        assert!(usage4.memory_used_size < usage3.memory_used_size);
        assert!(usage4.memory_used_size > usage.memory_used_size);
    }
}
