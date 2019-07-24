#[macro_use]
extern crate log;
#[macro_use]
extern crate cfg_if;

use std::ffi::CStr;
use std::mem;
use std::os::raw::{c_char, c_void};
use std::ptr::null_mut;
use std::time::{Duration, Instant};

use failure::Error;
use foreign_types::ForeignTypeRef;
use structopt::{clap::crate_version, StructOpt};

use qjs::{ffi, Context, ContextRef, MallocFunctions, Runtime};

#[derive(Debug, StructOpt)]
#[structopt(name = "qjs", about = "QuickJS stand alone interpreter")]
pub struct Opt {
    /// Evaluate EXPR
    #[structopt(name = "EXPR", long = "eval")]
    eval: Option<String>,

    /// Go to interactive mode
    #[structopt(short, long)]
    interactive: bool,

    /// Load as ES6 module (default if .mjs file extension)
    #[structopt(short, long)]
    module: bool,

    /// Load the QJSCalc runtime (default if invoked as qjscalc)
    #[cfg(feature = "bignum")]
    #[structopt(long)]
    qjscalc: bool,

    /// Trace memory allocation
    #[structopt(short = "T", long = "trace")]
    trace_memory: bool,

    /// Dump the memory usage stats
    #[structopt(short, long = "dump")]
    dump_memory: bool,

    /// Just instantiate the interpreter and quit
    #[structopt(short = "q", long = "quit")]
    empty_run: bool,

    /// Make 'std' and 'os' invisible to non module code
    #[structopt(long = "nostd")]
    no_std: bool,
}

cfg_if! {
    if #[cfg(any(target_os = "macos", target_os = "ios"))] {
        const MALLOC_OVERHEAD: usize = 0;
    } else {
        const MALLOC_OVERHEAD: usize = 8;
    }
}

unsafe extern "C" fn js_trace_malloc(s: *mut ffi::JSMallocState, size: usize) -> *mut c_void {
    let s = s.as_mut().expect("state");

    if s.malloc_size + size > s.malloc_limit {
        null_mut()
    } else {
        let ptr = libc::malloc(size);

        trace!("A {} -> {:p}", size, ptr);

        if !ptr.is_null() {
            s.malloc_count += 1;
            s.malloc_size += js_trace_malloc_usable_size(ptr) + MALLOC_OVERHEAD;
        }

        ptr
    }
}

unsafe extern "C" fn js_trace_free(s: *mut ffi::JSMallocState, ptr: *mut c_void) {
    if !ptr.is_null() {
        trace!("F {:p}", ptr);

        let s = s.as_mut().expect("state");

        s.malloc_count -= 1;
        s.malloc_size -= js_trace_malloc_usable_size(ptr) + MALLOC_OVERHEAD;

        libc::free(ptr);
    }
}

unsafe extern "C" fn js_trace_realloc(
    s: *mut ffi::JSMallocState,
    ptr: *mut c_void,
    size: usize,
) -> *mut c_void {
    if ptr.is_null() {
        if size == 0 {
            null_mut()
        } else {
            js_trace_malloc(s, size)
        }
    } else {
        let s = s.as_mut().expect("state");
        let old_size = js_trace_malloc_usable_size(ptr);

        if size == 0 {
            trace!("R {} {:p}", size, ptr);

            s.malloc_count -= 1;
            s.malloc_size -= old_size + MALLOC_OVERHEAD;

            libc::free(ptr);

            null_mut()
        } else if s.malloc_size + size - old_size > s.malloc_limit {
            null_mut()
        } else {
            trace!("R {} {:p}", size, ptr);

            let ptr = libc::realloc(ptr, size);

            trace!(" -> {:p}", ptr);

            if !ptr.is_null() {
                s.malloc_size += js_trace_malloc_usable_size(ptr) - old_size;
            }

            ptr
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
unsafe extern "C" fn js_trace_malloc_usable_size(ptr: *const c_void) -> usize {
    libc::malloc_usable_size(ptr)
}

cfg_if! {
    if #[cfg(any(target_os = "macos", target_os = "ios"))] {
        extern "C" {
            pub fn malloc_size(ptr: *const c_void) -> libc::size_t;
        }

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        unsafe extern "C" fn js_trace_malloc_usable_size(ptr: *const c_void) -> usize {
            malloc_size(ptr)
        }
    }
}

unsafe extern "C" fn jsc_module_loader(
    ctx: *mut ffi::JSContext,
    module_name: *const c_char,
    opaque: *mut c_void,
) -> *mut ffi::JSModuleDef {
    let ctxt = ContextRef::from_ptr(ctx);
    let module_name = CStr::from_ptr(module_name).to_string_lossy();

    null_mut()
}

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let opt = Opt::from_clap(
        &Opt::clap()
            .version(format!("{} (quickjs {})", crate_version!(), ffi::VERSION.trim()).as_str())
            .get_matches(),
    );
    debug!("opts: {:?}", opt);

    let rt = if opt.trace_memory {
        Runtime::with_malloc_funcs::<()>(
            &MallocFunctions {
                js_malloc: Some(js_trace_malloc),
                js_free: Some(js_trace_free),
                js_realloc: Some(js_trace_realloc),
                js_malloc_usable_size: Some(js_trace_malloc_usable_size),
            },
            None,
        )
    } else {
        Runtime::new()
    };
    let ctxt = Context::new(&rt);

    // loader for ES6 modules
    rt.set_module_loader::<()>(None, Some(jsc_module_loader), None);

    if !opt.empty_run {}

    if opt.dump_memory {
        let stats = rt.memory_usage();

        unsafe {
            ffi::JS_DumpMemoryUsage(cfile::stdout()?.as_ptr() as *mut _, &stats, rt.as_ptr())
        };
    }

    if opt.empty_run && opt.dump_memory {
        let (d1, d2, d3, d4) = (0..100).fold(
            (
                Duration::from_secs(1),
                Duration::from_secs(1),
                Duration::from_secs(1),
                Duration::from_secs(1),
            ),
            |(d1, d2, d3, d4), _| {
                let ts0 = Instant::now();
                let rt = Runtime::new();
                let ts1 = Instant::now();
                let ctxt = Context::new(&rt);
                let ts2 = Instant::now();
                mem::drop(ctxt);
                let ts3 = Instant::now();
                mem::drop(rt);
                let ts4 = Instant::now();

                (
                    d1.min(ts1.duration_since(ts0)),
                    d2.min(ts2.duration_since(ts1)),
                    d3.min(ts3.duration_since(ts2)),
                    d4.min(ts4.duration_since(ts3)),
                )
            },
        );

        println!(
            "\nInstantiation times (ms): {:.3} = {:.3} + {:.3} + {:.3} + {:.3}",
            ((d1 + d2 + d3 + d4).as_micros() as f64) / 1000.0,
            (d1.as_micros() as f64) / 1000.0,
            (d2.as_micros() as f64) / 1000.0,
            (d3.as_micros() as f64) / 1000.0,
            (d4.as_micros() as f64) / 1000.0
        );
    }

    Ok(())
}
