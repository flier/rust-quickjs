#![allow(clippy::cast_lossless)]

use core::cmp::Ordering;
use core::fmt;
use core::slice;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use failure::Error;
use foreign_types::ForeignTypeRef;

use crate::{ffi, Bindable, ClassId, ContextRef, Local, RuntimeRef};

pub const ERR: i32 = -1;

pub trait ToBool {
    type Bool;

    fn to_bool(self) -> Self::Bool;
}

impl ToBool for i32 {
    type Bool = bool;

    fn to_bool(self) -> bool {
        self != ffi::FALSE_VALUE
    }
}

impl ToBool for bool {
    type Bool = i32;

    fn to_bool(self) -> i32 {
        if self {
            ffi::TRUE_VALUE
        } else {
            ffi::FALSE_VALUE
        }
    }
}

/// `Value` represents a Javascript value which can be a primitive type or an object.
pub type Value<'a> = Local<'a, ffi::JSValue>;

impl Bindable for ffi::JSValue {
    fn bind(self, ctxt: &ContextRef) -> Value {
        ctxt.bind_value(self)
    }

    fn unbind(self, ctxt: &ContextRef) {
        ctxt.free_value(self)
    }
}

impl fmt::Debug for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Value").field(&self.inner()).finish()
    }
}

impl fmt::Display for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_cstring().unwrap().to_string_lossy().to_string())
    }
}

impl Clone for Value<'_> {
    fn clone(&self) -> Self {
        self.ctxt.clone_value(self).bind(self.ctxt)
    }
}

impl<'a> Value<'a> {
    pub fn check_undefined(self) -> Option<Self> {
        if self.is_undefined() {
            None
        } else {
            Some(self)
        }
    }

    pub fn is_error(&self) -> bool {
        self.ctxt.is_error(self)
    }

    pub fn is_function(&self) -> bool {
        self.ctxt.is_function(self)
    }

    pub fn is_constructor(&self) -> bool {
        self.ctxt.is_constructor(self)
    }

    pub fn to_bool(&self) -> Option<bool> {
        self.ctxt.to_bool(self)
    }

    pub fn to_int32(&self) -> Option<i32> {
        self.ctxt.to_int32(self)
    }

    pub fn to_int64(&self) -> Option<i64> {
        self.ctxt.to_int64(self)
    }

    pub fn to_index(&self) -> Option<u64> {
        self.ctxt.to_index(self)
    }

    pub fn to_float64(&self) -> Option<f64> {
        self.ctxt.to_float64(self)
    }

    #[cfg(feature = "bignum")]
    pub fn to_bigint64(&self) -> Option<i64> {
        self.ctxt.to_bigint64(self)
    }

    pub fn to_str(&self) -> Value {
        self.ctxt.to_str(self).bind(self.ctxt)
    }

    pub fn to_property_key(&self) -> Value {
        self.ctxt.to_property_key(self).bind(self.ctxt)
    }

    pub fn to_cstring(&self) -> Option<CString> {
        self.ctxt.to_cstring(self)
    }

    pub fn instance_of(&self, obj: &Value) -> Result<bool, Error> {
        self.ctxt.is_instance_of(self, obj)
    }
}

impl RuntimeRef {
    pub fn free_value(&self, v: ffi::JSValue) {
        if !v.is_undefined() && v.has_ref_cnt() {
            unsafe {
                let mut ref_cnt = v.as_ptr::<ffi::JSRefCountHeader>();

                ref_cnt.as_mut().ref_count -= 1;

                if ref_cnt.as_ref().ref_count <= 0 {
                    ffi::__JS_FreeValueRT(self.as_ptr(), v)
                }
            }
        }
    }
}

impl ContextRef {
    pub fn bind_value(&self, value: ffi::JSValue) -> Value {
        Local::new(self, value)
    }

    pub fn clone_value(&self, v: &ffi::JSValue) -> ffi::JSValue {
        unsafe {
            if v.has_ref_cnt() {
                v.as_ptr::<ffi::JSRefCountHeader>().as_mut().ref_count += 1;
            }
        }

        *v
    }

    pub fn free_value(&self, v: ffi::JSValue) {
        if v.has_ref_cnt() {
            unsafe {
                let mut ref_cnt = v.as_ptr::<ffi::JSRefCountHeader>();

                ref_cnt.as_mut().ref_count -= 1;

                if ref_cnt.as_ref().ref_count <= 0 {
                    ffi::__JS_FreeValue(self.as_ptr(), v)
                }
            }
        }
    }

    pub fn nan(&self) -> Value {
        ffi::NAN.bind(self)
    }

    pub fn null(&self) -> Value {
        ffi::NULL.bind(self)
    }

    pub fn undefined(&self) -> Value {
        ffi::UNDEFINED.bind(self)
    }

    pub fn false_value(&self) -> Value {
        ffi::FALSE.bind(self)
    }

