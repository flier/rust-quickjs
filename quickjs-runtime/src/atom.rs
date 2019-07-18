use std::ffi::CStr;
use std::fmt;
use std::ops::Deref;
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

impl NewAtom for Atom<'_> {
    fn new_atom(self, _context: &ContextRef) -> ffi::JSAtom {
        self.0.inner
    }
}

pub struct Atom<'a>(Local<'a, ffi::JSAtom>);

impl Unbindable for ffi::JSAtom {
    fn unbind(ctxt: &ContextRef, atom: ffi::JSAtom) {
        ctxt.free_atom(atom)
    }
}

impl<'a> Deref for Atom<'a> {
    type Target = Local<'a, ffi::JSAtom>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Clone for Atom<'_> {
    fn clone(&self) -> Self {
        self.ctxt.clone_atom(self.inner)
    }
}

impl fmt::Display for Atom<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.to_cstr().to_string_lossy())
    }
}

impl Atom<'_> {
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
    pub fn new_atom<T: NewAtom>(&self, v: T) -> Atom {
        Atom(self.bind(v.new_atom(self)))
    }

    pub fn free_atom(&self, atom: ffi::JSAtom) {
        unsafe { ffi::JS_FreeAtom(self.as_ptr(), atom) }
    }

    pub fn clone_atom(&self, atom: ffi::JSAtom) -> Atom {
        Atom(self.bind(unsafe { ffi::JS_DupAtom(self.as_ptr(), atom) }))
    }

    pub fn atom_to_value(&self, atom: ffi::JSAtom) -> Local<Value> {
        self.bind(unsafe { ffi::JS_AtomToValue(self.as_ptr(), atom) }.into())
    }

    pub fn atom_to_string(&self, atom: ffi::JSAtom) -> Local<Value> {
        self.bind(unsafe { ffi::JS_AtomToString(self.as_ptr(), atom) }.into())
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
