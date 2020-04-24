#![allow(clippy::cast_lossless)]

use std::cmp::Ordering;
use std::ffi::{CStr, CString};
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::os::raw::c_char;
use std::ptr::NonNull;
use std::slice;

use failure::Error;
use foreign_types::ForeignTypeRef;

use crate::{
    ffi,
    handle::{Bindable, Unbindable},
    ClassId, ContextRef, Local, RuntimeRef,
};

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
#[repr(transparent)]
pub struct Value(ffi::JSValue);

impl Default for Value {
    fn default() -> Self {
        UNDEFINED
    }
}

impl Default for &Value {
    fn default() -> Self {
        &UNDEFINED
    }
}

impl Unbindable for Value {
    fn unbind(ctxt: &ContextRef, inner: Self) {
        ctxt.free_value(inner)
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            match self.tag() {
                ffi::JS_TAG_BIG_INT => f.debug_tuple("BigInt").field(&self.u.ptr).finish(),
                ffi::JS_TAG_BIG_FLOAT => f.debug_tuple("BigFloat").field(&self.u.ptr).finish(),
                ffi::JS_TAG_SYMBOL => f.debug_tuple("Symbol").field(&self.u.ptr).finish(),
                ffi::JS_TAG_STRING => f.debug_tuple("String").field(&self.u.ptr).finish(),
                ffi::JS_TAG_MODULE => f.debug_tuple("Module").field(&self.u.ptr).finish(),
                ffi::JS_TAG_FUNCTION_BYTECODE => {
                    f.debug_tuple("Function").field(&self.u.ptr).finish()
                }
                ffi::JS_TAG_OBJECT => f.debug_tuple("Object").field(&self.u.ptr).finish(),
                ffi::JS_TAG_INT => f.debug_tuple("Value").field(&self.u.int32).finish(),
                ffi::JS_TAG_BOOL => f
                    .debug_tuple("Value")
                    .field(&self.u.int32.to_bool())
                    .finish(),
                ffi::JS_TAG_NULL => f.write_str("Null"),
                ffi::JS_TAG_UNDEFINED => f.write_str("Undefined"),
                ffi::JS_TAG_UNINITIALIZED => f.write_str("Uninitialized"),
                ffi::JS_TAG_CATCH_OFFSET => {
                    f.debug_tuple("CatchOffset").field(&self.u.int32).finish()
                }
                ffi::JS_TAG_EXCEPTION => f.write_str("Exception"),
                ffi::JS_TAG_FLOAT64 => f.debug_tuple("Value").field(&self.u.float64).finish(),
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

impl Into<ffi::JSValue> for Value {
    fn into(self) -> ffi::JSValue {
        self.0
    }
}

impl<'a> Into<Value> for Local<'a, Value> {
    fn into(self) -> Value {
        self.into_inner()
    }
}

impl<'a> Into<ffi::JSValue> for Local<'a, Value> {
    fn into(self) -> ffi::JSValue {
        self.into_inner().raw()
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

impl fmt::Display for Local<'_, Value> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_cstring().unwrap().to_string_lossy().to_string())
    }
}

impl fmt::Debug for Local<'_, Value> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Value").field(&self.to_string()).finish()
    }
}

impl Clone for Local<'_, Value> {
    fn clone(&self) -> Self {
        self.ctxt.clone_value(self)
    }
}

impl<'a> Local<'a, Value> {
    pub fn check_undefined(self) -> Option<Local<'a, Value>> {
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

    pub fn to_str(&self) -> Local<Value> {
        self.ctxt.bind(self.ctxt.to_str(self))
    }

    pub fn to_property_key(&self) -> Local<Value> {
        self.ctxt.bind(self.ctxt.to_property_key(self))
    }

    pub fn to_cstring(&self) -> Option<CString> {
        self.ctxt.to_cstring(self)
    }

    pub fn instance_of(&self, obj: &Value) -> Result<bool, Error> {
        self.ctxt.is_instance_of(self, obj)
    }
}

impl ContextRef {
    pub fn clone_value(&self, v: &Value) -> Local<Value> {
        unsafe {
            if v.has_ref_cnt() {
                v.as_ptr::<ffi::JSRefCountHeader>().as_mut().ref_count += 1;
            }
        }

        self.bind(v.0)
    }

    pub fn to_local(&self, v: Value) -> Local<Value> {
        let ret = self.clone_value(&v);
        Value::unbind(&self, v);
        ret
    }