    pub fn true_value(&self) -> Value {
        ffi::TRUE.bind(self)
    }

    pub fn exception(&self) -> Value {
        ffi::EXCEPTION.bind(self)
    }

    pub fn uninitialized(&self) -> Value {
        ffi::UNINITIALIZED.bind(self)
    }

    pub fn new_value<T: LazyValue>(&self, s: T) -> Option<Value> {
        s.new_value(self).bind(self).check_undefined()
    }

    pub fn new_object_proto_class(&self, proto: &Value, class_id: ClassId) -> Value {
        unsafe { ffi::JS_NewObjectProtoClass(self.as_ptr(), proto.inner(), class_id) }.bind(self)
    }

    pub fn new_object_class(&self, class_id: ClassId) -> Value {
        unsafe { ffi::JS_NewObjectClass(self.as_ptr(), class_id as i32) }.bind(self)
    }

    pub fn new_object_proto(&self, proto: &Value) -> Value {
        unsafe { ffi::JS_NewObjectProto(self.as_ptr(), proto.inner()) }.bind(self)
    }

    pub fn new_object(&self) -> Value {
        unsafe { ffi::JS_NewObject(self.as_ptr()) }.bind(self)
    }

    pub fn new_array(&self) -> Value {
        unsafe { ffi::JS_NewArray(self.as_ptr()) }.bind(self)
    }

    pub fn new_catch_offset(&self, off: i32) -> Value {
        ffi::JS_NewCatchOffset(self.as_ptr(), off).bind(self)
    }

    #[cfg(feature = "bignum")]
    pub fn new_bigint64(&self, n: i64) -> Value {
        unsafe { ffi::JS_NewBigInt64(self.as_ptr(), n) }.bind(self)
    }

    #[cfg(feature = "bignum")]
    pub fn new_biguint64(&self, n: u64) -> Value {
        unsafe { ffi::JS_NewBigUint64(self.as_ptr(), n) }.bind(self)
    }

    pub fn to_bool(&self, val: &Value) -> Option<bool> {
        self.check_error(unsafe { ffi::JS_ToBool(self.as_ptr(), val.inner()) })
            .ok()
            .map(ToBool::to_bool)
    }

    pub fn to_int32(&self, val: &Value) -> Option<i32> {
        let mut n = 0;

        self.check_error(unsafe { ffi::JS_ToInt32(self.as_ptr(), &mut n, val.inner()) })
            .ok()
            .map(|_| n)
    }

    pub fn to_int64(&self, val: &Value) -> Option<i64> {
        let mut n = 0;

        self.check_error(unsafe { ffi::JS_ToInt64(self.as_ptr(), &mut n, val.inner()) })
            .ok()
            .map(|_| n)
    }

    pub fn to_int64_ext(&self, val: &Value) -> Option<i64> {
        let mut n = 0;

        self.check_error(unsafe { ffi::JS_ToInt64Ext(self.as_ptr(), &mut n, val.inner()) })
            .ok()
            .map(|_| n)
    }

    pub fn to_index(&self, val: &Value) -> Option<u64> {
        let mut n = 0;

        self.check_error(unsafe { ffi::JS_ToIndex(self.as_ptr(), &mut n, val.inner()) })
            .ok()
            .map(|_| n)
    }

    pub fn to_float64(&self, val: &Value) -> Option<f64> {
        let mut n = 0.0;

        self.check_error(unsafe { ffi::JS_ToFloat64(self.as_ptr(), &mut n, val.inner()) })
            .ok()
            .map(|_| n)
    }

    #[cfg(feature = "bignum")]
    pub fn to_bigint64(&self, val: &Value) -> Option<i64> {
        let mut n = 0;

        self.check_error(unsafe { ffi::JS_ToBigInt64(self.as_ptr(), &mut n, val.inner()) })
            .ok()
            .map(|_| n)
    }

    pub fn to_str(&self, val: &Value) -> Value {
        unsafe { ffi::JS_ToString(self.as_ptr(), val.inner()) }.bind(self)
    }

    pub fn to_property_key(&self, val: &Value) -> Value {
        unsafe { ffi::JS_ToPropertyKey(self.as_ptr(), val.inner()) }.bind(self)
    }

    /// Convert Javascript String to C UTF-8 encoded strings.
    pub fn to_cstring(&self, val: &Value) -> Option<CString> {
        let mut len = 0;

        unsafe {
            let p = ffi::JS_ToCStringLen2(self.as_ptr(), &mut len, val.inner(), ffi::FALSE_VALUE);

            if p.is_null() {
                None
            } else {
                let s = CStr::from_bytes_with_nul_unchecked(slice::from_raw_parts(
                    p as *const _,
                    len as usize + 1,
                ))
                .to_owned();

                ffi::JS_FreeCString(self.as_ptr(), p);

                Some(s)
            }
        }
    }

