use std::ops::Deref;
use std::ptr;
use std::slice;

use foreign_types::ForeignTypeRef;

use crate::{
    ffi,
    value::{NewValue, FALSE, TRUE},
    ContextRef, Local, Value,
};

#[repr(transparent)]
#[derive(Debug)]
pub struct ArrayBuffer<'a>(Local<'a, Value>);

#[repr(transparent)]
#[derive(Debug)]
pub struct SharedArrayBuffer<'a>(Local<'a, Value>);

impl<'a> NewValue for ArrayBuffer<'a> {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        self.0.into_inner().0
    }
}

impl<'a> Deref for ArrayBuffer<'a> {
    type Target = Local<'a, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> AsRef<[u8]> for ArrayBuffer<'a> {
    fn as_ref(&self) -> &[u8] {
        unsafe {
            let mut size = 0;
            let data = ffi::JS_GetArrayBuffer(self.ctxt.as_ptr(), &mut size, self.inner.0);

            slice::from_raw_parts(data, size)
        }
    }
}

impl<'a> AsMut<[u8]> for ArrayBuffer<'a> {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe {
            let mut size = 0;
            let data = ffi::JS_GetArrayBuffer(self.ctxt.as_ptr(), &mut size, self.inner.0);

            slice::from_raw_parts_mut(data, size)
        }
    }
}

impl<'a> ArrayBuffer<'a> {
    pub fn slice(&self, begin: usize, end: usize) -> &[u8] {
        &self.as_ref()[begin..end]
    }

    pub fn slice_mut(&mut self, begin: usize, end: usize) -> &mut [u8] {
        &mut self.as_mut()[begin..end]
    }

    pub fn detach(&self) {
        unsafe { ffi::JS_DetachArrayBuffer(self.ctxt.as_ptr(), self.inner.0) }
    }
}

impl<'a> NewValue for SharedArrayBuffer<'a> {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        self.0.inner.0
    }
}

impl<'a> Deref for SharedArrayBuffer<'a> {
    type Target = Local<'a, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> AsRef<[u8]> for SharedArrayBuffer<'a> {
    fn as_ref(&self) -> &[u8] {
        unsafe {
            let mut size = 0;
            let data = ffi::JS_GetArrayBuffer(self.ctxt.as_ptr(), &mut size, self.inner.0);
            slice::from_raw_parts(data, size)
        }
    }
}

impl<'a> AsMut<[u8]> for SharedArrayBuffer<'a> {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe {
            let mut size = 0;
            let data = ffi::JS_GetArrayBuffer(self.ctxt.as_ptr(), &mut size, self.inner.0);
            slice::from_raw_parts_mut(data, size)
        }
    }
}

impl<'a> SharedArrayBuffer<'a> {
    pub fn slice(&self, begin: usize, end: usize) -> &[u8] {
        &self.as_ref()[begin..end]
    }

    pub fn slice_mut(&mut self, begin: usize, end: usize) -> &mut [u8] {
        &mut self.as_mut()[begin..end]
    }
}

impl ContextRef {
    pub fn new_array_buffer<T: AsMut<[u8]>>(&self, buf: &mut T) -> ArrayBuffer {
        let buf = buf.as_mut();

        ArrayBuffer(self.bind(unsafe {
            ffi::JS_NewArrayBuffer(
                self.as_ptr(),
                buf.as_mut_ptr(),
                buf.len(),
                None,
                ptr::null_mut(),
                FALSE,
            )
        }))
    }

    pub fn new_shared_array_buffer<T: Into<Vec<u8>>>(&self, buf: T) -> SharedArrayBuffer {
        let mut buf = Box::new(buf.into());
        let data = buf.as_mut_ptr();
        let len = buf.len();

        SharedArrayBuffer(self.bind(unsafe {
            ffi::JS_NewArrayBuffer(
                self.as_ptr(),
                data,
                len,
                None,
                Box::into_raw(buf) as *mut _,
                TRUE,
            )
        }))
    }

    pub fn new_array_buffer_copy(&self, buf: &mut [u8]) -> ArrayBuffer {
        ArrayBuffer(self.bind(unsafe {
            ffi::JS_NewArrayBufferCopy(self.as_ptr(), buf.as_mut_ptr(), buf.len())
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, Eval, Runtime};

    #[test]
    fn array_buffer() {
        let _ = pretty_env_logger::try_init();

        let rt = Runtime::new();
        let ctxt = Context::new(&rt);

        let mut buf = [0; 16];
        let arr_buf = ctxt.new_array_buffer(&mut buf);

        assert!(ctxt.global_object().set_property("buf", arr_buf).unwrap());

        assert_eq!(
            ctxt.eval("buf.byteLength", "<evalScript>", Eval::GLOBAL)
                .unwrap()
                .as_int()
                .unwrap(),
            16
        );

        ctxt.eval(
            r#"
                var arr = new Uint16Array(buf);

                arr[0] = 123;
                arr[1] = 456;
                arr[2] = 567;
                "#,
            "<evalScript>",
            Eval::GLOBAL,
        )
        .unwrap();

        assert_eq!(buf, [123, 0, 200, 1, 55, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }
}
