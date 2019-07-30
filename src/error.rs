use std::convert::TryFrom;
use std::ffi::CString;
use std::ptr::NonNull;

use failure::{err_msg, Error};
use foreign_types::ForeignTypeRef;

use crate::{
    ffi,
    value::{ERR, FALSE, TRUE},
    ContextRef, Local, NewValue, Prop, Value,
};

/// Javascript error.
#[derive(Debug, Clone, Fail, PartialEq)]
pub enum ErrorKind {
    #[fail(display = "Throw: {}", _0)]
    Throw(String),

    #[fail(display = "Error: {}", _0)]
    Error(String, Option<String>),

    #[fail(display = "{}: {}", _0, _1)]
    Custom(String, String, Option<String>),

    /// an error that occurs regarding the global function eval().
    #[fail(display = "EvalError: {}", _0)]
    EvalError(String, Option<String>),

    /// an error that occurs when an internal error in the JavaScript engine is thrown.
    #[fail(display = "InternalError: {}", _0)]
    InternalError(String, Option<String>),

    /// an error that occurs when a numeric variable or parameter is outside of its valid range.
    #[fail(display = "RangeError: {}", _0)]
    RangeError(String, Option<String>),

    /// an error that occurs when de-referencing an invalid reference.
    #[fail(display = "ReferenceError: {}", _0)]
    ReferenceError(String, Option<String>),

    /// a syntax error that occurs while parsing code in eval().
    #[fail(display = "SyntaxError: {}", _0)]
    SyntaxError(String, Option<String>),

    /// an error that occurs when a variable or parameter is not of a valid type.
    #[fail(display = "TypeError: {}", _0)]
    TypeError(String, Option<String>),

    /// an error that occurs when encodeURI() or decodeURI() are passed invalid parameters.
    #[fail(display = "URIError: {}", _0)]
    URIError(String, Option<String>),
}

impl ErrorKind {
    pub fn message(&self) -> &str {
        use ErrorKind::*;

        match self {
            Throw(msg)
            | Error(msg, _)
            | Custom(_, msg, _)
            | EvalError(msg, _)
            | InternalError(msg, _)
            | RangeError(msg, _)
            | ReferenceError(msg, _)
            | SyntaxError(msg, _)
            | TypeError(msg, _)
            | URIError(msg, _) => msg.as_str(),
        }
    }

    pub fn stack(&self) -> Option<&str> {
        use ErrorKind::*;

        match self {
            Throw(_) => None,
            Error(_, ref stack)
            | Custom(_, _, ref stack)
            | EvalError(_, ref stack)
            | InternalError(_, ref stack)
            | RangeError(_, ref stack)
            | ReferenceError(_, ref stack)
            | SyntaxError(_, ref stack)
            | TypeError(_, ref stack)
            | URIError(_, ref stack) => stack.as_ref().map(|s| s.as_str()),
        }
    }
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
                .ok_or_else(|| err_msg("missing `name` property"))?
                .to_string();
            let msg = value
                .get_property("message")
                .ok_or_else(|| err_msg("missing `message` property"))?
                .to_string();
            let stack = value.get_property("stack").map(|s| s.to_string());

            match name.as_str() {
                "EvalError" => EvalError(msg, stack),
                "InternalError" => InternalError(msg, stack),
                "RangeError" => RangeError(msg, stack),
                "ReferenceError" => ReferenceError(msg, stack),
                "SyntaxError" => SyntaxError(msg, stack),
                "TypeError" => TypeError(msg, stack),
                "URIError" => URIError(msg, stack),
                "Error" => Error(msg, stack),
                _ => Custom(name, msg, stack),
            }
        } else {
            Throw(value.to_string())
        })
    }
}

impl NewValue for Result<Local<'_, Value>, Error> {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        match self {
            Ok(v) => v,
            Err(err) => match err.downcast::<ErrorKind>() {
                Ok(err) => ctxt.throw(err),
                Err(err) => ctxt.throw(err.to_string()),
            },
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
            Error(msg, stack) => ctxt.throw_error(msg, stack),
            Custom(name, msg, stack) => ctxt.throw_custom_error(&name, msg, stack),
            EvalError(msg, stack) => ctxt.throw_custom_error("EvalError", msg, stack),
            InternalError(msg, _) => ctxt.throw_internal_error(msg),
            RangeError(msg, _) => ctxt.throw_range_error(msg),
            ReferenceError(msg, _) => ctxt.throw_reference_error(msg),
            SyntaxError(msg, _) => ctxt.throw_syntax_error(msg),
            TypeError(msg, _) => ctxt.throw_type_error(msg),
            URIError(msg, stack) => ctxt.throw_custom_error("URIError", msg, stack),
        }
        .into_inner()
        .raw()
    }
}

