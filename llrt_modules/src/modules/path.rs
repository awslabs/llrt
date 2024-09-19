// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    borrow::Cow,
    path::{Component, Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR},
};

use llrt_utils::module::export_default;
use rquickjs::{
    function::Opt,
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Rest},
    Ctx, Object, Result,
};

use crate::module_info::ModuleInfo;

pub struct PathModule;

#[cfg(windows)]
const DELIMITER: char = ';';
#[cfg(not(windows))]
const DELIMITER: char = ':';

#[cfg(windows)]
pub const CURRENT_DIR_STR: &str = ".\\";

#[cfg(windows)]
const FORWARD_SLASH_STR: &str = "/";

#[cfg(not(windows))]
pub const CURRENT_DIR_STR: &str = "./";

#[cfg(windows)]
use memchr::memchr2;

#[cfg(windows)]
fn find_next_separator(s: &str) -> Option<usize> {
    memchr2(b'\\', b'/', s.as_bytes())
}

#[cfg(not(windows))]
fn find_next_separator(s: &str) -> Option<usize> {
    s.find(MAIN_SEPARATOR)
}

#[cfg(windows)]
fn find_last_sep(path: &str) -> Option<usize> {
    memchr::memchr2_iter(b'\\', b'/', path.as_bytes()).next_back()
}

#[cfg(not(windows))]
fn find_last_sep(path: &str) -> Option<usize> {
    path.rfind(MAIN_SEPARATOR)
}

pub fn dirname(path: String) -> String {
    if path.is_empty() {
        return String::from(".");
    }

    #[cfg(windows)]
    {
        if path == MAIN_SEPARATOR_STR || path == FORWARD_SLASH_STR {
            return path;
        }
    }
    #[cfg(not(windows))]
    {
        if path == MAIN_SEPARATOR_STR {
            return path;
        }
    }

    let path = strip_last_sep(&path);
    let sep_pos = find_last_sep(path);

    match sep_pos {
        Some(idx) => {
            let parent = &path[..idx];
            if parent.is_empty() {
                MAIN_SEPARATOR_STR
            } else {
                parent
            }
        },
        None => ".",
    }
    .to_string()
}

fn name_extname(path: &str) -> (&str, &str) {
    let path = strip_last_sep(path);
    let sep_pos = find_last_sep(path);

    let path = match sep_pos {
        Some(idx) => &path[idx + 1..],
        None => path,
    };
    if path.starts_with('.') {
        return (path, "");
    }
    match path.rfind('.') {
        Some(idx) => path.split_at(idx),
        None => (path, ""),
    }
}

fn strip_last_sep(path: &str) -> &str {
    if ends_with_sep(path) {
        &path[..path.len() - 1]
    } else {
        path
    }
}

fn basename(path: String, suffix: Opt<String>) -> String {
    #[cfg(windows)]
    {
        if path.is_empty() || path == MAIN_SEPARATOR_STR || path == FORWARD_SLASH_STR {
            return String::from("");
        }
    }
    #[cfg(not(windows))]
    {
        if path.is_empty() || path == MAIN_SEPARATOR_STR {
            return String::from("");
        }
    }

    let (base, ext) = name_extname(&path);
    let mut name = [base, ext].concat();
    if let Some(suffix) = suffix.0 {
        if let Some(location) = name.rfind(&suffix) {
            name.truncate(location);
            return name;
        }
    }
    name
}

fn extname(path: String) -> String {
    let (_, ext) = name_extname(&path);
    ext.to_string()
}

