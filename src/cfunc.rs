use std::ffi::CString;
use std::os::raw::c_int;
use std::panic;
use std::ptr;
use std::slice;

use failure::Error;
use foreign_types::ForeignTypeRef;

use crate::{
    ffi::{self, JSCFunctionEnum::*},
    Args, Bindable, ContextRef, ExtractValue, LazyValue, Prop, Value,
};

/// `CFunction` is a shortcut to easily add functions, setters and getters properties to a given object.
pub type CFunction<T> = fn(&ContextRef, Option<ffi::JSValue>, &[ffi::JSValue]) -> T;

/// Unsafe C function
pub type UnsafeCFunction = unsafe extern "C" fn(
    ctx: *mut ffi::JSContext,
    this_val: ffi::JSValue,
    argc: c_int,
    argv: *mut ffi::JSValue,
) -> ffi::JSValue;

/// Unsafe C function with magic
pub type UnsafeCFunctionMagic = unsafe extern "C" fn(
    ctx: *mut ffi::JSContext,
    this_val: ffi::JSValue,
    argc: c_int,
    argv: *mut ffi::JSValue,
    magic: c_int,
) -> ffi::JSValue;

/// Unsafe C function with data
pub type UnsafeCFunctionData = unsafe extern "C" fn(
    ctx: *mut ffi::JSContext,
    this_val: ffi::JSValue,
    argc: c_int,
    argv: *mut ffi::JSValue,
    magic: c_int,
    func_data: *mut ffi::JSValue,
) -> ffi::JSValue;

/// C function definition
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
    /// Create a new C function.
    pub fn new_c_function<T: LazyValue>(
        &self,
        func: CFunction<T>,
        name: Option<&str>,
        length: usize,
    ) -> Result<Value, Error> {
        unsafe extern "C" fn stub<T: LazyValue>(
            ctx: *mut ffi::JSContext,
            this_val: ffi::JSValue,
            argc: c_int,
            argv: *mut ffi::JSValue,
            magic: c_int,
            data: *mut ffi::JSValue,
        ) -> ffi::JSValue {
            panic::catch_unwind(|| {
                let ctxt = ContextRef::from_ptr(ctx);
                let this = this_val.check_undefined();
                let args = slice::from_raw_parts(argv, argc as usize);
                let func = ctxt.get_userdata_unchecked::<CFunction<T>>(ptr::read(data));
                let func = *func.as_ref();

                trace!(
                    "call C function @ {:p} with {} args, this = {:?}, magic = {}",
                    &func,
                    args.len(),
                    this,
                    magic
                );

                func(ctxt, this, args).new_value(ctxt)
            })
            .unwrap_or_default()
        }

        trace!("new C function @ {:p}", &func);

        let data = self.new_userdata(func);
        let func = self.new_c_function_data(stub::<T>, length, 0, data.into_inner())?;

        if let Some(name) = name {
            func.define_property_value("name", name, Prop::CONFIGURABLE)?;
        }

        Ok(func)
    }

    /// Create a new C function with magic.
    pub fn new_c_function_magic(
        &self,
        func: UnsafeCFunctionMagic,
        name: Option<&str>,
        length: usize,
        cproto: CFunc,
        magic: i32,
    ) -> Result<Value, Error> {
        let name = name.map(CString::new).transpose()?;
        unsafe {
            ffi::JS_NewCFunction2(
                self.as_ptr(),
                Some(*(&func as *const _ as *const _)),
                name.map_or_else(ptr::null_mut, |s| s.as_ptr() as *mut _),
                length as i32,
                cproto as u32,
                magic,
            )
        }
        .bind(self)
        .ok()
    }

    /// Create a new C function with prototype and magic.
    pub fn new_c_function2(
        &self,
        func: UnsafeCFunction,
        name: Option<&str>,
        length: usize,
        cproto: CFunc,
        magic: i32,
    ) -> Result<Value, Error> {
        let name = name.map(CString::new).transpose()?;
        unsafe {
            ffi::JS_NewCFunction2(
                self.as_ptr(),
                Some(func),
                name.map_or_else(ptr::null_mut, |s| s.as_ptr() as *mut _),
                length as i32,
                cproto as u32,
                magic,
            )
        }
        .bind(self)
        .ok()
    }

    /// Create a new C function with magic and data.
    pub fn new_c_function_data<T: Args>(
        &self,
        func: UnsafeCFunctionData,
        length: usize,
        magic: i32,
        data: T,
    ) -> Result<Value, Error> {
        let mut data = data.into_values(self);
        let func_obj = unsafe {
            ffi::JS_NewCFunctionData(
                self.as_ptr(),
                Some(func),
                length as i32,
                magic,
                data.len() as i32,
                data.as_mut_ptr() as *mut _,
            )
        };
        for v in data {
            // self.free_value(v);
        }
        func_obj.bind(self).ok()
    }
}

