// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    borrow::Cow,
    collections::HashMap,
    env,
    fs::{self},
    path::{Path, PathBuf},
    rc::Rc,
    sync::Mutex,
};

use llrt_modules::path::{
    self, is_absolute, name_extname, replace_backslash, resolve_path_with_separator,
};
use llrt_utils::result::ResultExt;
use once_cell::sync::Lazy;
use rquickjs::{loader::Resolver, Ctx, Error, Result};
use simd_json::{derived::ValueObjectAccessAsScalar, BorrowedValue};
use tracing::trace;

use crate::{
    module_loader::CJS_LOADER_PREFIX,
    utils::io::{is_supported_ext, JS_EXTENSIONS, SUPPORTED_EXTENSIONS},
};

use super::{CJS_IMPORT_PREFIX, LLRT_PLATFORM};

include!(concat!(env!("OUT_DIR"), "/bytecode_cache.rs"));

fn rc_string_to_cow<'a>(rc: Rc<String>) -> Cow<'a, str> {
    match Rc::try_unwrap(rc) {
        Ok(string) => Cow::Owned(string),
        Err(rc) => Cow::Owned((*rc).clone()),
    }
}

static NODE_MODULES_PATHS_CACHE: Lazy<Mutex<HashMap<String, Vec<Box<str>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static FILESYSTEM_ROOT: Lazy<Box<str>> = Lazy::new(|| {
    #[cfg(unix)]
    {
        "/".into()
    }
    #[cfg(windows)]
    {
        if let Some(path) = home::home_dir() {
            if let Some(std::path::Component::Prefix(prefix)) = path.components().next() {
                return prefix
                    .as_os_str()
                    .to_string_lossy()
                    .into_owned()
                    .into_boxed_str();
            }
        }

        "C:".to_string().into_boxed_str()
    }
});

#[derive(Debug, Default)]
pub struct CustomResolver;

#[allow(clippy::manual_strip)]
impl Resolver for CustomResolver {
    fn resolve(&mut self, ctx: &Ctx, base: &str, name: &str) -> Result<String> {
        if name.starts_with(CJS_IMPORT_PREFIX) {
            return Ok(name.to_string());
        }

        let base = base.trim_start_matches(CJS_IMPORT_PREFIX);

        trace!("Try resolve '{}' from '{}'", name, base);

        require_resolve(ctx, name, base, true).map(|name| name.into_owned())
    }
}

