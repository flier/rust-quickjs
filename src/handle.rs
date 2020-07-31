use core::cell::Cell;
use core::mem;
use core::ops::Deref;

use crate::ContextRef;

pub trait Bindable: Default {
    fn bind(self, ctxt: &ContextRef) -> Local<Self>;

    fn unbind(self, ctxt: &ContextRef);
}

pub struct Local<'a, T>
where
    T: Bindable,
{
    pub(crate) ctxt: &'a ContextRef,
    inner: Cell<T>,
}

impl<'a, T> Drop for Local<'a, T>
where
    T: Bindable,
{
    fn drop(&mut self) {
        self.inner.take().unbind(self.ctxt)
    }
}

impl<'a, T> Deref for Local<'a, T>
where
    T: Bindable,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner.as_ptr() }
    }
}

impl<'a, T> Local<'a, T>
where
    T: Bindable,
{
    pub fn new(ctxt: &'a ContextRef, inner: T) -> Self {
        Local {
            ctxt,
            inner: Cell::new(inner),
        }
    }

    pub fn inner(&self) -> T
    where
        T: Copy,
    {
        self.inner.get()
    }

    pub fn into_inner(self) -> T {
        let inner = self.inner.take();
        mem::drop(self);
        inner
    }

    pub fn map<U, F>(self, f: F) -> Local<'a, U>
    where
        F: FnOnce(T) -> U,
        U: Bindable,
    {
        Local {
            ctxt: self.ctxt,
            inner: Cell::new(f(self.inner.take())),
        }
    }
}
