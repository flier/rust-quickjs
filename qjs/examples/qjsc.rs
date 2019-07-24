#[macro_use]
extern crate log;

use std::collections::HashMap;
use std::ffi::{CStr, OsStr};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::ops::{Deref, DerefMut};
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr::{null_mut, NonNull};

use failure::Error;
use structopt::StructOpt;

use foreign_types::ForeignTypeRef;
use qjs::{ffi, Context, ContextRef, Eval, Runtime, Value, WriteObj};

#[derive(Clone, Copy, Debug, PartialEq)]
enum OutputType {
    C,
    CMain,
    Executable,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "qjsc", about = "QuickJS command line compiler")]
pub struct Opt {
    /// Set the output filename
    #[structopt(name = "output", short = "o", parse(from_os_str))]
    out_filename: Option<PathBuf>,

    /// Only output bytecode in a C file
    #[structopt(short = "c")]
    output_c: bool,

    /// Output main() and bytecode in a C file (default = executable output)
    #[structopt(short = "e")]
    output_c_main: bool,

    /// Set the C name of the generated data
    #[structopt(short = "N")]
    cname: Option<String>,

    /// Compile as Javascript module
    #[structopt(short = "m")]
    module: bool,

    /// Add initialization code for an external C module
    #[structopt(name = "module_name[,cname]", short = "M")]
    module_names: Vec<String>,

    /// Byte swapped output
    #[structopt(short = "x")]
    byte_swap: bool,

    /// Disable selected language features (smaller code size)
    #[structopt(short = "f")]
    features: Vec<String>,

    /// Use link time optimization
    #[structopt(long = "lto")]
    use_lto: bool,

    /// Disable the `Date` feature
    #[structopt(long = "no-date")]
    no_date: bool,

    /// Disable the `Eval` feature
    #[structopt(long = "no-eval")]
    no_eval: bool,

    /// Disable the `StringNormalize` feature
    #[structopt(long = "no-string-normalize")]
    no_string_normalize: bool,

    /// Disable the `RegExp` feature
    #[structopt(long = "no-regexp")]
    no_regexp: bool,

    /// Disable the `JSON` feature
    #[structopt(long = "no-json")]
    no_json: bool,

    /// Disable the `Proxy` feature
    #[structopt(long = "no-proxy")]
    no_proxy: bool,

    /// Disable the `MapSet` feature
    #[structopt(long = "no-map")]
    no_map: bool,

    /// Disable the `TypedArrays` feature
    #[structopt(long = "no-typedarray")]
    no_typedarray: bool,

    /// Disable the `Promise` feature
    #[structopt(long = "no-promise")]
    no_promise: bool,

    /// Compile Javascript files to C module
    #[structopt(parse(from_os_str))]
    files: Vec<PathBuf>,
}

impl Opt {
    fn output_filename(&self) -> &Path {
        self.out_filename.as_ref().map_or_else(
            || {
                Path::new(match self.output_type() {
                    OutputType::Executable => "a.out",
                    _ => "out.c",
                })
            },
            |p| p.as_path(),
        )
    }

    fn output_type(&self) -> OutputType {
        if self.output_c {
            OutputType::C
        } else if self.output_c_main {
            OutputType::CMain
        } else {
            OutputType::Executable
        }
    }

    fn cmodules(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();

        m.insert("std".to_owned(), "std".to_owned());
        m.insert("os".to_owned(), "os".to_owned());

        for s in &self.module_names {
            if let Some(pos) = s.find(',') {
                let (path, rest) = s.split_at(pos);
                let (_, cname) = rest.split_at(1);

                m.insert(path.to_owned(), cname.to_owned());
            } else {
                let cname = get_c_name(s).expect("cname");

                m.insert(s.to_owned(), cname.to_owned());
            }
        }

        m
    }
}

unsafe extern "C" fn jsc_module_loader(
    ctx: *mut ffi::JSContext,
    module_name: *const c_char,
    opaque: *mut c_void,
) -> *mut ffi::JSModuleDef {
    let ctxt = ContextRef::from_ptr(ctx);
    let module_name = CStr::from_ptr(module_name).to_string_lossy();
    let mut loader = NonNull::new(opaque).expect("loader").cast::<Loader>();

    debug!("load module {}", module_name);

    loader
        .as_mut()
        .load_module(ctxt, &module_name)
        .map_or_else(null_mut, |p| p.as_ptr())
}

unsafe extern "C" fn js_module_dummy_init(
    _ctx: *mut ffi::JSContext,
    _m: *mut ffi::JSModuleDef,
) -> c_int {
    unreachable!()
}

pub struct Loader {
    gen: Generator<File>,
    cmodules: HashMap<String, String>,
    static_init_modules: HashMap<String, String>,
    cnames: HashMap<String, bool>,
}

impl Deref for Loader {
    type Target = Generator<File>;

    fn deref(&self) -> &Self::Target {
        &self.gen
    }
}

impl DerefMut for Loader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.gen
    }
}

impl Loader {
    fn new(gen: Generator<File>) -> Self {
        let cmodules = gen.cmodules();

        Loader {
            gen,
            cmodules,
            static_init_modules: HashMap::new(),
            cnames: HashMap::new(),
        }
    }

