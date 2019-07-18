use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::os::raw::c_char;
use std::ptr::NonNull;
use std::slice;

use failure::Error;
use foreign_types::ForeignTypeRef;

pub use crate::ffi::_bindgen_ty_1::*;
use crate::{ffi, Atom, ContextRef, ErrorKind, Local, RuntimeRef};

#[repr(transparent)]
pub struct Value(pub(crate) ffi::JSValue);

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            match self.tag() {
                JS_TAG_INT => f.debug_tuple("Value").field(&self.0.u.int32).finish(),
                JS_TAG_BOOL => f
                    .debug_tuple("Value")
                    .field(&(self.0.u.int32 != FALSE))
                    .finish(),
                JS_TAG_NULL => f.write_str("Null"),
                JS_TAG_UNDEFINED => f.write_str("Undefined"),
                JS_TAG_UNINITIALIZED => f.write_str("Uninitialized"),
                tag => f.debug_struct("Value").field("tag", &tag).finish(),
            }
        }
    }
}

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

impl<'a> Local<'a, Value> {
    pub fn ok(self) -> Result<Local<'a, Value>, Error> {
        self.ctxt.to_result(self.inner)
    }

    pub fn free(self) {
        self.ctxt.free_value(self.inner)
    }

    pub fn is_error(&self) -> bool {
        self.ctxt.is_error(&self.inner)
    }

    pub fn is_function(&self) -> bool {
        self.ctxt.is_function(&self.inner)
    }

    pub fn is_constructor(&self) -> bool {
        self.ctxt.is_constructor(&self.inner)
    }

    pub fn to_bool(&self) -> Option<bool> {
        self.ctxt.to_bool(&self.inner)
    }

    pub fn to_int32(&self) -> Option<i32> {
        self.ctxt.to_int32(&self.inner)
    }

    pub fn to_int64(&self) -> Option<i64> {
        self.ctxt.to_int64(&self.inner)
    }

    pub fn to_index(&self) -> Option<u64> {
        self.ctxt.to_index(&self.inner)
    }

    pub fn to_float64(&self) -> Option<f64> {
        self.ctxt.to_float64(&self.inner)
    }

    pub fn to_string(&self) -> Value {
        self.ctxt.to_string(&self.inner)
    }

    pub fn to_cstr(&self) -> Option<CStrBuf> {
        self.ctxt.to_cstr(&self.inner)
    }

    pub fn get_property<T: GetProperty>(&self, prop: T) -> Option<Local<Value>> {
        self.ctxt.get_property(&self.inner, prop)
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

    pub fn to_result(&self, v: Value) -> Result<Local<Value>, Error> {
        if v.is_exception() {
            self.reset_uncatchable_error();

            let exc = self.exception();
            let err = ErrorKind::try_from(exc)?;

            trace!("-> {:?}", err);

            Err(err.into())
        } else {
            trace!("-> {:?}", v);

            Ok(self.bind(v))
        }
    }

    pub fn new_value<T: NewValue>(&self, s: T) -> Local<Value> {
        self.bind(s.new_value(self))
    }

    pub fn new_error(&self) -> Local<Value> {
        self.bind(Value(unsafe { ffi::JS_NewError(self.as_ptr()) }))
    }

    pub fn throw_out_of_memory(&self) -> Local<Value> {
        self.bind(Value(unsafe { ffi::JS_ThrowOutOfMemory(self.as_ptr()) }))
    }

    pub fn exception(&self) -> Local<Value> {
        self.bind(Value(unsafe { ffi::JS_GetException(self.as_ptr()) }))
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

    pub fn to_bool(&self, val: &Value) -> Option<bool> {
        match unsafe { ffi::JS_ToBool(self.as_ptr(), val.0) } {
            ERR => None,
            FALSE => Some(false),
            _ => Some(true),
        }
    }

    pub fn to_int32(&self, val: &Value) -> Option<i32> {
        let mut n = 0;

        match unsafe { ffi::JS_ToInt32(self.as_ptr(), &mut n, val.0) } {
            ERR => None,
            _ => Some(n),
        }
    }

    pub fn to_int64(&self, val: &Value) -> Option<i64> {
        let mut n = 0;

        match unsafe { ffi::JS_ToInt64(self.as_ptr(), &mut n, val.0) } {
            ERR => None,
            _ => Some(n),
        }
    }

    pub fn to_index(&self, val: &Value) -> Option<u64> {
        let mut n = 0;

        match unsafe { ffi::JS_ToIndex(self.as_ptr(), &mut n, val.0) } {
            ERR => None,
            _ => Some(n),
        }
    }

    pub fn to_float64(&self, val: &Value) -> Option<f64> {
        let mut n = 0.0;

        match unsafe { ffi::JS_ToFloat64(self.as_ptr(), &mut n, val.0) } {
            ERR => None,
            _ => Some(n),
        }
    }

    pub fn to_string(&self, val: &Value) -> Value {
        Value(unsafe { ffi::JS_ToString(self.as_ptr(), val.0) })
    }

    pub fn to_cstr(&self, val: &Value) -> Option<CStrBuf> {
        let mut len = 0;

        unsafe {
            let p = ffi::JS_ToCStringLen(self.as_ptr(), &mut len, val.0, FALSE);

            if p.is_null() {
                None
            } else {
                Some(CStrBuf(self.bind(CStr::from_bytes_with_nul_unchecked(
                    slice::from_raw_parts(p as *const _, len as usize + 1),
                ))))
            }
        }
    }

    pub fn get_property<T: GetProperty>(&self, val: &Value, prop: T) -> Option<Local<Value>> {
        prop.get_property(self, val)
    }
}

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

pub struct CStrBuf<'a>(pub(crate) Local<'a, &'a CStr>);

impl Drop for CStrBuf<'_> {
    fn drop(&mut self) {
        unsafe { ffi::JS_FreeCString(self.0.ctxt.as_ptr(), self.0.inner.as_ptr()) }
    }
}

impl<'a> Deref for CStrBuf<'a> {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        self.0.inner
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

const ERR: i32 = -1;
const TRUE: i32 = 1;
const FALSE: i32 = 0;

const fn mkval(tag: i32, val: i32) -> Value {
    Value(ffi::JSValue {
        u: ffi::JSValueUnion { int32: val },
        tag: tag as i64,
    })
}

impl Value {
    pub fn new<T: NewValue>(ctxt: &ContextRef, v: T) -> Local<Self> {
        ctxt.bind(v.new_value(ctxt))
    }

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

    pub fn ok(self) -> Option<Value> {
        if self.is_undefined() {
            None
        } else {
            Some(self)
        }
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