// [CJS Reference Implementation](https://nodejs.org/api/modules.html#all-together)
// require(X) from module at path Y
pub fn require_resolve<'a>(
    ctx: &Ctx<'_>,
    x: &'a str,
    y: &str,
    is_esm: bool,
) -> Result<Cow<'a, str>> {
    trace!("require_resolve(x, y):({}, {})", x, y);

    // 1'. If X is a bytecode cache,
    if BYTECODE_CACHE.contains_key(x) {
        return resolved_by_bytecode_cache(x.into());
    }

    //fast path for when we have supported extensions
    let (_, ext_name) = name_extname(x);
    let is_supported_ext = is_supported_ext(ext_name);

    if is_supported_ext && Path::new(x).is_file() {
        return resolved_by_file_exists(x.into());
    }

    let x_normalized = path::normalize(x);
    if BYTECODE_CACHE.contains_key(&x_normalized) {
        return resolved_by_bytecode_cache(x_normalized.into());
    }

    if is_supported_ext && Path::new(&x_normalized).is_file() {
        return resolved_by_file_exists(x_normalized.into());
    }

    let x_is_absolute = path::is_absolute(x);
    let x_starts_with_current_dir = x.starts_with("./");

    // 2. If X begins with '/'
    let y = if path::is_absolute(x) {
        // a. set Y to be the file system root
        &*FILESYSTEM_ROOT
    } else {
        y
    };

    // Normalize path Y to generate dirname(Y)
    let dirname_y = if Path::new(y).is_dir() {
        path::resolve_path([y].iter())?
    } else {
        let dirname_y = path::dirname(y);
        path::resolve_path([&dirname_y].iter())?
    };

    // 3. If X begins with './' or '/' or '../'
    if x_starts_with_current_dir || x_is_absolute || x.starts_with("../") {
        let y_plus_x = if x_is_absolute {
            x.into()
        } else if x_starts_with_current_dir {
            [&dirname_y, "/", &x[2..]].concat()
        } else {
            [&dirname_y, "/", x].concat()
        };

        let y_plus_x = Rc::new(y_plus_x);

        // a. LOAD_AS_FILE(Y + X)
        if let Ok(Some(path)) = load_as_file(ctx, y_plus_x.clone()) {
            trace!("+- Resolved by `LOAD_AS_FILE`: {}\n", path);
            return to_abs_path(path);
        } else {
            // b. LOAD_AS_DIRECTORY(Y + X)
            if let Ok(Some(path)) = load_as_directory(ctx, y_plus_x) {
                trace!("+- Resolved by `LOAD_AS_DIRECTORY`: {}\n", path);
                return to_abs_path(path);
            }
        }

        // c. THROW "not found"
        return Err(Error::new_resolving(y.to_owned(), x.to_owned()));
    }

    // 4. If X begins with '#'
    if x.starts_with('#') {
        // a. LOAD_PACKAGE_IMPORTS(X, dirname(Y))
        if let Ok(Some(path)) = load_package_imports(ctx, x, &dirname_y) {
            trace!("+- Resolved by `LOAD_PACKAGE_IMPORTS`: {}\n", path);
            return Ok(path.into());
        }
    }

    // 5. LOAD_PACKAGE_SELF(X, dirname(Y))
    if let Ok(Some(path)) = load_package_self(ctx, x, &dirname_y, is_esm) {
        trace!("+- Resolved by `LOAD_PACKAGE_SELF`: {}\n", path);
        return Ok(path.into());
    }

    // 6. LOAD_NODE_MODULES(X, dirname(Y))
    if let Some(path) = load_node_modules(ctx, x, &dirname_y, is_esm) {
        trace!("+- Resolved by `LOAD_NODE_MODULES`: {}\n", path);
        return Ok(path);
    }

    // 6.5. LOAD_AS_FILE(X)
    if let Ok(Some(path)) = load_as_file(ctx, Rc::new(x.to_owned())) {
        trace!("+- Resolved by `LOAD_AS_FILE`: {}\n", path);
        return to_abs_path(path);
    }

    // 7. THROW "not found"
    Err(Error::new_resolving(y.to_string(), x.to_string()))
}

