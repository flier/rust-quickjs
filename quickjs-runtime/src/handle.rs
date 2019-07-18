use std::mem;
use std::ops::{Deref, DerefMut};

use crate::ContextRef;

#[derive(Debug)]
pub struct Local<'a, T>
where
    T: Unbindable,
{
    pub(crate) ctxt: &'a ContextRef,
    pub(crate) inner: T,
}

pub trait Unbindable {
    fn unbind(ctxt: &ContextRef, inner: Self);
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
    pub fn context(&self) -> &ContextRef {
        self.ctxt
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn into_inner(mut self) -> T {
        let inner = self.take();

        mem::forget(self);

        inner
    }

    pub(crate) fn take(&mut self) -> T {
        mem::replace(&mut self.inner, unsafe { mem::zeroed() })
    }
}

impl ContextRef {
    pub fn bind<T>(&self, val: T) -> Local<T>
    where
        T: Unbindable,
    {
        Local {
            ctxt: self,
            inner: val,
        }
    }
}
