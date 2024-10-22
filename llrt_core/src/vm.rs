// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cmp::min,
    collections::{HashMap, HashSet},
    env,
    ffi::CStr,
    fmt::Write,
    io,
    path::{Path, PathBuf},
    process::exit,
    rc::Rc,
    result::Result as StdResult,
    sync::Mutex,
};

use llrt_json::{parse::json_parse, stringify::json_stringify_replacer_space};
use llrt_modules::{
    path::{resolve_path, resolve_path_with_separator},
    timers::{self, poll_timers},
};
use llrt_utils::{bytes::ObjectBytes, error::ErrorExtensions, object::ObjectExt};
use once_cell::sync::Lazy;
use ring::rand::SecureRandom;
use rquickjs::{
    atom::PredefinedAtom,
    context::EvalOptions,
    function::Opt,
    loader::{BuiltinLoader, FileResolver, Loader, ScriptLoader},
    module::Declared,
    object::Accessor,
    prelude::{Func, Rest},
    qjs, AsyncContext, AsyncRuntime, CatchResultExt, CaughtError, Ctx, Error, Filter, Function,
    IntoJs, Module, Object, Result, Value,
};
use tokio::time::Instant;
use tracing::trace;
use zstd::{bulk::Decompressor, dict::DecoderDictionary};

use crate::{
    bytecode::{BYTECODE_COMPRESSED, BYTECODE_UNCOMPRESSED, BYTECODE_VERSION, SIGNATURE_LENGTH},
    custom_resolver::{require_resolve, CustomResolver},
    environment, http,
    modules::{console, crypto::SYSTEM_RANDOM},
    number::number_to_string,
    security,
    utils::clone::structured_clone,
};

include!(concat!(env!("OUT_DIR"), "/bytecode_cache.rs"));
#[cfg(feature = "lambda")]
include!(concat!(env!("OUT_DIR"), "/sdk_client_endpoints.rs"));

#[inline]
pub fn uncompressed_size(input: &[u8]) -> StdResult<(usize, &[u8]), io::Error> {
    let size = input.get(..4).ok_or(io::ErrorKind::InvalidInput)?;
    let size: &[u8; 4] = size.try_into().map_err(|_| io::ErrorKind::InvalidInput)?;
    let uncompressed_size = u32::from_le_bytes(*size) as usize;
    let rest = &input[4..];
    Ok((uncompressed_size, rest))
}

pub(crate) static COMPRESSION_DICT: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/compression.dict"));

static DECOMPRESSOR_DICT: Lazy<DecoderDictionary> =
    Lazy::new(|| DecoderDictionary::copy(COMPRESSION_DICT));

fn print(value: String, stdout: Opt<bool>) {
    if stdout.0.unwrap_or_default() {
        println!("{value}");
    } else {
        eprintln!("{value}")
    }
}

struct LoaderContainer<T>
where
    T: Loader + 'static,
{
    loader: T,
}
impl<T> LoaderContainer<T>
where
    T: Loader + 'static,
{
    fn new(loader: T) -> Self {
        Self { loader }
    }
}

impl<T> Loader for LoaderContainer<T>
where
    T: Loader + 'static,
{
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> Result<Module<'js, Declared>> {
        let res = self.loader.load(ctx, name)?;

        let name = if let Some(name) = name.strip_prefix("./") {
            name
        } else {
            name
        };

        if name.starts_with('/') {
            set_import_meta(&res, name)?;
        } else {
            set_import_meta(&res, &resolve_path_with_separator([name], true))?;
        };

        Ok(res)
    }
}

#[derive(Debug, Default)]
pub struct CustomLoader;

impl Loader for CustomLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> Result<Module<'js, Declared>> {
        trace!("Loading module: {}", name);
        if name.ends_with(".json") {
            let source = std::fs::read_to_string(name)?;
            return Module::declare(ctx.clone(), name, ["export default ", &source].concat());
        }

        let ctx = ctx.clone();
        if let Some(bytes) = BYTECODE_CACHE.get(name) {
            #[cfg(feature = "lambda")]
            init_client_connection(&ctx, name)?;

            trace!("Loading embedded module: {}", name);

            return load_bytecode_module(ctx, name, bytes);
        }

        let path = PathBuf::from(name);
        let mut bytes: &[u8] = &std::fs::read(path)?;

        if name.ends_with(".lrt") {
            trace!("Loading binary module: {}", name);
            return load_bytecode_module(ctx, name, bytes);
        }
        if bytes.starts_with(b"#!") {
            bytes = bytes.splitn(2, |&c| c == b'\n').nth(1).unwrap_or(bytes);
        }
        Module::declare(ctx, name, bytes)
    }
}

