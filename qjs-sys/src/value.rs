#![allow(clippy::missing_safety_doc)]

use core::fmt;
use core::ptr::NonNull;

use crate::{
    JSContext, JSObject, JSRefCountHeader, JSRuntime, JSValue, JSValueUnion, __JS_FreeValue,
    __JS_FreeValueRT,
};

pub const TRUE_VALUE: i32 = 1;
pub const FALSE_VALUE: i32 = 0;

pub use crate::_bindgen_ty_1::*;

#[macro_export]
macro_rules! JS_VALUE_GET_TAG {
    ($v:expr) => {
        (*$v).tag as crate::_bindgen_ty_1::Type
    };
}

#[macro_export]
macro_rules! JS_VALUE_GET_INT {
    ($v:expr) => {
        (*$v).u.int32
    };
}

#[macro_export]
macro_rules! JS_VALUE_GET_BOOL {
    ($v:expr) => {
        (*$v).u.int32 != 0
    };
}

#[macro_export]
macro_rules! JS_VALUE_GET_FLOAT64 {
    ($v:expr) => {
        (*$v).u.float64
    };
}

#[macro_export]
macro_rules! JS_VALUE_GET_PTR {
    ($v:expr) => {
        (*$v).u.ptr
    };
}

#[macro_export]
macro_rules! JS_MKVAL {
    ($tag:ident, $val:expr) => {
        JSValue {
            tag: $tag as i64,
            u: JSValueUnion { int32: $val },
        }
    };
}

#[macro_export]
macro_rules! JS_MKPTR {
    ($tag:ident, $p:expr) => {
        JSValue {
            tag: $tag as i64,
            u: JSValueUnion { ptr: $p as *mut _ },
        }
    };
}

macro_rules! JS_VALUE_HAS_REF_COUNT {
    ($v:expr) => {
        $v.tag as crate::_bindgen_ty_1::Type >= $crate::_bindgen_ty_1::JS_TAG_FIRST
    };
}

pub const NAN: JSValue = JSValue {
    u: JSValueUnion {
        float64: std::f64::NAN,
    },
    tag: JS_TAG_FLOAT64 as i64,
};

pub const NULL: JSValue = JS_MKVAL!(JS_TAG_NULL, 0);
pub const UNDEFINED: JSValue = JS_MKVAL!(JS_TAG_UNDEFINED, 0);
pub const FALSE: JSValue = JS_MKVAL!(JS_TAG_BOOL, FALSE_VALUE);
pub const TRUE: JSValue = JS_MKVAL!(JS_TAG_BOOL, TRUE_VALUE);
pub const EXCEPTION: JSValue = JS_MKVAL!(JS_TAG_EXCEPTION, 0);
pub const UNINITIALIZED: JSValue = JS_MKVAL!(JS_TAG_UNINITIALIZED, 0);

#[inline(always)]
pub fn JS_NewBool(_ctx: *mut JSContext, val: bool) -> JSValue {
    JS_MKVAL!(JS_TAG_BOOL, if val { TRUE_VALUE } else { FALSE_VALUE })
}

#[inline(always)]
pub fn JS_NewInt32(_ctx: *mut JSContext, val: i32) -> JSValue {
    JS_MKVAL!(JS_TAG_INT, val)
}

#[inline(always)]
pub fn JS_NewCatchOffset(_ctx: *mut JSContext, val: i32) -> JSValue {
    JS_MKVAL!(JS_TAG_CATCH_OFFSET, val)
}

#[inline(always)]
pub fn JS_NewInt64(ctx: *mut JSContext, val: i64) -> JSValue {
    if val as i32 as i64 == val {
        JS_MKVAL!(JS_TAG_INT, val as i32)
    } else {
        __JS_NewFloat64(ctx, val as f64)
    }
}

#[inline(always)]
pub fn JS_NewUint32(ctx: *mut JSContext, val: u32) -> JSValue {
    if val <= 0x7fffffff {
        JS_MKVAL!(JS_TAG_INT, val as i32)
    } else {
        __JS_NewFloat64(ctx, val as f64)
    }
}

