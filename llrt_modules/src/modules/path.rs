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
    s.find(std::path::MAIN_SEPARATOR)
}

pub const FORWARD_SLASH: char = '/';
pub const FORWARD_SLASH_STR: &str = "/";

// Constants kept for potential use elsewhere
#[cfg(windows)]
const SEP_PAT: [char; 2] = ['\\', FORWARD_SLASH];
#[cfg(not(windows))]
const SEP_PAT: [char; 1] = [std::path::MAIN_SEPARATOR];

pub fn dirname(path: String) -> String {
    if path.is_empty() {
        return String::from(".");
    }
    if path == MAIN_SEPARATOR_STR {
        return path;
    }
    let path = path.strip_suffix(SEP_PAT).unwrap_or(&path);
    match path.rfind(SEP_PAT) {
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
    let path = path.strip_suffix(SEP_PAT).unwrap_or(path);
    let path = match path.rfind(SEP_PAT) {
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

fn basename(path: String, suffix: Opt<String>) -> String {
    if path.is_empty() || path == MAIN_SEPARATOR_STR {
        return String::from("");
    }

    let (base, ext) = name_extname(&path);
    let name = [base, ext].concat();
    if let Some(suffix) = suffix.0 {
        name.strip_suffix(&suffix).unwrap_or(&name)
    } else {
        &name
    }
    .to_string()
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
        if !dir.ends_with(SEP_PAT) {
            path.push(MAIN_SEPARATOR);
        }
    } else if !root.is_empty() {
        path.push_str(&root);
        if !root.ends_with(SEP_PAT) {
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
    I: Iterator<Item = S>,
{
    join_resolve_path(parts, false)
}

pub fn resolve_path<S, I>(parts: I) -> String
where
    S: AsRef<str>,
    I: Iterator<Item = S>,
{
    join_resolve_path(parts, true)
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
                + FORWARD_SLASH_STR
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
                + FORWARD_SLASH_STR
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
            relative.push(FORWARD_SLASH);
        }
        relative.push_str("..");
        from_index = from_next + 1; // Move past the separator
    }
    // add the remaining components from 'to'
    while to_index < to_ref.len() {
        let to_next =
            find_next_separator(&to_ref[to_index..]).unwrap_or(to_ref.len() - to_index) + to_index;
        if !relative.is_empty() {
            relative.push(FORWARD_SLASH);
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

fn join_resolve_path<S, I>(parts: I, resolve: bool) -> String
where
    S: AsRef<str>,
    I: Iterator<Item = S>,
{
    let mut empty = true;
    let size = parts.size_hint().1.unwrap_or_default();

    let mut result = if resolve {
        let cwd = std::env::current_dir().expect("Unable to access working directory");

        let mut result = to_slash_lossy(cwd);
        if !result.ends_with(FORWARD_SLASH) {
            result.push(FORWARD_SLASH);
        }
        result
    } else {
        String::with_capacity(size * 4)
    };

    let mut resolve_cow: Cow<str>;
    let mut resolve_path_buf: PathBuf;

    let mut index_stack = Vec::with_capacity(16);

    for part in parts {
        let mut part_ref: &str = part.as_ref();
        let mut start = 0;
        if resolve {
            if cfg!(not(windows)) {
                if part_ref.starts_with(FORWARD_SLASH) {
                    empty = false;
                    result = FORWARD_SLASH.into();
                    start = 1;
                }
            } else {
                let path_buf = PathBuf::from(part_ref);
                if path_buf.is_absolute() {
                    empty = false;
                    start = 1;

                    let mut components = path_buf.components().peekable();
                    result = if let Some(Component::Prefix(a)) = components.next() {
                        a.as_os_str().to_str().unwrap().to_string()
                    } else {
                        FORWARD_SLASH.into()
                    };
                    resolve_path_buf = components.collect();
                    resolve_cow = resolve_path_buf.to_string_lossy();
                    part_ref = resolve_cow.as_ref();
                }
            }
        } else if part_ref.starts_with(SEP_PAT) && empty {
            empty = false;
            result.push(FORWARD_SLASH);
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
                    result.push(FORWARD_SLASH);
                    empty = false;
                    #[cfg(windows)]
                    if sub_part.ends_with(":") {
                        result.push(FORWARD_SLASH);
                    }
                    index_stack.push(len);
                },
            }
            start = end + 1;
        }
    }

    if result.len() > 1 && result.ends_with(FORWARD_SLASH) {
        result.truncate(result.len() - 1);
    }

    result
}

pub fn resolve(path: Rest<String>) -> String {
    join_resolve_path(path.iter(), true)
}

fn normalize(path: String) -> String {
    join_resolve_path([path].iter(), false)
}

#[allow(dead_code)] //used by windows
fn starts_with_sep(path: &str) -> bool {
    matches!(path.as_bytes().first().unwrap_or(&0), b'/' | b'\\')
}

#[cfg(windows)]
pub fn is_absolute(path: &str) -> bool {
    starts_with_sep(path) || PathBuf::from(path).is_absolute()
}

#[cfg(not(windows))]
pub fn is_absolute(path: &str) -> bool {
    path.starts_with(std::path::MAIN_SEPARATOR)
}

#[cfg(windows)]
pub fn to_slash_lossy(path: PathBuf) -> String {
    use crate::path::FORWARD_SLASH;
    use std::path::Component;
    let capacity = path.as_os_str().len();
    let mut buf = String::with_capacity(capacity);
    for c in path.components() {
        match c {
            Component::Prefix(prefix) => {
                buf.push_str(&prefix.as_os_str().to_string_lossy());
                continue;
            },
            Component::Normal(s) => buf.push_str(&s.to_string_lossy()),
            _ => {},
        }
        buf.push(FORWARD_SLASH);
    }
    buf
}

#[cfg(not(windows))]
pub fn to_slash_lossy(path: PathBuf) -> String {
    path.to_string_lossy().to_string()
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
    use std::env::{current_dir, set_current_dir};

    use super::*;

    #[test]
    fn test_relative() {
        let cwd = current_dir().expect("Unable to access working directory");
        set_current_dir("/").expect("unable to set working directory to /");

        assert_eq!(relative_path("a/b/c", "b/c"), "../../../b/c");
        assert_eq!(
            relative_path("/data/orandea/test/aaa", "/data/orandea/impl/bbb"),
            "../../impl/bbb"
        );
        assert_eq!(relative_path("/a/b/c", "/a/d"), "../../d");
        assert_eq!(relative_path("/a/b/c", "/a/b/c/d"), "d");
        assert_eq!(relative_path("/a/b/c", "/a/b/c"), "");

        assert_eq!(relative_path("a/b", "a/b/c/d"), "c/d");
        assert_eq!(relative_path("a/b/c", "b/c"), "../../../b/c");

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
        assert_eq!(join_path(["/usr", "local", "bin"].iter()), "/usr/local/bin");
        assert_eq!(
            join_path(["/usr", "/local", "bin"].iter()),
            "/usr/local/bin"
        );
        assert_eq!(join_path(["usr", "local", "bin"].iter()), "usr/local/bin");
        assert_eq!(join_path(["", "bin"].iter()), "bin");

        // Complex cases
        assert_eq!(
            join_path(["/usr", "..", "local", "bin"].iter()),
            "/local/bin"
        ); // Parent dir
        assert_eq!(join_path([".", "usr", "local"].iter()), "usr/local"); // Current dir
        assert_eq!(join_path(["/usr", ".", "bin"].iter()), "/usr/bin"); // Current dir in middle
        assert_eq!(join_path(["usr", "local", "bin", ".."].iter()), "usr/local"); // Ending with parent dir
        assert_eq!(
            join_path(["/usr", "local", "", "bin"].iter()),
            "/usr/local/bin"
        ); // Empty component in path
        assert_eq!(
            join_path(["/usr", "local", ".hidden"].iter()),
            "/usr/local/.hidden"
        ); // Hidden file
    }

    #[test]
    fn test_resolve_path() {
        assert_eq!(resolve_path(["/"].iter()), "/");

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
                .join("foo/bar")
                .to_string_lossy()
                .to_string()
                .replace("\\", "/")
        );

        // Standard cases
        assert_eq!(resolve_path(["/"].iter()), prefix.clone() + "/");

        // Standard cases
        assert_eq!(
            resolve_path(["/foo/bar", "../baz"].iter()),
            prefix.clone() + "/foo/baz"
        );
        assert_eq!(
            resolve_path(["/foo/bar", "./baz"].iter()),
            prefix.clone() + "/foo/bar/baz"
        );
        assert_eq!(
            resolve_path(["foo/bar", "/baz"].iter()),
            prefix.clone() + "/baz"
        );

        // Complex cases
        assert_eq!(
            resolve_path(["/foo", "bar", ".", "baz"].iter()),
            prefix.clone() + "/foo/bar/baz"
        ); // Current dir in middle
        assert_eq!(
            resolve_path(["/foo", "bar", "..", "baz"].iter()),
            prefix.clone() + "/foo/baz"
        ); // Parent dir in middle
        assert_eq!(resolve_path(["/foo", "bar", "../..", "baz"].iter()), "/baz"); // Double parent dir
        assert_eq!(
            resolve_path(["/foo", "bar", ".hidden"].iter()),
            prefix.clone() + "/foo/bar/.hidden"
        ); // Hidden file
        assert_eq!(
            resolve_path(["/foo", ".", "bar", "."].iter()),
            prefix.clone() + "/foo/bar"
        ); // Multiple current dirs
        assert_eq!(
            resolve_path(["/foo", "..", "..", "bar"].iter()),
            prefix.clone() + "/bar"
        ); // Multiple parent dirs
        assert_eq!(
            resolve_path(["/foo/bar", "/..", "baz"].iter()),
            prefix.clone() + "/baz"
        ); // Parent dir with absolute path
    }

    #[test]
    fn test_normalize() {
        assert_eq!(normalize("/foo//bar//baz".to_string()), "/foo/bar/baz");
        assert_eq!(normalize("/foo/./bar/../baz".to_string()), "/foo/baz");
        assert_eq!(normalize("foo/bar/".to_string()), "foo/bar");
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
