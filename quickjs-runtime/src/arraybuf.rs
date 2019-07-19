use std::ptr;

use foreign_types::ForeignTypeRef;

use crate::{ffi, value::FALSE, ContextRef, Local, Value};

impl ContextRef {
    pub fn new_array_buffer(&self, buf: &mut [u8]) -> Local<Value> {
        self.bind(unsafe {
            ffi::JS_NewArrayBuffer(
                self.as_ptr(),
                buf.as_mut_ptr(),
                buf.len(),
                None,
                ptr::null_mut(),
                FALSE,
            )
        })
    }
}