    pub fn free_value<T: Into<Value>>(&self, v: T) {
        let v = v.into();

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

    pub fn nan(&self) -> Local<Value> {
        self.bind(NAN)
    }

    pub fn null(&self) -> Local<Value> {
        self.bind(NULL)
    }

    pub fn undefined(&self) -> Local<Value> {
        self.bind(UNDEFINED)
    }

    pub fn false_value(&self) -> Local<Value> {
        self.bind(FALSE)
    }

    pub fn true_value(&self) -> Local<Value> {
        self.bind(TRUE)
    }

    pub fn exception(&self) -> Local<Value> {
        self.bind(EXCEPTION)
    }

    pub fn uninitialized(&self) -> Local<Value> {
        self.bind(UNINITIALIZED)
    }

    pub fn new_value<T: NewValue>(&self, s: T) -> Value {
        Value(s.new_value(self))
    }

    pub fn new_object_proto_class(&self, proto: &Value, class_id: ClassId) -> Value {
        Value(unsafe { ffi::JS_NewObjectProtoClass(self.as_ptr(), proto.raw(), class_id) })
    }

    pub fn new_object_class(&self, class_id: ClassId) -> Value {
        Value(unsafe { ffi::JS_NewObjectClass(self.as_ptr(), class_id as i32) })
    }

    pub fn new_object_proto(&self, proto: &Value) -> Value {
        Value(unsafe { ffi::JS_NewObjectProto(self.as_ptr(), proto.raw()) })
    }

    pub fn new_object(&self) -> Value {
        Value(unsafe { ffi::JS_NewObject(self.as_ptr()) })
    }

    pub fn new_array(&self) -> Value {
        Value(unsafe { ffi::JS_NewArray(self.as_ptr()) })
    }

    pub fn new_catch_offset(&self, off: i32) -> Value {
        Value(ffi::mkval(ffi::JS_TAG_CATCH_OFFSET, off))
    }

    #[cfg(feature = "bignum")]
    pub fn new_bigint64(&self, n: i64) -> Value {
        Value(unsafe { ffi::JS_NewBigInt64(self.as_ptr(), n) })
    }

    #[cfg(feature = "bignum")]
    pub fn new_biguint64(&self, n: u64) -> Value {
        Value(unsafe { ffi::JS_NewBigUint64(self.as_ptr(), n) })
    }

    pub fn to_bool(&self, val: &Value) -> Option<bool> {
        self.check_error(unsafe { ffi::JS_ToBool(self.as_ptr(), val.0) })
            .ok()
            .map(ToBool::to_bool)
    }

    pub fn to_int32(&self, val: &Value) -> Option<i32> {
        let mut n = 0;

        self.check_error(unsafe { ffi::JS_ToInt32(self.as_ptr(), &mut n, val.0) })
            .ok()
            .map(|_| n)
    }

    pub fn to_int64(&self, val: &Value) -> Option<i64> {
        let mut n = 0;

        self.check_error(unsafe { ffi::JS_ToInt64(self.as_ptr(), &mut n, val.0) })
            .ok()
            .map(|_| n)
    }

    pub fn to_index(&self, val: &Value) -> Option<u64> {
        let mut n = 0;

        self.check_error(unsafe { ffi::JS_ToIndex(self.as_ptr(), &mut n, val.0) })
            .ok()
            .map(|_| n)
    }

    pub fn to_float64(&self, val: &Value) -> Option<f64> {
        let mut n = 0.0;

        self.check_error(unsafe { ffi::JS_ToFloat64(self.as_ptr(), &mut n, val.0) })
            .ok()
            .map(|_| n)
    }

    #[cfg(feature = "bignum")]
    pub fn to_bigint64(&self, val: &Value) -> Option<i64> {
        let mut n = 0;

        self.check_error(unsafe { ffi::JS_ToBigInt64(self.as_ptr(), &mut n, val.0) })
            .ok()
            .map(|_| n)
    }

    pub fn to_str(&self, val: &Value) -> Value {
        Value(unsafe { ffi::JS_ToString(self.as_ptr(), val.0) })
    }

    pub fn to_property_key(&self, val: &Value) -> Value {
        Value(unsafe { ffi::JS_ToPropertyKey(self.as_ptr(), val.0) })
    }

    /// Convert Javascript String to C UTF-8 encoded strings.
    pub fn to_cstring(&self, val: &Value) -> Option<CString> {
        let mut len = 0;

        unsafe {
            let p = ffi::JS_ToCStringLen2(self.as_ptr(), &mut len, val.0, ffi::FALSE_VALUE);

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
        self.check_bool(unsafe { ffi::JS_IsInstanceOf(self.as_ptr(), val.raw(), obj.raw()) })
    }
}

impl<'a, T> Bindable<'a> for T
where
    T: NewValue,
{
    type Output = Value;

    fn bind_to(self, ctxt: &ContextRef) -> Self::Output {
        Value(self.new_value(ctxt))
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value(ffi::mkval(ffi::JS_TAG_BOOL, v.to_bool()))
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value(ffi::mkval(ffi::JS_TAG_INT, v))
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value(ffi::JSValue {
            u: ffi::JSValueUnion { float64: v },
            tag: ffi::JS_TAG_FLOAT64 as i64,
        })
    }
}

/// Create new `Value` from primitive.
pub trait NewValue {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue;
}

impl NewValue for bool {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        Value::from(self).0
    }
}

impl NewValue for u8 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        i32::from(self).new_value(ctxt)
    }
}

