// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::HashMap,
    env, fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use lazy_static::lazy_static;
use llrt_modules::path::{is_absolute, join_path_with_separator};
use llrt_utils::result::ResultExt;
use rquickjs::{loader::Resolver, Ctx, Error, Exception, Result};
use simd_json::BorrowedValue;
use tracing::trace;

use crate::{modules::path::dirname, utils::io::get_js_path};

include!(concat!(env!("OUT_DIR"), "/bytecode_cache.rs"));

lazy_static! {
    static ref NODE_MODULES_PATHS_CACHE: Mutex<HashMap<String, Vec<String>>> =
        Mutex::new(HashMap::new());
}

#[derive(Debug)]
pub struct CustomResolver {
    paths: Vec<PathBuf>,
    cwd: PathBuf,
}
impl CustomResolver {
    pub fn add_path<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        self.paths.push(path.into());
        self
    }

    pub fn get_bin_path(path: &Path) -> PathBuf {
        path.with_extension("lrt")
    }

    pub fn new() -> io::Result<Self> {
        Ok(Self {
            paths: Vec::with_capacity(10),
            cwd: env::current_dir()?,
        })
    }
}

#[allow(clippy::manual_strip)]
impl Resolver for CustomResolver {
    fn resolve(&mut self, ctx: &Ctx, base: &str, name: &str) -> Result<String> {
        trace!("Try resolve '{}' from '{}'", name, base);

        // Resolve: precompiled binaries (from bytecode_cache and on filesystem)
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
        let normalized_path = normalized_path.to_string_lossy().to_string();
        let normalized_path = join_path_with_separator([normalized_path].iter(), true);
        let mut normalized_path = normalized_path.as_str();
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
            if Path::new(name).exists() {
                return Ok(name.to_string());
            }
        }

        if Path::new(normalized_path).is_file() {
            return Ok(normalized_path.to_string());
        }

        let path = self.paths.iter().find_map(|path| {
            let path = path.join(normalized_path);
            let bin_path = CustomResolver::get_bin_path(&path);
            if bin_path.exists() {
                return Some(bin_path);
            }
            get_js_path(path.to_str().unwrap())
        });

        if let Some(valid_path) = path {
            let valid_path = valid_path.into_os_string().into_string().unwrap();
            trace!("Valideted path: {}", valid_path);
            return Ok(valid_path);
        }

        // Resolve: node_modules via ESM
        if !(is_absolute(name)
            || name.ends_with(".js")
            || name.ends_with(".mjs")
            || name.ends_with(".cjs")
            || name.ends_with(".json"))
        {
            let start = dirname(base.to_string());
            if let Ok(node_modules_path) = load_node_modules(ctx, name, &start, true) {
                trace!("Node modules path: {}", node_modules_path);
                return Ok(node_modules_path);
            }
        }

        Err(Error::new_resolving(base, name))
    }
}

pub fn load_node_modules(
    ctx: &Ctx<'_>,
    specifier: &str,
    start: &str,
    is_esm: bool,
) -> Result<String> {
    trace!("load_node_modules(start): {}", start);
    let mut cache = NODE_MODULES_PATHS_CACHE.lock().unwrap();
    let dirs = cache
        .entry(start.to_string())
        .or_insert_with(|| node_modules_paths(start));

    for dir in dirs {
        trace!("load_node_modules(dir): {}", dir);
        if let Ok(v) = load_package_exports(ctx, specifier, dir, is_esm) {
            return Ok(v);
        }
    }

    Err(Exception::throw_reference(
        ctx,
        &["Error resolving module '", specifier, "'"].concat(),
    ))
}

