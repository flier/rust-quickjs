use std::mem;
use std::ops::{Deref, DerefMut};

use crate::ContextRef;

pub trait Bindable<'a> {
    type Output: Unbindable;

    fn bind_to(self, ctxt: &ContextRef) -> Self::Output;
}

pub trait Unbindable {
    fn unbind(ctxt: &ContextRef, inner: Self);
}

#[derive(Debug)]
pub struct Local<'a, T>
where
    T: Unbindable,
{
    pub(crate) ctxt: &'a ContextRef,
    pub(crate) inner: T,
}

impl<'a, T> Drop for Local<'a, T>
where
    T: Unbindable,
{
    fn drop(&mut self) {
        let inner = self.take();

        T::unbind(self.ctxt, inner)
    }
}

impl<'a, T> Deref for Local<'a, T>
where
    T: Unbindable,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> DerefMut for Local<'a, T>
where
    T: Unbindable,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a, T> Local<'a, T>
where
    T: Unbindable,
{
    pub fn into_inner(mut self) -> T {
        let inner = self.take();

        mem::forget(self);

        inner
    }

    pub(crate) fn take(&mut self) -> T {
        mem::replace(&mut self.inner, unsafe { mem::zeroed() })
    }

    pub fn map<U, F>(mut self, f: F) -> Local<'a, U>
    where
        F: FnOnce(T) -> U,
        U: Unbindable,
    {
        let inner = f(self.take());

        Local {
            ctxt: self.ctxt,
            inner,
        }
    }
}

impl ContextRef {
    pub fn bind<'a, T: Bindable<'a>>(&'a self, val: T) -> Local<'a, T::Output> {
        Local {
            ctxt: self,
            inner: val.bind_to(self),
        }
    }
}