#[cfg(feature = "lambda")]
fn init_client_connection(ctx: &Ctx<'_>, specifier: &str) -> Result<()> {
    use crate::{
        modules::http::HTTP_CLIENT,
        runtime_client::{check_client_inited, mark_client_inited},
    };
    use http_body_util::BodyExt;
    use llrt_utils::result::ResultExt;

    if let Some(sdk_import) = specifier.strip_prefix("@aws-sdk/") {
        let client_name = sdk_import.strip_prefix("client-").unwrap_or(sdk_import);
        if let Some(endpoint) = SDK_CLIENT_ENDPOINTS.get(client_name) {
            let endpoint = if endpoint.is_empty() {
                client_name
            } else {
                endpoint
            };

            let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };
            let rt_ptr = rt as usize; //hack to move, is safe since runtime is still alive in spawn

            if !check_client_inited(rt, endpoint) {
                let client = HTTP_CLIENT.as_ref().or_throw(ctx)?;

                trace!("Started client init {}", client_name);
                let region = env::var("AWS_REGION").unwrap();

                let url = ["https://", endpoint, ".", &region, ".amazonaws.com/sping"].concat();

                tokio::task::spawn(async move {
                    let start = Instant::now();

                    if let Ok(url) = url.parse() {
                        if let Ok(mut res) = client.get(url).await {
                            if let Ok(res) = res.body_mut().collect().await {
                                let _ = res;

                                mark_client_inited(rt_ptr as _);

                                trace!("Client connection initialized in {:?}", start.elapsed());
                            }
                        }
                    }
                });
            }
        }
    }

    Ok(())
}

pub fn load_bytecode_module<'js>(
    ctx: Ctx<'js>,
    _name: &str,
    buf: &[u8],
) -> Result<Module<'js, Declared>> {
    let bytes = get_module_bytecode(buf)?;
    unsafe { Module::load(ctx, &bytes) }
}