fn resolved_by_bytecode_cache(x: Cow<'_, str>) -> Result<Cow<'_, str>> {
    trace!("+- Resolved by `BYTECODE_CACHE`: {}\n", x);
    Ok(x)
}

fn resolved_by_file_exists(path: Cow<'_, str>) -> Result<Cow<'_, str>> {
    trace!("+- Resolved by `FILE`: {}\n", path);
    to_abs_path(path)
}

fn to_abs_path(path: Cow<'_, str>) -> Result<Cow<'_, str>> {
    Ok(if !is_absolute(&path) {
        resolve_path_with_separator([path], true)?.into()
    } else if cfg!(windows) {
        replace_backslash(path).into()
    } else {
        path
    })
}

// LOAD_AS_FILE(X)
fn load_as_file<'a>(ctx: &Ctx<'_>, x: Rc<String>) -> Result<Option<Cow<'a, str>>> {
    trace!("|  load_as_file(x): {}", x);

    // 1. If X is a file, load X as its file extension format. STOP
    if Path::new(x.as_ref()).is_file() {
        trace!("|  load_as_file(1): {}", x);
        return Ok(Some(rc_string_to_cow(x)));
    }

    // 2. If X.js is a file,
    for extension in SUPPORTED_EXTENSIONS.iter() {
        let file = [x.as_str(), extension].concat();
        if Path::new(&file).is_file() {
            // a. Find the closest package scope SCOPE to X.
            match find_the_closest_package_scope(&x) {
                // b. If no scope was found
                None => {
                    // 1. MAYBE_DETECT_AND_LOAD(X.js)
                    trace!("|  load_as_file(2.b.1): {}", file);
                    return Ok(Some(file.into()));
                },
                Some(path) => {
                    let mut package_json = fs::read(path.as_ref()).or_throw(ctx)?;
                    let package_json =
                        simd_json::to_borrowed_value(&mut package_json).or_throw(ctx)?;
                    // c. If the SCOPE/package.json contains "type" field,
                    if let Some(_type) = get_string_field(&package_json, "type") {
                        // 1. If the "type" field is "module", load X.js as an ECMAScript module. STOP.
                        // 2. If the "type" field is "commonjs", load X.js as an CommonJS module. STOP.
                        if _type == "module" || _type == "commonjs" {
                            trace!("|  load_as_file(2.c.[1|2]): {}", file);
                            return Ok(Some(file.into()));
                        }
                    }
                },
            }
            // d. MAYBE_DETECT_AND_LOAD(X.js)
            trace!("|  load_as_file(2.d): {}", file);
            return Ok(Some(file.into()));
        }
    }

    // 3. If X.json is a file, load X.json to a JavaScript Object. STOP
    let file = [&x, ".json"].concat();
    if Path::new(&file).is_file() {
        trace!("|  load_as_file(3): {}", file);
        return Ok(Some(file.into()));
    }

    // 4. If X.node is a file, load X.node as binary addon. STOP

    Ok(None)
}

// LOAD_INDEX(X)
fn load_index<'a>(ctx: &Ctx<'_>, x: Rc<String>) -> Result<Option<Cow<'a, str>>> {
    trace!("|  load_index(x): {}", x);

    // 1. If X/index.js is a file
    for extension in SUPPORTED_EXTENSIONS.iter() {
        let file = [x.as_str(), "/index", extension].concat();
        if Path::new(&file).is_file() {
            // a. Find the closest package scope SCOPE to X.
            match find_the_closest_package_scope(&x) {
                // b. If no scope was found, load X/index.js as a CommonJS module. STOP.
                None => {
                    trace!("|  load_index(1.b): {}", file);
                    return Ok(Some(file.into()));
                },
                // c. If the SCOPE/package.json contains "type" field,
                Some(path) => {
                    let mut package_json = fs::read(path.as_ref()).or_throw(ctx)?;
                    let package_json =
                        simd_json::to_borrowed_value(&mut package_json).or_throw(ctx)?;
                    if let Some(_type) = get_string_field(&package_json, "type") {
                        // 1. If the "type" field is "module", load X/index.js as an ECMAScript module. STOP.
                        if _type == "module" {
                            trace!("|  load_index(1.c.1): {}", file);
                            return Ok(Some(file.into()));
                        }
                    }
                    // 2. Else, load X/index.js as an CommonJS module. STOP.
                    trace!("|  load_index(1.c.2): {}", file);
                    return Ok(Some(file.into()));
                },
            }
        }
    }

    // 2. If X/index.json is a file, parse X/index.json to a JavaScript object. STOP
    let file = [x.as_str(), "/index.json"].concat();
    if Path::new(&file).is_file() {
        trace!("|  load_index(2): {}", file);
        return Ok(Some(file.into()));
    }

    // 3. If X/index.node is a file, load X/index.node as binary addon. STOP

    Ok(None)
}

