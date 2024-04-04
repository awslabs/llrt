// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cmp::min,
    collections::{HashMap, HashSet},
    env::{self},
    ffi::CStr,
    future::Future,
    io::{self},
    path::{Component, Path, PathBuf},
    process::exit,
    result::Result as StdResult,
    sync::atomic::{AtomicUsize, Ordering},
    sync::{Arc, Mutex},
};

use once_cell::sync::Lazy;

use chrono::Utc;
use ring::rand::SecureRandom;
use rquickjs::{
    atom::PredefinedAtom,
    context::EvalOptions,
    function::{Constructor, Opt},
    loader::{
        BuiltinLoader, BuiltinResolver, FileResolver, Loader, ModuleLoader, RawLoader, Resolver,
        ScriptLoader,
    },
    module::{ModuleData, ModuleDef},
    prelude::{Func, Rest},
    qjs, AsyncContext, AsyncRuntime, CatchResultExt, CaughtError, Ctx, Error, Function, IntoJs,
    Module, Object, Result, String as JsString, Value,
};
use tokio::sync::oneshot::{self, Receiver};
use tracing::trace;
use zstd::{bulk::Decompressor, dict::DecoderDictionary};

include!("./bytecode_cache.rs");

use crate::{
    buffer::BufferModule,
    bytecode::{BYTECODE_COMPRESSED, BYTECODE_UNCOMPRESSED, BYTECODE_VERSION, SIGNATURE_LENGTH},
    child_process::ChildProcessModule,
    console,
    console::ConsoleModule,
    crypto::{CryptoModule, SYSTEM_RANDOM},
    encoding::HexModule,
    environment,
    events::EventsModule,
    fs::{FsModule, FsPromisesModule},
    json::{parse::json_parse, stringify::json_stringify_replacer_space},
    module::ModuleModule,
    navigator::NavigatorModule,
    net::NetModule,
    number::number_to_string,
    os::OsModule,
    path::{dirname, join_path, resolve_path, PathModule},
    performance::PerformanceModule,
    process::ProcessModule,
    timers::TimersModule,
    url::UrlModule,
    utils::{
        class::get_class_name,
        clone::structured_clone,
        io::get_js_path,
        object::{get_bytes, ObjectExt},
        UtilModule,
    },
    uuid::UuidModule,
    xml::XmlModule,
};

pub static TIME_ORIGIN: AtomicUsize = AtomicUsize::new(0);

macro_rules! create_modules {
    ($($name:expr => $module:expr),*) => {

        pub fn create_module_instances() -> (ModuleResolver, ModuleLoader, HashSet<&'static str>) {
            let mut builtin_resolver = ModuleResolver::default();
            let mut module_loader = ModuleLoader::default();
            let mut module_names = HashSet::new();

            $(
                let module_info = ModuleInfo {
                    name: $name,
                    module: $module,
                };

                builtin_resolver = builtin_resolver.with_module(module_info.name);
                module_loader = module_loader.with_module(module_info.name, module_info.module);
                module_names.insert(module_info.name);
            )*

            (builtin_resolver, module_loader, module_names)
        }
    };
}

#[derive(Debug, Default)]
pub struct ModuleResolver {
    builtin_resolver: BuiltinResolver,
}

impl ModuleResolver {
    #[must_use]
    pub fn with_module<P: Into<String>>(mut self, path: P) -> Self {
        self.builtin_resolver.add_module(path.into());
        self
    }
}

impl Resolver for ModuleResolver {
    fn resolve(&mut self, ctx: &Ctx<'_>, base: &str, name: &str) -> Result<String> {
        // Strip node prefix so that we support both with and without
        let name = name.strip_prefix("node:").unwrap_or(name);

        self.builtin_resolver.resolve(ctx, base, name)
    }
}

