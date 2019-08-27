use std::ffi::CString;
use std::ptr::{null_mut, NonNull};

use failure::Error;
use foreign_types::ForeignTypeRef;

use crate::{ffi, value::FALSE, ContextRef, RuntimeRef};

/// The C module definition.
pub type ModuleDef = ffi::JSModuleDef;

/// The C module init function.
pub type ModuleInitFunc = ffi::JSModuleInitFunc;

/// The module loader function.
pub type ModuleLoaderFunc = ffi::JSModuleLoaderFunc;

/// The filename normalizer function.
pub type ModuleNormalizeFunc = ffi::JSModuleNormalizeFunc;

impl RuntimeRef {
    /// Set the module loader and normalizer functions.
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
    /// Create a new C module.
    pub fn new_c_module<T: Into<Vec<u8>>>(
        &self,
        name: T,
        init: ModuleInitFunc,
    ) -> Result<NonNull<ffi::JSModuleDef>, Error> {
        self.check_null(unsafe {
            ffi::JS_NewCModule(
                self.as_ptr(),
                CString::new(name).expect("name").as_ptr(),
                init,
            )
        })
    }

    /// return true if `input` contains the source of a module (heuristic).
    ///
    /// Heuristic: skip comments and expect 'import' keyword not followed by '(' or '.'
    pub fn detect_module<T: Into<Vec<u8>>>(&self, input: T) -> bool {
        let input = input.into();

        unsafe { ffi::JS_DetectModule(input.as_ptr() as *const _, input.len()) != FALSE }
    }
}
