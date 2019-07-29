#[macro_use]
extern crate log;

use std::collections::HashMap;
use std::env::current_dir;
use std::ffi::{CStr, OsStr};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::mem;
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

    /// quickjs directory
    #[structopt(short = "I", parse(from_os_str))]
    quickjs_dir: Option<PathBuf>,

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

    fn features(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        let mut date = !self.no_date;
        let mut eval = !self.no_eval;
        let mut string_normalize = !self.no_string_normalize;
        let mut regexp = !self.no_regexp;
        let mut json = !self.no_json;
        let mut proxy = !self.no_proxy;
        let mut map = !self.no_map;
        let mut typedarray = !self.no_typedarray;
        let mut promise = !self.no_promise;

        for feature in &self.features {
            match feature.as_str() {
                "date" => date = true,
                "no-date" => date = false,
                "eval" => eval = true,
                "no-eval" => eval = false,
                "string-normalize" => string_normalize = true,
                "no-string-normalize" => string_normalize = false,
                "regexp" => regexp = true,
                "no-regexp" => regexp = false,
                "json" => json = true,
                "no-json" => json = false,
                "proxy" => proxy = true,
                "no-proxy" => proxy = false,
                "map" => map = true,
                "no-map" => map = false,
                "typedarray" => typedarray = true,
                "no-typedarray" => typedarray = false,
                "promise" => promise = true,
                "no-promise" => promise = false,
                s => {
                    warn!("unknown feature: {}", s);
                }
            }
        }

        if date {
            v.push("Date");
        }
        if eval {
            v.push("Eval");
        }
        if string_normalize {
            v.push("StringNormalize");
        }
        if regexp {
            v.push("RegExp");
        }
        if json {
            v.push("JSON");
        }
        if proxy {
            v.push("Proxy");
        }
        if map {
            v.push("MapSet");
        }
        if typedarray {
            v.push("TypedArrays");
        }
        if promise {
            v.push("Promise")
        }

        v
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

pub struct Loader(Generator);

impl Deref for Loader {
    type Target = Generator;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Loader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Loader {
    fn new(gen: Generator) -> Self {
        Loader(gen)
    }
}

impl Loader {
    fn compile_file(
        &mut self,
        ctxt: &ContextRef,
        filename: &Path,
        cname: Option<String>,
        is_module: bool,
    ) -> Result<(), Error> {
        debug!("compile file {:?}", filename);

        let func = ctxt.eval_file(
            filename,
            Eval::COMPILE_ONLY
                | if is_module {
                    Eval::MODULE
                } else {
                    Eval::GLOBAL
                },
        )?;

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
        if let Some(short_name) = self.cmodules.get(module_name).cloned() {
            // add in the static init module list
            self.static_init_modules
                .insert(module_name.to_owned(), short_name.to_owned());

            // create a dummy module
            ctxt.new_c_module(module_name, Some(js_module_dummy_init))
                .ok()
        } else if module_name.ends_with(".so") || module_name.ends_with(".dylib") {
            warn!("binary module '{}' is not compiled", module_name);

            // create a dummy module
            ctxt.new_c_module(module_name, Some(js_module_dummy_init))
                .ok()
        } else {
            // compile the module
            ctxt.eval_file(module_name, Eval::MODULE | Eval::COMPILE_ONLY)
                .and_then(|func| {
                    let cname = get_c_name(module_name).expect("cname");

                    self.cnames.insert(cname.to_string(), true);

                    self.output_object_code(ctxt, &func, &cname)?;

                    Ok(func)
                })
                .map_err(|err| {
                    ctxt.throw_reference_error(format!(
                        "could not load module filename `{}`, {}",
                        module_name, err
                    ));

                    err
                })
                .ok()
                .map(|func| func.as_ptr())
        }
    }
}

fn get_c_name<S: AsRef<OsStr> + ?Sized>(s: &S) -> Option<&str> {
    Path::new(s).file_stem().and_then(|s| s.to_str())
}

enum Output {
    TempFile(tempfile::NamedTempFile),
    File(File),
    Stdout,
}

impl io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Output::TempFile(f) => f.write(buf),
            Output::File(f) => f.write(buf),
            Output::Stdout => io::stdout().lock().write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Output::TempFile(f) => f.flush(),
            Output::File(f) => f.flush(),
            Output::Stdout => io::stdout().lock().flush(),
        }
    }
}

pub struct Generator {
    opt: Opt,
    w: Output,
    cmodules: HashMap<String, String>,
    cnames: HashMap<String, bool>,
    static_init_modules: HashMap<String, String>,
}

impl Deref for Generator {
    type Target = Opt;

    fn deref(&self) -> &Self::Target {
        &self.opt
    }
}

