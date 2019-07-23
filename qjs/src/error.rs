use std::convert::TryFrom;
use std::ffi::CString;

use failure::{err_msg, Error};
use foreign_types::ForeignTypeRef;

use crate::{
    ffi,
    value::{ERR, FALSE, TRUE},
    ContextRef, Local, NewValue, Prop, Value,
};

#[derive(Debug, Clone, Fail, PartialEq)]
pub enum ErrorKind {
    #[fail(display = "Throw: {}", _0)]
    Throw(String),

    #[fail(display = "Error: {}", _0)]
    Error(String),

    #[fail(display = "{}: {}", _0, _1)]
    Custom(String, String),

    /// an error that occurs regarding the global function eval().
    #[fail(display = "EvalError: {}", _0)]
    EvalError(String),

    /// an error that occurs when an internal error in the JavaScript engine is thrown.
    #[fail(display = "InternalError: {}", _0)]
    InternalError(String),

    /// an error that occurs when a numeric variable or parameter is outside of its valid range.
    #[fail(display = "RangeError: {}", _0)]
    RangeError(String),

    /// an error that occurs when de-referencing an invalid reference.
    #[fail(display = "ReferenceError: {}", _0)]
    ReferenceError(String),

    /// a syntax error that occurs while parsing code in eval().
    #[fail(display = "SyntaxError: {}", _0)]
    SyntaxError(String),

    /// an error that occurs when a variable or parameter is not of a valid type.
    #[fail(display = "TypeError: {}", _0)]
    TypeError(String),

    /// an error that occurs when encodeURI() or decodeURI() are passed invalid parameters.
    #[fail(display = "URIError: {}", _0)]
    URIError(String),
}

impl TryFrom<Local<'_, Value>> for ErrorKind {
    type Error = Error;

    fn try_from(
        value: Local<'_, Value>,
    ) -> Result<Self, <Self as TryFrom<Local<'_, Value>>>::Error> {
        use ErrorKind::*;

        Ok(if value.is_error() {
            let name = value
                .get_property("name")
                .ok_or_else(|| err_msg("missing `name` property"))?;
            let name = name
                .to_cstr()
                .ok_or_else(|| err_msg("invalid `name` property"))?;
            let msg = value
                .get_property("message")
                .ok_or_else(|| err_msg("missing `message` property"))?;
            let msg = msg
                .to_cstr()
                .ok_or_else(|| err_msg("invalid `message` property"))?
                .to_string_lossy()
                .to_string();

            match &*name.to_string_lossy() {
                "EvalError" => EvalError(msg),
                "InternalError" => InternalError(msg),
                "RangeError" => RangeError(msg),
                "ReferenceError" => ReferenceError(msg),
                "SyntaxError" => SyntaxError(msg),
                "TypeError" => TypeError(msg),
                "URIError" => URIError(msg),
                "Error" => Error(msg),
                _ => Custom(name.to_string_lossy().to_string(), msg),
            }
        } else {
            let msg = value
                .to_cstr()
                .ok_or_else(|| err_msg("invalid value"))?
                .to_string_lossy()
                .to_string();

            Throw(msg)
        })
    }
}

impl NewValue for Result<Local<'_, Value>, Error> {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        match self {
            Ok(v) => v,
            Err(err) => {
                if let Some(err) = err.downcast_ref::<ErrorKind>() {
                    ctxt.throw(err)
                } else {
                    ctxt.throw(err.to_string())
                }
            }
        }
        .into_inner()
        .raw()
    }
}

impl NewValue for ErrorKind {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        use ErrorKind::*;

        match self {
            Throw(msg) => ctxt.throw(msg),
            Error(msg) => ctxt.throw_error(msg),
            Custom(name, msg) => ctxt.throw_custom_error(&name, msg),
            EvalError(msg) => ctxt.throw_custom_error("EvalError", msg),
            InternalError(msg) => ctxt.throw_internal_error(msg),
            RangeError(msg) => ctxt.throw_range_error(msg),
            ReferenceError(msg) => ctxt.throw_reference_error(msg),
            SyntaxError(msg) => ctxt.throw_syntax_error(msg),
            TypeError(msg) => ctxt.throw_type_error(msg),
            URIError(msg) => ctxt.throw_custom_error("URIError", msg),
        }
        .into_inner()
        .raw()
    }
}