create_modules!(
    "crypto" => CryptoModule,
    "hex" => HexModule,
    "fs/promises" => FsPromisesModule,
    "fs" => FsModule,
    "os" => OsModule,
    "timers" => TimersModule,
    "events" => EventsModule,
    "module" => ModuleModule,
    "net" => NetModule,
    "console" => ConsoleModule,
    "path" => PathModule,
    "xml" => XmlModule,
    "buffer" => BufferModule,
    "child_process" => ChildProcessModule,
    "util" => UtilModule,
    "uuid" => UuidModule,
    "process" => ProcessModule,
    "navigator" => NavigatorModule,
    "url" => UrlModule,
    "performance" => PerformanceModule
);

struct ModuleInfo<T: ModuleDef> {
    name: &'static str,
    module: T,
}

pub struct ErrorDetails {
    pub msg: String,
    pub r#type: String,
    pub stack: String,
}

#[inline]
pub fn uncompressed_size(input: &[u8]) -> StdResult<(usize, &[u8]), io::Error> {
    let size = input.get(..4).ok_or(io::ErrorKind::InvalidInput)?;
    let size: &[u8; 4] = size.try_into().map_err(|_| io::ErrorKind::InvalidInput)?;
    let uncompressed_size = u32::from_le_bytes(*size) as usize;
    let rest = &input[4..];
    Ok((uncompressed_size, rest))
}

pub(crate) static COMPRESSION_DICT: &[u8] = include_bytes!("../bundle/compression.dict");

static DECOMPRESSOR_DICT: Lazy<DecoderDictionary> =
    Lazy::new(|| DecoderDictionary::copy(COMPRESSION_DICT));

fn print(value: String, stdout: Opt<bool>) {
    if stdout.0.unwrap_or_default() {
        println!("{value}");
    } else {
        eprintln!("{value}")
    }
}

#[derive(Debug)]
pub struct BinaryResolver {
    paths: Vec<PathBuf>,
    cwd: PathBuf,
}
impl BinaryResolver {
    pub fn add_path<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        self.paths.push(path.into());
        self
    }

    pub fn get_bin_path(path: &Path) -> PathBuf {
        path.with_extension("lrt")
    }

    pub fn normalize<P: AsRef<Path>>(path: P) -> PathBuf {
        let ends_with_slash = path.as_ref().to_str().map_or(false, |s| s.ends_with('/'));
        let mut normalized = PathBuf::new();
        for component in path.as_ref().components() {
            match &component {
                Component::ParentDir => {
                    if !normalized.pop() {
                        normalized.push(component);
                    }
                }
                _ => {
                    normalized.push(component);
                }
            }
        }
        if ends_with_slash {
            normalized.push("");
        }
        normalized
    }
}

impl Default for BinaryResolver {
    fn default() -> Self {
        let cwd = env::current_dir().unwrap();
        Self {
            cwd,
            paths: Vec::with_capacity(10),
        }
    }
}

#[allow(clippy::manual_strip)]
impl Resolver for BinaryResolver {
    fn resolve(&mut self, _ctx: &Ctx, base: &str, name: &str) -> Result<String> {
        trace!("Try resolve \"{}\" from \"{}\"", name, base);

        if BYTECODE_CACHE.contains_key(name) {
            return Ok(name.to_string());
        }

        let base_path = Path::new(base);
        let base_path = if base_path.is_dir() {
            if base_path == self.cwd {
                Path::new(".")
            } else {
                base_path
            }
        } else {
            base_path.parent().unwrap_or(base_path)
        };

        let normalized_path = base_path.join(name);

        let normalized_path = BinaryResolver::normalize(normalized_path);
        let mut normalized_path = normalized_path.to_str().unwrap();
        let cache_path = if normalized_path.starts_with("./") {
            &normalized_path[2..]
        } else {
            normalized_path
        };

        let cache_key = Path::new(cache_path).with_extension("js");
        let cache_key = cache_key.to_str().unwrap();

        trace!("Normalized path: {}, key: {}", normalized_path, cache_key);

        if BYTECODE_CACHE.contains_key(cache_key) {
            return Ok(cache_key.to_string());
        }

        if BYTECODE_CACHE.contains_key(base) {
            normalized_path = name;
            if Path::new(normalized_path).exists() {
                return Ok(normalized_path.to_string());
            }
        }

        if Path::new(normalized_path).exists() {
            return Ok(normalized_path.to_string());
        }

        let path = self
            .paths
            .iter()
            .find_map(|path| {
                let path = path.join(normalized_path);
                let bin_path = BinaryResolver::get_bin_path(&path);
                if bin_path.exists() {
                    return Some(bin_path);
                }
                get_js_path(path.to_str().unwrap())
            })
            .ok_or_else(|| Error::new_resolving(base, name))?;

        Ok(path.into_os_string().into_string().unwrap())
    }
}

