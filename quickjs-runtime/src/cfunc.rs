use std::ffi::CString;
use std::mem;
use std::os::raw::c_int;
use std::ptr;

use failure::Error;
use foreign_types::ForeignTypeRef;

use crate::{
    ffi::{self, JSCFunctionEnum::*},
    Args, ContextRef, Local, Value,
};

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CFunc {
    Generic = JS_CFUNC_generic,
    GenericMagic = JS_CFUNC_generic_magic,
    Constructor = JS_CFUNC_constructor,
    ConstructorMagic = JS_CFUNC_constructor_magic,
    ConstructorOrFunc = JS_CFUNC_constructor_or_func,
    ConstructorOrFuncMagic = JS_CFUNC_constructor_or_func_magic,
    FloatFloat = JS_CFUNC_f_f,
    FloatFloatFloat = JS_CFUNC_f_f_f,
    Getter = JS_CFUNC_getter,
    Setter = JS_CFUNC_setter,
    GetterMagic = JS_CFUNC_getter_magic,
    SetterMagic = JS_CFUNC_setter_magic,
    IteratorNext = JS_CFUNC_iterator_next,
}

impl ContextRef {
    pub fn new_c_function<F>(
        &self,
        func: F,
        name: Option<&str>,
        length: usize,
    ) -> Result<Local<Value>, Error> {
        self.new_c_function2(func, name, length, CFunc::Generic, 0)
    }

    pub fn new_c_function_magic<F>(
        &self,
        func: F,
        name: Option<&str>,
        length: usize,
        cproto: CFunc,
        magic: i32,
    ) -> Result<Local<Value>, Error> {
        unsafe extern "C" fn stub(
            ctx: *mut ffi::JSContext,
            this_val: ffi::JSValue,
            argc: c_int,
            argv: *mut ffi::JSValue,
            magic: c_int,
        ) -> ffi::JSValue {
            Value::undefined().into()
        }

        let name = name.map(CString::new).transpose()?;
        self.bind(unsafe {
            ffi::JS_NewCFunction2(
                self.as_ptr(),
                Some(*(&stub as *const _ as *const _)),
                name.map_or_else(ptr::null_mut, |s| s.as_ptr() as *mut _),
                length as i32,
                cproto as u32,
                magic,
            )
        })
        .ok()
    }

    pub fn new_c_function2<F>(
        &self,
        func: F,
        name: Option<&str>,
        length: usize,
        cproto: CFunc,
        magic: i32,
    ) -> Result<Local<Value>, Error> {
        unsafe extern "C" fn stub(
            ctx: *mut ffi::JSContext,
            this: ffi::JSValue,
            argc: c_int,
            argv: *mut ffi::JSValue,
        ) -> ffi::JSValue {
            Value::undefined().into()
        }

        let name = name.map(CString::new).transpose()?;
        self.bind(unsafe {
            ffi::JS_NewCFunction2(
                self.as_ptr(),
                Some(stub),
                name.map_or_else(ptr::null_mut, |s| s.as_ptr() as *mut _),
                length as i32,
                cproto as u32,
                magic,
            )
        })
        .ok()
    }

    pub fn new_c_function_data<F, T: Args>(
        &self,
        func: F,
        length: usize,
        magic: i32,
        data: T,
    ) -> Result<Local<Value>, Error> {
        unsafe extern "C" fn stub(
            ctx: *mut ffi::JSContext,
            this: ffi::JSValue,
            argc: c_int,
            argv: *mut ffi::JSValue,
            magic: c_int,
            func_data: *mut ffi::JSValue,
        ) -> ffi::JSValue {
            Value::undefined().into()
        }

        let data = data.into_values(self);
        let data = data.as_ref();

        self.bind(unsafe {
            ffi::JS_NewCFunctionData(
                self.as_ptr(),
                Some(stub),
                length as i32,
                magic,
                data.len() as i32,
                data.as_ptr() as *mut _,
            )
        })
        .ok()
    }
}