fn node_modules_paths(start: &str) -> Vec<String> {
    // 1. let PARTS = path split(START)
    let parts: Vec<&str> = Path::new(start)
        .components()
        .filter_map(|comp| comp.as_os_str().to_str())
        .collect();
    // 2. let I = count of PARTS - 1
    let mut i = parts.len() as isize - 1;
    // 3. let DIRS = []
    let mut dirs: Vec<String> = Vec::new();
    // 4. while I >= 0,
    while i >= 0 {
        //    a. if PARTS[I] = "node_modules" CONTINUE
        if parts[i as usize] == "node_modules" {
            i -= 1;
            continue;
        }
        //    b. DIR = path join(PARTS[0 .. I] + "node_modules")
        let mut dir: Vec<&str> = parts[0..=i as usize].to_vec();
        dir.push("node_modules");
        //    c. DIRS = DIR + DIRS
        let dir = dir.join("/");
        let dir = if dir.starts_with("//") {
            dir[1..].to_string()
        } else {
            dir.to_string()
        };
        dirs.push(dir);
        //    d. let I = I - 1
        i -= 1;
    }
    // 5. return DIRS + GLOBAL_FOLDERS
    let home = home::home_dir().unwrap().to_string_lossy().into_owned();
    dirs.push([home.clone(), "/.node_modules".to_string()].concat());
    dirs.push([home.clone(), "/.node_libraries".to_string()].concat());
    // It does not support up to $PREFIX/lib/node. $PREFIX is the Node.js configured node_prefix.
    dirs
}

fn load_package_exports(ctx: &Ctx<'_>, specifier: &str, dir: &str, is_esm: bool) -> Result<String> {
    let (scope, name) = match specifier.split_once('/') {
        Some((s, n)) => (s, ["./", n].concat()),
        None => (specifier, ".".to_string()),
    };

    let mut package_json_path = [dir, "/"].concat();
    let base_path_length = package_json_path.len();
    package_json_path.push_str(scope);
    package_json_path.push_str("/package.json");

    let (scope, name) = if name != "." && !Path::new(&package_json_path).exists() {
        package_json_path.truncate(base_path_length);
        package_json_path.push_str(specifier);
        package_json_path.push_str("/package.json");
        (specifier, ".")
    } else {
        (scope, name.as_str())
    };

    if !Path::new(&package_json_path).exists() {
        return Err(Exception::throw_reference(
            ctx,
            &["Error resolving module '", specifier, "'"].concat(),
        ));
    };

    let mut package_json = fs::read(&package_json_path).unwrap_or_default();
    let package_json = simd_json::to_borrowed_value(&mut package_json).or_throw(ctx)?;

    let module_path = package_exports_resolve(&package_json, name, is_esm)?;

    Ok([dir, "/", scope, "/", module_path].concat())
}

fn package_exports_resolve<'a>(
    package_json: &'a BorrowedValue<'a>,
    modules_name: &str,
    is_esm: bool,
) -> Result<&'a str> {
    let ident = if is_esm { "import" } else { "require" };

    if let BorrowedValue::Object(map) = package_json {
        if let Some(BorrowedValue::Object(exports)) = map.get("exports") {
            if let Some(BorrowedValue::Object(name)) = exports.get(modules_name) {
                // Check for exports -> name -> [import | require] -> default
                if let Some(BorrowedValue::Object(ident)) = name.get(ident) {
                    if let Some(BorrowedValue::String(default)) = ident.get("default") {
                        return Ok(default.as_ref());
                    }
                }
                // Check for exports -> name -> [import | require]
                if let Some(BorrowedValue::String(ident)) = name.get(ident) {
                    return Ok(ident.as_ref());
                }
                // [CJS only] Check for exports -> name -> default
                if !is_esm {
                    if let Some(BorrowedValue::String(default)) = name.get("default") {
                        return Ok(default.as_ref());
                    }
                }
            }
            // Check for exports -> [import | require] -> default
            if let Some(BorrowedValue::Object(ident)) = exports.get(ident) {
                if let Some(BorrowedValue::String(default)) = ident.get("default") {
                    return Ok(default.as_ref());
                }
            }
            // Check for exports -> [import | require]
            if let Some(BorrowedValue::String(ident)) = exports.get(ident) {
                return Ok(ident.as_ref());
            }
            // [CJS only] Check for exports -> default
            if !is_esm {
                if let Some(BorrowedValue::String(default)) = exports.get("default") {
                    return Ok(default.as_ref());
                }
            }
        }
        // [ESM only] Check for module field
        if is_esm {
            if let Some(BorrowedValue::String(module)) = map.get("module") {
                return Ok(module.as_ref());
            }
        }
        // Check for main field
        // Workaround for modules that have only “main” defined and whose entrypoint is not “index.js”
        if let Some(BorrowedValue::String(main)) = map.get("main") {
            return Ok(main.as_ref());
        }
    }
    Ok("./index.js")
}
