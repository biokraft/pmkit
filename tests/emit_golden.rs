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

#[test]
fn cursor_matches_its_golden_files() {
    check(Target::Cursor);
}

#[test]
fn codex_matches_its_golden_files() {
    check(Target::Codex);
}

#[test]
fn codex_emits_both_agents_md_and_the_workspace_agents_metadata() {
    use pmkit::emit::FileKind;
    let dest = destination_for(Target::Codex, Path::new("/p"), Path::new("/h"));
    let files = plan_files(Target::Codex, &Capabilities::all_present(), &dest);
    let configs: Vec<_> = files
        .iter()
        .filter(|f| f.kind == FileKind::Config)
        .map(|f| f.path.clone())
        .collect();
    assert!(configs.contains(&PathBuf::from("/p/AGENTS.md")));
    assert!(configs.contains(&PathBuf::from("/p/agents/openai.yaml")));
}

#[test]
fn cowork_matches_its_golden_files() {
    check(Target::Cowork);
}

#[test]
fn chatgpt_matches_its_golden_files() {
    check(Target::ChatGpt);
}

#[test]
fn every_target_plans_at_least_one_skill_bearing_file() {
    use pmkit::emit::FileKind;
    for t in Target::all() {
        let dest = destination_for(t, Path::new("/p"), Path::new("/h"));
        let files = plan_files(t, &Capabilities::all_present(), &dest);
        assert!(
            files
                .iter()
                .any(|f| matches!(f.kind, FileKind::Skill | FileKind::Instructions)),
            "{} carries no skill text",
            t.as_str()
        );
    }
}

#[test]
fn chatgpt_ships_a_single_pasteable_document() {
    use pmkit::emit::FileKind;
    let dest = destination_for(Target::ChatGpt, Path::new("/p"), Path::new("/h"));
    let files = plan_files(Target::ChatGpt, &Capabilities::all_present(), &dest);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].kind, FileKind::Instructions);
    // Every skill's rules must survive the flattening.
    for s in pmkit::skills::SKILLS.iter() {
        assert!(files[0].contents.contains(s.name), "{} missing", s.name);
    }
}
