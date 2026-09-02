#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl RunResult {
    pub fn missing() -> Self {
        Self {
            code: 127,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    pub fn ok(&self) -> bool {
        self.code == 0
    }
}

/// Every probe goes through this, so the whole doctor is testable without a
/// configured machine and without touching the network.
pub trait Runner {
    fn run(&self, program: &str, args: &[&str]) -> RunResult;
    fn exists(&self, program: &str) -> bool;
}

pub struct RealRunner;

impl Runner for RealRunner {
    fn run(&self, program: &str, args: &[&str]) -> RunResult {
        match std::process::Command::new(program).args(args).output() {
            Ok(out) => RunResult {
                code: out.status.code().unwrap_or(1),
                stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            },
            Err(_) => RunResult::missing(),
        }
    }

    fn exists(&self, program: &str) -> bool {
        // `command -v` is a shell builtin, not an executable, so spawning
        // `Command::new("command")` fails on every system without a shell in
        // between. Search PATH directly instead: cheap, no subprocess, and
        // correct on both macOS and Linux.
        std::env::var_os("PATH")
            .map(|paths| {
                std::env::split_paths(&paths)
                    // An empty PATH entry (e.g. PATH="" or a leading/trailing
                    // `:`) resolves relative to the current directory, which
                    // would make a same-named file in the cwd a false
                    // positive. Skip those.
                    .filter(|dir| !dir.as_os_str().is_empty())
                    .any(|dir| is_executable_file(&dir.join(program)))
            })
            .unwrap_or(false)
    }
}

#[cfg(unix)]
fn is_executable_file(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(meta) => meta.is_file() && meta.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_executable_file(path: &std::path::Path) -> bool {
    // The release matrix is macOS and Linux only; keep this so a build on
    // another platform still compiles, treating any present file as usable.
    path.is_file()
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeRunner {
    responses: std::collections::BTreeMap<String, RunResult>,
}

#[cfg(test)]
impl FakeRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, program: &str, code: i32, stdout: &str) -> Self {
        self.responses.insert(
            program.to_string(),
            RunResult {
                code,
                stdout: stdout.to_string(),
                stderr: String::new(),
            },
        );
        self
    }
}

#[cfg(test)]
impl Runner for FakeRunner {
    fn run(&self, program: &str, _args: &[&str]) -> RunResult {
        self.responses
            .get(program)
            .cloned()
            .unwrap_or_else(RunResult::missing)
    }

    fn exists(&self, program: &str) -> bool {
        self.responses.contains_key(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial(env_path)]
    fn a_program_on_path_is_reported_present() {
        // `ls` exists on every unix-like CI/dev box this crate targets.
        assert!(RealRunner.exists("ls"));
    }

    #[test]
    #[serial(env_path)]
    fn a_program_not_on_path_is_reported_absent() {
        assert!(!RealRunner.exists("this-program-does-not-exist-pmkit-doctor-test"));
    }

    #[test]
    #[serial(env_path)]
    #[allow(clippy::unwrap_used)]
    fn a_present_non_executable_file_is_not_reported_as_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pmkit-doctor-data-file");
        std::fs::write(&path, b"not a program").unwrap();
        // Explicitly clear the executable bits; `write` may leave 0644 but
        // don't rely on the umask.
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            perms.set_mode(0o644);
        }
        std::fs::set_permissions(&path, perms).unwrap();

        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", dir.path());
        let found = RealRunner.exists("pmkit-doctor-data-file");
        restore_path(original_path);
        assert!(!found);
    }

    #[test]
    #[serial(env_path)]
    #[allow(clippy::unwrap_used)]
    fn a_present_executable_file_is_reported_as_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pmkit-doctor-executable-file");
        std::fs::write(&path, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let original_path = std::env::var_os("PATH");
        std::env::set_var("PATH", dir.path());
        let found = RealRunner.exists("pmkit-doctor-executable-file");
        restore_path(original_path);
        assert!(found);
    }

    #[test]
    #[serial(env_path)]
    #[allow(clippy::unwrap_used)]
    fn an_empty_path_does_not_fall_back_to_the_current_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pmkit-doctor-cwd-trap");
        std::fs::write(&path, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let original_dir = std::env::current_dir().unwrap();
        let original_path = std::env::var_os("PATH");
        std::env::set_current_dir(dir.path()).unwrap();
        std::env::set_var("PATH", "");
        let found = RealRunner.exists("pmkit-doctor-cwd-trap");
        std::env::set_current_dir(original_dir).unwrap();
        restore_path(original_path);
        assert!(!found);
    }

    fn restore_path(original: Option<std::ffi::OsString>) {
        match original {
            Some(v) => std::env::set_var("PATH", v),
            None => std::env::remove_var("PATH"),
        }
    }
}
