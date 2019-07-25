#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::unreadable_literal)]

#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate lazy_static;

cfg_if! {
    if #[cfg(feature = "gen")] {
        include!(concat!(env!("OUT_DIR"), "/raw.rs"));
    } else {
        include!("raw.rs");
    }
}

lazy_static! {
    pub static ref VERSION: &'static str =
        include_str!(concat!(env!("OUT_DIR"), "/quickjs/VERSION")).trim();
}

cfg_if! {
    if #[cfg(feature = "repl")] {
        extern "C" {
            #[no_mangle]
            pub static repl: [u8; 0];

            #[no_mangle]
            pub static repl_size: u32;
        }

        lazy_static! {
            pub static ref REPL: &'static [u8] = unsafe {
                std::slice::from_raw_parts(repl.as_ptr(), repl_size as usize)
            };
        }
    }
}

cfg_if! {
    if #[cfg(feature = "qjscalc")] {
        extern "C" {
            #[no_mangle]
            pub static qjscalc: [u8; 0];

            #[no_mangle]
            pub static qjscalc_size: u32;
        }

        lazy_static! {
            pub static ref QJSCALC: &'static [u8] = unsafe {
                std::slice::from_raw_parts(qjscalc.as_ptr(), qjscalc_size as usize)
            };
        }
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
