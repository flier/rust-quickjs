use std::mem;
use std::ptr::{null_mut, NonNull};

use foreign_types::ForeignTypeRef;

use crate::{ffi, Bindable, ClassId, ContextRef, Runtime, Value};

lazy_static! {
    static ref RUNTIME_USERDATA_CLASS_ID: ClassId = Runtime::new_class_id();
}

impl Runtime {
    pub fn userdata_class_id() -> ClassId {
        *RUNTIME_USERDATA_CLASS_ID
    }

    pub(crate) fn register_userdata_class(&self) -> bool {
        unsafe extern "C" fn userdata_finalizer(_rt: *mut ffi::JSRuntime, obj: ffi::JSValue) {
            let ptr = ffi::JS_GetOpaque(obj, Runtime::userdata_class_id());

            trace!("free userdata {:p} @ {:?}", ptr, obj.u.ptr);

            mem::drop(Box::from_raw(ptr));
        }

        self.new_class(
            Runtime::userdata_class_id(),
            &ffi::JSClassDef {
                class_name: cstr!(Userdata).as_ptr(),
                finalizer: Some(userdata_finalizer),
                gc_mark: None,
                call: None,
                exotic: null_mut(),
            },
        )
    }
}

impl ContextRef {
    pub fn new_userdata<T>(&self, v: T) -> Value<'_> {
        let obj = self.new_object_class(Runtime::userdata_class_id());
        let ptr = Box::into_raw(Box::new(v));

        trace!("new userdata {:p} @ {:?}", ptr, obj.as_ptr::<()>());

        obj.set_opaque(ptr);
        obj.bind(self)
    }

    pub fn get_userdata_unchecked<T>(&self, obj: ffi::JSValue) -> NonNull<T> {
        let ptr = self.get_opaque(obj, Runtime::userdata_class_id());

        trace!("got userdata {:p} @ {:?}", ptr, obj.as_ptr::<()>());

        unsafe { NonNull::new_unchecked(ptr) }
    }
}

impl Value<'_> {
    pub fn set_opaque<T>(&self, opaque: *mut T) {
        unsafe { ffi::JS_SetOpaque(self.inner(), opaque as *mut _) }
    }

    pub fn get_opaque<T>(&self, class_id: ClassId) -> *mut T {
        unsafe { ffi::JS_GetOpaque(self.inner(), class_id) as *mut _ }
    }
}

impl ContextRef {
    pub fn get_opaque<T>(&self, obj: ffi::JSValue, class_id: ClassId) -> *mut T {
        unsafe { ffi::JS_GetOpaque2(self.as_ptr(), obj, class_id) as *mut _ }
    }
}
