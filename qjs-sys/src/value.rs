use core::fmt;
use core::ptr::NonNull;

use crate::{JSContext, JSObject, JSRefCountHeader, JSValue, JSValueUnion};

pub const TRUE_VALUE: i32 = 1;
pub const FALSE_VALUE: i32 = 0;

pub use crate::_bindgen_ty_1::*;

#[inline(always)]
const fn mkval(tag: i32, val: i32) -> JSValue {
    JSValue {
        tag: tag as i64,
        u: JSValueUnion { int32: val },
    }
}

#[inline(always)]
const fn mkptr<T>(tag: i32, val: *mut T) -> JSValue {
    JSValue {
        tag: tag as i64,
        u: JSValueUnion { ptr: val as *mut _ },
    }
}

pub const NAN: JSValue = JSValue {
    u: JSValueUnion {
        float64: std::f64::NAN,
    },
    tag: JS_TAG_FLOAT64 as i64,
};

pub const NULL: JSValue = mkval(JS_TAG_NULL, 0);
pub const UNDEFINED: JSValue = mkval(JS_TAG_UNDEFINED, 0);
pub const FALSE: JSValue = mkval(JS_TAG_BOOL, FALSE_VALUE);
pub const TRUE: JSValue = mkval(JS_TAG_BOOL, TRUE_VALUE);
pub const EXCEPTION: JSValue = mkval(JS_TAG_EXCEPTION, 0);
pub const UNINITIALIZED: JSValue = mkval(JS_TAG_UNINITIALIZED, 0);

#[inline(always)]
pub fn JS_NewBool(_ctx: *mut JSContext, val: bool) -> JSValue {
    mkval(JS_TAG_BOOL, if val { TRUE_VALUE } else { FALSE_VALUE })
}

#[inline(always)]
pub fn JS_NewInt32(_ctx: *mut JSContext, val: i32) -> JSValue {
    mkval(JS_TAG_INT, val)
}

#[inline(always)]
pub fn JS_NewCatchOffset(_ctx: *mut JSContext, val: i32) -> JSValue {
    mkval(JS_TAG_CATCH_OFFSET, val)
}

#[inline(always)]
pub fn JS_NewInt64(ctx: *mut JSContext, val: i64) -> JSValue {
    if val as i32 as i64 == val {
        mkval(JS_TAG_INT, val as i32)
    } else {
        __JS_NewFloat64(ctx, val as f64)
    }
}

#[inline(always)]
pub fn JS_NewUint32(ctx: *mut JSContext, val: u32) -> JSValue {
    if val <= 0x7fffffff {
        mkval(JS_TAG_INT, val as i32)
    } else {
        __JS_NewFloat64(ctx, val as f64)
    }
}

#[inline(always)]
pub fn JS_NewFloat64(ctx: *mut JSContext, val: f64) -> JSValue {
    if val as i32 as u64 == val as u64 {
        mkval(JS_TAG_INT, val as i32)
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

    pub fn is_integer(&self) -> bool {
        let tag = self.tag();

        tag == JS_TAG_INT || tag == JS_TAG_BIG_INT
    }

    pub fn is_number(&self) -> bool {
        let tag = self.tag();

        tag == JS_TAG_INT || tag == JS_TAG_FLOAT64
    }

    pub fn is_float(&self) -> bool {
        let tag = self.tag();

        tag == JS_TAG_FLOAT64 || tag == JS_TAG_BIG_FLOAT
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

    pub fn is_symbol(&self) -> bool {
        self.tag() == JS_TAG_SYMBOL
    }

    pub fn is_string(&self) -> bool {
        self.tag() == JS_TAG_STRING
    }

    pub fn is_module(&self) -> bool {
        self.tag() == JS_TAG_MODULE
    }

    pub fn is_function_bytecode(&self) -> bool {
        self.tag() == JS_TAG_FUNCTION_BYTECODE
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
        if self.has_ref_cnt() {
            Some(unsafe { self.as_ptr::<JSRefCountHeader>().as_ref().ref_count })
        } else {
            None
        }
    }

    pub fn has_ref_cnt(&self) -> bool {
        (self.tag() as u32) >= (JS_TAG_FIRST as u32)
    }
}
