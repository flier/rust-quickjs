use std::mem;
use std::ptr::{null_mut, NonNull};

use foreign_types::ForeignTypeRef;

use crate::{ffi, ClassId, ContextRef, Runtime, Value};

lazy_static! {
    static ref RUNTIME_USERDATA_CLASS_ID: ClassId = Runtime::new_class_id();
}

impl Runtime {
    pub fn userdata_class_id() -> ClassId {
        *RUNTIME_USERDATA_CLASS_ID
    }

    pub(crate) fn register_userdata_class(&self) -> bool {
        unsafe extern "C" fn userdata_finalizer(_rt: *mut ffi::JSRuntime, val: ffi::JSValue) {
            mem::drop(Box::from_raw(ffi::JS_GetOpaque(
                val,
                Runtime::userdata_class_id(),
            )));
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
    pub fn new_userdata<T>(&self, v: T) -> Value {
        unsafe {
            let obj = ffi::JS_NewObjectClass(self.as_ptr(), Runtime::userdata_class_id() as i32);

            ffi::JS_SetOpaque(obj, Box::into_raw(Box::new(v)) as *mut _);

            obj.into()
        }
    }

    pub fn get_userdata_unchecked<T>(&self, v: &Value) -> NonNull<T> {
        unsafe {
            NonNull::new_unchecked(ffi::JS_GetOpaque2(
                self.as_ptr(),
                v.raw(),
                Runtime::userdata_class_id(),
            ))
            .cast()
        }
    }
}
