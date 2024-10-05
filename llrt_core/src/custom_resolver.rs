// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use lazy_static::lazy_static;
use llrt_modules::path::{self, is_absolute};
use llrt_utils::result::ResultExt;
use rquickjs::{loader::Resolver, prelude::Rest, Ctx, Error, Result};
use simd_json::BorrowedValue;
use tracing::trace;

use crate::modules::path::dirname;

include!(concat!(env!("OUT_DIR"), "/bytecode_cache.rs"));

lazy_static! {
    static ref NODE_MODULES_PATHS_CACHE: Mutex<HashMap<String, Vec<String>>> =
        Mutex::new(HashMap::new());
}

#[derive(Debug, Default)]
pub struct CustomResolver;

#[allow(clippy::manual_strip)]
impl Resolver for CustomResolver {
    fn resolve(&mut self, ctx: &Ctx, base: &str, name: &str) -> Result<String> {
        trace!("Try resolve '{}' from '{}'", name, base);
        require_from_module(ctx, name, base, true)
    }
}

// require(X) from module at path Y
pub fn require_from_module(ctx: &Ctx<'_>, x: &str, y: &str, is_esm: bool) -> Result<String> {
    trace!("require_from_module(x, y):({}, {})", x, y);

    // 1. If X is a core module,
    //   a. return the core module
    //   b. STOP

    // 1'. If X is a bytecode cache,
    let normalized_x = path::normalize(x.to_string());

    if BYTECODE_CACHE.contains_key(&normalized_x) {
        // a. return the core module
        // b. STOP
        trace!(
            "+- Resolved by phase 1 of `BYTECODE_CACHE`: {}\n",
            normalized_x
        );
        return Ok(normalized_x);
    }

    if BYTECODE_CACHE.contains_key(y) && Path::new(&normalized_x).exists() {
        trace!("+- Resolved by phase 2 of `BYTECODE_CACHE`: {}\n", x);
        return Ok(x.to_string());
    }

    // 2. If X begins with '/'
    let y = if is_absolute(x) {
        // a. set Y to be the file system root
        let file_system_root = match env::consts::OS {
            "windows" => Path::new("C:\\"),
            _ => Path::new("/"),
        };
        file_system_root.to_str().unwrap()
    } else {
        y
    };

    // Normalized path Y
    let dirname_y = if Path::new(y).is_dir() {
        path::resolve(Rest(vec![y.to_string()]))
    } else {
        let dirname_y = dirname(y.to_string());
        path::resolve(Rest(vec![dirname_y]))
    };

    // 3. If X begins with './' or '/' or '../'
    if x.starts_with("./") || is_absolute(x) || x.starts_with("../") {
        let y_plus_x = [&dirname_y, "/", x].concat();
        let y_plus_x = y_plus_x.as_str();
        // a. LOAD_AS_FILE(Y + X)
        if let Ok(Some(path)) = load_as_file(ctx, y_plus_x) {
            trace!("+- Resolved by `LOAD_AS_FILE`: {}\n", path);
            return Ok(path);
        }
        // b. LOAD_AS_DIRECTORY(Y + X)
        if let Ok(Some(path)) = load_as_directory(ctx, y_plus_x) {
            trace!("+- Resolved by `LOAD_AS_DIRECTORY`: {}\n", path);
            return Ok(path);
        }
        // c. THROW "not found"
        return Err(Error::new_resolving(y.to_string(), x.to_string()));
    }

    // 4. If X begins with '#'
    //     a. LOAD_PACKAGE_IMPORTS(X, dirname(Y))

    // 5. LOAD_PACKAGE_SELF(X, dirname(Y))
    if let Some(path) = load_package_self(ctx, x, &dirname_y, is_esm) {
        trace!("+- Resolved by `LOAD_PACKAGE_SELF`: {}\n", path);
        return Ok(path);
    }

    // 6. LOAD_NODE_MODULES(X, dirname(Y))
    if let Some(path) = load_node_modules(ctx, x, &dirname_y, is_esm) {
        trace!("+- Resolved by `LOAD_NODE_MODULES`: {}\n", path);
        return Ok(path);
    }

    // 7. THROW "not found"
    Err(Error::new_resolving(y.to_string(), x.to_string()))
}

