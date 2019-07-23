use foreign_types::ForeignTypeRef;

use crate::{ffi, value::FALSE, ContextRef, Local, Runtime, RuntimeRef, Value};

pub type ClassId = ffi::JSClassID;
pub type ClassDef = ffi::JSClassDef;

impl Runtime {
    pub fn new_class_id() -> ClassId {
        let mut class_id = 0;

        unsafe { ffi::JS_NewClassID(&mut class_id) }
    }
}

impl RuntimeRef {
    pub fn new_class(&self, class_id: ClassId, class_def: &ClassDef) -> bool {
        unsafe { ffi::JS_NewClass(self.as_ptr(), class_id, class_def as *const _) != FALSE }
    }

    pub fn is_registered_class(&self, class_id: ClassId) -> bool {
        unsafe { ffi::JS_IsRegisteredClass(self.as_ptr(), class_id) != FALSE }
    }
}

impl ContextRef {
    pub fn set_class_proto<T: Into<ffi::JSValue>>(&self, class_id: ClassId, obj: T) {
        unsafe { ffi::JS_SetClassProto(self.as_ptr(), class_id, obj.into()) }
    }

    pub fn get_class_proto(&self, class_id: ClassId) -> Local<Value> {
        self.bind(unsafe { ffi::JS_GetClassProto(self.as_ptr(), class_id) })
    }
}