#[derive(Debug)]
pub struct BinaryLoader;

impl Default for BinaryLoader {
    fn default() -> Self {
        Self
    }
}

struct RawLoaderContainer<T>
where
    T: RawLoader + 'static,
{
    loader: T,
    cwd: String,
}
impl<T> RawLoaderContainer<T>
where
    T: RawLoader + 'static,
{
    fn new(loader: T) -> Self {
        Self {
            loader,
            cwd: std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string(),
        }
    }
}

unsafe impl<T> RawLoader for RawLoaderContainer<T>
where
    T: RawLoader + 'static,
{
    #[allow(clippy::manual_strip)]
    unsafe fn raw_load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> Result<Module<'js>> {
        let res = self.loader.raw_load(ctx, name)?;

        let name = if name.starts_with("./") {
            &name[2..]
        } else {
            name
        };

        if name.starts_with('/') {
            set_import_meta(&res, name)?;
        } else {
            set_import_meta(&res, &format!("{}/{}", &self.cwd, name))?;
        };

        Ok(res)
    }
}

impl Loader for BinaryLoader {
    fn load(&mut self, _ctx: &Ctx<'_>, name: &str) -> Result<ModuleData> {
        trace!("Loading module: {}", name);
        if let Some(bytes) = BYTECODE_CACHE.get(name) {
            trace!("Loading embedded module: {}", name);

            return load_bytecode_module(name, bytes);
        }
        let path = PathBuf::from(name);
        let mut bytes: &[u8] = &std::fs::read(path)?;

        if name.ends_with(".lrt") {
            trace!("Loading binary module: {}", name);
            return load_bytecode_module(name, bytes);
        }
        if bytes.starts_with(b"#!") {
            bytes = bytes.splitn(2, |&c| c == b'\n').nth(1).unwrap_or(bytes);
        }
        Ok(ModuleData::source(name, bytes))
    }
}

pub fn load_bytecode_module(name: &str, buf: &[u8]) -> Result<ModuleData> {
    let bytes = load_module(buf)?;
    Ok(unsafe { ModuleData::bytecode(name, bytes) })
}

fn load_module(input: &[u8]) -> Result<Vec<u8>> {
    let (_, compressed, input) = get_bytecode_signature(input)?;

    if compressed {
        let (size, input) = uncompressed_size(input)?;
        let mut buf = Vec::with_capacity(size);
        let mut decompressor = Decompressor::with_prepared_dictionary(&DECOMPRESSOR_DICT)?;
        decompressor.decompress_to_buffer(input, &mut buf)?;
        return Ok(buf);
    }

    Ok(input.to_vec())
}