fn format(obj: Object) -> String {
    let dir: String = obj.get("dir").unwrap_or_default();
    let root: String = obj.get("root").unwrap_or_default();
    let base: String = obj.get("base").unwrap_or_default();
    let name: String = obj.get("name").unwrap_or_default();
    let ext: String = obj.get("ext").unwrap_or_default();

    let mut path = String::new();
    if !dir.is_empty() {
        path.push_str(&dir);
        if !ends_with_sep(&dir) {
            path.push(MAIN_SEPARATOR);
        }
    } else if !root.is_empty() {
        path.push_str(&root);
        if !ends_with_sep(&root) {
            path.push(MAIN_SEPARATOR);
        }
    }
    if !base.is_empty() {
        path.push_str(&base);
    } else {
        path.push_str(&name);
        if !ext.is_empty() {
            if !ext.starts_with('.') {
                path.push('.');
            }
            path.push_str(&ext);
        }
    }
    path
}

fn parse(ctx: Ctx, path_str: String) -> Result<Object> {
    let obj = Object::new(ctx)?;
    let path = Path::new(&path_str);
    let parent = path
        .parent()
        .map(|p| p.to_str().unwrap())
        .unwrap_or_default();
    let filename = path
        .file_name()
        .map(|n| n.to_str().unwrap())
        .unwrap_or_default();

    let (name, extension) = name_extname(filename);

    let root = path
        .components()
        .next()
        .and_then(|c| match c {
            Component::Prefix(prefix) => prefix.as_os_str().to_str(),
            Component::RootDir => c.as_os_str().to_str(),
            _ => Some(""),
        })
        .unwrap_or_default();

    obj.set("root", root)?;
    obj.set("dir", parent)?;
    obj.set("base", [name, extension].concat())?;
    obj.set("ext", extension)?;
    obj.set("name", name)?;

    Ok(obj)
}

fn join(parts: Rest<String>) -> String {
    join_path(parts.0.iter())
}

fn relative(from: String, to: String) -> String {
    relative_path(from, to)
}

pub fn join_path<S, I>(parts: I) -> String
where
    S: AsRef<str>,
    I: IntoIterator<Item = S>,
{
    join_path_with_separator(parts, false)
}

pub fn join_path_with_separator<S, I>(parts: I, force_posix_sep: bool) -> String
where
    S: AsRef<str>,
    I: IntoIterator<Item = S>,
{
    //fine because we're either moving or storing references
    let parts_vec: Vec<S> = parts.into_iter().collect();
    //add one slash plus drive letter
    //max is probably parts+size
    let likely_max_size = parts_vec
        .iter()
        .map(|p| p.as_ref().len() + 1)
        .sum::<usize>()
        + 10;
    let result = String::with_capacity(likely_max_size);
    join_resolve_path(parts_vec, false, result, PathBuf::new(), force_posix_sep)
}

pub fn resolve_path<S, I>(parts: I) -> String
where
    S: AsRef<str>,
    I: IntoIterator<Item = S>,
{
    resolve_path_with_separator(parts, false)
}

pub fn resolve_path_with_separator<S, I>(parts: I, force_posix_sep: bool) -> String
where
    S: AsRef<str>,
    I: IntoIterator<Item = S>,
{
    let cwd = std::env::current_dir().expect("Unable to access working directory");

    let mut result = cwd.to_string_lossy().to_string();
    //add MAIN_SEPARATOR if we're not on already MAIN_SEPARATOR
    if !result.ends_with(MAIN_SEPARATOR) {
        result.push(MAIN_SEPARATOR);
    }
    #[cfg(windows)]
    {
        if force_posix_sep {
            result = result.replace(MAIN_SEPARATOR, FORWARD_SLASH_STR);
        }
    }
    join_resolve_path(parts, true, result, cwd, force_posix_sep)
}

