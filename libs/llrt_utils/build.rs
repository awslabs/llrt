// WARN: Do not modify this function without changing the other build.rs files in the other crates.
fn set_nightly_cfg() {
    let version_meta = rustc_version::version_meta().unwrap();
    println!("cargo::rustc-check-cfg=cfg(rust_nightly)");
    if version_meta.channel == rustc_version::Channel::Nightly {
        println!("cargo:rustc-cfg=rust_nightly");
    }
}

fn main() {
    set_nightly_cfg();
}
