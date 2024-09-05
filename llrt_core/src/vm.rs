// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    cmp::min,
    collections::{HashMap, HashSet},
    env,
    ffi::CStr,
    fmt::Write,
    fs::{self, File},
    io::{self, Read, Seek},
    mem::size_of,
    ops::Range,
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
    process::exit,
    rc::Rc,
    result::Result as StdResult,
    sync::{Arc, Mutex, RwLock},
};

use llrt_modules::timers::{self, poll_timers};
use llrt_utils::{
    bytes::ObjectBytes, encoding::bytes_to_hex_string, error::ErrorExtensions, object::ObjectExt,
};
use memmap2::{Advice, MmapOptions};
use once_cell::sync::Lazy;
use ring::rand::SecureRandom;
use rquickjs::{
    atom::PredefinedAtom,
    context::EvalOptions,
    function::Opt,
    loader::{BuiltinLoader, FileResolver, Loader, Resolver, ScriptLoader},
    module::Declared,
    prelude::{Func, Rest},
    qjs, AsyncContext, AsyncRuntime, CatchResultExt, CaughtError, Ctx, Error, Function, IntoJs,
    Module, Object, Result, Value,
};
use tokio::time::Instant;
use tracing::trace;
use zstd::{bulk::Decompressor, dict::DecoderDictionary};

//include!(concat!(env!("OUT_DIR"), "/../../../../bytecode_cache.rs"));

#[cfg(unix)]
fn file_from_raw_fd(raw_fd: i32) -> std::io::Result<File> {
    #[cfg(not(unix))]
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Unsupported on non-unix platforms",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::fd::FromRawFd;
        let dup_fd = unsafe { libc::dup(raw_fd) };
        if dup_fd == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(unsafe { File::from_raw_fd(dup_fd) })
    }
}

fn u32_from_le_byte_slice_unchecked(bytes: &[u8]) -> u32 {
    (bytes[0] as u32)
        | ((bytes[1] as u32) << 8)
        | ((bytes[2] as u32) << 16)
        | ((bytes[3] as u32) << 24)
}

fn u16_from_le_byte_slice_unchecked(bytes: &[u8]) -> u16 {
    (bytes[0] as u16) | ((bytes[1] as u16) << 8)
}

#[derive(Default)]
struct BytecodeCache {
    data: Vec<u8>,
    map: HashMap<Box<str>, Range<usize>>,
}

impl BytecodeCache {
    pub fn new(data: Vec<u8>, start_position: usize, length: usize, package_count: usize) -> Self {
        let mut offset = start_position;
        let mut map = HashMap::with_capacity(package_count);

        loop {
            let name_len_start = offset;
            let name_len_end = offset + size_of::<u16>();
            let name_len =
                u16_from_le_byte_slice_unchecked(&data[name_len_start..name_len_end]) as usize;
            let bytecode_pos_start = name_len_end + name_len;

            let name =
                unsafe { std::str::from_utf8_unchecked(&data[name_len_end..bytecode_pos_start]) };

            let bytecode_pos_end = bytecode_pos_start + size_of::<u32>();
            let bytecode_pos =
                u32_from_le_byte_slice_unchecked(&data[bytecode_pos_start..bytecode_pos_end])
                    as usize;

            let bytecode_size_start = bytecode_pos_end;
            let bytecode_size_end = bytecode_size_start + size_of::<u32>();
            let bytecode_size =
                u32_from_le_byte_slice_unchecked(&data[bytecode_size_start..bytecode_size_end])
                    as usize;

            map.insert(name.into(), bytecode_pos..bytecode_pos + bytecode_size);

            offset = bytecode_size_end;

            if offset >= length - 1 {
                break;
            }
        }

        Self { data, map }
    }

    fn has(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    fn get(&self, name: &str) -> Option<&[u8]> {
        self.map.get(name).map(|range| &self.data[range.clone()])
    }
}

static EMBEDDED_BYTECODE_DATA: Lazy<RwLock<BytecodeCache>> = Lazy::new(|| {
    let init = || {
        let now = Instant::now();
        trace!("Loading embedded bytecode");
        let argv_0 = env::args().next().expect("Failed to get argv0");

        let mut file = if let Ok(fd_string) = env::var("LLRT_MEM_FD") {
            let mem_fd: i32 = fd_string.parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Other, "Invalid bytecode-cache fd")
            })?;
            trace!("Using raw memfd bytecode cache");
            file_from_raw_fd(mem_fd)
        } else {
            File::open(argv_0)
        }?;

