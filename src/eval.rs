use std::ffi::CString;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use failure::{Error, ResultExt};
use foreign_types::ForeignTypeRef;

use crate::{ffi, Bindable, Context, ContextRef, ExtractValue, ReadObj, Runtime, Value};

bitflags! {
    /// Flags for `eval` method.
    pub struct Eval: u32 {
        /// global code (default)
        const GLOBAL = ffi::JS_EVAL_TYPE_GLOBAL;
        /// module code
        const MODULE = ffi::JS_EVAL_TYPE_MODULE;
        /// direct call (internal use)
        const DIRECT = ffi::JS_EVAL_TYPE_DIRECT;
        /// indirect call (internal use)
        const INDIRECT = ffi::JS_EVAL_TYPE_INDIRECT;

        /// force 'strict' mode
        const STRICT = ffi::JS_EVAL_FLAG_STRICT;
        /// force 'strip' mode
        const STRIP = ffi::JS_EVAL_FLAG_STRIP;
        /// compile but do not run.
        ///
        /// The result is an object with a `JS_TAG_FUNCTION_BYTECODE` or `JS_TAG_MODULE` tag.
        /// It can be executed with `JS_EvalFunction()`.
        const COMPILE_ONLY = ffi::JS_EVAL_FLAG_COMPILE_ONLY;
    }
}

/// Script source.
pub trait Source: Sized {
    type Flags;

    /// Default eval flags.
    fn default_flags() -> Self::Flags;

    /// Evaluate a script or module source.
    fn eval(self, ctxt: &'_ ContextRef, flags: Self::Flags) -> Result<Value<'_>, Error>;
}

impl Source for &str {
    type Flags = Eval;

    fn default_flags() -> Self::Flags {
        Eval::GLOBAL
    }

    fn eval(self, ctxt: &'_ ContextRef, flags: Self::Flags) -> Result<Value<'_>, Error> {
        ctxt.eval_script(self, "<evalScript>", flags)
    }
}

impl Source for &Path {
    type Flags = Eval;

    fn default_flags() -> Self::Flags {
        Eval::GLOBAL
    }

    fn eval(self, ctxt: &'_ ContextRef, flags: Self::Flags) -> Result<Value<'_>, Error> {
        ctxt.eval_file(self, flags)
    }
}

impl Source for &[u8] {
    type Flags = ();

    fn default_flags() -> Self::Flags {}

    fn eval(self, ctxt: &'_ ContextRef, _flags: Self::Flags) -> Result<Value<'_>, Error> {
        ctxt.eval_binary(self, false)
    }
}

/// Evaluate a script or module source.
///
/// The `eval` function accept the source code `&str`, filename `&Path` or precompiled bytecode `&[u8]`,
/// and returns the primitive value as you special, including `bool`, `i32`, `i64`, `u64`, `f64` or `String`.
///
/// - The Javascript `undefined` and `null` value will be returned as `None`.
/// - The Javascript `exception` will be convert to a `ErrorKind` error.
///
/// # Examples
///
/// The `eval` function accept the source code `&str` and returns the primitive value.
///
/// ```
/// let v: Option<i32> = qjs::eval("1+2").unwrap();
///
/// assert_eq!(v, Some(3));
/// ```
///
/// The Javascript `exception` will be convert to a `ErrorKind` error.
///
/// ```
/// assert_eq!(
///     qjs::eval::<_, ()>("throw new Error('Whoops!')")
///         .unwrap_err()
///         .downcast::<qjs::ErrorKind>()
///         .unwrap(),
///     qjs::ErrorKind::Error(
///         "Whoops!".into(),
///         Some("    at <eval> (<evalScript>)\n".into())
///     )
/// );
/// ```
pub fn eval<T: Source, V: ExtractValue>(source: T) -> Result<Option<V>, Error> {
    let rt = Runtime::new();
    let ctxt = Context::new(&rt);

    rt.set_module_loader::<()>(None, Some(ffi::js_module_loader), None);

    ctxt.std_add_helpers::<_, String>(None)?;

    ctxt.init_module_std()?;
    ctxt.init_module_os()?;

    if cfg!(feature = "qjscalc") {
        ctxt.eval_binary(&*ffi::QJSCALC, false)?;
    }

    let res = source.eval(&ctxt, T::default_flags()).map(|v| {
        if v.is_undefined() {
            None
        } else {
            V::extract_value(&v)
        }
    });

    rt.std_free_handlers();

    res
}

pub fn load_file<P: AsRef<Path>>(path: P) -> Result<String, Error> {
    let mut f = File::open(path)?;
    let mut s = String::new();

    f.read_to_string(&mut s)?;

    Ok(s)
}

impl ContextRef {
    /// Evaluate a script or module source.
    pub fn eval<T: Source, V: ExtractValue>(
        &self,
        source: T,
        flags: T::Flags,
    ) -> Result<Option<V>, Error> {
        source.eval(self, flags).map(|v| {
            if v.is_undefined() {
                None
            } else {
                V::extract_value(&v)
            }
        })
    }