    pub fn is_instance_of(&self, val: &Value, obj: &Value) -> Result<bool, Error> {
        self.check_bool(unsafe { ffi::JS_IsInstanceOf(self.as_ptr(), val.inner(), obj.inner()) })
    }
}

/// Create new `Value` from primitive.
pub trait LazyValue {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue;
}

impl LazyValue for bool {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        ffi::JS_NewBool(ctxt.as_ptr(), self)
    }
}

impl LazyValue for u8 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        i32::from(self).new_value(ctxt)
    }
}

impl LazyValue for u16 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        i32::from(self).new_value(ctxt)
    }
}

impl LazyValue for u32 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        i64::from(self).new_value(ctxt)
    }
}

impl LazyValue for u64 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        if cfg!(feature = "bignum") {
            unsafe { ffi::JS_NewBigUint64(ctxt.as_ptr(), self) }
        } else {
            (self as i64).new_value(ctxt)
        }
    }
}

impl LazyValue for i8 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        i32::from(self).new_value(ctxt)
    }
}

impl LazyValue for i16 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        i32::from(self).new_value(ctxt)
    }
}

impl LazyValue for i32 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        ffi::JS_NewInt32(ctxt.as_ptr(), self)
    }
}

impl LazyValue for i64 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        if cfg!(feature = "bignum") {
            unsafe { ffi::JS_NewBigInt64(ctxt.as_ptr(), self) }
        } else {
            ffi::JS_NewInt64(ctxt.as_ptr(), self)
        }
    }
}

impl LazyValue for f32 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        f64::from(self).new_value(ctxt)
    }
}

impl LazyValue for f64 {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        ffi::JSValue {
            u: ffi::JSValueUnion { float64: self },
            tag: ffi::JS_TAG_FLOAT64 as i64,
        }
    }
}

impl LazyValue for String {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        self.as_str().new_value(ctxt)
    }
}

impl<'a> LazyValue for &'a str {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        unsafe { ffi::JS_NewStringLen(ctxt.as_ptr(), self.as_ptr() as *const _, self.len()) }
    }
}

impl LazyValue for *const c_char {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        unsafe { ffi::JS_NewString(ctxt.as_ptr(), self) }
    }
}

impl LazyValue for ffi::JSValue {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        self
    }
}

impl LazyValue for &'_ ffi::JSValue {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        ctxt.clone_value(self)
    }
}

impl LazyValue for Value<'_> {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        self.into_inner()
    }
}

impl<'a> LazyValue for &'a Value<'a> {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        ctxt.clone_value(self)
    }
}

/// Extract primitive from `Value`.
pub trait ExtractValue: Sized {
    /// Extract primitive from `Value`.
    fn extract_value(v: &Value) -> Option<Self>;
}

impl ExtractValue for () {
    fn extract_value(v: &Value) -> Option<Self> {
        if v.is_null() {
            None
        } else {
            Some(())
        }
    }
}

impl ExtractValue for bool {
    fn extract_value(v: &Value) -> Option<Self> {
        v.as_bool().or_else(|| v.to_bool())
    }
}

impl ExtractValue for i32 {
    fn extract_value(v: &Value) -> Option<Self> {
        v.as_int().or_else(|| v.to_int32())
    }
}

impl ExtractValue for i64 {
    fn extract_value(v: &Value) -> Option<Self> {
        v.as_int().map(i64::from).or_else(|| v.to_int64())
    }
}

impl ExtractValue for u64 {
    fn extract_value(v: &Value) -> Option<Self> {
        v.to_index()
    }
}

impl ExtractValue for f64 {
    fn extract_value(v: &Value) -> Option<Self> {
        v.as_float().or_else(|| v.to_float64())
    }
}

impl ExtractValue for String {
    fn extract_value(v: &Value) -> Option<Self> {
        v.to_cstring().map(|s| s.to_string_lossy().to_string())
    }
}

impl<T: ExtractValue + PartialEq> PartialEq<T> for Value<'_> {
    fn eq(&self, other: &T) -> bool {
        T::extract_value(self).map_or(false, |v| v.eq(other))
    }
}

impl<T: ExtractValue + PartialOrd> PartialOrd<T> for Value<'_> {
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        T::extract_value(self).and_then(|v| v.partial_cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, Eval, Runtime};

    #[test]
    fn instance_of() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let car = ctxt
            .eval_script(
                r#"
function Car(make, model, year) {
    this.make = make;
    this.model = model;
    this.year = year;
}

function Person(name, age) {
    this.name = name;
    this.age = age;
}

new Car('Honda', 'Accord', 1998)"#,
                "<evalScript>",
                Eval::GLOBAL,
            )
            .unwrap();

        let global = ctxt.global_object();

        assert!(car
            .instance_of(&global.get_property("Car").unwrap())
            .unwrap());
        assert!(!car
            .instance_of(&global.get_property("Person").unwrap())
            .unwrap());
    }
}