impl<'a> Local<'a, Value> {
    pub fn ok(self) -> Result<Local<'a, Value>, Error> {
        if self.is_exception() {
            let err = self.ctxt.take_exception()?;

            trace!("-> Err({:?})", err);

            Err(err.into())
        } else {
            trace!("-> Ok({:?})", self.inner);

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

    pub fn get_exception(&self) -> Option<Local<Value>> {
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

    pub fn throw_error<T: ToString>(&self, msg: T, stack: Option<String>) -> Local<Value> {
        let err = self.new_error();

        err.define_property_value(
            "message",
            msg.to_string(),
            Prop::WRITABLE | Prop::CONFIGURABLE,
        )
        .expect("message");

        if let Some(stack) = stack {
            err.define_property_value("stack", stack, Prop::WRITABLE | Prop::CONFIGURABLE)
                .expect("stack");
        }

        self.throw(err)
    }

    pub fn throw_out_of_memory(&self) -> Local<Value> {
        self.bind(unsafe { ffi::JS_ThrowOutOfMemory(self.as_ptr()) })
    }

    pub fn throw_custom_error<T: ToString>(
        &self,
        name: &str,
        msg: T,
        stack: Option<String>,
    ) -> Local<Value> {
        if let Some(ctor) = self.global_object().get_property(name) {
            match ctor.call_constructor(msg.to_string()) {
                Ok(err) => {
                    if let Some(stack) = stack {
                        err.define_property_value(
                            "stack",
                            stack,
                            Prop::WRITABLE | Prop::CONFIGURABLE,
                        )
                        .expect("stack");
                    }

                    self.throw(err)
                }
                Err(err) => self.throw_error(err, stack),
            }
        } else {
            self.throw_error(format!("class `{}` not found", name), stack)
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

    pub fn check_null<T>(&self, ptr: *mut T) -> Result<NonNull<T>, Error> {
        match NonNull::new(ptr) {
            Some(ptr) => {
                trace!("-> Ok({:?})", ptr);

                Ok(ptr)
            }
            None => {
                let err = self.take_exception()?;

                trace!("-> Err({:?})", err);

                Err(err.into())
            }
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

        self.get_exception()
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
            ctxt.eval::<_, ()>("foobar", Eval::GLOBAL)
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            ReferenceError(
                "foobar is not defined".into(),
                Some("    at <eval> (<evalScript>)\n".into())
            )
        );

        assert_eq!(
            ctxt.throw_syntax_error("foobar is not defined")
                .ok()
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            SyntaxError("foobar is not defined".into(), None)
        );

        assert_eq!(
            ctxt.throw_out_of_memory()
                .ok()
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            InternalError("out of memory".into(), None)
        );

        assert_eq!(
            ctxt.throw_custom_error(
                "URIError",
                "malformed URI sequence",
                Some("    at <eval> (<evalScript>)\n".into())
            )
            .ok()
            .unwrap_err()
            .downcast::<ErrorKind>()
            .unwrap(),
            URIError(
                "malformed URI sequence".into(),
                Some("    at <eval> (<evalScript>)\n".into())
            )
        );
    }

    #[test]
    fn generic_error() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        assert_eq!(
            ctxt.eval::<_, ()>("throw new Error('Whoops!');", Eval::GLOBAL)
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            Error(
                "Whoops!".into(),
                Some("    at <eval> (<evalScript>)\n".into())
            )
        );

        assert_eq!(
            ctxt.throw_error("Whoops!", Some("    at <eval> (<evalScript>)\n".into()))
                .ok()
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            Error(
                "Whoops!".into(),
                Some("    at <eval> (<evalScript>)\n".into())
            )
        );
    }

    #[test]
    fn custom_error() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        ctxt.eval::<_, ()>(
            r#"
class CustomError extends Error {
    constructor(...params) {
        super(...params);

        this.name = 'CustomError';
    }
}
"#,
            Eval::GLOBAL,
        )
        .unwrap();

        assert_eq!(
            ctxt.eval::<_, ()>("throw new CustomError('Whoops!')", Eval::GLOBAL,)
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            Custom(
                "CustomError".into(),
                "Whoops!".into(),
                Some("    at <eval> (<evalScript>)\n".into())
            ),
        );

        // assert_eq!(
        //     ctxt.throw_custom_error("CustomError", "Whoops!", None)
        //         .ok()
        //         .unwrap_err()
        //         .downcast::<ErrorKind>()
        //         .unwrap(),
        //     &Custom("CustomError".into(), "Whoops!".into(), None)
        // );
    }

    #[test]
    fn throw_string() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        assert_eq!(
            ctxt.eval::<_, ()>("throw 'Whoops!';", Eval::GLOBAL)
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            Throw("Whoops!".into())
        );

        assert_eq!(
            ctxt.throw("Whoops!")
                .ok()
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            Throw("Whoops!".into())
        );
    }

    #[test]
    fn throw_int() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        assert_eq!(
            ctxt.eval::<_, ()>("throw 123;", Eval::GLOBAL)
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            Throw("123".into())
        );

        assert_eq!(
            ctxt.throw(123)
                .ok()
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            Throw("123".into())
        );
    }
}