fn get_bytecode_signature(input: &[u8]) -> StdResult<(&[u8], bool, &[u8]), io::Error> {
    let raw_signature = input
        .get(..SIGNATURE_LENGTH)
        .ok_or(io::Error::new::<String>(
            io::ErrorKind::InvalidInput,
            "Invalid bytecode signature length".into(),
        ))?;

    let (last, signature) = raw_signature.split_last().unwrap();

    if signature != BYTECODE_VERSION.as_bytes() {
        return Err(io::Error::new::<String>(
            io::ErrorKind::InvalidInput,
            "Invalid bytecode version".into(),
        ));
    }

    let mut compressed = None;
    if *last == BYTECODE_COMPRESSED {
        compressed = Some(true)
    } else if *last == BYTECODE_UNCOMPRESSED {
        compressed = Some(false)
    }

    let rest = &input[SIGNATURE_LENGTH..];
    Ok((
        signature,
        compressed.ok_or(io::Error::new::<String>(
            io::ErrorKind::InvalidInput,
            "Invalid bytecode signature".into(),
        ))?,
        rest,
    ))
}

pub struct Vm {
    pub runtime: AsyncRuntime,
    pub ctx: AsyncContext,
}

struct LifetimeArgs<'js>(Ctx<'js>);

#[allow(dead_code)]
struct ExportArgs<'js>(Ctx<'js>, Object<'js>, Value<'js>, Value<'js>);

impl Vm {
    pub const ENV_LAMBDA_TASK_ROOT: &'static str = "LAMBDA_TASK_ROOT";

    pub async fn new() -> StdResult<Self, Box<dyn std::error::Error + Send + Sync>> {
        if TIME_ORIGIN.load(Ordering::Relaxed) == 0 {
            let time_origin = Utc::now().timestamp_nanos_opt().unwrap_or_default() as usize;
            TIME_ORIGIN.store(time_origin, Ordering::Relaxed)
        }

        SYSTEM_RANDOM
            .fill(&mut [0; 8])
            .expect("Failed to initialize SystemRandom");

        let mut file_resolver = FileResolver::default();
        let mut binary_resolver = BinaryResolver::default();
        let mut paths: Vec<&str> = Vec::with_capacity(10);

        paths.push(".");

        let task_root = env::var(Self::ENV_LAMBDA_TASK_ROOT).unwrap_or_else(|_| String::from(""));
        let task_root = task_root.as_str();
        if cfg!(debug_assertions) {
            paths.push("bundle");
        } else {
            paths.push("/opt");
        }

        if !task_root.is_empty() {
            paths.push(task_root);
        }

        for path in paths.iter() {
            file_resolver.add_path(*path);
            binary_resolver.add_path(*path);
        }

        let (builtin_resolver, module_loader, module_names) = create_module_instances();

        let resolver = (builtin_resolver, binary_resolver, file_resolver);

        let loader = RawLoaderContainer::new((
            module_loader,
            BinaryLoader,
            BuiltinLoader::default(),
            ScriptLoader::default()
                .with_extension("mjs")
                .with_extension("cjs"),
        ));

        const DEFAULT_GC_THRESHOLD_MB: usize = 20;

        let gc_threshold_mb: usize = env::var(environment::ENV_LLRT_GC_THRESHOLD_MB)
            .map(|threshold| threshold.parse().unwrap_or(DEFAULT_GC_THRESHOLD_MB))
            .unwrap_or(DEFAULT_GC_THRESHOLD_MB);

        let runtime = AsyncRuntime::new()?;
        runtime.set_max_stack_size(512 * 1024).await;
        runtime
            .set_gc_threshold(gc_threshold_mb * 1024 * 1024)
            .await;
        runtime.set_loader(resolver, loader).await;
        let ctx = AsyncContext::full(&runtime).await?;
        ctx.with(|ctx| {
            crate::console::init(&ctx)?;
            crate::encoding::init(&ctx)?;
            crate::http::init(&ctx)?;
            crate::timers::init(&ctx)?;
            crate::process::init(&ctx)?;
            crate::events::init(&ctx)?;
            crate::buffer::init(&ctx)?;
            crate::navigator::init(&ctx)?;
            crate::performance::init(&ctx)?;
            init(&ctx, module_names)?;
            Ok::<_, Error>(())
        })
        .await?;

        Ok(Vm { runtime, ctx })
    }