impl<'a> Local<'a, Value> {
    pub fn ok(self) -> Result<Local<'a, Value>, Error> {
        if self.inner.is_exception() {
            let err = self.ctxt.take_exception()?;

            trace!("-> {:?}", err);

            Err(err.into())
        } else {
            trace!("-> {:?}", self.inner);

            Ok(self)
        }
    }
}

impl ContextRef {
    pub fn is_error(&self, val: &Value) -> bool {
        unsafe { ffi::JS_IsError(self.as_ptr(), val.raw()) != FALSE }
    }

    pub fn throw<T: NewValue>(&self, exc: T) -> Local<Value> {
        self.bind(unsafe { ffi::JS_Throw(self.as_ptr(), exc.new_value(self)) })
    }

    pub fn exception(&self) -> Option<Local<Value>> {
        self.bind(unsafe { ffi::JS_GetException(self.as_ptr()) })
            .check_undefined()
    }

    pub fn enable_is_error_property(&self, enable: bool) {
        unsafe { ffi::JS_EnableIsErrorProperty(self.as_ptr(), if enable { TRUE } else { FALSE }) }
    }

    pub fn reset_uncatchable_error(&self) {
        unsafe { ffi::JS_ResetUncatchableError(self.as_ptr()) }
    }

    pub fn new_error(&self) -> Local<Value> {
        self.bind(unsafe { ffi::JS_NewError(self.as_ptr()) })
    }

    pub fn throw_error<T: ToString>(&self, msg: T) -> Local<Value> {
        let err = self.new_error();

        err.define_property_value(
            "message",
            msg.to_string(),
            Prop::WRITABLE | Prop::CONFIGURABLE,
        )
        .expect("message");

        self.throw(err)
    }

    pub fn throw_out_of_memory(&self) -> Local<Value> {
        self.bind(unsafe { ffi::JS_ThrowOutOfMemory(self.as_ptr()) })
    }

    pub fn throw_custom_error<T: ToString>(&self, name: &str, msg: T) -> Local<Value> {
        if let Some(ctor) = self.global_object().get_property(name) {
            match ctor.call_constructor(msg.to_string()) {
                Ok(err) => self.throw(err),
                Err(err) => self.throw_error(err),
            }
        } else {
            self.throw_error(format!("class `{}` not found", name))
        }
    }

    pub fn throw_syntax_error<T: Into<Vec<u8>>>(&self, msg: T) -> Local<Value> {
        self.bind(unsafe {
            ffi::JS_ThrowSyntaxError(
                self.as_ptr(),
                cstr!("%s").as_ptr(),
                CString::new(msg).expect("msg").as_ptr(),
            )
        })
    }

    pub fn throw_type_error<T: Into<Vec<u8>>>(&self, msg: T) -> Local<Value> {
        self.bind(unsafe {
            ffi::JS_ThrowTypeError(
                self.as_ptr(),
                cstr!("%s").as_ptr(),
                CString::new(msg).expect("msg").as_ptr(),
            )
        })
    }

    pub fn throw_reference_error<T: Into<Vec<u8>>>(&self, msg: T) -> Local<Value> {
        self.bind(unsafe {
            ffi::JS_ThrowReferenceError(
                self.as_ptr(),
                cstr!("%s").as_ptr(),
                CString::new(msg).expect("msg").as_ptr(),
            )
        })
    }

    pub fn throw_range_error<T: Into<Vec<u8>>>(&self, msg: T) -> Local<Value> {
        self.bind(unsafe {
            ffi::JS_ThrowRangeError(
                self.as_ptr(),
                cstr!("%s").as_ptr(),
                CString::new(msg).expect("msg").as_ptr(),
            )
        })
    }

    pub fn throw_internal_error<T: Into<Vec<u8>>>(&self, msg: T) -> Local<Value> {
        self.bind(unsafe {
            ffi::JS_ThrowInternalError(
                self.as_ptr(),
                cstr!("%s").as_ptr(),
                CString::new(msg).expect("msg").as_ptr(),
            )
        })
    }