pub fn relative_path<F, T>(from: F, to: T) -> String
where
    F: AsRef<str>,
    T: AsRef<str>,
{
    let from_ref = from.as_ref();
    let to_ref = to.as_ref();
    if from_ref == to_ref {
        return "".to_string();
    }

    let mut abs_from = None;

    if !is_absolute(from_ref) {
        abs_from = Some(
            std::env::current_dir()
                .expect("Unable to access working directory")
                .to_string_lossy()
                .to_string()
                + MAIN_SEPARATOR_STR
                + from_ref,
        );
    }

    let mut abs_to = None;

    if !is_absolute(to_ref) {
        abs_to = Some(
            std::env::current_dir()
                .expect("Unable to access working directory")
                .to_string_lossy()
                .to_string()
                + MAIN_SEPARATOR_STR
                + to_ref,
        );
    }

    let from_ref = abs_from.as_deref().unwrap_or(from_ref);
    let to_ref = abs_to.as_deref().unwrap_or(to_ref);

    let mut from_index = 0;
    let mut to_index = 0;
    // skip common prefix
    while from_index < from_ref.len() && to_index < to_ref.len() {
        let from_next = find_next_separator(&from_ref[from_index..])
            .unwrap_or(from_ref.len() - from_index)
            + from_index;
        let to_next =
            find_next_separator(&to_ref[to_index..]).unwrap_or(to_ref.len() - to_index) + to_index;
        if from_ref[from_index..from_next] != to_ref[to_index..to_next] {
            break;
        }
        from_index = from_next + 1; //move past the separator
        to_index = to_next + 1; //move past the separator
    }
    let mut relative = String::new();
    // add ".." for each remaining component in 'from'
    while from_index < from_ref.len() {
        let from_next = find_next_separator(&from_ref[from_index..])
            .unwrap_or(from_ref.len() - from_index)
            + from_index;
        if !relative.is_empty() {
            relative.push(MAIN_SEPARATOR);
        }
        relative.push_str("..");
        from_index = from_next + 1; // Move past the separator
    }
    // add the remaining components from 'to'
    while to_index < to_ref.len() {
        let to_next =
            find_next_separator(&to_ref[to_index..]).unwrap_or(to_ref.len() - to_index) + to_index;
        if !relative.is_empty() {
            relative.push(MAIN_SEPARATOR);
        }
        let component = &to_ref[to_index..to_next];
        if component != "." {
            relative.push_str(component);
        }
        to_index = to_next + 1; // Move past the separator
    }
    if relative.is_empty() {
        ".".to_string()
    } else {
        relative
    }
}

fn join_resolve_path<S, I>(
    parts: I,
    resolve: bool,
    mut result: String,
    cwd: PathBuf,
    force_posix_sep: bool,
) -> String
where
    S: AsRef<str>,
    I: IntoIterator<Item = S>,
{
    let sep = if force_posix_sep { '/' } else { MAIN_SEPARATOR };
    let mut resolve_cow: Cow<str>;
    let mut resolve_path_buf: PathBuf;
    let mut empty = true;
    let mut prefix_len = 0;

    let mut index_stack = Vec::with_capacity(16);

    for part in parts {
        let mut part_ref: &str = part.as_ref();
        let mut start = 0;
        if resolve {
            if cfg!(not(windows)) {
                if part_ref.starts_with(MAIN_SEPARATOR) {
                    empty = false;
                    result = MAIN_SEPARATOR.into();
                    start = 1;
                }
            } else {
                let starts_with_sep = starts_with_sep(part_ref);
                if starts_with_sep {
                    let (prefix, _) = get_path_prefix(&cwd);
                    prefix_len = prefix.len();
                    result = prefix;
                    empty = false;
                    result.push(sep);
                } else {
                    let path_buf: PathBuf = PathBuf::from(part_ref);
                    if path_buf.is_absolute() {
                        empty = false;
                        let (prefix, mut components) = get_path_prefix(&path_buf);
                        if !prefix.is_empty() {
                            components.next(); //consume prefix
                        }
                        prefix_len = prefix.len();
                        result = prefix;
                        result.push(sep);
                        resolve_path_buf = components.collect();
                        resolve_cow = resolve_path_buf.to_string_lossy();
                        part_ref = resolve_cow.as_ref();
                    }
                }
            }
        } else if starts_with_sep(part_ref) && empty {
            empty = false;
            result.push(sep);
            start = 1;
        }

        while start < part_ref.len() {
            let end = find_next_separator(&part_ref[start..]).map_or(part_ref.len(), |i| i + start);
            match &part_ref[start..end] {
                ".." => {
                    empty = false;
                    if let Some(last_index) = index_stack.pop() {
                        result.truncate(last_index);
                    }
                },
                "" | "." => {
                    //ignore
                },
                sub_part => {
                    let len = result.len();
                    result.push_str(sub_part);
                    result.push(sep);
                    empty = false;
                    index_stack.push(len);
                },
            }
            start = end + 1;
        }
    }

    if result.len() > prefix_len + 1 && ends_with_sep(&result) {
        result.truncate(result.len() - 1);
    }

    result
}