    pub fn load_module<'js>(ctx: &Ctx<'js>, filename: PathBuf) -> Result<Object<'js>> {
        Module::import(ctx, filename.to_string_lossy().to_string())
    }

    pub async fn run_module(ctx: &AsyncContext, filename: &Path) {
        Self::run_and_handle_exceptions(ctx, |ctx| {
            let _res = Vm::load_module(&ctx, filename.to_path_buf())?;
            Ok(())
        })
        .await
    }

    pub async fn run_and_handle_exceptions<'js, F>(ctx: &AsyncContext, f: F)
    where
        F: FnOnce(Ctx) -> rquickjs::Result<()> + Send,
    {
        ctx.with(|ctx| {
            f(ctx.clone())
                .catch(&ctx)
                .unwrap_or_else(|err| Self::print_error_and_exit(&ctx, err));
        })
        .await;
    }

    pub fn print_error_and_exit<'js>(ctx: &Ctx<'js>, err: CaughtError<'js>) -> ! {
        let ErrorDetails {
            msg,
            r#type: _,
            stack: _,
        } = Self::error_details(ctx, &err);
        eprintln!("{}", msg);
        exit(1)
    }

    pub fn error_details<'js>(ctx: &Ctx<'js>, err: &CaughtError<'js>) -> ErrorDetails {
        let (mut err_stack, mut err_type): (String, String) =
            (String::default(), String::from("Error"));
        let error_msg = match err {
            CaughtError::Error(err) => format!("Error: {:?}", &err),
            CaughtError::Exception(ex) => {
                let error_name = get_class_name(ex)
                    .unwrap_or(None)
                    .unwrap_or(String::from("Error"));

                let mut str = String::with_capacity(100);
                str.push_str(&error_name);
                str.push_str(": ");
                str.push_str(&ex.message().unwrap_or_default());
                str.push('\n');
                err_type = error_name;
                if let Some(stack) = ex.stack() {
                    str.push_str(&stack);
                    err_stack = stack;
                }
                str
            }
            CaughtError::Value(value) => {
                let log_msg = console::format(ctx, Rest(vec![value.clone()]))
                    .unwrap_or(String::from("{unknown value}"));
                format!("Error: {}", &log_msg)
            }
        };
        ErrorDetails {
            msg: error_msg,
            r#type: err_type,
            stack: err_stack,
        }
    }

    pub async fn idle(self) -> StdResult<(), Box<dyn std::error::Error + Sync + Send>> {
        self.runtime.idle().await;

        drop(self.ctx);
        drop(self.runtime);
        Ok(())
    }
}

fn json_parse_string<'js>(ctx: Ctx<'js>, value: Value<'js>) -> Result<Value<'js>> {
    let bytes = get_bytes(&ctx, value)?;
    json_parse(&ctx, bytes)
}

fn run_gc(ctx: Ctx<'_>) {
    trace!("Running GC");

    unsafe {
        let rt = qjs::JS_GetRuntime(ctx.as_raw().as_ptr());
        qjs::JS_RunGC(rt);
    };
}

