use std::convert::TryFrom;

use failure::{err_msg, Error};

use crate::{Local, Value};

#[derive(Debug, Fail, PartialEq)]
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
                "Error" => ErrorKind::Error(msg),
                _ => ErrorKind::Custom(name.to_string_lossy().to_string(), msg),
            })
        } else {
            let msg = value
                .to_cstr()
                .ok_or_else(|| err_msg("invalid value"))?
                .to_string_lossy()
                .to_string();

            Ok(ErrorKind::Throw(msg))
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
    fn std_error() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let err = ctxt
            .eval("foobar", "<evalScript>", Eval::GLOBAL)
            .unwrap_err();

        assert_eq!(
            err.downcast_ref::<ErrorKind>().unwrap(),
            &ErrorKind::ReferenceError("foobar is not defined".into())
        )
    }

    #[test]
    fn generic_error() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let err = ctxt
            .eval("throw new Error('Whoops!');", "<evalScript>", Eval::GLOBAL)
            .unwrap_err();

        assert_eq!(
            err.downcast_ref::<ErrorKind>().unwrap(),
            &ErrorKind::Error("Whoops!".into())
        )
    }

    #[test]
    fn custom_error() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let err = ctxt
            .eval(
                r#"
class CustomError extends Error {
    constructor(...params) {
        super(...params);

        this.name = 'CustomError';
    }
}

throw new CustomError('foobar');
"#,
                "<evalScript>",
                Eval::GLOBAL,
            )
            .unwrap_err();

        assert_eq!(
            err.downcast_ref::<ErrorKind>().unwrap(),
            &ErrorKind::Custom("CustomError".into(), "foobar".into())
        )
    }

    #[test]
    fn throw_error() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let err = ctxt
            .eval("throw 'Whoops!';", "<evalScript>", Eval::GLOBAL)
            .unwrap_err();

        assert_eq!(
            err.downcast_ref::<ErrorKind>().unwrap(),
            &ErrorKind::Throw("Whoops!".into())
        )
    }

}
