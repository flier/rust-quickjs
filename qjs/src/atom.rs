use std::ffi::{CStr, CString};
use std::fmt;
use std::os::raw::c_char;

use foreign_types::ForeignTypeRef;

use crate::{ffi, handle::Unbindable, ContextRef, Local, RuntimeRef, Value};

pub trait NewAtom {
    fn new_atom(self, context: &ContextRef) -> ffi::JSAtom;
}

impl<'a> NewAtom for &'a str {
    fn new_atom(self, context: &ContextRef) -> ffi::JSAtom {
        unsafe {
            ffi::JS_NewAtomLen(
                context.as_ptr(),
                self.as_ptr() as *const _,
                self.len() as i32,
            )
        }
    }
}

impl NewAtom for *const c_char {
    fn new_atom(self, context: &ContextRef) -> ffi::JSAtom {
        unsafe { ffi::JS_NewAtom(context.as_ptr(), self) }
    }
}

impl NewAtom for u32 {
    fn new_atom(self, context: &ContextRef) -> ffi::JSAtom {
        unsafe { ffi::JS_NewAtomUInt32(context.as_ptr(), self) }
    }
}

impl Unbindable for ffi::JSAtom {
    fn unbind(ctxt: &ContextRef, atom: ffi::JSAtom) {
        ctxt.free_atom(atom)
    }
}

impl Into<ffi::JSAtom> for Local<'_, ffi::JSAtom> {
    fn into(self) -> ffi::JSAtom {
        self.into_inner()
    }
}

impl Clone for Local<'_, ffi::JSAtom> {
    fn clone(&self) -> Self {
        self.ctxt.clone_atom(self.inner)
    }
}

impl fmt::Display for Local<'_, ffi::JSAtom> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_cstr().to_string_lossy())
    }
}

impl fmt::Debug for Local<'_, ffi::JSAtom> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Atom")
            .field(&self.to_cstr().to_string_lossy())
            .finish()
    }
}

impl Local<'_, ffi::JSAtom> {
    pub fn free(&self) {
        self.ctxt.free_atom(self.inner)
    }

    pub fn to_value(&self) -> Local<Value> {
        self.ctxt.atom_to_value(self.inner)
    }

    pub fn to_string(&self) -> Local<Value> {
        self.ctxt.atom_to_string(self.inner)
    }

    pub fn to_cstr(&self) -> Local<&CStr> {
        self.ctxt.atom_to_cstr(self.inner)
    }
}

impl RuntimeRef {
    pub fn free_atom(&self, atom: ffi::JSAtom) {
        unsafe { ffi::JS_FreeAtomRT(self.as_ptr(), atom) }
    }
}

impl ContextRef {
    pub fn new_atom<T: NewAtom>(&self, v: T) -> Local<ffi::JSAtom> {
        self.bind_atom(v.new_atom(self))
    }

    fn bind_atom(&self, atom: ffi::JSAtom) -> Local<ffi::JSAtom> {
        Local {
            ctxt: self,
            inner: atom,
        }
    }

    pub fn new_atom_string<T: Into<Vec<u8>>>(&self, s: T) -> Local<Value> {
        self.bind(unsafe {
            ffi::JS_NewAtomString(
                self.as_ptr(),
                CString::new(s)
                    .expect("atom string should not contain an internal 0 byte")
                    .as_ptr(),
            )
        })
    }

    pub fn free_atom(&self, atom: ffi::JSAtom) {
        unsafe { ffi::JS_FreeAtom(self.as_ptr(), atom) }
    }

    pub fn clone_atom(&self, atom: ffi::JSAtom) -> Local<ffi::JSAtom> {
        self.bind_atom(unsafe { ffi::JS_DupAtom(self.as_ptr(), atom) })
    }

    pub fn atom_to_value(&self, atom: ffi::JSAtom) -> Local<Value> {
        self.bind(unsafe { ffi::JS_AtomToValue(self.as_ptr(), atom) })
    }

    pub fn atom_to_string(&self, atom: ffi::JSAtom) -> Local<Value> {
        self.bind(unsafe { ffi::JS_AtomToString(self.as_ptr(), atom) })
    }

    pub fn atom_to_cstr(&self, atom: ffi::JSAtom) -> Local<&CStr> {
        self.bind(unsafe { CStr::from_ptr(ffi::JS_AtomToCString(self.as_ptr(), atom)) })
    }
}

#[cfg(test)]
mod tests {
    use std::string::ToString;

    use crate::*;

    #[test]
    fn atom() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let foo = ctxt.new_atom("foo");
        let bar = ctxt.new_atom("bar");

        assert_eq!(ToString::to_string(&foo), "foo");
        assert_eq!(ToString::to_string(&bar), "bar");
        assert_ne!(foo.inner, bar.inner);
    }
}