#[inline(always)]
pub fn JS_NewFloat64(ctx: *mut JSContext, val: f64) -> JSValue {
    if val as i32 as u64 == val as u64 {
        JS_MKVAL!(JS_TAG_INT, val as i32)
    } else {
        __JS_NewFloat64(ctx, val)
    }
}

#[inline(always)]
fn __JS_NewFloat64(_ctx: *mut JSContext, val: f64) -> JSValue {
    JSValue {
        tag: JS_TAG_FLOAT64 as i64,
        u: JSValueUnion { float64: val },
    }
}

#[allow(clippy::deref_addrof)]
#[inline(always)]
pub unsafe fn JS_FreeValue(ctx: *mut JSContext, v: JSValue) {
    if JS_VALUE_HAS_REF_COUNT!(v) {
        let hdr = JS_VALUE_GET_PTR!(&v).cast::<JSRefCountHeader>();

        (*hdr).ref_count -= 1;

        if (*hdr).ref_count <= 0 {
            __JS_FreeValue(ctx, v)
        }
    }
}

#[allow(clippy::deref_addrof)]
#[inline(always)]
pub unsafe fn JS_FreeValueRT(rt: *mut JSRuntime, v: JSValue) {
    if JS_VALUE_HAS_REF_COUNT!(v) {
        let hdr = JS_VALUE_GET_PTR!(&v).cast::<JSRefCountHeader>();

        (*hdr).ref_count -= 1;

        if (*hdr).ref_count <= 0 {
            __JS_FreeValueRT(rt, v)
        }
    }
}

#[inline(always)]
pub unsafe fn JS_DupValue(_ctx: *mut JSContext, v: *const JSValue) -> JSValue {
    if JS_VALUE_HAS_REF_COUNT!(*v) {
        let hdr = JS_VALUE_GET_PTR!(v).cast::<JSRefCountHeader>();

        (*hdr).ref_count += 1;
    }
    *v
}

#[inline(always)]
pub unsafe fn JS_DupValueRT(_rt: *mut JSRuntime, v: *const JSValue) -> JSValue {
    if JS_VALUE_HAS_REF_COUNT!(*v) {
        let hdr = JS_VALUE_GET_PTR!(v).cast::<JSRefCountHeader>();

        (*hdr).ref_count += 1;
    }
    *v
}

impl Default for JSValue {
    fn default() -> JSValue {
        UNDEFINED
    }
}

impl Default for &JSValue {
    fn default() -> &'static JSValue {
        &UNDEFINED
    }
}

impl fmt::Debug for JSValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            match self.tag() {
                JS_TAG_BIG_INT => f.debug_tuple("BigInt").field(&self.u.ptr).finish(),
                JS_TAG_BIG_FLOAT => f.debug_tuple("BigFloat").field(&self.u.ptr).finish(),
                JS_TAG_SYMBOL => f.debug_tuple("Symbol").field(&self.u.ptr).finish(),
                JS_TAG_STRING => f.debug_tuple("String").field(&self.u.ptr).finish(),
                JS_TAG_MODULE => f.debug_tuple("Module").field(&self.u.ptr).finish(),
                JS_TAG_FUNCTION_BYTECODE => f.debug_tuple("Function").field(&self.u.ptr).finish(),
                JS_TAG_OBJECT => f.debug_tuple("Object").field(&self.u.ptr).finish(),
                JS_TAG_INT => f.debug_tuple("Value").field(&self.u.int32).finish(),
                JS_TAG_BOOL => f
                    .debug_tuple("Value")
                    .field(&(self.u.int32 != FALSE_VALUE))
                    .finish(),
                JS_TAG_NULL => f.write_str("Null"),
                JS_TAG_UNDEFINED => f.write_str("Undefined"),
                JS_TAG_UNINITIALIZED => f.write_str("Uninitialized"),
                JS_TAG_CATCH_OFFSET => f.debug_tuple("CatchOffset").field(&self.u.int32).finish(),
                JS_TAG_EXCEPTION => f.write_str("Exception"),
                JS_TAG_FLOAT64 => f.debug_tuple("Value").field(&self.u.float64).finish(),
                tag => f.debug_struct("Value").field("tag", &tag).finish(),
            }
        }
    }
}

