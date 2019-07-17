use std::ffi::CString;
use std::ops::{Deref, DerefMut};
use std::os::raw::c_char;
use std::ptr::NonNull;

use foreign_types::ForeignTypeRef;

use crate::{
    ffi::{self, _bindgen_ty_1::*},
    ContextRef, RuntimeRef,
};

#[repr(transparent)]
pub struct Value(ffi::JSValue);

impl Deref for Value {
    type Target = ffi::JSValue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Value {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<ffi::JSValue> for Value {
    fn from(v: ffi::JSValue) -> Self {
        Value(v)
    }
}

impl RuntimeRef {
    pub fn free_value(&self, v: Value) {
        if v.has_ref_cnt() {
            unsafe {
                let mut ref_cnt = v.as_ptr::<ffi::JSRefCountHeader>();

                ref_cnt.as_mut().ref_count -= 1;

                if ref_cnt.as_ref().ref_count <= 0 {
                    ffi::__JS_FreeValueRT(self.as_ptr(), v.0)
                }
            }
        }
    }
}

impl ContextRef {
    pub fn clone_value(&self, v: &Value) -> Value {
        unsafe {
            if v.has_ref_cnt() {
                v.as_ptr::<ffi::JSRefCountHeader>().as_mut().ref_count += 1;
            }
        }

        Value(v.0)
    }

    pub fn free_value(&self, v: Value) {
        if v.has_ref_cnt() {
            unsafe {
                let mut ref_cnt = v.as_ptr::<ffi::JSRefCountHeader>();

                ref_cnt.as_mut().ref_count -= 1;

                if ref_cnt.as_ref().ref_count <= 0 {
                    ffi::__JS_FreeValue(self.as_ptr(), v.0)
                }
            }
        }
    }

    pub fn new_value<T: NewValue>(&self, s: T) -> Value {
        s.new_value(self)
    }

    pub fn new_error(&self) -> Value {
        Value(unsafe { ffi::JS_NewError(self.as_ptr()) })
    }

    pub fn new_atom_string<T: Into<Vec<u8>>>(&self, s: T) -> Value {
        Value(unsafe {
            ffi::JS_NewAtomString(
                self.as_ptr(),
                CString::new(s)
                    .expect("atom string should not contain an internal 0 byte")
                    .as_ptr(),
            )
        })
    }

    pub fn new_object(&self) -> Value {
        Value(unsafe { ffi::JS_NewObject(self.as_ptr()) })
    }

    pub fn new_array(&self) -> Value {
        Value(unsafe { ffi::JS_NewArray(self.as_ptr()) })
    }

    pub fn is_array(&self, val: &Value) -> bool {
        unsafe { ffi::JS_IsArray(self.as_ptr(), val.0) != FALSE }
    }

    pub fn new_catch_offset(&self, off: i32) -> Value {
        mkval(JS_TAG_CATCH_OFFSET, off)
    }

    pub fn is_error(&self, val: &Value) -> bool {
        unsafe { ffi::JS_IsError(self.as_ptr(), val.0) != FALSE }
    }

    pub fn is_function(&self, val: &Value) -> bool {
        unsafe { ffi::JS_IsFunction(self.as_ptr(), val.0) != FALSE }
    }

    pub fn is_constructor(&self, val: &Value) -> bool {
        unsafe { ffi::JS_IsConstructor(self.as_ptr(), val.0) != FALSE }
    }
}

pub trait NewValue {
    fn new_value(self, context: &ContextRef) -> Value;
}

impl NewValue for bool {
    fn new_value(self, _context: &ContextRef) -> Value {
        mkval(JS_TAG_BOOL, if self { TRUE } else { FALSE })
    }
}

impl NewValue for i32 {
    fn new_value(self, _context: &ContextRef) -> Value {
        mkval(JS_TAG_INT, self)
    }
}

impl NewValue for i64 {
    fn new_value(self, context: &ContextRef) -> Value {
        Value(unsafe { ffi::JS_NewInt64(context.as_ptr(), self) })
    }
}

impl NewValue for f64 {
    fn new_value(self, _context: &ContextRef) -> Value {
        Value(ffi::JSValue {
            u: ffi::JSValueUnion { float64: self },
            tag: JS_TAG_FLOAT64 as i64,
        })
    }
}

impl<'a> NewValue for &'a str {
    fn new_value(self, context: &ContextRef) -> Value {
        Value(unsafe {
            ffi::JS_NewStringLen(
                context.as_ptr(),
                self.as_ptr() as *const _,
                self.len() as i32,
            )
        })
    }
}

impl NewValue for *const c_char {
    fn new_value(self, context: &ContextRef) -> Value {
        Value(unsafe { ffi::JS_NewString(context.as_ptr(), self) })
    }
}

const TRUE: i32 = 1;
const FALSE: i32 = 0;

const fn mkval(tag: i32, val: i32) -> Value {
    Value(ffi::JSValue {
        u: ffi::JSValueUnion { int32: val },
        tag: tag as i64,
    })
}

impl Value {
    pub const fn nan() -> Self {
        Value(ffi::JSValue {
            u: ffi::JSValueUnion {
                float64: core::f64::NAN,
            },
            tag: JS_TAG_FLOAT64 as i64,
        })
    }

    pub const fn null() -> Self {
        mkval(JS_TAG_NULL, 0)
    }

    pub const fn undefined() -> Self {
        mkval(JS_TAG_UNDEFINED, 0)
    }

    pub const fn false_value() -> Self {
        mkval(JS_TAG_BOOL, FALSE)
    }

    pub const fn true_value() -> Self {
        mkval(JS_TAG_BOOL, TRUE)
    }

    pub const fn exception() -> Self {
        mkval(JS_TAG_EXCEPTION, 0)
    }

    pub const fn uninitialized() -> Self {
        mkval(JS_TAG_UNINITIALIZED, 0)
    }

    pub fn tag(&self) -> i32 {
        self.0.tag as i32
    }

    pub fn is_number(&self) -> bool {
        unsafe { ffi::JS_IsNumber(self.0) != FALSE }
    }

    pub fn is_integer(&self) -> bool {
        let tag = self.tag();

        tag == JS_TAG_INT || tag == JS_TAG_BIG_INT
    }

    pub fn is_big_float(&self) -> bool {
        self.tag() == JS_TAG_BIG_FLOAT
    }

    pub fn is_bool(&self) -> bool {
        self.tag() == JS_TAG_BOOL
    }

    pub fn is_null(&self) -> bool {
        self.tag() == JS_TAG_NULL
    }

    pub fn is_undefined(&self) -> bool {
        self.tag() == JS_TAG_UNDEFINED
    }

    pub fn is_exception(&self) -> bool {
        self.tag() == JS_TAG_EXCEPTION
    }

    pub fn is_uninitialized(&self) -> bool {
        self.tag() == JS_TAG_UNINITIALIZED
    }

    pub fn is_string(&self) -> bool {
        self.tag() == JS_TAG_STRING
    }

    pub fn is_symbol(&self) -> bool {
        self.tag() == JS_TAG_SYMBOL
    }

    pub fn is_object(&self) -> bool {
        self.tag() == JS_TAG_OBJECT
    }

    pub fn as_int(&self) -> Option<i32> {
        if self.tag() == JS_TAG_INT {
            Some(unsafe { self.0.u.int32 })
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if self.tag() == JS_TAG_BOOL {
            Some(unsafe { self.0.u.int32 != 0 })
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        if self.tag() == JS_TAG_FLOAT64 {
            Some(unsafe { self.0.u.float64 })
        } else {
            None
        }
    }

    pub fn as_object(&self) -> Option<NonNull<ffi::JSObject>> {
        if self.tag() == JS_TAG_OBJECT {
            Some(unsafe { self.as_ptr() })
        } else {
            None
        }
    }

    unsafe fn as_ptr<T>(&self) -> NonNull<T> {
        NonNull::new_unchecked(self.0.u.ptr).cast()
    }

    fn has_ref_cnt(&self) -> bool {
        self.tag() >= JS_TAG_FIRST
    }
}
