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
                std::env::split_paths(&paths).any(|dir| {
                    let candidate = dir.join(program);
                    candidate.is_file()
                })
            })
            .unwrap_or(false)
    }
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

    #[test]
    fn a_program_on_path_is_reported_present() {
        // `ls` exists on every unix-like CI/dev box this crate targets.
        assert!(RealRunner.exists("ls"));
    }

    #[test]
    fn a_program_not_on_path_is_reported_absent() {
        assert!(!RealRunner.exists("this-program-does-not-exist-pmkit-doctor-test"));
    }
}