// LOAD_AS_DIRECTORY(X)
fn load_as_directory<'a>(ctx: &Ctx<'_>, x: Rc<String>) -> Result<Option<Cow<'a, str>>> {
    trace!("|  load_as_directory(x): {}", x);

    // 1. If X/package.json is a file,
    let file = [&x, "/package.json"].concat();
    if Path::new(&file).is_file() {
        // a. Parse X/package.json, and look for "main" field.
        let mut package_json = fs::read(file).or_throw(ctx)?;
        let package_json = simd_json::to_borrowed_value(&mut package_json).or_throw(ctx)?;
        // b. If "main" is a falsy value, GOTO 2.
        if let Some(main) = get_string_field(&package_json, "main") {
            // c. let M = X + (json main field)
            let m = Rc::new([&x, "/", main].concat());
            // d. LOAD_AS_FILE(M)
            if let Ok(Some(path)) = load_as_file(ctx, m.clone()) {
                trace!("|  load_as_directory(1.d): {}", path);
                return Ok(Some(path));
            }
            // e. LOAD_INDEX(M)
            if let Ok(Some(path)) = load_index(ctx, m) {
                trace!("|  load_as_directory(1.e): {}", path);
                return Ok(Some(path));
            }
            // f. LOAD_INDEX(X) DEPRECATED

            // g. THROW "not found"
            return Err(Error::new_resolving("", x.to_string()));
        }
    }

    // 2. LOAD_INDEX(X)
    if let Ok(Some(path)) = load_index(ctx, x) {
        trace!("|  load_as_directory(2): {}", path);
        return Ok(Some(path));
    }

    Ok(None)
}

// LOAD_NODE_MODULES(X, START)
fn load_node_modules<'a>(
    ctx: &Ctx<'_>,
    x: &str,
    start: &str,
    is_esm: bool,
) -> Option<Cow<'a, str>> {
    trace!("|  load_node_modules(x, start): ({}, {})", x, start);

    // 1. let DIRS = NODE_MODULES_PATHS(START)
    let mut cache = NODE_MODULES_PATHS_CACHE.lock().unwrap();
    let dirs = cache
        .entry(start.to_string())
        .or_insert_with(|| node_modules_paths(start));

    // 2. for each DIR in DIRS:
    for dir in dirs {
        // a. LOAD_PACKAGE_EXPORTS(X, DIR)
        if let Ok(path) = load_package_exports(ctx, x, dir, is_esm) {
            trace!("|  load_node_modules(2.a): {}", path);
            return Some(path);
        }
        let dir_slash_x = Rc::new([dir, "/", x].concat());
        // b. LOAD_AS_FILE(DIR/X)
        if let Ok(Some(path)) = load_as_file(ctx, dir_slash_x.clone()) {
            trace!("|  load_node_modules(2.b): {}", path);
            return Some(path);
        }
        // c. LOAD_AS_DIRECTORY(DIR/X)
        if let Ok(Some(path)) = load_as_directory(ctx, dir_slash_x.clone()) {
            trace!("|  load_node_modules(2.c): {}", path);
            return Some(path);
        }
    }

    None
}

// NODE_MODULES_PATHS(START)
fn node_modules_paths(start: &str) -> Vec<Box<str>> {
    let path = Path::new(start);
    let mut dirs = Vec::new();
    let mut current = Some(path);

    // Iterate through parent directories
    while let Some(dir) = current {
        if dir.file_name().is_some_and(|name| name != "node_modules") {
            let mut node_modules = dir.to_path_buf();
            node_modules.push("node_modules");
            dirs.push(Box::from(node_modules.to_string_lossy()));
        }
        current = dir.parent();
    }

    // Add global folders
    if let Some(home) = home::home_dir() {
        dirs.push(Box::from(home.join(".node_modules").to_string_lossy()));
        dirs.push(Box::from(home.join(".node_libraries").to_string_lossy()));
    }

    dirs
}

