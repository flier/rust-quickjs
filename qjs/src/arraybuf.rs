use std::ops::Deref;
use std::ptr;
use std::slice::{self, SliceIndex};

use foreign_types::ForeignTypeRef;

use crate::{
    ffi,
    value::{NewValue, FALSE, TRUE},
    ContextRef, Local, Value,
};

/// `ArrayBuffer` represent a generic, fixed-length raw binary data buffer.
#[repr(transparent)]
#[derive(Debug)]
pub struct ArrayBuffer<'a>(Local<'a, Value>);

/// `SharedArrayBuffer` represent a generic, fixed-length raw binary data buffer,
/// similar to the ArrayBuffer object, but in a way that they can be used to create views on shared memory.
#[repr(transparent)]
#[derive(Debug)]
pub struct SharedArrayBuffer<'a>(Local<'a, Value>);

impl<'a> NewValue for ArrayBuffer<'a> {
    fn new_value(self, ctxt: &ContextRef) -> ffi::JSValue {
        self.0.new_value(ctxt)
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
            let data = ffi::JS_GetArrayBuffer(self.ctxt.as_ptr(), &mut size, self.raw());

            slice::from_raw_parts(data, size)
        }
    }
}

impl<'a> AsMut<[u8]> for ArrayBuffer<'a> {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe {
            let mut size = 0;
            let data = ffi::JS_GetArrayBuffer(self.ctxt.as_ptr(), &mut size, self.raw());

            slice::from_raw_parts_mut(data, size)
        }
    }
}

impl<'a> ArrayBuffer<'a> {
    /// Returns a reference to an element or subslice depending on the type of index.
    pub fn get<I>(&self, index: I) -> Option<&<I as SliceIndex<[u8]>>::Output>
    where
        I: SliceIndex<[u8]>,
    {
        self.as_ref().get(index)
    }

    /// Returns a mutable reference to an element or subslice depending on the type of index (see get) or None if the index is out of bounds.
    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut <I as SliceIndex<[u8]>>::Output>
    where
        I: SliceIndex<[u8]>,
    {
        self.as_mut().get_mut(index)
    }

    /// Detach the buffer and the underlying memory is released.
    pub fn detach(&self) {
        unsafe { ffi::JS_DetachArrayBuffer(self.ctxt.as_ptr(), self.raw()) }
    }
}

impl<'a> NewValue for SharedArrayBuffer<'a> {
    fn new_value(self, _ctxt: &ContextRef) -> ffi::JSValue {
        self.raw()
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
            let data = ffi::JS_GetArrayBuffer(self.ctxt.as_ptr(), &mut size, self.raw());
            slice::from_raw_parts(data, size)
        }
    }
}

impl<'a> AsMut<[u8]> for SharedArrayBuffer<'a> {
    fn as_mut(&mut self) -> &mut [u8] {
        unsafe {
            let mut size = 0;
            let data = ffi::JS_GetArrayBuffer(self.ctxt.as_ptr(), &mut size, self.raw());
            slice::from_raw_parts_mut(data, size)
        }
    }
}

impl<'a> SharedArrayBuffer<'a> {
    /// Returns a reference to an element or subslice depending on the type of index.
    pub fn get<I>(&self, index: I) -> Option<&<I as SliceIndex<[u8]>>::Output>
    where
        I: SliceIndex<[u8]>,
    {
        self.as_ref().get(index)
    }

    /// Returns a mutable reference to an element or subslice depending on the type of index (see get) or None if the index is out of bounds.
    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut <I as SliceIndex<[u8]>>::Output>
    where
        I: SliceIndex<[u8]>,
    {
        self.as_mut().get_mut(index)
    }
}

impl ContextRef {
    /// Creates a new `ArrayBuffer` of the given bytes.
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

    /// Creates a new `SharedArrayBuffer` of the given bytes.
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

    /// Creates a new `ArrayBuffer` which copy the given bytes.
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

        assert_eq!(ctxt.eval("buf.byteLength", Eval::GLOBAL).unwrap(), Some(16));

        ctxt.eval::<_, ()>(
            r#"
                var arr = new Uint16Array(buf);

                arr[0] = 123;
                arr[1] = 456;
                arr[2] = 567;
                "#,
            Eval::GLOBAL,
        )
        .unwrap();

        assert_eq!(buf, [123, 0, 200, 1, 55, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }
}
