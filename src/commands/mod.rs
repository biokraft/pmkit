pub mod skill;

use crate::doctor::runner::RealRunner;
use crate::forge::{detect_forge, Forge};
use std::path::{Path, PathBuf};

/// Flag wins; otherwise guess from the project's git remote; otherwise
/// GitHub, which is what pmkit assumed before it knew about forges.
pub fn resolve_forge(flag: Option<Forge>, project_dir: &Path) -> Forge {
    flag.or_else(|| detect_forge(project_dir, &RealRunner))
        .unwrap_or(Forge::GitHub)
}

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
