use std::ffi::CString;

use failure::{format_err, Error};
use foreign_types::ForeignTypeRef;

use crate::{
    atom::NewAtom,
    ffi,
    value::{FALSE, TRUE},
    Atom, ContextRef, Local, NewValue, Value,
};

pub trait GetProperty {
    fn get_property<'a>(&self, ctxt: &'a ContextRef, this: &Value) -> Option<Local<'a, Value>>;
}

impl GetProperty for &str {
    fn get_property<'a>(&self, ctxt: &'a ContextRef, this: &Value) -> Option<Local<'a, Value>> {
        Value(unsafe {
            ffi::JS_GetPropertyStr(
                ctxt.as_ptr(),
                this.0,
                CString::new(*self).expect("prop").as_ptr(),
            )
        })
        .ok()
        .map(|v| ctxt.bind(v))
    }
}

impl GetProperty for u32 {
    fn get_property<'a>(&self, ctxt: &'a ContextRef, this: &Value) -> Option<Local<'a, Value>> {
        Value(unsafe { ffi::JS_GetPropertyUint32(ctxt.as_ptr(), this.0, *self) })
            .ok()
            .map(|v| ctxt.bind(v))
    }
}

impl GetProperty for Atom<'_> {
    fn get_property<'a>(&self, ctxt: &'a ContextRef, this: &Value) -> Option<Local<'a, Value>> {
        Value(unsafe {
            ffi::JS_GetPropertyInternal(ctxt.as_ptr(), this.0, self.inner, this.0, FALSE)
        })
        .ok()
        .map(|v| ctxt.bind(v))
    }
}

pub trait SetProperty {
    fn set_property<T: NewValue>(
        &self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
    ) -> Result<bool, Error>;
}

impl SetProperty for u32 {
    fn set_property<T: NewValue>(
        &self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
    ) -> Result<bool, Error> {
        let ret = unsafe {
            ffi::JS_SetPropertyUint32(
                ctxt.as_ptr(),
                this.0,
                *self,
                val.new_value(ctxt).into_inner(),
            )
        };

        ctxt.check_error(ret).and_then(|ret| match ret {
            TRUE => Ok(true),
            FALSE => Ok(false),
            _ => Err(format_err!("unexpected result: {}", ret)),
        })
    }
}

impl SetProperty for i64 {
    fn set_property<T: NewValue>(
        &self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
    ) -> Result<bool, Error> {
        let ret = unsafe {
            ffi::JS_SetPropertyInt64(
                ctxt.as_ptr(),
                this.0,
                *self,
                val.new_value(ctxt).into_inner(),
            )
        };

        ctxt.check_error(ret).and_then(|ret| match ret {
            TRUE => Ok(true),
            FALSE => Ok(false),
            _ => Err(format_err!("unexpected result: {}", ret)),
        })
    }
}

impl SetProperty for &str {
    fn set_property<T: NewValue>(
        &self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
    ) -> Result<bool, Error> {
        ctxt.check_bool(unsafe {
            ffi::JS_SetPropertyStr(
                ctxt.as_ptr(),
                this.0,
                CString::new(*self)?.as_ptr(),
                val.new_value(ctxt).into_inner(),
            )
        })
    }
}

impl SetProperty for Atom<'_> {
    fn set_property<T: NewValue>(
        &self,
        ctxt: &ContextRef,
        this: &Value,
        val: T,
    ) -> Result<bool, Error> {
        ctxt.check_bool(unsafe {
            ffi::JS_SetPropertyInternal(
                ctxt.as_ptr(),
                this.0,
                self.inner,
                val.new_value(ctxt).into_inner(),
                ffi::JS_PROP_THROW as i32,
            )
        })
    }
}

pub trait HasProperty {
    fn has_property(self, ctxt: &ContextRef, this: &Value) -> bool;
}

impl<'a, T> HasProperty for T
where
    T: NewAtom,
{
    fn has_property(self, ctxt: &ContextRef, this: &Value) -> bool {
        let atom = self.new_atom(ctxt);

        unsafe { ffi::JS_HasProperty(ctxt.as_ptr(), this.0, atom) != FALSE }
    }
}

impl<'a> Local<'a, Value> {
    pub fn get_property<T: GetProperty>(&self, prop: T) -> Option<Local<Value>> {
        self.ctxt.get_property(&self.inner, prop)
    }

    pub fn set_property<T: SetProperty, V: NewValue>(
        &self,
        prop: T,
        val: V,
    ) -> Result<bool, Error> {
        self.ctxt.set_property(&self.inner, prop, val)
    }

    pub fn has_property<T: HasProperty>(&self, prop: T) -> bool {
        self.ctxt.has_property(&self.inner, prop)
    }

    pub fn is_extensible(&self) -> Result<bool, Error> {
        self.ctxt.is_extensible(&self.inner)
    }
}

impl ContextRef {
    pub fn get_property<T: GetProperty>(&self, this: &Value, prop: T) -> Option<Local<Value>> {
        prop.get_property(self, this)
    }

    pub fn set_property<T: SetProperty, V: NewValue>(
        &self,
        this: &Value,
        prop: T,
        val: V,
    ) -> Result<bool, Error> {
        prop.set_property(self, this, val)
    }

    pub fn has_property<T: HasProperty>(&self, this: &Value, prop: T) -> bool {
        prop.has_property(self, this)
    }

    pub fn is_extensible(&self, obj: &Value) -> Result<bool, Error> {
        self.check_bool(unsafe { ffi::JS_IsExtensible(self.as_ptr(), obj.0) })
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, Eval, Runtime};

    #[test]
    fn props() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let obj = ctxt
            .eval("new Object();", "<evalScript>", Eval::GLOBAL)
            .unwrap();

        assert!(!obj.has_property("foo"));
        assert!(obj.get_property("foo").is_none());
        assert!(obj.set_property("foo", "bar").unwrap());
        assert!(obj.has_property("foo"));
        assert_eq!(obj.get_property("foo").unwrap().to_str().unwrap(), "bar");
        assert!(obj.is_extensible().unwrap());
    }
}
