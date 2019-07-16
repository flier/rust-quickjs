use std::ptr::NonNull;

use foreign_types::ForeignTypeRef;

use crate::{
    ffi::{self, _bindgen_ty_1::*},
    ContextRef, RuntimeRef,
};

#[repr(transparent)]
pub struct Value(ffi::JSValue);

impl RuntimeRef {
    pub unsafe fn free_value(&self, v: Value) {
        if v.has_ref_cnt() {
            let mut ref_cnt = v.as_ptr::<ffi::JSRefCountHeader>();

            ref_cnt.as_mut().ref_count -= 1;

            if ref_cnt.as_ref().ref_count <= 0 {
                ffi::__JS_FreeValueRT(self.as_ptr(), v.0)
            }
        }
    }
}

impl ContextRef {
    pub unsafe fn free_value(&self, v: Value) {
        if v.has_ref_cnt() {
            let mut ref_cnt = v.as_ptr::<ffi::JSRefCountHeader>();

            ref_cnt.as_mut().ref_count -= 1;

            if ref_cnt.as_ref().ref_count <= 0 {
                ffi::__JS_FreeValue(self.as_ptr(), v.0)
            }
        }
    }

    pub fn new_bool(&self, v: bool) -> Value {
        mkval(JS_TAG_BOOL, if v { TRUE } else { FALSE })
    }

    pub fn new_int32(&self, v: i32) -> Value {
        mkval(JS_TAG_INT, v)
    }

    pub fn new_int64(&self, v: i64) -> Value {
        Value(unsafe { ffi::JS_NewInt64(self.as_ptr(), v) })
    }

    pub fn new_float64(&self, v: f64) -> Value {
        Value(ffi::JSValue {
            u: ffi::JSValueUnion { float64: v },
            tag: JS_TAG_FLOAT64 as i64,
        })
    }

    pub fn new_catch_offset(&self, off: i32) -> Value {
        mkval(JS_TAG_CATCH_OFFSET, off)
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        unsafe {
            if self.has_ref_cnt() {
                self.as_ptr::<ffi::JSRefCountHeader>().as_mut().ref_count += 1;
            }
        }

        Value(self.0)
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

    pub const fn false_() -> Self {
        mkval(JS_TAG_BOOL, FALSE)
    }

    pub const fn true_() -> Self {
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

    pub fn as_int(&self) -> i32 {
        unsafe { self.0.u.int32 }
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

    pub fn as_ptr<T>(&self) -> NonNull<T> {
        unsafe { NonNull::new_unchecked(self.0.u.ptr).cast() }
    }

    pub fn as_obj(&self) -> NonNull<ffi::JSObject> {
        self.as_ptr()
    }
    fn has_ref_cnt(&self) -> bool {
        self.tag() >= JS_TAG_FIRST
    }
}
