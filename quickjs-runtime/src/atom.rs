use std::ffi::CStr;

use foreign_types::ForeignTypeRef;

use crate::{ffi, ContextRef, RuntimeRef, Value};

pub type Atom = ffi::JSAtom;

impl RuntimeRef {
    pub fn free_atom(&self, atom: Atom) {
        unsafe { ffi::JS_FreeAtomRT(self.as_ptr(), atom) }
    }
}

pub trait IntoAtom {
    fn into_atom(self, context: &ContextRef) -> Atom;
}

impl<'a> IntoAtom for &'a str {
    fn into_atom(self, context: &ContextRef) -> Atom {
        unsafe {
            ffi::JS_NewAtomLen(
                context.as_ptr(),
                self.as_ptr() as *const _,
                self.len() as i32,
            )
        }
    }
}

impl IntoAtom for *const i8 {
    fn into_atom(self, context: &ContextRef) -> Atom {
        unsafe { ffi::JS_NewAtom(context.as_ptr(), self) }
    }
}

impl IntoAtom for u32 {
    fn into_atom(self, context: &ContextRef) -> Atom {
        unsafe { ffi::JS_NewAtomUInt32(context.as_ptr(), self) }
    }
}

impl ContextRef {
    pub fn new_atom<T: IntoAtom>(&self, v: T) -> Atom {
        v.into_atom(self)
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

    pub fn atom_to_cstr(&self, atom: Atom) -> &CStr {
        unsafe { CStr::from_ptr(ffi::JS_AtomToCString(self.as_ptr(), atom)) }
    }
}