// LOAD_PACKAGE_IMPORTS(X, DIR)
fn load_package_imports(ctx: &Ctx<'_>, x: &str, dir: &str) -> Result<Option<String>> {
    trace!("|  load_package_imports(x, dir): ({}, {})", x, dir);

    // 1. Find the closest package scope SCOPE to DIR.
    // 2. If no scope was found, return.
    if let Some(path) = find_the_closest_package_scope(dir) {
        let mut package_json_file = fs::read(path.as_ref()).or_throw(ctx)?;
        let package_json: BorrowedValue =
            simd_json::to_borrowed_value(&mut package_json_file).or_throw(ctx)?;

        // 3. If the SCOPE/package.json "imports" is null or undefined, return.
        // 4. If `--experimental-require-module` is enabled
        //   a. let CONDITIONS = ["node", "require", "module-sync"]
        //   b. Else, let CONDITIONS = ["node", "require"]
        // 5. let MATCH = PACKAGE_IMPORTS_RESOLVE(X, pathToFileURL(SCOPE),
        //   CONDITIONS) <a href="esm.md#resolver-algorithm-specification">defined in the ESM resolver</a>.
        // 6. RESOLVE_ESM_MATCH(MATCH).
        if let Some(module_path) = package_imports_resolve(&package_json, x) {
            trace!("|  load_package_imports(6): {}", module_path);
            let dir = path.as_ref().trim_end_matches("package.json");
            let module_path = to_abs_path(correct_extensions([dir, module_path].concat()))?;
            return Ok(Some(module_path.into()));
        }
    };

    Ok(None)
}

// LOAD_PACKAGE_EXPORTS(X, DIR)
fn load_package_exports<'a>(
    ctx: &Ctx<'_>,
    x: &str,
    dir: &str,
    is_esm: bool,
) -> Result<Cow<'a, str>> {
    trace!("|  load_package_exports(x, dir): ({}, {})", x, dir);
    //1. Try to interpret X as a combination of NAME and SUBPATH where the name
    //   may have a @scope/ prefix and the subpath begins with a slash (`/`).
    let (name, scope) = get_name_and_scope(x);

    //2. If X does not match this pattern or DIR/NAME/package.json is not a file,
    //   return.
    let mut package_json_path = String::with_capacity(dir.len() + 64);
    package_json_path.push_str(dir);
    package_json_path.push('/');
    let base_path_length = package_json_path.len();
    package_json_path.push_str(scope);
    package_json_path.push_str("/package.json");

    let mut sub_module = None;

    let (scope, name) = if name != "." && !Path::new(&package_json_path).exists() {
        package_json_path.truncate(base_path_length);
        package_json_path.push_str(x);
        package_json_path.push_str("/package.json");
        (x, ".")
    } else {
        for ext in JS_EXTENSIONS {
            let path = [
                &package_json_path[0..base_path_length],
                scope,
                name.as_ref().trim_start_matches("."),
                *ext,
            ]
            .concat();
            if Path::new(&path).exists() {
                if *ext == ".mjs" {
                    //we know its an ESM module
                    return Ok(path.into());
                }
                sub_module = Some(path);
            }
        }
        (scope, name.as_ref())
    };

    if !Path::new(&package_json_path).exists() {
        return Err(Error::new_resolving(dir.to_string(), x.to_string()));
    };

    //3. Parse DIR/NAME/package.json, and look for "exports" field.
    //4. If "exports" is null or undefined, return.
    //5. let MATCH = PACKAGE_EXPORTS_RESOLVE(pathToFileURL(DIR/NAME), "." + SUBPATH,
    //   `package.json` "exports", ["node", "require"]) <a href="esm.md#resolver-algorithm-specification">defined in the ESM resolver</a>.
    //6. RESOLVE_ESM_MATCH(MATCH)
    let mut package_json = fs::read(&package_json_path).or_throw(ctx)?;
    let package_json = simd_json::to_borrowed_value(&mut package_json).or_throw(ctx)?;

    if let Some(sub_module) = sub_module {
        if package_json.get_str("type") != Some("module") {
            if is_esm {
                return Ok([CJS_LOADER_PREFIX, &sub_module].concat().into());
            }
            return Ok(sub_module.into());
        }
        return Ok(sub_module.into());
    }

    let (module_path, is_cjs) = package_exports_resolve(&package_json, name, is_esm)?;

    let module_path = to_abs_path(correct_extensions(
        [dir, "/", scope, "/", module_path].concat(),
    ))?;

    let prefix = if is_cjs && is_esm {
        CJS_LOADER_PREFIX
    } else {
        ""
    };

    Ok([prefix, &module_path].concat().into())
}

