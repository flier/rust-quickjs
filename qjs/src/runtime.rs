use std::mem;
use std::os::raw::{c_int, c_void};
use std::ptr;

use foreign_types::{ForeignType, ForeignTypeRef};

use crate::{
    ffi,
    value::{FALSE, TRUE},
    Value,
};

const NO_LIMIT: isize = -1;

foreign_type! {
    /// `Runtime` represents a Javascript runtime corresponding to an object heap.
    ///
    /// Several runtimes can exist at the same time but they cannot exchange objects.
    /// Inside a given runtime, no multi-threading is supported.
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
        let runtime = unsafe { Runtime::from_ptr(ffi::JS_NewRuntime()) };

        runtime.register_userdata_class();

        runtime
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

    pub fn is_live_object(&self, obj: &Value) -> bool {
        unsafe { ffi::JS_IsLiveObject(self.as_ptr(), obj.raw()) != FALSE }
    }

    pub fn is_gc_swap(&self) -> bool {
        unsafe { ffi::JS_IsInGCSweep(self.as_ptr()) != FALSE }
    }

    pub fn memory_usage(&self) -> ffi::JSMemoryUsage {
        unsafe {
            let mut usage: ffi::JSMemoryUsage = mem::zeroed();

            ffi::JS_ComputeMemoryUsage(self.as_ptr(), &mut usage);

            usage
        }
    }

    pub fn set_interrupt_handler(&self, handler: InterruptHandler) {
        unsafe {
            if let Some(func) = handler {
                unsafe extern "C" fn stub(rt: *mut ffi::JSRuntime, opaque: *mut c_void) -> c_int {
                    let rt = RuntimeRef::from_ptr(rt);
                    let func: fn(rt: &RuntimeRef) -> Interrupt = *(opaque as *mut _);

                    match func(rt) {
                        Interrupt::Break => TRUE,
                        Interrupt::Continue => FALSE,
                    }
                }

                ffi::JS_SetInterruptHandler(self.as_ptr(), Some(stub), func as *mut _)
            } else {
                ffi::JS_SetInterruptHandler(self.as_ptr(), None, ptr::null_mut())
            }
        }
    }
}

pub enum Interrupt {
    Break,
    Continue,
}
pub type InterruptHandler = Option<fn(rt: &RuntimeRef) -> Interrupt>;

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
