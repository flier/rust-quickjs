use std::ffi::CString;
use std::mem;
use std::os::raw::c_int;
use std::ptr;
use std::slice;

use failure::Error;
use foreign_types::ForeignTypeRef;

use crate::{
    ffi::{self, JSCFunctionEnum::*},
    Args, ContextRef, Local, NewValue, Value,
};

pub type CFunction<T> = fn(&ContextRef, Option<&Value>, &[Value]) -> T;

pub type UnsafeCFunction = unsafe extern "C" fn(
    ctx: *mut ffi::JSContext,
    this_val: ffi::JSValue,
    argc: c_int,
    argv: *mut ffi::JSValue,
) -> ffi::JSValue;
pub type UnsafeCFunctionMagic = unsafe extern "C" fn(
    ctx: *mut ffi::JSContext,
    this_val: ffi::JSValue,
    argc: c_int,
    argv: *mut ffi::JSValue,
    magic: c_int,
) -> ffi::JSValue;
pub type UnsafeCFunctionData = unsafe extern "C" fn(
    ctx: *mut ffi::JSContext,
    this_val: ffi::JSValue,
    argc: c_int,
    argv: *mut ffi::JSValue,
    magic: c_int,
    func_data: *mut ffi::JSValue,
) -> ffi::JSValue;

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
    pub fn new_c_function<T: NewValue>(
        &self,
        func: CFunction<T>,
        name: Option<&str>,
        length: usize,
    ) -> Result<Local<Value>, Error> {
        unsafe extern "C" fn stub<T: NewValue>(
            ctx: *mut ffi::JSContext,
            this_val: ffi::JSValue,
            argc: c_int,
            argv: *mut ffi::JSValue,
            magic: c_int,
            data: *mut ffi::JSValue,
        ) -> ffi::JSValue {
            let ctxt = ContextRef::from_ptr(ctx);
            let this = Value::from(this_val);
            let this = this.check_undefined();
            let args = slice::from_raw_parts(argv, argc as usize);
            let data = ptr::NonNull::new_unchecked(data);
            let func = ctxt.get_userdata_unchecked::<CFunction<T>>(data.cast().as_ref());
            let func = *func.as_ref();

            trace!(
                "call C function @ {:p} with {} args, this = {:?}, magic = {}",
                &func,
                args.len(),
                this,
                magic
            );

            func(ctxt, this, mem::transmute(args)).new_value(ctxt)
        }

        trace!("new C function @ {:p}", &func);

        let func = self.new_c_function_data(stub::<T>, length, 0, self.new_userdata(func))?;

        if let Some(name) = name {}

        Ok(func)
    }

    pub fn new_c_function_magic(
        &self,
        func: UnsafeCFunctionMagic,
        name: Option<&str>,
        length: usize,
        cproto: CFunc,
        magic: i32,
    ) -> Result<Local<Value>, Error> {
        let name = name.map(CString::new).transpose()?;
        self.bind(unsafe {
            ffi::JS_NewCFunction2(
                self.as_ptr(),
                Some(*(&func as *const _ as *const _)),
                name.map_or_else(ptr::null_mut, |s| s.as_ptr() as *mut _),
                length as i32,
                cproto as u32,
                magic,
            )
        })
        .ok()
    }

    pub fn new_c_function2(
        &self,
        func: UnsafeCFunction,
        name: Option<&str>,
        length: usize,
        cproto: CFunc,
        magic: i32,
    ) -> Result<Local<Value>, Error> {
        let name = name.map(CString::new).transpose()?;
        self.bind(unsafe {
            ffi::JS_NewCFunction2(
                self.as_ptr(),
                Some(func),
                name.map_or_else(ptr::null_mut, |s| s.as_ptr() as *mut _),
                length as i32,
                cproto as u32,
                magic,
            )
        })
        .ok()
    }

    pub fn new_c_function_data<T: Args>(
        &self,
        func: UnsafeCFunctionData,
        length: usize,
        magic: i32,
        data: T,
    ) -> Result<Local<Value>, Error> {
        let data = data.into_values(self);
        let data = data.as_ref();
        let func_obj = unsafe {
            ffi::JS_NewCFunctionData(
                self.as_ptr(),
                Some(func),
                length as i32,
                magic,
                data.len() as i32,
                data.as_ptr() as *mut _,
            )
        };
        for v in data {
            self.free_value(*v);
        }
        self.bind(func_obj).ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, Eval, Runtime};

    #[test]
    fn cfunc() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);
        let hello = ctxt
            .new_c_function(
                |ctxt, _this, args| format!("hello {}", ctxt.to_cstr(&args[0]).unwrap()),
                Some("hello"),
                1,
            )
            .unwrap();

        ctxt.global_object().set_property("hello", hello).unwrap();

        assert_eq!(
            ctxt.eval("hello('world')", "<evalScript>", Eval::GLOBAL)
                .unwrap()
                .to_str()
                .unwrap(),
            "hello world"
        );
    }
}