        let offset: u64 = if let Ok(offset_string) = env::var("LLRT_BYTECODE_OFFSET") {
            offset_string.parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Other, "Invalid bytecode-cache offset")
            })?
        } else {
            0
        };

        let size: usize = if let Ok(size_string) = env::var("LLRT_BYTECODE_SIZE") {
            size_string.parse().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::Other, "Invalid bytecode-cache size")
            })?
        } else {
            file.metadata()?.size() as usize
        };

        let mmap = unsafe { MmapOptions::new().offset(offset).len(size).map(&file)? };
        mmap.advise(Advice::Sequential).unwrap();

        println!(
            "Size + offset : {},{},{}",
            size,
            offset,
            file.metadata()?.size()
        );

        let mut buf2 = Vec::new();

        file.read_to_end(&mut buf2)?;

        let compressed_bytes_end_index = mmap.len() - 4;

        println!("End index: {}", compressed_bytes_end_index);

        let uncompressed_size =
            u32_from_le_byte_slice_unchecked(&mmap[compressed_bytes_end_index..]) as usize;

        println!("Uncompressed size : {}", uncompressed_size);

        let mut bytecode_bundle = Vec::with_capacity(uncompressed_size);
        let mut decompressor = Decompressor::with_prepared_dictionary(&DECOMPRESSOR_DICT)?;
        decompressor
            .decompress_to_buffer(&mmap[0..compressed_bytes_end_index], &mut bytecode_bundle)?;

        println!("Extraction took {:?}", now.elapsed());

        drop(mmap);

        let total_file_size = bytecode_bundle.len();

        let signature_len = BYTECODE_EMBEDDED_SIGNATURE.len();
        let signed_signature_len = signature_len as isize;

        if &bytecode_bundle[(total_file_size as isize - signed_signature_len) as usize..]
            != BYTECODE_EMBEDDED_SIGNATURE
        {
            return Ok(RwLock::new(BytecodeCache::default()));
        }

        #[repr(C)]
        #[derive(Debug)]
        struct EmbeddedMeta {
            package_count: u32,
            bytecode_pos: u32,
            package_index_pos: u32,
        }

        let embedded_meta_size = size_of::<EmbeddedMeta>();
        let meta_and_signature_size = embedded_meta_size + signature_len;

        let meta_start = (total_file_size as isize - meta_and_signature_size as isize) as usize;
        let meta_end = (total_file_size as isize - signature_len as isize) as usize;

        let embedded_metadata: EmbeddedMeta =
            unsafe { std::ptr::read(bytecode_bundle[meta_start..meta_end].as_ptr() as *const _) };

        println!("Metadata: {:?}", embedded_metadata);

        let bytecode_pos = embedded_metadata.bytecode_pos as usize;
        let start_position = embedded_metadata.package_index_pos as usize;
        let end_pos = total_file_size as usize - meta_and_signature_size;
        let length = end_pos - bytecode_pos;

        trace!(
            "Loading bytecode cache of {} kB",
            (end_pos - bytecode_pos) / 1024
        );

        let bytecode_cache = BytecodeCache::new(
            bytecode_bundle,
            start_position,
            length,
            embedded_metadata.package_count as usize,
        );

        trace!("Building cache took: {:?}", now.elapsed());

        io::Result::Ok(RwLock::new(bytecode_cache))
    };

    init().unwrap()
});

use crate::{
    bytecode::BYTECODE_EMBEDDED_SIGNATURE,
    modules::{
        console,
        crypto::SYSTEM_RANDOM,
        path::{dirname, join_path, resolve_path},
    },
};

use crate::{
    bytecode::{BYTECODE_COMPRESSED, BYTECODE_UNCOMPRESSED, BYTECODE_VERSION, SIGNATURE_LENGTH},
    environment,
    json::{parse::json_parse, stringify::json_stringify_replacer_space},
    number::number_to_string,
    security,
    utils::{clone::structured_clone, io::get_js_path},
};
#[inline]
pub fn uncompressed_size(input: &[u8]) -> StdResult<(usize, &[u8]), io::Error> {
    let size = input.get(..4).ok_or(io::ErrorKind::InvalidInput)?;
    let size: &[u8; 4] = size.try_into().map_err(|_| io::ErrorKind::InvalidInput)?;
    let uncompressed_size = u32::from_le_bytes(*size) as usize;
    let rest = &input[4..];
    Ok((uncompressed_size, rest))
}