    /// Evaluate a script or module source.
    pub fn eval_script<T: Into<Vec<u8>>>(
        &self,
        input: T,
        filename: &str,
        flags: Eval,
    ) -> Result<Value, Error> {
        let input = CString::new(input).context("input")?;

        trace!(
            "eval `{}` {:?}: {}",
            filename,
            flags,
            input.to_string_lossy()
        );

        let input = input.to_bytes_with_nul();
        let filename = CString::new(filename).context("filename")?;

        unsafe {
            ffi::JS_Eval(
                self.as_ptr(),
                input.as_ptr() as *const _,
                input.len() - 1,
                filename.as_ptr() as *const _,
                flags.bits as i32,
            )
        }
        .bind(self)
        .ok()
    }

    /// Evaluate a script or module source in file.
    pub fn eval_file<P: AsRef<Path>>(&self, path: P, flags: Eval) -> Result<Value, Error> {
        let filename = path.as_ref().to_string_lossy().to_string();

        load_file(path).and_then(|s| self.eval_script(s, &filename, flags))
    }

    /// Evaluate a script or module source in bytecode.
    pub fn eval_binary(&self, buf: &[u8], load_only: bool) -> Result<Value, Error> {
        trace!(
            "eval {} bytes function{}",
            buf.len(),
            if load_only { " (load only)" } else { "" }
        );

        let obj = self.read_object(buf, ReadObj::BYTECODE)?;

        if load_only {
            if obj.is_module() {
                self.set_import_meta(&obj, false, false)?;
            }

            Ok(self.undefined())
        } else {
            if obj.is_module() {
                self.resolve_module(&obj)?;
                self.set_import_meta(&obj, false, true)?;
            }

            self.eval_function(obj)
        }
    }

    /// Parse JSON expression.
    pub fn parse_json<T: Into<Vec<u8>>>(&self, input: T, filename: &str) -> Result<Value, Error> {
        let input = CString::new(input)?;
        let input = input.to_bytes_with_nul();
        let filename = CString::new(filename)?;

        unsafe {
            ffi::JS_ParseJSON(
                self.as_ptr(),
                input.as_ptr() as *const _,
                input.len(),
                filename.as_ptr(),
            )
        }
        .bind(self)
        .ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::{ffi::JS_TAG_INT, Context, ErrorKind, Runtime};

    use super::*;

    #[test]
    fn context() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        assert_eq!(ctxt.runtime(), &rt);

        let res = ctxt
            .eval_script("1+2", "<evalScript>", Eval::GLOBAL)
            .unwrap();

        assert_eq!(res.tag(), JS_TAG_INT);
        assert!(res.is_number());
        assert_eq!(res.as_int().unwrap(), 3);

        assert_eq!(
            ctxt.eval::<_, ()>("foobar", Eval::GLOBAL)
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            ErrorKind::ReferenceError(
                "foobar is not defined".into(),
                Some("    at <eval> (<evalScript>)\n".into())
            )
        );
    }

    #[test]
    fn str() {
        assert_eq!(eval::<_, i32>("1+2").unwrap(), Some(3));
    }

    #[test]
    fn file() {
        let mut f = tempfile::NamedTempFile::new().unwrap();

        write!(&mut f, "Float.PI").unwrap();

        let path = f.into_temp_path();
        let path: &Path = path.as_ref();

        assert!((eval::<_, f64>(path).unwrap().unwrap() - 3.14).abs() < 0.01);
    }

    #[test]
    fn binary() {
        assert_eq!(eval::<_, ()>(*ffi::REPL).unwrap(), None);
    }

    #[test]
    fn error() {
        assert_eq!(
            eval::<_, i32>("throw new Error('Whoops!')")
                .unwrap_err()
                .downcast::<ErrorKind>()
                .unwrap(),
            ErrorKind::Error(
                "Whoops!".into(),
                Some("    at <eval> (<evalScript>)\n".into())
            )
        );
    }

    #[test]
    fn parse_json() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let obj = ctxt
            .parse_json(
                r#"{ "name": "John", "age": 30, "city": "New York" }"#,
                "<evalScript>",
            )
            .unwrap();

        assert_eq!(obj.get_property("name").unwrap().to_string(), "John");
        assert_eq!(obj.get_property("age").unwrap().to_int32().unwrap(), 30);
        assert_eq!(obj.get_property("city").unwrap().to_string(), "New York");
    }
}