impl NewValue for u16 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        i32::from(self).new_value(ctxt)
    }
}

impl NewValue for u32 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        i64::from(self).new_value(ctxt)
    }
}

impl NewValue for u64 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        if cfg!(feature = "bignum") {
            unsafe { ffi::JS_NewBigUint64(ctxt.as_ptr(), self) }
        } else {
            (self as i64).new_value(ctxt)
        }
    }
}

impl NewValue for i8 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        i32::from(self).new_value(ctxt)
    }
}

impl NewValue for i16 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        i32::from(self).new_value(ctxt)
    }
}

impl NewValue for i32 {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        Value::from(self).0
    }
}

impl NewValue for i64 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        if cfg!(feature = "bignum") {
            unsafe { ffi::JS_NewBigInt64(ctxt.as_ptr(), self) }
        } else {
            unsafe { ffi::JS_NewInt64(ctxt.as_ptr(), self) }
        }
    }
}

impl NewValue for f32 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        f64::from(self).new_value(ctxt)
    }
}

impl NewValue for f64 {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        Value::from(self).0
    }
}

impl NewValue for String {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        self.as_str().new_value(ctxt)
    }
}

impl<'a> NewValue for &'a str {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        unsafe { ffi::JS_NewStringLen(ctxt.as_ptr(), self.as_ptr() as *const _, self.len()) }
    }
}

impl NewValue for *const c_char {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        unsafe { ffi::JS_NewString(ctxt.as_ptr(), self) }
    }
}

impl NewValue for ffi::JSValue {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        self
    }
}

impl NewValue for Value {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        self.raw()
    }
}

impl<'a> NewValue for &'a Value {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        ctxt.clone_value(self).into()
    }
}

impl<'a> NewValue for Local<'a, Value> {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        self.into_inner().into()
    }
}

impl<'a> NewValue for &'a Local<'a, Value> {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        ctxt.clone_value(self).into()
    }
}

/// Extract primitive from `Local<Value>`.
pub trait ExtractValue: Sized {
    /// Extract primitive from `Local<Value>`.
    fn extract_value(v: &Local<Value>) -> Option<Self>;
}

impl ExtractValue for () {
    fn extract_value(v: &Local<Value>) -> Option<Self> {
        if v.is_null() {
            None
        } else {
            Some(())
        }
    }
}

impl ExtractValue for bool {
    fn extract_value(v: &Local<Value>) -> Option<Self> {
        v.as_bool().or_else(|| v.to_bool())
    }
}

impl ExtractValue for i32 {
    fn extract_value(v: &Local<Value>) -> Option<Self> {
        v.as_int().or_else(|| v.to_int32())
    }
}

impl ExtractValue for i64 {
    fn extract_value(v: &Local<Value>) -> Option<Self> {
        v.as_int().map(i64::from).or_else(|| v.to_int64())
    }
}

impl ExtractValue for u64 {
    fn extract_value(v: &Local<Value>) -> Option<Self> {
        v.to_index()
    }
}

impl ExtractValue for f64 {
    fn extract_value(v: &Local<Value>) -> Option<Self> {
        v.as_float().or_else(|| v.to_float64())
    }
}

impl ExtractValue for String {
    fn extract_value(v: &Local<Value>) -> Option<Self> {
        v.to_cstring().map(|s| s.to_string_lossy().to_string())
    }
}

