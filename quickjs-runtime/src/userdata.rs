use std::mem;
use std::ptr::{null_mut, NonNull};

use foreign_types::ForeignTypeRef;

use crate::{ffi, ClassId, ContextRef, Local, Runtime, Value};

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
    pub fn new_userdata<T>(&self, v: T) -> Local<'_, Value> {
        unsafe {
            let obj = ffi::JS_NewObjectClass(self.as_ptr(), Runtime::userdata_class_id() as i32);
            let ptr: *mut T = Box::into_raw(Box::new(v));

            trace!("new userdata {:p} @ {:?}", ptr, obj.u.ptr);

            ffi::JS_SetOpaque(obj, ptr as *mut _);

            self.bind(obj)
        }
    }

    pub fn get_userdata_unchecked<T>(&self, obj: &Value) -> NonNull<T> {
        unsafe {
            let ptr: *mut T =
                ffi::JS_GetOpaque2(self.as_ptr(), obj.raw(), Runtime::userdata_class_id())
                    as *mut _;

            trace!("got userdata {:p} @ {:?}", ptr, obj.u.ptr);

            NonNull::new_unchecked(ptr).cast()
        }
    }
}