#[inline(always)]
pub unsafe fn JS_IsNumber(v: *const JSValue) -> bool {
    let tag = JS_VALUE_GET_TAG!(v);

    tag == JS_TAG_INT || tag == JS_TAG_FLOAT64
}

#[inline(always)]
pub unsafe fn JS_IsBigInt(_ctx: *mut JSContext, v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_BIG_INT
}

#[inline(always)]
pub unsafe fn JS_IsBigFloat(v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_BIG_FLOAT
}

#[inline(always)]
pub unsafe fn JS_IsBigDecimal(v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_BIG_DECIMAL
}

#[inline(always)]
pub unsafe fn JS_IsBool(v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_BOOL
}

#[inline(always)]
pub unsafe fn JS_IsNull(v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_NULL
}

#[inline(always)]
pub unsafe fn JS_IsUndefined(v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_UNDEFINED
}

#[inline(always)]
pub unsafe fn JS_IsException(v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_EXCEPTION
}

#[inline(always)]
pub unsafe fn JS_IsUninitialized(v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_UNINITIALIZED
}

#[inline(always)]
pub unsafe fn JS_IsString(v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_STRING
}

#[inline(always)]
pub unsafe fn JS_IsSymbol(v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_SYMBOL
}

#[inline(always)]
pub unsafe fn JS_IsModule(v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_MODULE
}

#[inline(always)]
pub unsafe fn JS_IsFunctionByteCode(v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_FUNCTION_BYTECODE
}

#[inline(always)]
pub unsafe fn JS_IsObject(v: *const JSValue) -> bool {
    JS_VALUE_GET_TAG!(v) == JS_TAG_OBJECT
}

impl JSValue {
    pub fn check_undefined(self) -> Option<Self> {
        if self.is_undefined() {
            None
        } else {
            Some(self)
        }
    }

    pub fn tag(&self) -> i32 {
        self.tag as i32
    }

    pub fn is_number(&self) -> bool {
        unsafe { JS_IsNumber(self) }
    }

    pub fn is_big_float(&self) -> bool {
        unsafe { JS_IsBigFloat(self) }
    }

    pub fn is_bool(&self) -> bool {
        unsafe { JS_IsBool(self) }
    }

    pub fn is_null(&self) -> bool {
        unsafe { JS_IsNull(self) }
    }

    pub fn is_undefined(&self) -> bool {
        unsafe { JS_IsUndefined(self) }
    }

    pub fn is_exception(&self) -> bool {
        unsafe { JS_IsException(self) }
    }

    pub fn is_uninitialized(&self) -> bool {
        unsafe { JS_IsUninitialized(self) }
    }

    pub fn is_symbol(&self) -> bool {
        unsafe { JS_IsSymbol(self) }
    }

    pub fn is_string(&self) -> bool {
        unsafe { JS_IsString(self) }
    }

    pub fn is_module(&self) -> bool {
        unsafe { JS_IsModule(self) }
    }

    pub fn is_function_bytecode(&self) -> bool {
        unsafe { JS_IsFunctionByteCode(self) }
    }

    pub fn is_object(&self) -> bool {
        unsafe { JS_IsObject(self) }
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

    pub fn as_object(&self) -> Option<NonNull<JSObject>> {
        if self.tag() == JS_TAG_OBJECT {
            Some(self.as_ptr())
        } else {
            None
        }
    }

    pub fn as_ptr<T>(&self) -> NonNull<T> {
        unsafe { NonNull::new_unchecked(self.u.ptr).cast() }
    }

    pub fn ref_cnt(&self) -> Option<i32> {
        if JS_VALUE_HAS_REF_COUNT!(self) {
            Some(unsafe { self.as_ptr::<JSRefCountHeader>().as_ref().ref_count })
        } else {
            None
        }
    }
}
