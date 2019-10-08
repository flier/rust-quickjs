use std::ffi::CString;
use std::ptr::NonNull;

use failure::Error;
use foreign_types::ForeignTypeRef;

use crate::{ffi, ContextRef, ModuleDef, RuntimeRef};

impl ContextRef {
    pub fn init_module_std(&self) -> Result<NonNull<ModuleDef>, Error> {
        debug!("init `std` module");

        self.check_null(unsafe { ffi::js_init_module_std(self.as_ptr(), cstr!(std).as_ptr()) })
    }

    pub fn init_module_os(&self) -> Result<NonNull<ModuleDef>, Error> {
        debug!("init `os` module");

        self.check_null(unsafe { ffi::js_init_module_os(self.as_ptr(), cstr!(os).as_ptr()) })
    }

    pub fn std_add_helpers<I: IntoIterator<Item = S>, S: Into<Vec<u8>>>(
        &self,
        args: I,
    ) -> Result<(), Error> {
        let args = args
            .into_iter()
            .map(CString::new)
            .collect::<Result<Vec<_>, _>>()?;

        debug!("add global helpers with script args: {:?}", args);

        let args = args.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();

        unsafe {
            ffi::js_std_add_helpers(self.as_ptr(), args.len() as i32, args.as_ptr() as *mut _);
        }

        Ok(())
    }

    pub fn std_loop(&self) {
        unsafe { ffi::js_std_loop(self.as_ptr()) }
    }

    pub fn std_dump_error(&self) {
        unsafe { ffi::js_std_dump_error(self.as_ptr()) }
    }
}

impl RuntimeRef {
    pub fn std_free_handlers(&self) {
        unsafe { ffi::js_std_free_handlers(self.as_ptr()) }
    }
}