macro_rules! new_func_value {
    () => {
        impl<Ret: LazyValue> LazyValue for fn() -> Ret {
            fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
                unsafe extern "C" fn stub<Ret: LazyValue>(
                    ctx: *mut ffi::JSContext,
                    _this_val: ffi::JSValue,
                    _argc: c_int,
                    _argv: *mut ffi::JSValue,
                    _magic: c_int,
                    data: *mut ffi::JSValue,
                ) -> ffi::JSValue {
                    panic::catch_unwind(|| {
                        let ctxt = ContextRef::from_ptr(ctx);
                        let func = ctxt.get_userdata_unchecked::<fn() -> Ret>(ptr::read(data));
                        let func = *func.as_ref();

                        func().new_value(ctxt).into()
                    })
                    .unwrap_or_default()
                }

                ctxt.new_c_function_data(stub::<Ret>, 0, 0, ctxt.new_userdata(self))
                    .unwrap()
                    .into_inner()
                    .into()

            }
        }
    };

    ($($Arg:ident)+) => {
        impl<Ret: LazyValue, $($Arg : ExtractValue),*> LazyValue for fn($( $Arg ),*) -> Ret {
            fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
                unsafe extern "C" fn stub<Ret: LazyValue, $($Arg : ExtractValue),*>(
                    ctx: *mut ffi::JSContext,
                    _this_val: ffi::JSValue,
                    argc: c_int,
                    argv: *mut ffi::JSValue,
                    _magic: c_int,
                    data: *mut ffi::JSValue,
                ) -> ffi::JSValue {
                    panic::catch_unwind(|| {
                        let ctxt = ContextRef::from_ptr(ctx);
                        let func = ctxt.get_userdata_unchecked::<fn($( $Arg ),*) -> Ret>(ptr::read(data));
                        let func = *func.as_ref();
                        let args = slice::from_raw_parts(argv, argc as usize);
                        let mut iter = args.iter();

                        func($({
                            let value = iter.next().unwrap().bind(ctxt);
                            <$Arg as ExtractValue>::extract_value(&value).unwrap()
                        }),*)
                            .new_value(&ctxt)
                            .into()
                    })
                    .unwrap_or_default()
                }

                ctxt.new_c_function_data(stub::<Ret, $($Arg),*>, 0, 0, ctxt.new_userdata(self))
                    .unwrap()
                    .into_inner()
                    .into()
            }
        }
    }
}

new_func_value! {}
new_func_value! { T0 }
new_func_value! { T0 T1 }
new_func_value! { T0 T1 T2 }
new_func_value! { T0 T1 T2 T3 }
new_func_value! { T0 T1 T2 T3 T4 }
new_func_value! { T0 T1 T2 T3 T4 T5 }
new_func_value! { T0 T1 T2 T3 T4 T5 T6 }
new_func_value! { T0 T1 T2 T3 T4 T5 T6 T7 }
new_func_value! { T0 T1 T2 T3 T4 T5 T6 T7 T8 }
new_func_value! { T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 }
new_func_value! { T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 }
new_func_value! { T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11 }
new_func_value! { T0 T1 T2 T3 T4 T5 T6 T7 T8 T9 T10 T11 T12 }

#[cfg(test)]
mod tests {
    use crate::{Bindable, Context, Eval, ExtractValue, Runtime};

    #[test]
    fn cfunc() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);
        let hello = ctxt
            .new_c_function(
                |ctxt, _this, args| {
                    format!(
                        "hello {}",
                        args[0].bind(ctxt).to_cstring().unwrap().to_string_lossy()
                    )
                },
                Some("hello"),
                1,
            )
            .unwrap();

        ctxt.global_object().set_property("hello", hello).unwrap();

        assert_eq!(
            ctxt.eval("hello('world')", Eval::GLOBAL).unwrap(),
            Some("hello world".to_owned())
        );
    }

    #[test]
    fn new_value() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let hello: fn(String) -> String = hello;
        // let func = ctxt.bind(hello);
        // let res = func.call(None, "world").unwrap();

        // assert_eq!(String::extract_value(&res).unwrap(), "hello world");
    }

    pub fn hello(name: String) -> String {
        format!("hello {}", name)
    }
}
