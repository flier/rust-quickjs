#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

#[macro_use]
extern crate cfg_if;

cfg_if! {
    if #[cfg(feature = "gen")] {
        include!(concat!(env!("OUT_DIR"), "/raw.rs"));
    } else {
        include!("raw.rs");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime() {
        let rt = unsafe { JS_NewRuntime() };

        assert!(!rt.is_null());

        unsafe {
            JS_FreeRuntime(rt);
        }
    }
}
