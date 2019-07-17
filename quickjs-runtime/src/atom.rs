use std::ffi::CStr;
use std::os::raw::c_char;

use foreign_types::ForeignTypeRef;

use crate::{ffi, value::CStrBuf, ContextRef, RuntimeRef, Value};

pub type Atom = ffi::JSAtom;

impl RuntimeRef {
    pub fn free_atom(&self, atom: Atom) {
        unsafe { ffi::JS_FreeAtomRT(self.as_ptr(), atom) }
    }
}

pub trait NewAtom {
    fn new_atom(self, context: &ContextRef) -> Atom;
}

impl<'a> NewAtom for &'a str {
    fn new_atom(self, context: &ContextRef) -> Atom {
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
    fn new_atom(self, context: &ContextRef) -> Atom {
        unsafe { ffi::JS_NewAtom(context.as_ptr(), self) }
    }
}

impl NewAtom for u32 {
    fn new_atom(self, context: &ContextRef) -> Atom {
        unsafe { ffi::JS_NewAtomUInt32(context.as_ptr(), self) }
    }
}

impl ContextRef {
    pub fn new_atom<T: NewAtom>(&self, v: T) -> Atom {
        v.new_atom(self)
    }

    pub fn free_atom(&self, atom: Atom) {
        unsafe { ffi::JS_FreeAtom(self.as_ptr(), atom) }
    }

    pub fn clone_atom(&self, atom: Atom) -> Atom {
        unsafe { ffi::JS_DupAtom(self.as_ptr(), atom) }
    }

    pub fn atom_to_value(&self, atom: Atom) -> Value {
        unsafe { ffi::JS_AtomToValue(self.as_ptr(), atom) }.into()
    }

    pub fn atom_to_string(&self, atom: Atom) -> Value {
        unsafe { ffi::JS_AtomToString(self.as_ptr(), atom) }.into()
    }

    pub fn atom_to_cstr(&self, atom: Atom) -> CStrBuf {
        CStrBuf(self, unsafe {
            CStr::from_ptr(ffi::JS_AtomToCString(self.as_ptr(), atom))
        })
    }
}