fn get_module_bytecode(input: &[u8]) -> Result<Vec<u8>> {
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

#[allow(dead_code)]
struct ExportArgs<'js>(Ctx<'js>, Object<'js>, Value<'js>, Value<'js>);

pub struct VmOptions {
    pub module_builder: crate::module_builder::ModuleBuilder,
    pub max_stack_size: usize,
    pub gc_threshold_mb: usize,
}

impl Default for VmOptions {
    fn default() -> Self {
        Self {
            module_builder: crate::module_builder::ModuleBuilder::default(),
            max_stack_size: 512 * 1024,
            gc_threshold_mb: {
                const DEFAULT_GC_THRESHOLD_MB: usize = 20;

                let gc_threshold_mb: usize = env::var(environment::ENV_LLRT_GC_THRESHOLD_MB)
                    .map(|threshold| threshold.parse().unwrap_or(DEFAULT_GC_THRESHOLD_MB))
                    .unwrap_or(DEFAULT_GC_THRESHOLD_MB);

                gc_threshold_mb * 1024 * 1024
            },
        }
    }
}

impl Vm {
    pub const ENV_LAMBDA_TASK_ROOT: &'static str = "LAMBDA_TASK_ROOT";

    pub async fn from_options(
        vm_options: VmOptions,
    ) -> StdResult<Self, Box<dyn std::error::Error + Send + Sync>> {
        llrt_modules::time::init();
        http::init()?;
        security::init()?;

        SYSTEM_RANDOM
            .fill(&mut [0; 8])
            .expect("Failed to initialize SystemRandom");

        let mut file_resolver = FileResolver::default();
        let custom_resolver = CustomResolver;
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
        }

        let (builtin_resolver, module_loader, module_names, init_globals) =
            vm_options.module_builder.build();

        let resolver = (builtin_resolver, custom_resolver, file_resolver);

        let loader = LoaderContainer::new((
            module_loader,
            CustomLoader,
            BuiltinLoader::default(),
            ScriptLoader::default()
                .with_extension("mjs")
                .with_extension("cjs"),
        ));

        let runtime = AsyncRuntime::new()?;
        runtime.set_max_stack_size(vm_options.max_stack_size).await;
        runtime.set_gc_threshold(vm_options.gc_threshold_mb).await;
        runtime.set_loader(resolver, loader).await;

        let ctx = AsyncContext::full(&runtime).await?;
        ctx.with(|ctx| {
            (|| {
                for init_global in init_globals {
                    init_global(&ctx)?;
                }
                timers::init_timers(&ctx)?;
                init(&ctx, module_names)?;
                Ok(())
            })()
            .catch(&ctx)
            .unwrap_or_else(|err| Self::print_error_and_exit(&ctx, err));
            Ok::<_, Error>(())
        })
        .await?;

        Ok(Vm { runtime, ctx })
    }

    pub async fn new() -> StdResult<Self, Box<dyn std::error::Error + Send + Sync>> {
        let vm = Self::from_options(VmOptions::default()).await?;
        Ok(vm)
    }

    pub async fn run_with<F>(&self, f: F)
    where
        F: for<'js> FnOnce(&Ctx<'js>) -> Result<()> + std::marker::Send,
    {
        self.ctx
            .with(|ctx| {
                if let Err(err) = f(&ctx).catch(&ctx) {
                    Self::print_error_and_exit(&ctx, err);
                }
            })
            .await;
    }

    pub async fn run_file(&self, filename: &Path, strict: bool, global: bool) {
        let source = [
            r#"try{require(""#,
            &filename.to_string_lossy(),
            r#"")}catch(e){console.error(e);process.exit(1)}"#,
        ]
        .concat();

        self.run(source, strict, global).await;
    }

    pub async fn run<S: Into<Vec<u8>> + Send>(&self, source: S, strict: bool, global: bool) {
        self.run_with(|ctx| {
            let mut options = EvalOptions::default();
            options.strict = strict;
            options.promise = true;
            options.global = global;
            let _ = ctx.eval_with_options::<Value, _>(source, options)?;
            Ok::<_, Error>(())
        })
        .await;
    }

    pub fn print_error_and_exit<'js>(ctx: &Ctx<'js>, err: CaughtError<'js>) -> ! {
        let mut error_str = String::new();
        write!(error_str, "Error: {:?}", err).unwrap();
        if let Ok(error) = err.into_value(ctx) {
            if console::log_std_err(ctx, Rest(vec![error.clone()]), console::LogLevel::Fatal)
                .is_err()
            {
                eprintln!("{}", error_str);
            };
            if cfg!(test) {
                panic!("{:?}", error);
            } else {
                exit(1)
            }
        } else if cfg!(test) {
            panic!("{}", error_str);
        } else {
            eprintln!("{}", error_str);
            exit(1)
        };
    }

    pub async fn idle(self) -> StdResult<(), Box<dyn std::error::Error + Sync + Send>> {
        self.runtime.idle().await;
        Ok(())
    }
}

fn json_parse_string<'js>(ctx: Ctx<'js>, bytes: ObjectBytes<'js>) -> Result<Value<'js>> {
    let bytes = bytes.as_bytes();
    json_parse(&ctx, bytes)
}

