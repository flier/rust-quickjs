#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::unreadable_literal)]

#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate lazy_static;

cfg_if! {
    if #[cfg(feature = "gen")] {
        include!(concat!(env!("OUT_DIR"), "/raw.rs"));
    } else {
        include!("raw.rs");
    }
}

lazy_static! {
    pub static ref VERSION: &'static str =
        include_str!(concat!(env!("OUT_DIR"), "/VERSION")).trim();
}

cfg_if! {
    if #[cfg(feature = "repl")] {
        extern "C" {
            #[no_mangle]
            pub static qjsc_repl: [u8; 0];

            #[no_mangle]
            pub static qjsc_repl_size: u32;
        }

        lazy_static! {
            pub static ref REPL: &'static [u8] = unsafe {
                std::slice::from_raw_parts(qjsc_repl.as_ptr(), qjsc_repl_size as usize)
            };
        }
    }
}

cfg_if! {
    if #[cfg(feature = "qjscalc")] {
        extern "C" {
            #[no_mangle]
            pub static qjsc_qjscalc: [u8; 0];

            #[no_mangle]
            pub static qjsc_qjscalc_size: u32;
        }

        lazy_static! {
            pub static ref QJSCALC: &'static [u8] = unsafe {
                std::slice::from_raw_parts(qjsc_qjscalc.as_ptr(), qjsc_qjscalc_size as usize)
            };
        }
    }
}

pub const TRUE_VALUE: i32 = 1;
pub const FALSE_VALUE: i32 = 0;

pub use crate::_bindgen_ty_1::*;

pub const fn mkval(tag: i32, val: i32) -> JSValue {
    JSValue {
        tag: tag as i64,
        u: JSValueUnion { int32: val },
    }
}

pub const fn mkptr<T>(tag: i32, val: *mut T) -> JSValue {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime() {
        let rt = unsafe { JS_NewRuntime() };

        assert!(!rt.is_null());

        unsafe {
            JS_FreeRuntime(rt);
        }
    }
}