pub fn resolve(path: Rest<String>) -> String {
    resolve_path(path.iter())
}

fn get_path_prefix(cwd: &Path) -> (String, std::iter::Peekable<std::path::Components<'_>>) {
    let mut components = cwd.components().peekable();

    let prefix = if let Some(Component::Prefix(prefix)) = components.peek() {
        prefix.as_os_str().to_str().unwrap().to_string()
    } else {
        "".into()
    };

    (prefix, components)
}

fn normalize(path: String) -> String {
    join_path([path].iter())
}

#[allow(dead_code)] //used by windows
fn starts_with_sep(path: &str) -> bool {
    matches!(path.as_bytes().first().unwrap_or(&0), b'/' | b'\\')
}

#[cfg(windows)]
fn ends_with_sep(path: &str) -> bool {
    matches!(path.as_bytes().last().unwrap_or(&0), b'/' | b'\\')
}

#[cfg(not(windows))]
fn ends_with_sep(path: &str) -> bool {
    path.ends_with(MAIN_SEPARATOR)
}

#[cfg(windows)]
pub fn is_absolute(path: &str) -> bool {
    starts_with_sep(path) || PathBuf::from(path).is_absolute()
}

#[cfg(not(windows))]
pub fn is_absolute(path: &str) -> bool {
    path.starts_with(MAIN_SEPARATOR)
}

impl ModuleDef for PathModule {
    fn declare(declare: &Declarations) -> Result<()> {
        declare.declare("basename")?;
        declare.declare("dirname")?;
        declare.declare("extname")?;
        declare.declare("format")?;
        declare.declare("parse")?;
        declare.declare("join")?;
        declare.declare("resolve")?;
        declare.declare("relative")?;
        declare.declare("normalize")?;
        declare.declare("isAbsolute")?;
        declare.declare("delimiter")?;
        declare.declare("sep")?;

        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("dirname", Func::from(dirname))?;
            default.set("basename", Func::from(basename))?;
            default.set("extname", Func::from(extname))?;
            default.set("format", Func::from(format))?;
            default.set("parse", Func::from(parse))?;
            default.set("join", Func::from(join))?;
            default.set("relative", Func::from(relative))?;
            default.set("resolve", Func::from(resolve))?;
            default.set("normalize", Func::from(normalize))?;
            default.set("isAbsolute", Func::from(|s: String| is_absolute(&s)))?;
            default.prop("delimiter", DELIMITER.to_string())?;
            default.prop("sep", MAIN_SEPARATOR.to_string())?;
            Ok(())
        })
    }
}