fn init(ctx: &Ctx<'_>, module_names: HashSet<&'static str>) -> Result<()> {
    let globals = ctx.globals();

    globals.set("__gc", Func::from(run_gc))?;

    let number: Function = globals.get(PredefinedAtom::Number)?;
    let number_proto: Object = number.get(PredefinedAtom::Prototype)?;
    number_proto.set(PredefinedAtom::ToString, Func::from(number_to_string))?;

    globals.set("global", ctx.globals())?;
    globals.set("self", ctx.globals())?;
    globals.set("load", Func::from(load))?;
    globals.set("print", Func::from(print))?;
    globals.set(
        "structuredClone",
        Func::from(|ctx, value, options| structured_clone(&ctx, value, options)),
    )?;

    let json_module: Object = globals.get(PredefinedAtom::JSON)?;
    json_module.set("parse", Func::from(json_parse_string))?;
    json_module.set(
        "stringify",
        Func::from(|ctx, value, replacer, space| {
            struct StringifyArgs<'js>(Ctx<'js>, Value<'js>, Opt<Value<'js>>, Opt<Value<'js>>);
            let StringifyArgs(ctx, value, replacer, space) =
                StringifyArgs(ctx, value, replacer, space);

            let mut space_value = None;
            let mut replacer_value = None;

            if let Some(replacer) = replacer.0 {
                if let Some(space) = space.0 {
                    if let Some(space) = space.as_string() {
                        let mut space = space.clone().to_string()?;
                        space.truncate(20);
                        space_value = Some(space);
                    }
                    if let Some(number) = space.as_int() {
                        if number > 0 {
                            space_value = Some(" ".repeat(min(10, number as usize)));
                        }
                    }
                }
                replacer_value = Some(replacer);
            }

            json_stringify_replacer_space(&ctx, value, replacer_value, space_value)
                .map(|v| v.into_js(&ctx))?
        }),
    )?;

    #[allow(clippy::arc_with_non_send_sync)]
    let require_in_progress: Arc<Mutex<HashMap<String, Object>>> =
        Arc::new(Mutex::new(HashMap::new()));

    #[allow(clippy::arc_with_non_send_sync)]
    let require_exports: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
    let require_exports_ref = require_exports.clone();
    let require_exports_ref_2 = require_exports.clone();

    let js_bootstrap = Object::new(ctx.clone())?;
    js_bootstrap.set(
        "moduleExport",
        Func::from(move |ctx, obj, prop, value| {
            let ExportArgs(_ctx, _, _, value) = ExportArgs(ctx, obj, prop, value);
            let mut exports = require_exports.lock().unwrap();
            exports.replace(value);
            Result::Ok(true)
        }),
    )?;
    js_bootstrap.set(
        "exports",
        Func::from(move |ctx, obj, prop, value| {
            let ExportArgs(ctx, _, prop, value) = ExportArgs(ctx, obj, prop, value);
            let mut exports = require_exports_ref.lock().unwrap();
            let exports = if exports.is_some() {
                exports.as_ref().unwrap()
            } else {
                exports.replace(Object::new(ctx.clone())?.into_value());
                exports.as_ref().unwrap()
            };
            exports.as_object().unwrap().set(prop, value)?;
            Result::Ok(true)
        }),
    )?;
    globals.set("__bootstrap", js_bootstrap)?;

    globals.set(
        "require",
        Func::from(move |ctx, specifier: String| -> Result<Value> {
            let LifetimeArgs(ctx) = LifetimeArgs(ctx);
            let specifier: String = specifier
                .strip_prefix("node:")
                .unwrap_or(specifier.as_str())
                .into();
            let import_name = if module_names.contains(specifier.as_str())
                || BYTECODE_CACHE.contains_key(&specifier)
                || specifier.starts_with('/')
            {
                specifier
            } else {
                let module_name = get_script_or_module_name(ctx.clone());
                let abs_path = resolve_path([module_name].iter());
                let import_directory = dirname(abs_path);
                join_path(vec![import_directory, specifier])
            };

            let mut map = require_in_progress.lock().unwrap();
            if let Some(obj) = map.get(&import_name) {
                return Ok(obj.clone().into_value());
            }

            let obj = Object::new(ctx.clone())?;
            map.insert(import_name.clone(), obj.clone());
            drop(map);

            trace!("Require: {}", import_name);

            let imported_object: Object = Module::import(&ctx, import_name.clone())?;
            require_in_progress.lock().unwrap().remove(&import_name);

            if let Some(exports) = require_exports_ref_2.lock().unwrap().take() {
                if let Some(exports) = exports.as_object() {
                    for prop in exports.props::<Value, Value>() {
                        let (key, value) = prop?;
                        obj.set(key, value)?;
                    }
                } else {
                    return Ok(exports);
                }
            }

            for prop in imported_object.props::<String, Value>() {
                let (key, value) = prop?;
                obj.set(key, value)?;
            }

            Ok(obj.into_value())
        }),
    )?;

    Module::import(ctx, "@llrt/std")?;

    Ok(())
}

