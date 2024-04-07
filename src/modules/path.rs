// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    path::{Component, Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR},
    slice::Iter,
};

use rquickjs::{
    function::Opt,
    module::{Declarations, Exports, ModuleDef},
    prelude::{Func, Rest},
    Ctx, Object, Result,
};

use crate::modules::module::export_default;

pub struct PathModule;

#[cfg(windows)]
const DELIMITER: char = ';';
#[cfg(not(windows))]
const DELIMITER: char = ':';

#[cfg(windows)]
pub const CURRENT_DIR_STR: &'static str = ".\\";
#[cfg(not(windows))]
pub const CURRENT_DIR_STR: &str = "./";

pub fn dirname(path: String) -> String {
    if path == MAIN_SEPARATOR_STR {
        return path;
    }
    let path = path.strip_suffix(MAIN_SEPARATOR).unwrap_or(&path);
    match path.rfind(MAIN_SEPARATOR) {
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
    let path = path.strip_suffix(MAIN_SEPARATOR).unwrap_or(path);
    let path = match path.rfind(MAIN_SEPARATOR) {
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
    if path == MAIN_SEPARATOR_STR {
        return path;
    }
    if path.is_empty() {
        return String::from(".");
    }
    let (base, ext) = name_extname(&path);
    let name = format!("{}{}", base, ext);
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
        if !dir.ends_with(MAIN_SEPARATOR) {
            path.push(MAIN_SEPARATOR);
        }
    } else if !root.is_empty() {
        path.push_str(&root);
        if !root.ends_with(MAIN_SEPARATOR) {
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
    obj.set("base", format!("{}{}", name, extension))?;
    obj.set("ext", extension)?;
    obj.set("name", name)?;

    Ok(obj)
}

fn join(parts: Rest<String>) -> String {
    join_path(parts.0)
}

pub fn join_path(parts: Vec<String>) -> String {
    let mut result = PathBuf::new();
    let mut empty = true;
    for part in parts.iter() {
        if part.starts_with(MAIN_SEPARATOR) && empty {
            result.push(MAIN_SEPARATOR_STR);
            empty = false;
        }
        for sub_part in part.split(MAIN_SEPARATOR) {
            if !sub_part.is_empty() {
                if sub_part.starts_with("..") {
                    empty = false;
                    result.pop();
                } else {
                    result.push(sub_part.strip_prefix('.').unwrap_or(sub_part));
                    empty = false;
                }
            }
        }
    }
    remove_trailing_slash(result)
}

fn remove_trailing_slash(result: PathBuf) -> String {
    let path = result.to_string_lossy().to_string();
    path.strip_suffix(MAIN_SEPARATOR)
        .unwrap_or(&path)
        .to_string()
}

fn resolve(path: Rest<String>) -> String {
    resolve_path(path.iter())
}

pub fn resolve_path(iter: Iter<'_, String>) -> String {
    let mut dir = std::env::current_dir().unwrap();
    for part in iter {
        let p = part.strip_prefix(CURRENT_DIR_STR).unwrap_or(part);
        if p.starts_with(MAIN_SEPARATOR) {
            dir = PathBuf::from(p);
        } else {
            for sub_part in p.split(MAIN_SEPARATOR) {
                if sub_part.starts_with("..") {
                    dir.pop();
                } else {
                    dir.push(sub_part.strip_prefix('.').unwrap_or(sub_part))
                }
            }
        }
    }

    remove_trailing_slash(dir)
}

fn normalize(path: String) -> String {
    let path = PathBuf::from(path);
    let parts = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>();

    join_path(parts)
}

pub fn is_absolute(path: String) -> bool {
    PathBuf::from(path).is_absolute()
}

impl ModuleDef for PathModule {
    fn declare(declare: &mut Declarations) -> Result<()> {
        declare.declare("basename")?;
        declare.declare("dirname")?;
        declare.declare("extname")?;
        declare.declare("format")?;
        declare.declare("parse")?;
        declare.declare("join")?;
        declare.declare("resolve")?;
        declare.declare("normalize")?;
        declare.declare("isAbsolute")?;
        declare.declare("delimiter")?;

        declare.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &mut Exports<'js>) -> Result<()> {
        export_default(ctx, exports, |default| {
            default.set("dirname", Func::from(dirname))?;
            default.set("basename", Func::from(basename))?;
            default.set("extname", Func::from(extname))?;
            default.set("format", Func::from(format))?;
            default.set("parse", Func::from(parse))?;
            default.set("join", Func::from(join))?;
            default.set("resolve", Func::from(resolve))?;
            default.set("normalize", Func::from(normalize))?;
            default.set("isAbsolute", Func::from(is_absolute))?;
            default.prop("delimiter", DELIMITER.to_string())?;
            Ok(())
        })
    }
}