    pub fn check_error(&self, ret: i32) -> Result<i32, Error> {
        if ret == ERR {
            let err = self.take_exception()?;

            trace!("-> {:?}", err);

            Err(err.into())
        } else {
            trace!("-> {:?}", ret);

            Ok(ret)
        }
    }

    pub fn check_bool(&self, ret: i32) -> Result<bool, Error> {
        self.check_error(ret).and_then(|ret| match ret {
            TRUE => Ok(true),
            FALSE => Ok(false),
            _ => Err(format_err!("unexpected result: {}", ret)),
        })
    }

    fn take_exception(&self) -> Result<ErrorKind, Error> {
        self.reset_uncatchable_error();

        self.exception()
            .ok_or_else(|| err_msg("expected exception"))
            .and_then(ErrorKind::try_from)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, Eval, Runtime};

    use super::ErrorKind::{self, *};

    #[test]
    fn std_error() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        assert_eq!(
            ctxt.eval("foobar", "<evalScript>", Eval::GLOBAL)
                .unwrap_err()
                .downcast_ref::<ErrorKind>()
                .unwrap(),
            &ReferenceError("foobar is not defined".into())
        );

        assert_eq!(
            ctxt.throw_syntax_error("foobar is not defined")
                .ok()
                .unwrap_err()
                .downcast_ref::<ErrorKind>()
                .unwrap(),
            &SyntaxError("foobar is not defined".into())
        );

        assert_eq!(
            ctxt.throw_out_of_memory()
                .ok()
                .unwrap_err()
                .downcast_ref::<ErrorKind>()
                .unwrap(),
            &InternalError("out of memory".into())
        );

        assert_eq!(
            ctxt.throw_custom_error("URIError", "malformed URI sequence")
                .ok()
                .unwrap_err()
                .downcast_ref::<ErrorKind>()
                .unwrap(),
            &URIError("malformed URI sequence".into())
        );
    }

    #[test]
    fn generic_error() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        assert_eq!(
            ctxt.eval("throw new Error('Whoops!');", "<evalScript>", Eval::GLOBAL)
                .unwrap_err()
                .downcast_ref::<ErrorKind>()
                .unwrap(),
            &Error("Whoops!".into())
        );

        assert_eq!(
            ctxt.throw_error("Whoops!")
                .ok()
                .unwrap_err()
                .downcast_ref::<ErrorKind>()
                .unwrap(),
            &Error("Whoops!".into())
        );
    }

    #[test]
    fn custom_error() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        ctxt.eval(
            r#"
class CustomError extends Error {
    constructor(...params) {
        super(...params);

        this.name = 'CustomError';
    }
}
"#,
            "<evalScript>",
            Eval::GLOBAL,
        )
        .unwrap();

        assert_eq!(
            ctxt.eval(
                "throw new CustomError('Whoops!')",
                "<evalScript>",
                Eval::GLOBAL,
            )
            .unwrap_err()
            .downcast_ref::<ErrorKind>()
            .unwrap(),
            &Custom("CustomError".into(), "Whoops!".into())
        );

        // assert_eq!(
        //     ctxt.throw_custom_error("CustomError", "Whoops!")
        //         .ok()
        //         .unwrap_err()
        //         .downcast_ref::<ErrorKind>()
        //         .unwrap(),
        //     &Custom("CustomError".into(), "Whoops!".into())
        // );
    }

    #[test]
    fn throw_string() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        assert_eq!(
            ctxt.eval("throw 'Whoops!';", "<evalScript>", Eval::GLOBAL)
                .unwrap_err()
                .downcast_ref::<ErrorKind>()
                .unwrap(),
            &Throw("Whoops!".into())
        );

        assert_eq!(
            ctxt.throw("Whoops!")
                .ok()
                .unwrap_err()
                .downcast_ref::<ErrorKind>()
                .unwrap(),
            &Throw("Whoops!".into())
        );
    }

    #[test]
    fn throw_int() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        assert_eq!(
            ctxt.eval("throw 123;", "<evalScript>", Eval::GLOBAL)
                .unwrap_err()
                .downcast_ref::<ErrorKind>()
                .unwrap(),
            &Throw("123".into())
        );

        assert_eq!(
            ctxt.throw(123)
                .ok()
                .unwrap_err()
                .downcast_ref::<ErrorKind>()
                .unwrap(),
            &Throw("123".into())
        );
    }
}
