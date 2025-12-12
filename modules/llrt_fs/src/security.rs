// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::path::Path;
use std::sync::OnceLock;

use rquickjs::{Ctx, Exception, Result};

static FS_ALLOW_LIST: OnceLock<Vec<String>> = OnceLock::new();

static FS_DENY_LIST: OnceLock<Vec<String>> = OnceLock::new();

pub fn set_allow_list(values: Vec<String>) {
    _ = FS_ALLOW_LIST.set(values);
}

pub fn get_allow_list() -> Option<&'static Vec<String>> {
    FS_ALLOW_LIST.get()
}

pub fn set_deny_list(values: Vec<String>) {
    _ = FS_DENY_LIST.set(values);
}

pub fn get_deny_list() -> Option<&'static Vec<String>> {
    FS_DENY_LIST.get()
}

/// Check if a path matches a pattern.
/// Patterns can be:
/// - Exact paths: "/tmp/file.txt"
/// - Directory prefixes: "/tmp/" (matches all files under /tmp)
/// - Glob-like patterns: "/tmp/*.txt" (matches *.txt in /tmp)
fn path_matches(path: &str, pattern: &str) -> bool {
    // Normalize the path for comparison
    let path = Path::new(path);
    let pattern = pattern.trim();

    // Handle directory prefix patterns (ending with /)
    if let Some(prefix) = pattern.strip_suffix('/') {
        return path.starts_with(prefix);
    }

    // Handle glob patterns with *
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let path_str = path.to_string_lossy();
            let prefix = parts[0];
            let suffix = parts[1];

            // For patterns like "/tmp/*.txt", check if path starts with prefix dir
            // and the filename ends with the suffix
            if !prefix.is_empty() && !suffix.is_empty() {
                // Get the directory part of the pattern
                if let Some(pattern_dir) = Path::new(prefix).parent() {
                    if let Some(path_parent) = path.parent() {
                        // Check if the path is in the same directory as the pattern
                        if path_parent == pattern_dir || path_parent.starts_with(pattern_dir) {
                            return path_str.ends_with(suffix);
                        }
                    }
                }
                // Fallback: simple prefix/suffix matching
                return path_str.starts_with(prefix) && path_str.ends_with(suffix);
            }

            // For patterns like "*.txt" or "/tmp/*"
            return path_str.starts_with(prefix) && path_str.ends_with(suffix);
        }
    }

    // Exact match
    path == Path::new(pattern)
}

/// Check if access to a path is allowed based on allow/deny lists.
/// The allow list is checked first - if set, the path must match at least one pattern.
/// Then the deny list is checked - if set and the path matches, access is denied.
pub fn ensure_access(ctx: &Ctx<'_>, path: &str) -> Result<()> {
    // If allow list is set, path must match at least one pattern
    if let Some(allow_list) = FS_ALLOW_LIST.get() {
        let allowed = allow_list.iter().any(|pattern| path_matches(path, pattern));
        if !allowed {
            return Err(Exception::throw_message(
                ctx,
                &["Filesystem path not allowed: ", path].concat(),
            ));
        }
    }

    // If deny list is set, path must not match any pattern
    if let Some(deny_list) = FS_DENY_LIST.get() {
        let denied = deny_list.iter().any(|pattern| path_matches(path, pattern));
        if denied {
            return Err(Exception::throw_message(
                ctx,
                &["Filesystem path denied: ", path].concat(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_matches_exact() {
        assert!(path_matches("/tmp/file.txt", "/tmp/file.txt"));
        assert!(!path_matches("/tmp/file.txt", "/tmp/other.txt"));
    }

    #[test]
    fn test_path_matches_directory_prefix() {
        assert!(path_matches("/tmp/file.txt", "/tmp/"));
        assert!(path_matches("/tmp/subdir/file.txt", "/tmp/"));
        assert!(!path_matches("/var/file.txt", "/tmp/"));
    }

    #[test]
    fn test_path_matches_glob() {
        assert!(path_matches("/tmp/file.txt", "/tmp/*.txt"));
        assert!(path_matches("/tmp/other.txt", "/tmp/*.txt"));
        assert!(!path_matches("/tmp/file.json", "/tmp/*.txt"));
        assert!(path_matches("/tmp/file.txt", "*.txt"));
        assert!(path_matches("/tmp/subdir/test", "/tmp/*"));
    }
}
