#[cfg(target_os = "macos")]
pub const PLATFORM: &str = "darwin";
#[cfg(target_os = "windows")]
pub const PLATFORM: &str = "win32";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub const PLATFORM: &str = std::env::consts::OS;

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub const ARCH: &str = "x64";
#[cfg(target_arch = "aarch64")]
pub const ARCH: &str = "arm64";
#[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
pub const ARCH: &str = std::env::consts::ARCH;
