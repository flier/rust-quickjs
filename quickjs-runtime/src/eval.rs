use std::ffi::CString;

use failure::{Error, ResultExt};
use foreign_types::ForeignTypeRef;

use crate::{ffi, ContextRef, Local, Value};

bitflags! {
    pub struct Eval: u32 {
        /// global code (default)
        const GLOBAL = ffi::JS_EVAL_TYPE_GLOBAL;
        /// module code
        const MODULE = ffi::JS_EVAL_TYPE_MODULE;
        /// direct call (internal use)
        const DIRECT = ffi::JS_EVAL_TYPE_DIRECT;
        /// indirect call (internal use)
        const INDIRECT = ffi::JS_EVAL_TYPE_INDIRECT;

        const TYPE_MASK = ffi::JS_EVAL_TYPE_MASK;

        const LOAD_ONLY = ffi::JS_EVAL_BINARY_LOAD_ONLY;

        /// skip first line beginning with '#!'
        const SHEBANG = ffi::JS_EVAL_FLAG_SHEBANG;
        /// force 'strict' mode
        const STRICT = ffi::JS_EVAL_FLAG_STRICT;
        /// force 'strip' mode
        const STRIP = ffi::JS_EVAL_FLAG_STRIP;
        /// internal use
        const COMPILE_ONLY = ffi::JS_EVAL_FLAG_COMPILE_ONLY;
    }
}

impl ContextRef {
    pub fn eval<T: Into<Vec<u8>>>(
        &self,
        input: T,
        filename: &str,
        flags: Eval,
    ) -> Result<Local<Value>, Error> {
        let input = CString::new(input).context("input")?;

        trace!("eval @ {}: {:?}", filename, input);

        let input = input.to_bytes_with_nul();
        let filename = CString::new(filename).context("filename")?;

        self.bind(Value(unsafe {
            ffi::JS_Eval(
                self.as_ptr(),
                input.as_ptr() as *const _,
                input.len() - 1,
                filename.as_ptr() as *const _,
                flags.bits as i32,
            )
        }))
        .ok()
    }

    pub fn eval_binary(&self, buf: &[u8], flags: Eval) -> Result<Local<Value>, Error> {
        self.bind(Value(unsafe {
            ffi::JS_EvalBinary(self.as_ptr(), buf.as_ptr(), buf.len(), flags.bits as i32)
        }))
        .ok()
    }
}

#[cfg(test)]
mod tests {
    use crate::{value::JS_TAG_INT, Context, Runtime};

    use super::*;

    #[test]
    fn context() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        assert_eq!(ctxt.runtime(), &rt);

        let res = ctxt.eval("1+2", "<evalScript>", Eval::GLOBAL).unwrap();

        assert_eq!(res.tag(), JS_TAG_INT);
        assert!(res.is_number());
        assert_eq!(res.as_int().unwrap(), 3);

        assert_eq!(
            ctxt.eval("foobar", "<evalScript>", Eval::GLOBAL)
                .unwrap_err()
                .to_string(),
            "ReferenceError: foobar is not defined"
        );
    }
}