fn init(ctx: &Ctx<'_>, module_names: HashSet<&'static str>) -> Result<()> {
    llrt_utils::ctx::set_spawn_error_handler(|ctx, err| {
        Vm::print_error_and_exit(ctx, err);
    });

    let globals = ctx.globals();

    globals.set("__gc", Func::from(|ctx: Ctx| ctx.run_gc()))?;

    let number: Function = globals.get(PredefinedAtom::Number)?;
    let number_proto: Object = number.get(PredefinedAtom::Prototype)?;
    number_proto.set(PredefinedAtom::ToString, Func::from(number_to_string))?;

    let readable_stream_stub = ctx.eval::<Value,_>(
            r#"class ReadableStream{constructor(){throw Error(`ReadableStream is not supported via global scope. Enable this by adding this to your code:\nimport { ReadableStream } from "stream";\nglobalThis.ReadableStream = ReadableStream;`)}};"#
    )?;

    globals.set("ReadableStream", readable_stream_stub)?;
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

    let require_in_progress: Rc<Mutex<HashMap<Rc<str>, Object>>> =
        Rc::new(Mutex::new(HashMap::new()));

    let require_exports: Rc<Mutex<Option<Value>>> = Rc::new(Mutex::new(None));
    let require_exports2 = require_exports.clone();
    let require_exports3 = require_exports.clone();
    let require_exports4 = require_exports.clone();
    let require_exports5 = require_exports.clone();

    let module = Object::new(ctx.clone())?;

    module.prop(
        "exports",
        Accessor::from(move || require_exports2.lock().unwrap().as_ref().cloned())
            .set(move |exports| {
                require_exports3.lock().unwrap().replace(exports);
            })
            .configurable()
            .enumerable(),
    )?;

    globals.prop("module", module)?;

    globals.prop(
        "exports",
        Accessor::from(move || require_exports4.lock().unwrap().as_ref().cloned())
            .set(move |exports| {
                require_exports5.lock().unwrap().replace(exports);
            })
            .enumerable()
            .configurable(),
    )?;

    let require_cache: Object = Object::new(ctx.clone())?;
    globals.set("__require_cache", require_cache)?;

    globals.set(
        "require",
        Func::from(move |ctx, specifier: String| -> Result<Value> {
            struct Args<'js>(Ctx<'js>);
            let Args(ctx) = Args(ctx);
            let specifier = if let Some(striped_specifier) = specifier.strip_prefix("node:") {
                striped_specifier.to_string()
            } else {
                specifier
            };
            let import_name = if module_names.contains(specifier.as_str()) {
                specifier
            } else {
                let module_name = get_script_or_module_name(ctx.clone());
                let abs_path = resolve_path([module_name].iter());
                require_resolve(&ctx, &specifier, &abs_path, false)?
            };

            let import_name: Rc<str> = import_name.into();

            let globals = ctx.globals();
            let require_cache: Object = globals.get("__require_cache")?;

            if let Some(cached_value) =
                require_cache.get::<_, Option<Value>>(import_name.as_ref())?
            {
                return Ok(cached_value);
            }

            if import_name.ends_with(".json") {
                let source = std::fs::read_to_string(import_name.as_ref())?;
                let value = json_parse(&ctx, source)?;
                require_cache.set(import_name.as_ref(), value.clone())?;
                return Ok(value);
            }

            let mut map = require_in_progress.lock().unwrap();
            if let Some(obj) = map.get(import_name.as_ref()) {
                let value = obj.clone().into_value();
                require_cache.set(import_name.as_ref(), value.clone())?;
                return Ok(value);
            }

            trace!("Require: {}", import_name);

            let obj = Object::new(ctx.clone())?;
            map.insert(import_name.clone(), obj.clone());
            drop(map);

            let exports = Object::new(ctx.clone())?.into_value();

            let current_exports = require_exports.lock().unwrap().replace(exports);

            let import_promise = Module::import(&ctx, import_name.as_bytes().to_vec())?;

            let exports = if let Some(current_exports) = current_exports {
                require_exports.lock().unwrap().replace(current_exports)
            } else {
                require_exports.lock().unwrap().take()
            };

            let rt = unsafe { qjs::JS_GetRuntime(ctx.as_raw().as_ptr()) };

            let mut deadline = Instant::now();

            let mut executing_timers = Vec::new();

            let imported_object = loop {
                if let Some(x) = import_promise.result::<Object>() {
                    break x?;
                }

                if deadline < Instant::now() {
                    poll_timers(rt, &mut executing_timers, None, Some(&mut deadline))?;
                }

                ctx.execute_pending_job();
            };

            require_in_progress
                .lock()
                .unwrap()
                .remove(import_name.as_ref());

            if let Some(exports) = exports {
                if exports.type_of() == rquickjs::Type::Object {
                    if let Some(exports) = exports.as_object() {
                        for prop in exports
                            .own_props::<Value, Value>(Filter::new().private().string().symbol())
                        {
                            let (key, value) = prop?;
                            obj.set(key, value)?;
                        }
                    }
                } else {
                    //we have explicitly set it
                    require_cache.set(import_name.as_ref(), exports.clone())?;
                    return Ok(exports);
                }
            }

            for prop in imported_object.props::<String, Value>() {
                let (key, value) = prop?;
                obj.set(key, value)?;
            }

            let value = obj.into_value();

            require_cache.set(import_name.as_ref(), value.clone())?;
            Ok(value)
        }),
    )?;

    Ok(())
}

fn load<'js>(ctx: Ctx<'js>, filename: String, options: Opt<Object<'js>>) -> Result<Value<'js>> {
    let mut eval_options = EvalOptions::default();
    eval_options.strict = false;
    eval_options.promise = true;

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
    meta.prop("url", ["file://", filepath].concat())?;
    Ok(())
}
