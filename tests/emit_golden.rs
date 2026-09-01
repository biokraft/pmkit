#![allow(clippy::unwrap_used)]
use pmkit::capabilities::Capabilities;
use pmkit::emit::plan_files;
use pmkit::target::{destination_for, Target};
use std::path::{Path, PathBuf};

/// Compares the planned files against `tests/golden/<target>/`. Regenerate with
/// `UPDATE_GOLDEN=1 cargo test --test emit_golden`.
fn check(target: Target) {
    let dest = destination_for(target, Path::new("/p"), Path::new("/h"));
    let files = plan_files(target, &Capabilities::all_present(), &dest);
    let dir = PathBuf::from("tests/golden").join(target.as_str());
    for f in &files {
        let rel = f.path.strip_prefix(dest.root()).unwrap();
        let golden = dir.join(rel);
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
            std::fs::write(&golden, &f.contents).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&golden)
            .unwrap_or_else(|e| panic!("missing golden {}: {e}", golden.display()));
        assert_eq!(f.contents, want, "content drift in {}", golden.display());
    }
    assert!(!files.is_empty(), "{} planned no files", target.as_str());
}

#[test]
fn claude_code_matches_its_golden_files() {
    check(Target::ClaudeCode);
}
