use std::ptr::{null_mut, NonNull};

use foreign_types::{ForeignType, ForeignTypeRef};

use crate::{ffi, value::ToBool, Bindable, RuntimeRef, Value};

foreign_type! {
    /// `Context` represents a Javascript context (or Realm).
    ///
    /// Each `Context` has its own global objects and system objects.
    /// There can be several `Contexts` per `Runtime` and they can share objects,
    /// similary to frames of the same origin sharing Javascript objects in a web browser.
    pub unsafe type Context : Send {
        type CType = ffi::JSContext;

        fn drop = ffi::JS_FreeContext;
        fn clone = ffi::JS_DupContext;
    }
}

impl_foreign_type!(Context, ContextRef);

pub struct Builder(Context);

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
        unsafe { ffi::JS_AddIntrinsicBaseObjects(self.0.as_ptr()) };
        self
    }

    pub fn with_date(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicDate(self.0.as_ptr()) };
        self
    }

    pub fn with_eval(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicEval(self.0.as_ptr()) };
        self
    }

    pub fn with_string_normalize(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicStringNormalize(self.0.as_ptr()) };
        self
    }

    pub fn with_regexp_compiler(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicRegExpCompiler(self.0.as_ptr()) };
        self
    }

    pub fn with_regexp(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicRegExp(self.0.as_ptr()) };
        self
    }

    pub fn with_json(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicJSON(self.0.as_ptr()) };
        self
    }

    pub fn with_proxy(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicProxy(self.0.as_ptr()) };
        self
    }

    pub fn with_map(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicMapSet(self.0.as_ptr()) };
        self
    }

    pub fn with_typedarray(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicTypedArrays(self.0.as_ptr()) };
        self
    }

    pub fn with_promise(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicPromise(self.0.as_ptr()) };
        self
    }

    pub fn with_big_int(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicBigInt(self.0.as_ptr()) };
        self
    }

    pub fn with_big_float(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicBigFloat(self.0.as_ptr()) };
        self
    }

    pub fn with_big_decimal(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicBigDecimal(self.0.as_ptr()) };
        self
    }

    /// enable operator overloading
    pub fn with_operators(self) -> Self {
        unsafe { ffi::JS_AddIntrinsicOperators(self.0.as_ptr()) };
        self
    }

    /// enable "use math"
    pub fn with_big_num_ext(self, enable: bool) -> Self {
        unsafe { ffi::JS_EnableBignumExt(self.0.as_ptr(), enable.to_bool()) };
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

    pub fn global_object(&self) -> Value {
        unsafe { ffi::JS_GetGlobalObject(self.as_ptr()) }.bind(self)
    }
}
