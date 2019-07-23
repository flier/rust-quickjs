use std::ptr;

use failure::Error;
use foreign_types::ForeignTypeRef;

use crate::{ffi, value::FALSE, Args, ContextRef, RuntimeRef};

pub use ffi::JSJobFunc as JobFunc;

impl RuntimeRef {
    pub fn is_job_pending(&self) -> bool {
        unsafe { ffi::JS_IsJobPending(self.as_ptr()) != FALSE }
    }

    pub fn execute_pending_job(&self) -> Result<Option<&ContextRef>, Error> {
        let mut ctxt = ptr::null_mut();

        let ret = unsafe { ffi::JS_ExecutePendingJob(self.as_ptr(), &mut ctxt) };

        if ret == FALSE {
            Ok(None)
        } else {
            let ctxt = unsafe { ContextRef::from_ptr(ctxt) };

            ctxt.check_bool(ret).map(|_| Some(ctxt))
        }
    }
}

impl ContextRef {
    pub fn enqueue_job<T: Args>(&self, job_func: JobFunc, args: T) -> Result<(), Error> {
        let args = args.into_values(self);
        let args = args.as_ref();

        self.check_error(unsafe {
            ffi::JS_EnqueueJob(
                self.as_ptr(),
                job_func,
                args.len() as i32,
                args.as_ptr() as *mut _,
            )
        })
        .map(|_| ())
    }
}
