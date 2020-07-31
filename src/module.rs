use std::ffi::CString;
use std::ptr::{null_mut, NonNull};

use failure::Error;
use foreign_types::ForeignTypeRef;

use crate::{ffi, value::ToBool, Atom, Bindable, ContextRef, RuntimeRef, Value};

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

/// return true if `input` contains the source of a module (heuristic).
///
/// Heuristic: skip comments and expect 'import' keyword not followed by '(' or '.'
pub fn detect_module<T: Into<Vec<u8>>>(input: T) -> bool {
    let input = input.into();

    unsafe { ffi::JS_DetectModule(input.as_ptr() as *const _, input.len()).to_bool() }
}

impl ContextRef {
    /// Create a new C module.
    pub fn new_c_module<T: Into<Vec<u8>>>(
        &self,
        name: T,
        init: ModuleInitFunc,
    ) -> Result<NonNull<ffi::JSModuleDef>, Error> {
        let name = CString::new(name)?;
        self.check_null(unsafe { ffi::JS_NewCModule(self.as_ptr(), name.as_ptr(), init) })
    }

    /// return the name of a module
    pub fn module_name(&self, module: &ModuleDef) -> Atom {
        self.bind_atom(unsafe {
            ffi::JS_GetModuleName(self.as_ptr(), module as *const _ as *mut _)
        })
    }

    /// return the `import.meta` object of a module
    pub fn import_meta(&self, module: &ModuleDef) -> Result<Value, Error> {
        unsafe { ffi::JS_GetImportMeta(self.as_ptr(), module as *const _ as *mut _) }
            .bind(self)
            .ok()
    }

    /// set the `import.meta` object of a module
    pub fn set_import_meta(
        &self,
        module: &Value,
        use_realpath: bool,
        is_main: bool,
    ) -> Result<(), Error> {
        self.check_error(unsafe {
            ffi::js_module_set_import_meta(
                self.as_ptr(),
                module.inner(),
                use_realpath.to_bool(),
                is_main.to_bool(),
            )
        })
        .map(|_| ())
    }

    /// load the dependencies of the module 'obj'.
    ///
    /// Useful when `read_object()` returns a module.
    pub fn resolve_module(&self, module: &Value) -> Result<(), Error> {
        self.check_error(unsafe { ffi::JS_ResolveModule(self.as_ptr(), module.inner()) })
            .map(|_| ())
    }
}