fn find_closest_package_scope(start: &str) -> Option<String> {
    let mut current_dir = PathBuf::from(start);
    loop {
        let package_json_path = current_dir.join("package.json");
        if package_json_path.exists() {
            return package_json_path.to_str().map(|s| s.to_string());
        }
        if !current_dir.pop() {
            break;
        }
    }
    None
}

// MAYBE_DETECT_AND_LOAD(X)
#[allow(dead_code)]
fn maybe_detect_and_load(_ctx: &Ctx<'_>, x: &str) -> Result<Option<String>> {
    trace!("|  maybe_detect_and_load(x): {}", x);
    // 1. If X parses as a CommonJS module, load X as a CommonJS module. STOP.
    // 2. Else, if `--experimental-require-module` is
    //  enabled, and the source code of X can be parsed as ECMAScript module using
    //  <a href="esm.md#resolver-algorithm-specification">DETECT_MODULE_SYNTAX defined in
    //  the ESM resolver</a>,
    // a. Load X as an ECMAScript module. STOP.
    //3. THROW the SyntaxError from attempting to parse X as CommonJS in 1. STOP.
    Err(Error::new_resolving("", x.to_string()))
}

// LOAD_AS_FILE(X)
fn load_as_file(ctx: &Ctx<'_>, x: &str) -> Result<Option<String>> {
    trace!("|  load_as_file(x): {}", x);

    // 1. If X is a file, load X as its file extension format. STOP
    if Path::new(&x).is_file() {
        trace!("|  load_as_file(1): {}", x);
        return Ok(Some(x.to_string()));
    }

    // 2. If X.js is a file,
    for extension in [".js", ".mjs", ".cjs"].iter() {
        let file = [x, extension].concat();
        if Path::new(&file).is_file() {
            // a. Find the closest package scope SCOPE to X.
            match find_closest_package_scope(x) {
                // b. If no scope was found
                None => {
                    // 1. MAYBE_DETECT_AND_LOAD(X.js)
                    trace!("|  load_as_file(2.b.1): {}", file);
                    return Ok(Some(file.to_string()));
                },
                // c. If the SCOPE/package.json contains "type" field,
                Some(path) => {
                    let mut package_json = fs::read(&path).unwrap_or_default();
                    let package_json =
                        simd_json::to_borrowed_value(&mut package_json).or_throw(ctx)?;
                    if let Some(_type) = get_type_field(&package_json) {
                        // 1. If the "type" field is "module", load X.js as an ECMAScript module. STOP.
                        if matches!(_type, "module") {
                            trace!("|  load_as_file(2.c.1): {}", file);
                            return Ok(Some(file.to_string()));
                        }
                        // 2. If the "type" field is "commonjs", load X.js as an CommonJS module. STOP.
                        if matches!(_type, "commonjs") {
                            trace!("|  load_as_file(2.c.2): {}", file);
                            return Ok(Some(file.to_string()));
                        }
                    }
                    // If “type” is undefined, the path must be returned here for it to work.
                    trace!("|  load_as_file(2.c): {}", file);
                    return Ok(Some(file));
                },
            }
        }
    }

    // 3. If X.json is a file, load X.json to a JavaScript Object. STOP
    let file = [x, ".json"].concat();
    if Path::new(&file).is_file() {
        trace!("|  load_as_file(3): {}", file);
        return Ok(Some(file));
    }

    // 4. If X.node is a file, load X.node as binary addon. STOP

    // 4'. If X.lrt is a file, load X.lrt as JavaScript bytecode. STOP
    let file = [x, ".lrt"].concat();
    if Path::new(&file).is_file() {
        trace!("|  load_as_file(4'): {}", file);
        return Ok(Some(file));
    }

    Ok(None)
}

