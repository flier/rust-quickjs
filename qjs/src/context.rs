use std::ops::Deref;
use std::ptr::{null_mut, NonNull};

use foreign_types::{ForeignType, ForeignTypeRef};

use crate::{ffi, Local, RuntimeRef, Value};

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

pub struct Builder(Context);

impl Deref for Builder {
    type Target = ContextRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Context {
    pub fn new(runtime: &RuntimeRef) -> Context {
        unsafe { Context::from_ptr(ffi::JS_NewContext(runtime.as_ptr())) }
    }

    pub fn builder(runtime: &RuntimeRef) -> Builder {
        Builder(unsafe { Context::from_ptr(ffi::JS_NewContextRaw(runtime.as_ptr())) })
    }
}

impl Builder {
    pub fn with_base_objects(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicBaseObjects(self.as_ptr()) };
        self
    }

    pub fn with_date(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicDate(self.as_ptr()) };
        self
    }

    pub fn with_eval(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicEval(self.as_ptr()) };
        self
    }

    pub fn with_string_normalize(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicStringNormalize(self.as_ptr()) };
        self
    }

    pub fn with_regexp_compiler(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicRegExpCompiler(self.as_ptr()) };
        self
    }

    pub fn with_regexp(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicRegExp(self.as_ptr()) };
        self
    }

    pub fn with_json(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicJSON(self.as_ptr()) };
        self
    }

    pub fn with_proxy(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicProxy(self.as_ptr()) };
        self
    }

    pub fn with_map(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicMapSet(self.as_ptr()) };
        self
    }

    pub fn with_typedarray(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicTypedArrays(self.as_ptr()) };
        self
    }

    pub fn with_promise(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicPromise(self.as_ptr()) };
        self
    }

    pub fn build(self) -> Context {
        self.0
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

    pub fn global_object(&self) -> Local<Value> {
        self.bind(unsafe { ffi::JS_GetGlobalObject(self.as_ptr()) })
    }
}
