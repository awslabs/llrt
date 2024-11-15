use criterion::{criterion_group, criterion_main, Criterion};
use llrt_modules::path::replace_backslash;
use rand::seq::SliceRandom;
use rand::Rng;
use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 10;
const PATH_STYLE_PROB: f64 = 0.5;

const WORD_LIST: &[&str] = &[
    "home",
    "user",
    "admin",
    "documents",
    "downloads",
    "music",
    "videos",
    "projects",
    "notes",
    "desktop",
    "workspace",
    "archive",
    "backup",
    "code",
    "scripts",
    "logs",
    "build",
    "bin",
    "src",
    "include",
    "lib",
    "temp",
    "cache",
    "system",
    "private",
    "shared",
    "public",
    "common",
    "vacation",
    "pictures",
    "gallery",
    "photos",
    "library",
    "data",
    "storage",
    "cloud",
    "tasks",
    "files",
    "records",
    "history",
    "samples",
    "assets",
    "media",
    "config",
    "setup",
    "exports",
    "imports",
    "local",
    "global",
    "network",
    "remote",
    "main",
    "backup",
    "security",
    "mainframe",
    "tools",
    "resources",
    "info",
    "settings",
    "profile",
    "account",
    "group",
    "modules",
    "scripts",
    "test",
    "dist",
    "coverage",
    "docs",
    "models",
    "services",
    "components",
    "assets",
    "functions",
    "tests",
    "data",
    "results",
    "index",
    "source",
    "runtime",
    "example",
    "template",
    "styles",
    "layout",
    "config",
    "docs",
    "dependencies",
    "log",
    "controller",
    "service",
    "client",
    "server",
    "draft",
    "final",
    "old",
    "new",
    "review",
    "complete",
    "inprogress",
    "template",
    "empty",
    "readme",
    "license",
    "notes",
    "reference",
    "guide",
    "outline",
    "summary",
];

fn generate_random_path() -> String {
    let mut rng = rand::thread_rng();
    let mut path = PathBuf::new();
    let depth = rng.gen_range(1..=MAX_DEPTH);

    for _ in 0..depth {
        let name = WORD_LIST.choose(&mut rng).unwrap();
        if rng.gen_bool(PATH_STYLE_PROB) {
            path.push(format!("{}\\", name));
        } else {
            path.push(name);
        }
    }
    path.to_string_lossy().to_string()
}

fn replace_with_string_replace(path: String) -> String {
    path.replace('\\', "/")
}

fn benchmark(c: &mut Criterion) {
    c.bench_function("String Replace", |b| {
        b.iter(|| {
            let path = generate_random_path();
            replace_with_string_replace(path);
        })
    });

    c.bench_function("Memchr Replace", |b| {
        b.iter(|| {
            let path = generate_random_path();
            replace_backslash(path);
        })
    });

    c.bench_function("File slash replace", |b| {
        b.iter(|| {
            let path = generate_random_path();
            replace_filename(path);
        })
    });

    c.bench_function("File components replace", |b| {
        b.iter(|| {
            let path = generate_random_path();
            replace_components(path);
        })
    });
}

fn replace_components(path: String) -> String {
    let length = path.len();
    let path = Path::new(&path);
    let mut components = path.components();
    let mut new_path = String::with_capacity(length);

    for component in components {
        new_path.push_str(&component.as_os_str().to_string_lossy());
        new_path.push('/');
    }
    new_path.truncate(length);

    new_path
}

fn replace_filename(path: String) -> String {
    Path::new(&path)
        .to_string_lossy()
        .to_string()
        .replace('\\', "/")
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
