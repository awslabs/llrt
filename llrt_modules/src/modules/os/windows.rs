use once_cell::sync::Lazy;
use windows_registry::{Value, LOCAL_MACHINE};
use windows_result::{Error, Result};
use windows_version::OsVersion;

static OS_RELEASE: Lazy<String> = Lazy::new(release);
static OS_VERSION: Lazy<String> = Lazy::new(|| version().unwrap_or_default());

pub fn get_type() -> &'static str {
    // In theory there are more types linx MinGW but in practice this is good enough
    "Windows_NT"
}

pub fn get_release() -> &'static str {
    &OS_RELEASE
}

pub fn get_version() -> &'static str {
    &OS_VERSION
}

fn release() -> String {
    let version = OsVersion::current();
    format!("{}.{}.{}", version.major, version.minor, version.build)
}

fn version() -> Result<String> {
    let version = OsVersion::current();

    let registry_key = LOCAL_MACHINE.open("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")?;
    let value = registry_key.get_value("ProductName")?;
    let Value::String(mut value) = value else {
        return Err(Error::empty());
    };

    // Windows 11 shares dwMajorVersion with Windows 10
    // this workaround tries to disambiguate that by checking
    // if the dwBuildNumber is from Windows 11 releases (>= 22000).
    if version.major == 10 && version.build >= 22000 && value.starts_with("Windows 10") {
        value.replace_range(9..10, "1");
    }

    Ok(value)
}
