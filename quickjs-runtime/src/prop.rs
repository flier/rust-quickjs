use std::ffi::CString;

use foreign_types::ForeignTypeRef;

use crate::{ffi, Atom, ContextRef, Local, Value, FALSE};

pub trait GetProperty {
    fn get_property<'a>(&self, ctxt: &'a ContextRef, val: &Value) -> Option<Local<'a, Value>>;
}

impl GetProperty for &str {
    fn get_property<'a>(&self, ctxt: &'a ContextRef, val: &Value) -> Option<Local<'a, Value>> {
        Value(unsafe {
            ffi::JS_GetPropertyStr(
                ctxt.as_ptr(),
                val.0,
                CString::new(*self).expect("prop").as_ptr(),
            )
        })
        .ok()
        .map(|v| ctxt.bind(v))
    }
}

impl GetProperty for u32 {
    fn get_property<'a>(&self, ctxt: &'a ContextRef, val: &Value) -> Option<Local<'a, Value>> {
        Value(unsafe { ffi::JS_GetPropertyUint32(ctxt.as_ptr(), val.0, *self) })
            .ok()
            .map(|v| ctxt.bind(v))
    }
}

impl GetProperty for Atom<'_> {
    fn get_property<'a>(&self, ctxt: &'a ContextRef, val: &Value) -> Option<Local<'a, Value>> {
        Value(unsafe {
            ffi::JS_GetPropertyInternal(ctxt.as_ptr(), val.0, self.inner, val.0, FALSE)
        })
        .ok()
        .map(|v| ctxt.bind(v))
    }
}

impl<'a> Local<'a, Value> {
    pub fn get_property<T: GetProperty>(&self, prop: T) -> Option<Local<Value>> {
        self.ctxt.get_property(&self.inner, prop)
    }
}

impl ContextRef {
    pub fn get_property<T: GetProperty>(&self, val: &Value, prop: T) -> Option<Local<Value>> {
        prop.get_property(self, val)
    }
}