// LOAD_PACKAGE_SELF(X, DIR)
fn load_package_self(ctx: &Ctx<'_>, x: &str, dir: &str, is_esm: bool) -> Result<Option<String>> {
    trace!("|  load_package_self(x, dir): ({}, {})", x, dir);
    let (name, scope) = get_name_and_scope(x);

    // 1. Find the closest package scope SCOPE to DIR.
    let mut package_json_file: Vec<u8>;
    let package_json: BorrowedValue;
    let package_json_path: Box<str> = match find_the_closest_package_scope(dir) {
        // 2. If no scope was found, return.
        None => {
            return Ok(None);
        },
        Some(path) => {
            package_json_file = fs::read(path.as_ref()).or_throw(ctx)?;
            package_json = simd_json::to_borrowed_value(&mut package_json_file).or_throw(ctx)?;
            // 3. If the SCOPE/package.json "exports" is null or undefined, return.
            if !is_exports_field_exists(&package_json) {
                return Ok(None);
            }
            // 4. If the SCOPE/package.json "name" is not the first segment of X, return.
            if let Some(name) = get_string_field(&package_json, "name") {
                if name != scope {
                    return Ok(None);
                }
            }
            path
        },
    };
    // 5. let MATCH = PACKAGE_EXPORTS_RESOLVE(pathToFileURL(SCOPE),
    //    "." + X.slice("name".length), `package.json` "exports", ["node", "require"])
    //    <a href="esm.md#resolver-algorithm-specification">defined in the ESM resolver</a>.
    // 6. RESOLVE_ESM_MATCH(MATCH)
    if let Ok((path, _)) = package_exports_resolve(&package_json, &name, is_esm) {
        trace!("|  load_package_self(2.c): {}", path);
        let dir = package_json_path.trim_end_matches("package.json");
        let module_path = correct_extensions([dir, path].concat());
        return Ok(Some(module_path.into()));
    }

    Ok(None)
}

fn get_name_and_scope(x: &str) -> (Cow<'_, str>, &str) {
    if let Some((s, n)) = x.split_once('/') {
        (Cow::Owned(["./", n].concat()), s)
    } else {
        (Cow::Borrowed("."), x)
    }
}

// Implementation equivalent to PACKAGE_EXPORTS_RESOLVE including RESOLVE_ESM_MATCH
fn package_exports_resolve<'a>(
    package_json: &'a BorrowedValue<'a>,
    modules_name: &str,
    is_esm: bool,
) -> Result<(&'a str, bool)> {
    let ident = if is_esm { "import" } else { "require" };

    if let BorrowedValue::Object(map) = package_json {
        let is_cjs =
            !matches!(map.get("type"), Some(BorrowedValue::String(ref _type)) if _type == "module");

        if let Some(BorrowedValue::Object(exports)) = map.get("exports") {
            if let Some(BorrowedValue::Object(name)) = exports.get(modules_name) {
                // Check for exports -> name -> platform(browser or node) -> [import | require]
                if let Some(BorrowedValue::Object(platform)) = name.get(LLRT_PLATFORM.as_str()) {
                    if let Some(BorrowedValue::String(ident)) = platform.get(ident) {
                        return Ok((ident.as_ref(), is_cjs));
                    }
                }
                // Check for exports -> name -> [import | require] -> default
                if let Some(BorrowedValue::Object(ident)) = name.get(ident) {
                    if let Some(BorrowedValue::String(default)) = ident.get("default") {
                        return Ok((default.as_ref(), is_cjs));
                    }
                }
                // Check for exports -> name -> [import | require]
                if let Some(BorrowedValue::String(ident)) = name.get(ident) {
                    return Ok((ident.as_ref(), is_cjs));
                }
                // [CJS only] Check for exports -> name -> default
                if !is_esm {
                    if let Some(BorrowedValue::String(default)) = name.get("default") {
                        return Ok((default.as_ref(), is_cjs));
                    }
                }
            }
            // Check for exports -> [import | require] -> default
            if let Some(BorrowedValue::Object(ident)) = exports.get(ident) {
                if let Some(BorrowedValue::String(default)) = ident.get("default") {
                    return Ok((default.as_ref(), is_cjs));
                }
            }
            // Check for exports -> [import | require]
            if let Some(BorrowedValue::String(ident)) = exports.get(ident) {
                return Ok((ident.as_ref(), is_cjs));
            }
            // [CJS only] Check for exports -> default
            if !is_esm {
                if let Some(BorrowedValue::String(default)) = exports.get("default") {
                    return Ok((default.as_ref(), is_cjs));
                }
            }
        }
        // Check for platform(browser or node) field
        if let Some(BorrowedValue::String(platform)) = map.get(LLRT_PLATFORM.as_str()) {
            return Ok((platform.as_ref(), is_cjs));
        }
        // [ESM only] Check for module field
        if is_esm {
            if let Some(BorrowedValue::String(module)) = map.get("module") {
                return Ok((module.as_ref(), is_cjs));
            }
        }
        // Check for main field
        if let Some(BorrowedValue::String(main)) = map.get("main") {
            return Ok((main.as_ref(), is_cjs));
        }
    }
    Ok(("./index.js", true))
}