fn load<'js>(ctx: Ctx<'js>, filename: String, options: Opt<Object<'js>>) -> Result<Value<'js>> {
    let mut eval_options = EvalOptions {
        global: true,
        strict: false,
        backtrace_barrier: false,
    };

    if let Some(options) = options.0 {
        if let Some(global) = options.get_optional("global")? {
            eval_options.global = global;
        }

        if let Some(strict) = options.get_optional("strict")? {
            eval_options.strict = strict;
        }
    }

    ctx.eval_file_with_options(filename, eval_options)
}

fn get_script_or_module_name(ctx: Ctx<'_>) -> String {
    unsafe {
        let ctx_ptr = ctx.as_raw().as_ptr();
        let atom = qjs::JS_GetScriptOrModuleName(ctx_ptr, 0);
        let c_str = qjs::JS_AtomToCString(ctx_ptr, atom);
        if c_str.is_null() {
            qjs::JS_FreeCString(ctx_ptr, c_str);
            return String::from(".");
        }
        let bytes = CStr::from_ptr(c_str).to_bytes();
        let res = std::str::from_utf8_unchecked(bytes).to_string();
        qjs::JS_FreeCString(ctx_ptr, c_str);
        res
    }
}

fn set_import_meta(module: &Module<'_>, filepath: &str) -> Result<()> {
    let meta: Object = module.meta()?;
    meta.prop("url", format!("file://{}", filepath))?;
    Ok(())
}

pub trait ErrorExtensions<'js> {
    fn into_value(self, ctx: &Ctx<'js>) -> Result<Value<'js>>;
}

impl<'js> ErrorExtensions<'js> for Error {
    fn into_value(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        Err::<(), _>(self).catch(ctx).unwrap_err().into_value(ctx)
    }
}

impl<'js> ErrorExtensions<'js> for CaughtError<'js> {
    fn into_value(self, ctx: &Ctx<'js>) -> Result<Value<'js>> {
        Ok(match self {
            CaughtError::Error(err) => {
                JsString::from_str(ctx.clone(), &err.to_string())?.into_value()
            }
            CaughtError::Exception(ex) => ex.into_value(),
            CaughtError::Value(val) => val,
        })
    }
}

pub trait CtxExtension<'js> {
    fn spawn_exit<F, R>(&self, future: F) -> Result<Receiver<R>>
    where
        F: Future<Output = Result<R>> + 'js,
        R: 'js;
}

impl<'js> CtxExtension<'js> for Ctx<'js> {
    fn spawn_exit<F, R>(&self, future: F) -> Result<Receiver<R>>
    where
        F: Future<Output = Result<R>> + 'js,
        R: 'js,
    {
        let ctx = self.clone();

        let type_error_ctor: Constructor = ctx.globals().get(PredefinedAtom::TypeError)?;
        let type_error: Object = type_error_ctor.construct(())?;
        let stack: Option<String> = type_error.get(PredefinedAtom::Stack).ok();

        let (join_channel_tx, join_channel_rx) = oneshot::channel();

        self.spawn(async move {
            match future.await.catch(&ctx) {
                Ok(res) => {
                    //result here dosn't matter if receiver has dropped
                    let _ = join_channel_tx.send(res);
                }
                Err(err) => {
                    if let CaughtError::Exception(err) = err {
                        if err.stack().is_none() {
                            if let Some(stack) = stack {
                                err.set(PredefinedAtom::Stack, stack).unwrap();
                            }
                        }
                        Vm::print_error_and_exit(&ctx, CaughtError::Exception(err));
                    } else {
                        Vm::print_error_and_exit(&ctx, err);
                    }
                }
            }
        });
        Ok(join_channel_rx)
    }
}