impl DerefMut for Generator {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.opt
    }
}

impl Generator {
    fn new(opt: Opt) -> Result<Self, Error> {
        let w = if opt.output_type() == OutputType::Executable {
            Output::TempFile(
                tempfile::Builder::new()
                    .prefix("qjs-")
                    .suffix(".c")
                    .tempfile()?,
            )
        } else if opt.out_filename == Some("-".into()) {
            Output::Stdout
        } else {
            Output::File(File::create(opt.output_filename())?)
        };

        let cmodules = opt.cmodules();

        Ok(Generator {
            opt,
            w,
            cmodules,
            static_init_modules: HashMap::new(),
            cnames: HashMap::new(),
        })
    }

    fn output_header(&mut self) -> Result<(), Error> {
        writeln!(
            &mut self.w,
            "/* File generated automatically by the QuickJS compiler. */\n"
        )?;

        if self.opt.output_type() == OutputType::C {
            writeln!(&mut self.w, "#include <inttypes.h>\n")?;
        } else {
            writeln!(&mut self.w, "#include \"quickjs-libc.h\"\n")?;
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
        write!(
            &mut self.w,
            "const uint8_t {}[{}] = {{\n\t",
            cname,
            buf.len()
        )?;
        for (i, b) in buf.iter().enumerate() {
            if i > 0 && i % 8 == 0 {
                write!(&mut self.w, "\n\t")?;
            }

            write!(&mut self.w, "0x{:02x}, ", b)?;
        }
        writeln!(&mut self.w, "\n}};")?;

        Ok(())
    }

    fn output_c_main(&mut self) -> Result<(), Error> {
        writeln!(
            &mut self.w,
            r#"
int main(int argc, char **argv)
{{
    JSRuntime *rt;
    JSContext *ctx;

    rt = JS_NewRuntime();
    ctx = JS_NewContextRaw(rt);

    JS_AddIntrinsicBaseObjects(ctx);"#
        )?;

        for feature in self.features() {
            writeln!(&mut self.w, "    JS_AddIntrinsic{}(ctx);", feature)?;
        }

        writeln!(&mut self.w, "\n    js_std_add_helpers(ctx, argc, argv);")?;

        for (module, cname) in &self.static_init_modules {
            writeln!(
                &mut self.w,
                r#"
    {{
        extern JSModuleDef *js_init_module_{}(JSContext *ctx, const char *name);
        js_init_module_{}(ctx, "{}");
    }}
"#,
                cname, cname, module
            )?;
        }

        for (cname, &load_only) in &self.cnames {
            writeln!(
                &mut self.w,
                "    js_std_eval_binary(ctx, {}, {}_size, {});",
                cname,
                cname,
                if load_only {
                    "JS_EVAL_BINARY_LOAD_ONLY"
                } else {
                    "0"
                }
            )?;
        }

        writeln!(
            &mut self.w,
            r#"
    js_std_loop(ctx);

    JS_FreeContext(ctx);
    JS_FreeRuntime(rt);

    return 0;
}}
"#
        )?;
        Ok(())
    }

    fn output_executable(&mut self) -> Result<(), Error> {
        let out_filename = self.output_filename().to_path_buf();
        let target = platforms::guess_current().unwrap().target_triple;

        debug!("compile {:?} for `{}` target", out_filename, target);

        if let Output::TempFile(f) = mem::replace(&mut self.w, Output::Stdout) {
            f.as_file().sync_all()?;

            let path = f.into_temp_path();

            let mut build = cc::Build::new();

            if let Some(ref quickjs_dir) = self.quickjs_dir {
                build.include(quickjs_dir);
            }

            build
                .file(&path)
                .warnings(false)
                .extra_warnings(false)
                .cargo_metadata(false)
                .out_dir(
                    out_filename
                        .parent()
                        .map_or_else(|| current_dir().unwrap(), |p| p.to_path_buf()),
                )
                .target(target)
                .host(target)
                .opt_level(3)
                .compile(&out_filename.to_string_lossy());
        }

        Ok(())
    }
}

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let opt = Opt::from_clap(
        &Opt::clap()
            .version(qjs::LONG_VERSION.as_str())
            .get_matches(),
    );
    debug!("opts: {:?}", opt);

    let mut gen = Generator::new(opt)?;

    gen.output_header()?;

    let output_type = gen.output_type();

    let rt = Runtime::new();
    let ctxt = Context::builder(&rt)
        .with_eval()
        .with_regexp_compiler()
        .build();

    let mut loader = Loader::new(gen);

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

    if output_type != OutputType::C {
        loader.output_c_main()?;
    }

    if output_type == OutputType::Executable {
        loader.output_executable()?;
    }

    Ok(())
}
