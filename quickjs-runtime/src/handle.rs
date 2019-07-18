use std::ops::{Deref, DerefMut};

use crate::ContextRef;

#[derive(Debug)]
pub struct Local<'a, T> {
    pub(crate) ctxt: &'a ContextRef,
    pub(crate) inner: T,
}

impl<'a, T> Deref for Local<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> DerefMut for Local<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a, T> Local<'a, T> {
    pub fn context(&self) -> &ContextRef {
        self.ctxt
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }
}

impl<'a, T> Local<'a, T> {
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl ContextRef {
    pub fn bind<T>(&self, val: T) -> Local<T> {
        Local {
            ctxt: self,
            inner: val,
        }
    }
}
