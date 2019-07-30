use std::ops::{Deref, DerefMut};

use crate::ContextRef;

pub trait Bindable<'a> {
    type Output: Unbindable;

    fn bind_to(self, ctxt: &ContextRef) -> Self::Output;
}

pub trait Unbindable {
    fn unbind(ctxt: &ContextRef, inner: Self);
}

pub struct Local<'a, T>
where
    T: Unbindable,
{
    pub(crate) ctxt: &'a ContextRef,
    pub(crate) inner: Option<T>,
}

impl<'a, T> Drop for Local<'a, T>
where
    T: Unbindable,
{
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            T::unbind(self.ctxt, inner)
        }
    }
}

impl<'a, T> Deref for Local<'a, T>
where
    T: Unbindable,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl<'a, T> DerefMut for Local<'a, T>
where
    T: Unbindable,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}

impl<'a, T> Local<'a, T>
where
    T: Unbindable,
{
    pub fn into_inner(mut self) -> T {
        self.inner.take().unwrap()
    }

    pub fn map<U, F>(mut self, f: F) -> Local<'a, U>
    where
        F: FnOnce(T) -> U,
        U: Unbindable,
    {
        let inner = self.inner.take().map(f);

        Local {
            ctxt: self.ctxt,
            inner: inner,
        }
    }
}

impl ContextRef {
    pub fn bind<'a, T: Bindable<'a>>(&'a self, val: T) -> Local<'a, T::Output> {
        Local {
            ctxt: self,
            inner: Some(val.bind_to(self)),
        }
    }
}