// LOAD_INDEX(X)
fn load_index(ctx: &Ctx<'_>, x: &str) -> Result<Option<String>> {
    trace!("|  load_index(x): {}", x);

    // 1. If X/index.js is a file
    for extension in [".js", ".mjs", ".cjs"].iter() {
        let file = [x, "/index", extension].concat();
        if Path::new(&file).is_file() {
            // a. Find the closest package scope SCOPE to X.
            match find_closest_package_scope(x) {
                //    b. If no scope was found, load X/index.js as a CommonJS module. STOP.
                None => {
                    trace!("|  load_index(1.b): {}", file);
                    return Ok(Some(file.to_string()));
                },
                // c. If the SCOPE/package.json contains "type" field,
                Some(path) => {
                    let mut package_json = fs::read(&path).unwrap_or_default();
                    let package_json =
                        simd_json::to_borrowed_value(&mut package_json).or_throw(ctx)?;
                    if let Some(_type) = get_type_field(&package_json) {
                        // 1. If the "type" field is "module", load X/index.js as an ECMAScript module. STOP.
                        if matches!(_type, "module") {
                            trace!("|  load_index(1.c.1): {}", file);
                            return Ok(Some(file.to_string()));
                        }
                        // 2. Else, load X/index.js as an CommonJS module. STOP.
                        else {
                            trace!("|  load_index(1.c.2): {}", file);
                            return Ok(Some(file.to_string()));
                        }
                    }
                    // If “type” is undefined, the path must be returned here for it to work.
                    trace!("|  load_index(1.c): {}", file);
                    return Ok(Some(file));
                },
            }
        }
    }

    //2. If X/index.json is a file, parse X/index.json to a JavaScript object. STOP
    let file = [x, "/index.json"].concat();
    if Path::new(&file).is_file() {
        trace!("|  load_index(2): {}", file);
        return Ok(Some(file));
    }

    //3. If X/index.node is a file, load X/index.node as binary addon. STOP

    //3'. If X/index.lrt is a file, load X/index.lrt as JavaScript bytecode. STOP
    let file = [x, "/index.lrt"].concat();
    if Path::new(&file).is_file() {
        trace!("|  load_index(3'): {}", file);
        return Ok(Some(file));
    }

    Ok(None)
}