// Implementation equivalent to PACKAGE_IMPORTS_RESOLVE including RESOLVE_ESM_MATCH
fn package_imports_resolve<'a>(
    package_json: &'a BorrowedValue<'a>,
    modules_name: &str,
) -> Option<&'a str> {
    if let BorrowedValue::Object(map) = package_json {
        if let Some(BorrowedValue::Object(imports)) = map.get("imports") {
            if let Some(BorrowedValue::Object(name)) = imports.get(modules_name) {
                // Check for imports -> name -> platform(browser or node)
                if let Some(BorrowedValue::String(platform)) = name.get(LLRT_PLATFORM.as_str()) {
                    return Some(platform.as_ref());
                }
                // Check for imports -> name -> require
                if let Some(BorrowedValue::String(require)) = name.get("require") {
                    return Some(require.as_ref());
                }
                // Check for imports -> name -> module-sync
                if let Some(BorrowedValue::String(module_sync)) = name.get("module-sync") {
                    return Some(module_sync.as_ref());
                }
                // Check for imports -> name -> default
                if let Some(BorrowedValue::String(default)) = name.get("default") {
                    return Some(default.as_ref());
                }
            }
            // Check for imports -> name
            if let Some(BorrowedValue::String(name)) = imports.get(modules_name) {
                return Some(name.as_ref());
            }
        }
    }
    None
}

fn find_the_closest_package_scope(start: &str) -> Option<Box<str>> {
    let mut current_dir = PathBuf::from(start);
    loop {
        let package_json_path = current_dir.join("package.json");
        if package_json_path.exists() {
            return package_json_path.to_str().map(Box::from);
        }
        if !current_dir.pop() {
            break;
        }
    }
    None
}

fn get_string_field<'a>(package_json: &'a BorrowedValue<'a>, str: &str) -> Option<&'a str> {
    if let BorrowedValue::Object(map) = package_json {
        if let Some(BorrowedValue::String(val)) = map.get(str) {
            return Some(val.as_ref());
        }
    }
    None
}

fn is_exports_field_exists<'a>(package_json: &'a BorrowedValue<'a>) -> bool {
    if let BorrowedValue::Object(map) = package_json {
        if let Some(BorrowedValue::Object(_)) = map.get("exports") {
            return true;
        }
    }
    false
}

fn correct_extensions<'a>(x: String) -> Cow<'a, str> {
    let (x_is_file, x_is_dir) = if let Ok(md) = fs::metadata(&x) {
        (md.is_file(), md.is_dir())
    } else {
        (false, false)
    };

    if x_is_file {
        return x.into();
    };

    let index = if x_is_dir { "/index" } else { "" };

    for extension in JS_EXTENSIONS.iter() {
        let file = [x.as_str(), index, extension].concat();
        if Path::new(&file).is_file() {
            return file.into();
        }
    }
    x.into()
}