pub(crate) static COMPRESSION_DICT: &[u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/../../../llrt_bytecode/compression.dict"
));

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
                },
                _ => {
                    normalized.push(component);
                },
            }
        }
        if ends_with_slash {
            normalized.push("");
        }
        normalized
    }

    fn new() -> io::Result<Self> {
        Ok(Self {
            paths: Vec::with_capacity(10),
            cwd: env::current_dir()?,
        })
    }
}

#[allow(clippy::manual_strip)]
impl Resolver for BinaryResolver {
    fn resolve(&mut self, _ctx: &Ctx, base: &str, name: &str) -> Result<String> {
        trace!("Try resolve \"{}\" from \"{}\"", name, base);

        let cache = EMBEDDED_BYTECODE_DATA.read().unwrap();

        if cache.has(name) {
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

        if cache.has(cache_key) {
            return Ok(cache_key.to_string());
        }

        if cache.has(base) {
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

struct LoaderContainer<T>
where
    T: Loader + 'static,
{
    loader: T,
    cwd: String,
}
impl<T> LoaderContainer<T>
where
    T: Loader + 'static,
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
            set_import_meta(&res, &[self.cwd.as_str(), name].join("/"))?;
        };

        Ok(res)
    }
}

impl Loader for BinaryLoader {
    fn load<'js>(&mut self, ctx: &Ctx<'js>, name: &str) -> Result<Module<'js, Declared>> {
        trace!("Loading module: {}", name);
        let ctx = ctx.clone();

        let cache = EMBEDDED_BYTECODE_DATA.read().unwrap();

        if let Some(bytes) = cache.get(name) {
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
        security::init();

        SYSTEM_RANDOM
            .fill(&mut [0; 8])
            .expect("Failed to initialize SystemRandom");

        let mut file_resolver = FileResolver::default();
        let mut binary_resolver = BinaryResolver::new()?;
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

        let (builtin_resolver, module_loader, module_names, init_globals) =
            vm_options.module_builder.build();

        let resolver = (builtin_resolver, binary_resolver, file_resolver);

        let loader = LoaderContainer::new((
            module_loader,
            BinaryLoader,
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
                let _ = init(&ctx, module_names)?;
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

    pub async fn run_file(&self, filename: &Path) {
        self.run(
            [
                r#"import(""#,
                &filename.to_string_lossy(),
                r#"").catch((e) => {{console.error(e);process.exit(1)}})"#,
            ]
            .concat(),
            false,
        )
        .await;
    }

    pub async fn run<S: Into<Vec<u8>> + Send>(&self, source: S, strict: bool) {
        self.run_with(|ctx| {
            let mut options = EvalOptions::default();
            options.strict = strict;
            options.promise = true;
            options.global = false;
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
            exit(1)
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
            let ExportArgs(_, _, _, value) = ExportArgs(ctx, obj, prop, value);
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
            struct Args<'js>(Ctx<'js>);
            let Args(ctx) = Args(ctx);
            let specifier = if let Some(striped_specifier) = &specifier.strip_prefix("node:") {
                striped_specifier.to_string()
            } else {
                specifier
            };

            let cache = EMBEDDED_BYTECODE_DATA.read().unwrap();

            let specifier_ref = specifier.as_str();

            let import_name = if module_names.contains(specifier_ref)
                || cache.has(specifier_ref)
                || specifier.starts_with('/')
            {
                specifier
            } else {
                let module_name = get_script_or_module_name(ctx.clone());
                let abs_path = resolve_path([module_name].iter());
                let import_directory = dirname(abs_path);
                join_path(vec![import_directory, specifier])
            };

            drop(cache);

            let mut map = require_in_progress.lock().unwrap();
            if let Some(obj) = map.get(&import_name) {
                return Ok(obj.clone().into_value());
            }

            let obj = Object::new(ctx.clone())?;
            map.insert(import_name.clone(), obj.clone());
            drop(map);

            trace!("Require: {}", import_name);

            let mut options = EvalOptions::default();
            options.strict = false;
            options.promise = true;
            options.global = false;

            let import_promise = Module::import(&ctx, import_name.clone())?;

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

    let mut opts = EvalOptions::default();
    opts.global = false;
    opts.strict = false;
    opts.promise = true;

    ctx.eval_with_options(include_str!("../../bundle/@llrt/std.js"), opts)?;

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