impl From<PathModule> for ModuleInfo<PathModule> {
    fn from(val: PathModule) -> Self {
        ModuleInfo {
            name: "path",
            module: val,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env::{current_dir, set_current_dir},
        sync::Mutex,
    };

    static THREAD_LOCK: Lazy<Mutex<()>> = Lazy::new(Mutex::default);

    use once_cell::sync::Lazy;

    use super::*;

    #[test]
    fn test_relative() {
        let _shared = THREAD_LOCK.lock().unwrap();
        let cwd = current_dir().expect("Unable to access working directory");
        set_current_dir("/").expect("unable to set working directory to /");

        assert_eq!(
            relative_path("a/b/c", "b/c"),
            "../../../b/c".replace('/', MAIN_SEPARATOR_STR)
        );
        assert_eq!(
            relative_path("/data/orandea/test/aaa", "/data/orandea/impl/bbb"),
            "../../impl/bbb".replace('/', MAIN_SEPARATOR_STR)
        );
        assert_eq!(
            relative_path("/a/b/c", "/a/d"),
            "../../d".replace('/', MAIN_SEPARATOR_STR)
        );
        assert_eq!(relative_path("/a/b/c", "/a/b/c/d"), "d");
        assert_eq!(relative_path("/a/b/c", "/a/b/c"), "");

        assert_eq!(
            relative_path("a/b", "a/b/c/d"),
            "c/d".replace('/', MAIN_SEPARATOR_STR)
        );
        assert_eq!(
            relative_path("a/b/c", "b/c"),
            "../../../b/c".replace('/', MAIN_SEPARATOR_STR)
        );

        set_current_dir(cwd).expect("unable to set working directory back");
    }

    #[test]
    fn test_dirname() {
        assert_eq!(dirname("/usr/local/bin".to_string()), "/usr/local");
        assert_eq!(dirname("/usr/local/".to_string()), "/usr");
        assert_eq!(dirname("usr/local/bin".to_string()), "usr/local");
        assert_eq!(dirname("/".to_string()), "/");
        assert_eq!(dirname("".to_string()), ".");
    }

    #[test]
    fn test_basename() {
        assert_eq!(basename("/usr/local/bin".to_string(), Opt(None)), "bin");
        assert_eq!(
            basename("/usr/local/bin.txt".to_string(), Opt(None)),
            "bin.txt"
        );
        assert_eq!(
            basename(
                "/usr/local/bin.txt".to_string(),
                Opt(Some(".txt".to_string()))
            ),
            "bin"
        );
        assert_eq!(basename("".to_string(), Opt(None)), "");
        assert_eq!(basename("/".to_string(), Opt(None)), "");
    }

    #[test]
    fn test_extname() {
        assert_eq!(extname("/usr/local/bin.txt".to_string()), ".txt");
        assert_eq!(extname("/usr/local/bin".to_string()), "");
        assert_eq!(extname("file.tar.gz".to_string()), ".gz");
        assert_eq!(extname(".bashrc".to_string()), "");
        assert_eq!(extname("".to_string()), "");
    }

    #[test]
    fn test_join() {
        // Standard cases
        assert_eq!(
            join_path(["/usr", "local", "bin"].iter()),
            "/usr/local/bin".replace('/', MAIN_SEPARATOR_STR)
        );
        assert_eq!(
            join_path(["/usr", "/local", "bin"].iter()),
            "/usr/local/bin".replace('/', MAIN_SEPARATOR_STR)
        );
        assert_eq!(
            join_path(["usr", "local", "bin"].iter()),
            "usr/local/bin".replace('/', MAIN_SEPARATOR_STR)
        );
        assert_eq!(join_path(["", "bin"].iter()), "bin");

        // Complex cases
        assert_eq!(
            join_path(["/usr", "..", "local", "bin"].iter()),
            "/local/bin".replace('/', MAIN_SEPARATOR_STR)
        ); // Parent dir
        assert_eq!(
            join_path([".", "usr", "local"]),
            "usr/local".replace('/', MAIN_SEPARATOR_STR)
        ); // Current dir
        assert_eq!(
            join_path(["/usr", ".", "bin"].iter()),
            "/usr/bin".replace('/', MAIN_SEPARATOR_STR)
        ); // Current dir in middle
        assert_eq!(
            join_path(["usr", "local", "bin", ".."].iter()),
            "usr/local".replace('/', MAIN_SEPARATOR_STR)
        ); // Ending with parent dir
        assert_eq!(
            join_path(["/usr", "local", "", "bin"].iter()),
            "/usr/local/bin".replace('/', MAIN_SEPARATOR_STR)
        ); // Empty component in path
        assert_eq!(
            join_path(["/usr", "local", ".hidden"].iter()),
            "/usr/local/.hidden".replace('/', MAIN_SEPARATOR_STR)
        ); // Hidden file
    }

    #[test]
    fn test_resolve_path() {
        let _shared = THREAD_LOCK.lock().unwrap();
        let prefix = if cfg!(windows) {
            if let Some(Component::Prefix(prefix)) =
                std::env::current_dir().unwrap().components().next()
            {
                prefix.as_os_str().to_str().unwrap().to_string()
            } else {
                "".into()
            }
        } else {
            "".into()
        };

        assert_eq!(
            resolve_path(["", "foo/bar"].iter()),
            std::env::current_dir()
                .unwrap()
                .join("foo/bar".replace('/', MAIN_SEPARATOR_STR))
                .to_string_lossy()
                .to_string()
        );

        // Standard cases
        assert_eq!(
            resolve_path(["/"].iter()),
            prefix.clone() + MAIN_SEPARATOR_STR
        );

        // Standard cases
        assert_eq!(
            resolve_path(["/foo/bar", "../baz"].iter()),
            prefix.clone() + &"/foo/baz".replace('/', MAIN_SEPARATOR_STR)
        );
        assert_eq!(
            resolve_path(["/foo/bar", "./baz"].iter()),
            prefix.clone() + &"/foo/bar/baz".replace('/', MAIN_SEPARATOR_STR)
        );
        assert_eq!(
            resolve_path(["foo/bar", "/baz"].iter()),
            prefix.clone() + &"/baz".replace('/', MAIN_SEPARATOR_STR)
        );

        // Complex cases
        assert_eq!(
            resolve_path(["/foo", "bar", ".", "baz"].iter()),
            prefix.clone() + &"/foo/bar/baz".replace('/', MAIN_SEPARATOR_STR)
        ); // Current dir in middle
        assert_eq!(
            resolve_path(["/foo", "bar", "..", "baz"].iter()),
            prefix.clone() + &"/foo/baz".replace('/', MAIN_SEPARATOR_STR)
        ); // Parent dir in middle
        assert_eq!(
            resolve_path(["/foo", "bar", "../..", "baz"].iter()),
            prefix.clone() + &"/baz".replace('/', MAIN_SEPARATOR_STR)
        ); // Double parent dir
        assert_eq!(
            resolve_path(["/foo", "bar", ".hidden"].iter()),
            prefix.clone() + &"/foo/bar/.hidden".replace('/', MAIN_SEPARATOR_STR)
        ); // Hidden file
        assert_eq!(
            resolve_path(["/foo", ".", "bar", "."].iter()),
            prefix.clone() + &"/foo/bar".replace('/', MAIN_SEPARATOR_STR)
        ); // Multiple current dirs
        assert_eq!(
            resolve_path(["/foo", "..", "..", "bar"].iter()),
            prefix.clone() + &"/bar".replace('/', MAIN_SEPARATOR_STR)
        ); // Multiple parent dirs
        assert_eq!(
            resolve_path(["/foo/bar", "/..", "baz"].iter()),
            prefix.clone() + &"/baz".replace('/', MAIN_SEPARATOR_STR)
        ); // Parent dir with absolute path
    }

    #[test]
    fn test_normalize() {
        assert_eq!(
            normalize("/foo//bar//baz".to_string()),
            "/foo/bar/baz".replace('/', MAIN_SEPARATOR_STR)
        );
        assert_eq!(
            normalize("/foo/./bar/../baz".to_string()),
            "/foo/baz".replace('/', MAIN_SEPARATOR_STR)
        );
        assert_eq!(
            normalize("foo/bar/".to_string()),
            "foo/bar".replace('/', MAIN_SEPARATOR_STR)
        );
        assert_eq!(normalize("./foo".to_string()), "foo");
    }

    #[test]
    fn test_is_absolute() {
        assert!(is_absolute("/usr/local/bin"));
        assert!(!is_absolute("usr/local/bin"));
        #[cfg(windows)]
        assert!(is_absolute("C:\\Program Files")); // for Windows systems
        assert!(!is_absolute("./local/bin"));
    }
}
