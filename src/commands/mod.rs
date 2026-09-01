pub mod skill;

use std::path::PathBuf;

/// The state file, overridable for tests.
pub fn state_file() -> PathBuf {
    std::env::var_os("PMKIT_STATE_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(crate::state::state_path)
}

/// The home directory, overridable for tests.
pub fn home_dir() -> PathBuf {
    std::env::var_os("PMKIT_HOME")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_default()
}
