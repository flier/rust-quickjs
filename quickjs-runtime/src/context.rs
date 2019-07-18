use std::ptr::{null_mut, NonNull};

use foreign_types::{ForeignType, ForeignTypeRef};

use crate::{ffi, RuntimeRef, Value};

foreign_type! {
    /// `Context` represents a Javascript context (or Realm).
    ///
    /// Each `Context` has its own global objects and system objects.
    /// There can be several `Contexts` per `Runtime` and they can share objects,
    /// similary to frames of the same origin sharing Javascript objects in a web browser.
    pub type Context : Send {
        type CType = ffi::JSContext;

        fn drop = ffi::JS_FreeContext;
    }
}

impl_foreign_type!(Context, ContextRef);

impl Context {
    pub fn new(runtime: &RuntimeRef) -> Context {
        unsafe { Context::from_ptr(ffi::JS_NewContext(runtime.as_ptr())) }
    }
}

impl ContextRef {
    pub fn runtime(&self) -> &RuntimeRef {
        unsafe { RuntimeRef::from_ptr(ffi::JS_GetRuntime(self.as_ptr())) }
    }

    pub fn userdata<T>(&self) -> Option<NonNull<T>> {
        NonNull::new(unsafe { ffi::JS_GetContextOpaque(self.as_ptr()) } as *mut _)
    }

    pub fn set_userdata<T>(&self, userdata: Option<NonNull<T>>) -> &Self {
        trace!("{:?} set userdata to {:?}", self, userdata);

        unsafe {
            ffi::JS_SetContextOpaque(
                self.as_ptr(),
                userdata.map_or_else(null_mut, |p| p.as_ptr() as *mut _),
            );
        }
        self
    }

    pub fn set_max_stack_size(&self, stack_size: usize) -> &Self {
        trace!("{:?} set stack size to {:?}", self, stack_size);

        unsafe {
            ffi::JS_SetMaxStackSize(self.as_ptr(), stack_size);
        }
        self
    }

    pub fn global_object(&self) -> Value {
        unsafe { ffi::JS_GetGlobalObject(self.as_ptr()) }.into()
    }
}
