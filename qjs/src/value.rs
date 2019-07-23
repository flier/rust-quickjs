use std::borrow::Cow;
use std::ffi::CStr;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::os::raw::c_char;
use std::ptr::NonNull;
use std::slice;

use failure::Error;
use foreign_types::ForeignTypeRef;

pub use crate::ffi::_bindgen_ty_1::*;
use crate::{
    ffi,
    handle::{Bindable, Unbindable},
    ClassId, ContextRef, Local, RuntimeRef,
};

pub const ERR: i32 = -1;
pub const TRUE: i32 = 1;
pub const FALSE: i32 = 0;

#[repr(transparent)]
pub struct Value(ffi::JSValue);

impl Unbindable for Value {
    fn unbind(ctxt: &ContextRef, inner: Self) {
        ctxt.free_value(inner)
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            match self.tag() {
                JS_TAG_INT => f.debug_tuple("Value").field(&self.u.int32).finish(),
                JS_TAG_FLOAT64 => f.debug_tuple("Value").field(&self.u.float64).finish(),
                JS_TAG_BOOL => f
                    .debug_tuple("Value")
                    .field(&(self.u.int32 != FALSE))
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
        f.write_str(&self.to_cstr().unwrap().to_string_lossy())
    }
}

impl fmt::Debug for Local<'_, Value> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Value")
            .field(&self.to_cstr().unwrap().to_string_lossy())
            .finish()
    }
}

impl<'a> Local<'a, Value> {
    pub fn check_undefined(self) -> Option<Local<'a, Value>> {
        if self.inner.is_undefined() {
            None
        } else {
            Some(self)
        }
    }

    pub fn free(mut self) {
        let v = self.take();

        self.ctxt.free_value(v)
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

    pub fn to_string(&self) -> Local<Value> {
        self.ctxt.bind(self.ctxt.to_string(&self.inner))
    }

    pub fn to_property_key(&self) -> Local<Value> {
        self.ctxt.bind(self.ctxt.to_property_key(&self.inner))
    }

    pub fn to_cstr(&self) -> Option<Local<&CStr>> {
        self.ctxt.to_cstr(&self.inner)
    }

    pub fn to_str(&self) -> Option<Cow<str>> {
        self.to_cstr().map(|s| s.to_string_lossy())
    }

    pub fn instance_of(&self, obj: &Value) -> Result<bool, Error> {
        self.ctxt.is_instance_of(&self.inner, obj)
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
        Value(mkval(JS_TAG_CATCH_OFFSET, off))
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

    pub fn to_property_key(&self, val: &Value) -> Value {
        Value(unsafe { ffi::JS_ToPropertyKey(self.as_ptr(), val.0) })
    }

    pub fn to_cstr(&self, val: &Value) -> Option<Local<&CStr>> {
        let mut len = 0;

        unsafe {
            let p = ffi::JS_ToCStringLen(self.as_ptr(), &mut len, val.0, FALSE);

            if p.is_null() {
                None
            } else {
                Some(
                    self.bind(CStr::from_bytes_with_nul_unchecked(slice::from_raw_parts(
                        p as *const _,
                        len as usize + 1,
                    ))),
                )
            }
        }
    }

    pub fn is_instance_of(&self, val: &Value, obj: &Value) -> Result<bool, Error> {
        self.check_bool(unsafe { ffi::JS_IsInstanceOf(self.as_ptr(), val.raw(), obj.raw()) })
    }
}

impl<'a> Bindable<'a> for &'a CStr {
    type Output = &'a CStr;

    fn bind_to(self, _ctxt: &ContextRef) -> Self::Output {
        self
    }
}

impl Unbindable for &CStr {
    fn unbind(ctxt: &ContextRef, s: &CStr) {
        unsafe { ffi::JS_FreeCString(ctxt.as_ptr(), s.as_ptr()) }
    }
}

impl fmt::Display for Local<'_, &CStr> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
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

pub trait NewValue {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue;
}

impl<T> NewValue for &T
where
    T: NewValue + Clone,
{
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        self.clone().new_value(ctxt)
    }
}

impl NewValue for bool {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        mkval(JS_TAG_BOOL, if self { TRUE } else { FALSE })
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
        (self as i64).new_value(ctxt)
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
        mkval(JS_TAG_INT, self)
    }
}

impl NewValue for i64 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        unsafe { ffi::JS_NewInt64(ctxt.as_ptr(), self) }
    }
}

impl NewValue for f32 {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        f64::from(self).new_value(ctxt)
    }
}

impl NewValue for f64 {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        ffi::JSValue {
            u: ffi::JSValueUnion { float64: self },
            tag: JS_TAG_FLOAT64 as i64,
        }
    }
}

impl NewValue for String {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        self.as_str().new_value(ctxt)
    }
}

impl<'a> NewValue for &'a str {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        unsafe { ffi::JS_NewStringLen(ctxt.as_ptr(), self.as_ptr() as *const _, self.len() as i32) }
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

const fn mkval(tag: i32, val: i32) -> ffi::JSValue {
    ffi::JSValue {
        tag: tag as i64,
        u: ffi::JSValueUnion { int32: val },
    }
}

#[allow(dead_code)]
const fn mkptr<T>(tag: i32, val: *mut T) -> ffi::JSValue {
    ffi::JSValue {
        tag: tag as i64,
        u: ffi::JSValueUnion { ptr: val as *mut _ },
    }
}

impl Value {
    pub fn new<T: NewValue>(ctxt: &ContextRef, v: T) -> Local<Self> {
        ctxt.bind(v.new_value(ctxt))
    }

    pub const fn nan() -> Self {
        Value(ffi::JSValue {
            u: ffi::JSValueUnion {
                float64: std::f64::NAN,
            },
            tag: JS_TAG_FLOAT64 as i64,
        })
    }

    pub const fn null() -> Self {
        Value(mkval(JS_TAG_NULL, 0))
    }

    pub const fn undefined() -> Self {
        Value(mkval(JS_TAG_UNDEFINED, 0))
    }

    pub const fn false_value() -> Self {
        Value(mkval(JS_TAG_BOOL, FALSE))
    }

    pub const fn true_value() -> Self {
        Value(mkval(JS_TAG_BOOL, TRUE))
    }

    pub const fn exception() -> Self {
        Value(mkval(JS_TAG_EXCEPTION, 0))
    }

    pub const fn uninitialized() -> Self {
        Value(mkval(JS_TAG_UNINITIALIZED, 0))
    }

    pub fn raw(&self) -> ffi::JSValue {
        self.0
    }

    pub fn tag(&self) -> i32 {
        self.tag as i32
    }

    pub fn is_number(&self) -> bool {
        unsafe { ffi::JS_IsNumber(self.raw()) != FALSE }
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
            Some(unsafe { self.u.int32 })
        } else {
            None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if self.tag() == JS_TAG_BOOL {
            Some(unsafe { self.u.int32 != 0 })
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        if self.tag() == JS_TAG_FLOAT64 {
            Some(unsafe { self.u.float64 })
        } else {
            None
        }
    }

    pub fn as_object(&self) -> Option<NonNull<ffi::JSObject>> {
        if self.tag() == JS_TAG_OBJECT {
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

    fn has_ref_cnt(&self) -> bool {
        (self.tag() as u32) >= (JS_TAG_FIRST as u32)
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
            .eval(
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
