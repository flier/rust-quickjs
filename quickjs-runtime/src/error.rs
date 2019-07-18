use std::convert::TryFrom;

use failure::{err_msg, Error};

use crate::{Local, Value};

#[derive(Debug, Fail, PartialEq)]
pub enum ErrorKind {
    #[fail(display = "Error: {}", _0)]
    Error(String),

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
        let res = if value.is_error() {
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

            Ok(match &*name.to_string_lossy() {
                "EvalError" => ErrorKind::EvalError(msg),
                "InternalError" => ErrorKind::InternalError(msg),
                "RangeError" => ErrorKind::RangeError(msg),
                "ReferenceError" => ErrorKind::ReferenceError(msg),
                "SyntaxError" => ErrorKind::SyntaxError(msg),
                "TypeError" => ErrorKind::TypeError(msg),
                "URIError" => ErrorKind::URIError(msg),
                _ => ErrorKind::Error(msg),
            })
        } else {
            Err(err_msg("value is not an exception"))
        };

        value.free();

        res
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, Eval, Runtime};

    use super::*;

    #[test]
    fn error() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let res = ctxt.eval("foobar", "<evalScript>", Eval::GLOBAL);

        if let Some(err) = res.unwrap_err().downcast_ref::<ErrorKind>() {
            assert_eq!(
                err,
                &ErrorKind::ReferenceError("foobar is not defined".into())
            )
        } else {
            panic!("unexpected exception")
        }
    }
}