impl<T: ExtractValue + PartialEq> PartialEq<T> for Local<'_, Value> {
    fn eq(&self, other: &T) -> bool {
        T::extract_value(self).map_or(false, |v| v.eq(other))
    }
}

impl<T: ExtractValue + PartialOrd> PartialOrd<T> for Local<'_, Value> {
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        T::extract_value(self).and_then(|v| v.partial_cmp(other))
    }
}

pub const NAN: Value = Value(ffi::NAN);
pub const NULL: Value = Value(ffi::NULL);
pub const UNDEFINED: Value = Value(ffi::UNDEFINED);
pub const FALSE: Value = Value(ffi::FALSE);
pub const TRUE: Value = Value(ffi::TRUE);
pub const EXCEPTION: Value = Value(ffi::EXCEPTION);
pub const UNINITIALIZED: Value = Value(ffi::UNINITIALIZED);

impl Value {
    pub fn new(value: ffi::JSValue) -> Option<Self> {
        let value = Value(value);

        if value.is_undefined() {
            None
        } else {
            Some(value)
        }
    }

    pub fn raw(&self) -> ffi::JSValue {
        self.0
    }

    pub fn tag(&self) -> i32 {
        self.tag as i32
    }

    pub fn is_number(&self) -> bool {
        unsafe { ffi::JS_IsNumber(self.raw()).to_bool() }
    }

    pub fn is_integer(&self) -> bool {
        let tag = self.tag();

        tag == ffi::JS_TAG_INT || tag == ffi::JS_TAG_BIG_INT
    }

    pub fn is_float(&self) -> bool {
        let tag = self.tag();

        tag == ffi::JS_TAG_FLOAT64 || tag == ffi::JS_TAG_BIG_FLOAT
    }

    pub fn is_big_float(&self) -> bool {
        self.tag() == ffi::JS_TAG_BIG_FLOAT
    }

    pub fn is_bool(&self) -> bool {
        self.tag() == ffi::JS_TAG_BOOL
    }

    pub fn is_null(&self) -> bool {
        self.tag() == ffi::JS_TAG_NULL
    }

    pub fn is_undefined(&self) -> bool {
        self.tag() == ffi::JS_TAG_UNDEFINED
    }

    pub fn is_exception(&self) -> bool {
        self.tag() == ffi::JS_TAG_EXCEPTION
    }

    pub fn is_uninitialized(&self) -> bool {
        self.tag() == ffi::JS_TAG_UNINITIALIZED
    }

    pub fn is_symbol(&self) -> bool {
        self.tag() == ffi::JS_TAG_SYMBOL
    }

    pub fn is_string(&self) -> bool {
        self.tag() == ffi::JS_TAG_STRING
    }

    pub fn is_module(&self) -> bool {
        self.tag() == ffi::JS_TAG_MODULE
    }

    pub fn is_function_bytecode(&self) -> bool {
        self.tag() == ffi::JS_TAG_FUNCTION_BYTECODE
    }

    pub fn is_object(&self) -> bool {
        self.tag() == ffi::JS_TAG_OBJECT
    }

    pub fn as_int(&self) -> Option<i32> {
        if self.tag() == ffi::JS_TAG_INT {
            Some(unsafe { self.u.int32 })
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if self.tag() == ffi::JS_TAG_BOOL {
            Some(unsafe { self.u.int32 != 0 })
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        if self.tag() == ffi::JS_TAG_FLOAT64 {
            Some(unsafe { self.u.float64 })
        } else {
            None
        }
    }

    pub fn as_object(&self) -> Option<NonNull<ffi::JSObject>> {
        if self.tag() == ffi::JS_TAG_OBJECT {
            Some(self.as_ptr())
        } else {
            None
        }
    }

    pub fn check_undefined(&self) -> Option<&Value> {
        if self.is_undefined() {
            None
        } else {
            Some(self)
        }
    }

    pub fn as_ptr<T>(&self) -> NonNull<T> {
        unsafe { NonNull::new_unchecked(self.u.ptr).cast() }
    }

    pub fn ref_cnt(&self) -> Option<i32> {
        if self.has_ref_cnt() {
            Some(unsafe { self.as_ptr::<ffi::JSRefCountHeader>().as_ref().ref_count })
        } else {
            None
        }
    }

    fn has_ref_cnt(&self) -> bool {
        (self.tag() as u32) >= (ffi::JS_TAG_FIRST as u32)
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