    fn compile_file(
        &mut self,
        ctxt: &ContextRef,
        filename: &Path,
        cname: Option<String>,
        is_module: bool,
    ) -> Result<(), Error> {
        debug!("compile file {:?}", filename);

        let s = load_file(filename)?;
        let mut eval_flags = Eval::SHEBANG | Eval::COMPILE_ONLY;

        if is_module {
            eval_flags |= Eval::MODULE;
        } else {
            eval_flags |= Eval::GLOBAL;
        }

        let func = ctxt.eval(s, &filename.to_string_lossy(), eval_flags)?;

        let cname = cname
            .as_ref()
            .map(|s| s.as_str())
            .or_else(|| get_c_name(filename))
            .expect("cname");

        self.cnames.insert(cname.to_string(), false);

        self.output_object_code(ctxt, &func, &cname)?;

        Ok(())
    }

    fn load_module(
        &mut self,
        ctxt: &ContextRef,
        module_name: &str,
    ) -> Option<NonNull<ffi::JSModuleDef>> {
        // check if it is a declared C or system module
        if let Some(short_name) = self.cmodules.get(module_name) {
            // add in the static init module list
            self.static_init_modules
                .insert(module_name.to_owned(), short_name.clone());

            // create a dummy module
            ctxt.new_c_module(module_name, Some(js_module_dummy_init))
        } else if module_name.ends_with(".so") || module_name.ends_with(".dylib") {
            warn!("binary module '{}' is not compiled", module_name);

            // create a dummy module
            ctxt.new_c_module(module_name, Some(js_module_dummy_init))
        } else {
            match load_file(module_name) {
                Ok(s) => {
                    // compile the module
                    ctxt.eval(s, module_name, Eval::MODULE | Eval::COMPILE_ONLY)
                        .and_then(|func| {
                            let cname = get_c_name(module_name).expect("cname");

                            self.cnames.insert(cname.to_string(), true);

                            self.output_object_code(ctxt, &func, &cname)?;

                            Ok(func.as_ptr())
                        })
                        .ok()
                }
                Err(err) => {
                    ctxt.throw_reference_error(format!(
                        "could not load module filename `{}`, {}",
                        module_name, err
                    ));

                    None
                }
            }
        }
    }
}

fn load_file<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut f = File::open(path)?;
    let mut s = String::new();

    f.read_to_string(&mut s)?;

    Ok(s)
}

fn get_c_name<S: AsRef<OsStr> + ?Sized>(s: &S) -> Option<&str> {
    Path::new(s).file_stem().and_then(|s| s.to_str())
}

pub struct Generator<W> {
    opt: Opt,
    w: W,
}

impl<W> Deref for Generator<W> {
    type Target = Opt;

    fn deref(&self) -> &Self::Target {
        &self.opt
    }
}

impl<W> DerefMut for Generator<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.opt
    }
}

impl Generator<File> {
    fn new(opt: Opt) -> Result<Self, Error> {
        let w = if opt.output_type() == OutputType::Executable {
            tempfile::tempfile()?
        } else {
            File::open(opt.output_filename())?
        };

        Ok(Generator { opt, w: w })
    }
}

impl<W> Generator<W>
where
    W: io::Write,
{
    fn output_header(&mut self) -> Result<(), Error> {
        writeln!(
            &mut self.w,
            "/* File generated automatically by the QuickJS compiler. */"
        )?;

        if self.opt.output_type() == OutputType::C {
            writeln!(&mut self.w, "#include <inttypes.h>")?;
        } else {
            writeln!(&mut self.w, r#"#include "quickjs-libc.h""#)?;
        }

        Ok(())
    }

    fn output_object_code(
        &mut self,
        ctxt: &ContextRef,
        obj: &Value,
        cname: &str,
    ) -> Result<(), Error> {
        let mut flags = WriteObj::BYTECODE;

        if self.byte_swap {
            flags |= WriteObj::BSWAP;
        }

        let buf = ctxt.write_object(obj, flags)?;

        writeln!(
            &mut self.w,
            "const uint32_t {}_size = {};",
            cname,
            buf.len()
        )?;
        writeln!(&mut self.w, "const uint8_t {}[{}] = {{", cname, buf.len())?;
        for (i, b) in buf.iter().enumerate() {
            write!(&mut self.w, "0x{:02x}, ", b)?;

            if i > 0 && i % 8 == 0 {
                writeln!(&mut self.w, "")?;
            }
        }
        writeln!(&mut self.w, "}};")?;

        Ok(())
    }
}

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let opt = Opt::from_args();
    debug!("opts: {:?}", opt);

    let mut gen = Generator::new(opt)?;

    gen.output_header()?;

    let rt = Runtime::new();
    let ctxt = Context::builder(&rt)
        .with_eval()
        .with_regexp_compiler()
        .build();

    let mut loader = Box::pin(Loader::new(gen));

    // loader for ES6 modules
    rt.set_module_loader(None, Some(jsc_module_loader), Some(NonNull::from(&loader)));

    let files = loader.files.drain(..).collect::<Vec<_>>();
    let mut cname = loader.cname.take();
    let module = loader.module;

    for filename in files {
        loader.compile_file(
            &ctxt,
            &filename,
            cname.take(),
            module || filename.ends_with(".mjs"),
        )?;
    }

    Ok(())
}