// LOAD_AS_DIRECTORY(X)
fn load_as_directory(ctx: &Ctx<'_>, x: &str) -> Result<Option<String>> {
    trace!("|  load_as_directory(x): {}", x);

    // 1. If X/package.json is a file,
    let file = [x, "/package.json"].concat();
    if Path::new(&file).is_file() {
        // a. Parse X/package.json, and look for "main" field.
        let mut package_json = fs::read(&file).unwrap_or_default();
        let package_json = simd_json::to_borrowed_value(&mut package_json).or_throw(ctx)?;
        let main = get_main_field(&package_json);

        // b. If "main" is a falsy value, GOTO 2.
        if let Some(main) = main {
            // c. let M = X + (json main field)
            let m = [x, "/", main].concat();
            // d. LOAD_AS_FILE(M)
            if let Ok(Some(path)) = load_as_file(ctx, &m) {
                trace!("|  load_as_directory(1.d): {}", path);
                return Ok(Some(path));
            }
            // e. LOAD_INDEX(M)
            if let Ok(Some(path)) = load_index(ctx, &m) {
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
fn load_node_modules(ctx: &Ctx<'_>, x: &str, start: &str, is_esm: bool) -> Option<String> {
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
        let dir_slash_x = [dir, "/", x].concat();
        // b. LOAD_AS_FILE(DIR/X)
        if let Ok(Some(path)) = load_as_file(ctx, &dir_slash_x) {
            trace!("|  load_node_modules(2.b): {}", path);
            return Some(path);
        }
        // c. LOAD_AS_DIRECTORY(DIR/X)
        if let Ok(Some(path)) = load_as_directory(ctx, &dir_slash_x) {
            trace!("|  load_node_modules(2.c): {}", path);
            return Some(path);
        }
    }

    None
}

// NODE_MODULES_PATHS(START)
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
        // b. DIR = path join(PARTS[0 .. I] + "node_modules")
        let mut dir: Vec<&str> = parts[0..=i as usize].to_vec();
        dir.push("node_modules");
        // c. DIRS = DIR + DIRS
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

// LOAD_PACKAGE_IMPORTS(X, DIR)
// 1. Find the closest package scope SCOPE to DIR.
// 2. If no scope was found, return.
// 3. If the SCOPE/package.json "imports" is null or undefined, return.
// 4. let MATCH = PACKAGE_IMPORTS_RESOLVE(X, pathToFileURL(SCOPE),
//   ["node", "require"]) <a href="esm.md#resolver-algorithm-specification">defined in the ESM resolver</a>.
// 5. RESOLVE_ESM_MATCH(MATCH).

// LOAD_PACKAGE_EXPORTS(X, DIR)
fn load_package_exports(ctx: &Ctx<'_>, x: &str, dir: &str, is_esm: bool) -> Result<String> {
    //1. Try to interpret X as a combination of NAME and SUBPATH where the name
    //   may have a @scope/ prefix and the subpath begins with a slash (`/`).
    let (scope, name) = match x.split_once('/') {
        Some((s, n)) => (s, ["./", n].concat()),
        None => (x, ".".to_string()),
    };

    //2. If X does not match this pattern or DIR/NAME/package.json is not a file,
    //   return.
    let mut package_json_path = [dir, "/"].concat();
    let base_path_length = package_json_path.len();
    package_json_path.push_str(scope);
    package_json_path.push_str("/package.json");

    let (scope, name) = if name != "." && !Path::new(&package_json_path).exists() {
        package_json_path.truncate(base_path_length);
        package_json_path.push_str(x);
        package_json_path.push_str("/package.json");
        (x, ".")
    } else {
        (scope, name.as_str())
    };

    if !Path::new(&package_json_path).exists() {
        return Err(Error::new_resolving(dir.to_string(), x.to_string()));
    };

    //3. Parse DIR/NAME/package.json, and look for "exports" field.
    //4. If "exports" is null or undefined, return.
    //5. let MATCH = PACKAGE_EXPORTS_RESOLVE(pathToFileURL(DIR/NAME), "." + SUBPATH,
    //   `package.json` "exports", ["node", "require"]) <a href="esm.md#resolver-algorithm-specification">defined in the ESM resolver</a>.
    //6. RESOLVE_ESM_MATCH(MATCH)
    let mut package_json = fs::read(&package_json_path).unwrap_or_default();
    let package_json = simd_json::to_borrowed_value(&mut package_json).or_throw(ctx)?;

    let module_path = package_exports_resolve(&package_json, name, is_esm)?;

    Ok([dir, "/", scope, "/", module_path].concat())
}

// LOAD_PACKAGE_SELF(X, DIR)
fn load_package_self(_ctx: &Ctx<'_>, _x: &str, _dir: &str, _is_esm: bool) -> Option<String> {
    // 1. Find the closest package scope SCOPE to DIR.
    // 2. If no scope was found, return.
    // 3. If the SCOPE/package.json "exports" is null or undefined, return.
    // 4. If the SCOPE/package.json "name" is not the first segment of X, return.
    // 5. let MATCH = PACKAGE_EXPORTS_RESOLVE(pathToFileURL(SCOPE),
    //    "." + X.slice("name".length), `package.json` "exports", ["node", "require"])
    //    <a href="esm.md#resolver-algorithm-specification">defined in the ESM resolver</a>.
    // 6. RESOLVE_ESM_MATCH(MATCH)
    None
}

// PACKAGE_EXPORTS_RESOLVE(packageURL, subpath, exports, conditions)
// If exports is an Object with both a key starting with "." and a key not starting with ".", throw an Invalid Package Configuration error.
// If subpath is equal to ".", then
// Let mainExport be undefined.
// If exports is a String or Array, or an Object containing no keys starting with ".", then
// Set mainExport to exports.
// Otherwise if exports is an Object containing a "." property, then
// Set mainExport to exports["."].
// If mainExport is not undefined, then
// Let resolved be the result of PACKAGE_TARGET_RESOLVE( packageURL, mainExport, null, false, conditions).
// If resolved is not null or undefined, return resolved.
// Otherwise, if exports is an Object and all keys of exports start with ".", then
// Assert: subpath begins with "./".
// Let resolved be the result of PACKAGE_IMPORTS_EXPORTS_RESOLVE( subpath, exports, packageURL, false, conditions).
// If resolved is not null or undefined, return resolved.
// Throw a Package Path Not Exported error.
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

fn get_main_field<'a>(package_json: &'a BorrowedValue<'a>) -> Option<&'a str> {
    if let BorrowedValue::Object(map) = package_json {
        if let Some(BorrowedValue::String(str)) = map.get("main") {
            return Some(str.as_ref());
        }
    }
    None
}

fn get_type_field<'a>(package_json: &'a BorrowedValue<'a>) -> Option<&'a str> {
    if let BorrowedValue::Object(map) = package_json {
        if let Some(BorrowedValue::String(str)) = map.get("type") {
            return Some(str.as_ref());
        }
    }
    None
}
