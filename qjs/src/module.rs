use std::ffi::CString;
use std::ptr::{null_mut, NonNull};

use foreign_types::ForeignTypeRef;

use crate::{ffi, ContextRef, RuntimeRef};

pub use crate::ffi::{
    JSModuleInitFunc as ModuleInitFunc, JSModuleLoaderFunc as ModuleLoaderFunc,
    JSModuleNormalizeFunc as ModuleNormalizeFunc,
};

impl RuntimeRef {
    pub fn set_module_loader<T>(
        &self,
        module_normalize: ModuleNormalizeFunc,
        module_loader: ModuleLoaderFunc,
        opaque: Option<NonNull<T>>,
    ) {
        unsafe {
            ffi::JS_SetModuleLoaderFunc(
                self.as_ptr(),
                module_normalize,
                module_loader,
                opaque.map_or_else(null_mut, |p| p.cast().as_ptr()),
            )
        }
    }
}

impl ContextRef {
    pub fn new_c_module<T: Into<Vec<u8>>>(
        &self,
        name: T,
        init: ModuleInitFunc,
    ) -> Option<NonNull<ffi::JSModuleDef>> {
        NonNull::new(unsafe {
            ffi::JS_NewCModule(
                self.as_ptr(),
                CString::new(name).expect("name").as_ptr(),
                init,
            )
        })
    }
}
