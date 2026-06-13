//! Detection of optional external command-line tools.
//!
//! Some capabilities (notably video / animated-image compression) rely on
//! tools that ship outside the binary. The compression that exists today is
//! done in-process with Rust crates, so these tools are not required to run
//! the app — detection lets the UI show what is present and gates features
//! that will build on them (e.g. ffmpeg-based video compression).
//!
//! "Detection" here means a plain PATH lookup, matching the user-facing
//! promise of "just check whether it is on PATH", plus a best-effort version
//! query when the executable is found.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Status of an external command-line tool the app can make use of.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolStatus {
    /// Executable name as looked up on PATH (e.g. "ffmpeg")
    pub name: String,
    /// Whether the executable was found on PATH
    pub available: bool,
    /// Resolved absolute path, when found
    pub path: Option<String>,
    /// First line of the tool's version output, when it could be queried
    pub version: Option<String>,
    /// What the tool unlocks in Space-Saver
    pub purpose: String,
}

/// The tools we probe for, paired with the capability each enables.
fn known_tools() -> Vec<(&'static str, &'static str)> {
    vec![
        ("ffmpeg", "Video and animated-image compression"),
        ("ffprobe", "Inspecting video/audio streams for compression"),
        (
            "cwebp",
            "Standalone WebP encoding (the built-in encoder is used by default)",
        ),
    ]
}

/// Find an executable by name on the PATH, returning its absolute path.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let dirs: Vec<PathBuf> = std::env::split_paths(&path_var).collect();
    find_executable_in(&dirs, name)
}

/// Find an executable by name within an explicit list of directories. Split
/// out from [`find_executable`] so it can be tested without mutating the
/// process-wide PATH (which would race across parallel tests).
fn find_executable_in(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    let candidates = executable_candidates(name);
    for dir in dirs {
        for candidate in &candidates {
            let full = dir.join(candidate);
            if is_executable_file(&full) {
                return Some(full);
            }
        }
    }
    None
}

#[cfg(windows)]
fn executable_candidates(name: &str) -> Vec<String> {
    // Trust an explicit extension; otherwise try the common Windows ones.
    if Path::new(name).extension().is_some() {
        vec![name.to_string()]
    } else {
        vec![
            format!("{name}.exe"),
            format!("{name}.bat"),
            format!("{name}.cmd"),
            name.to_string(),
        ]
    }
}

#[cfg(not(windows))]
fn executable_candidates(name: &str) -> Vec<String> {
    vec![name.to_string()]
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(m) => m.is_file() && m.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

/// Best-effort version string: run the tool and return the first non-empty
/// line of its output. ffmpeg-family tools use `-version`; many others use
/// `--version`, so fall back to that. Any failure yields `None`.
fn query_version(path: &Path) -> Option<String> {
    for arg in ["-version", "--version"] {
        if let Ok(output) = Command::new(path).arg(arg).output() {
            let text = if !output.stdout.is_empty() {
                String::from_utf8_lossy(&output.stdout)
            } else {
                String::from_utf8_lossy(&output.stderr)
            };
            if let Some(line) = text.lines().map(str::trim).find(|l| !l.is_empty()) {
                return Some(line.to_string());
            }
        }
    }
    None
}

/// Probe every known tool and report its status. Never fails: a tool that is
/// missing is simply reported as unavailable.
pub fn detect_tools() -> Vec<ToolStatus> {
    known_tools()
        .into_iter()
        .map(|(name, purpose)| match find_executable(name) {
            Some(p) => ToolStatus {
                name: name.to_string(),
                available: true,
                version: query_version(&p),
                path: Some(p.display().to_string()),
                purpose: purpose.to_string(),
            },
            None => ToolStatus {
                name: name.to_string(),
                available: false,
                path: None,
                version: None,
                purpose: purpose.to_string(),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    /// Create a file that counts as an executable on the current platform.
    fn make_exe(dir: &Path, name: &str) -> PathBuf {
        #[cfg(windows)]
        let file = dir.join(format!("{name}.exe"));
        #[cfg(not(windows))]
        let file = dir.join(name);

        std::fs::write(&file, b"").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&file).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&file, perms).unwrap();
        }

        file
    }

    #[test]
    fn finds_executable_on_path() {
        let dir = tempdir().unwrap();
        let exe = make_exe(dir.path(), "mytool");
        let found = find_executable_in(&[dir.path().to_path_buf()], "mytool");
        assert_eq!(found, Some(exe));
    }

    #[test]
    fn missing_executable_returns_none() {
        let dir = tempdir().unwrap();
        assert_eq!(
            find_executable_in(&[dir.path().to_path_buf()], "definitely-not-on-path"),
            None
        );
    }

    #[test]
    fn empty_path_list_returns_none() {
        assert_eq!(find_executable_in(&[], "ffmpeg"), None);
    }

    #[cfg(unix)]
    #[test]
    fn non_executable_file_is_not_matched() {
        // A plain file with no execute bit must not count as the tool.
        let dir = tempdir().unwrap();
        let file = dir.path().join("plainfile");
        std::fs::write(&file, b"").unwrap();
        assert_eq!(
            find_executable_in(&[dir.path().to_path_buf()], "plainfile"),
            None
        );
    }

    #[test]
    fn detect_tools_reports_all_known_tools() {
        let tools = detect_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"ffmpeg"));
        assert!(names.contains(&"ffprobe"));
        assert!(names.contains(&"cwebp"));

        for tool in &tools {
            assert!(!tool.purpose.is_empty(), "every tool documents its purpose");
            // availability and resolved path must agree
            assert_eq!(tool.available, tool.path.is_some());
            if !tool.available {
                assert!(tool.version.is_none());
            }
        }
    }
}
