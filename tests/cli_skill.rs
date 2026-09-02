#![allow(clippy::unwrap_used)]
use assert_cmd::Command;
use predicates::str::contains;

fn pmkit(project: &std::path::Path, state: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("pmkit").unwrap();
    cmd.current_dir(project)
        .env("PMKIT_STATE_FILE", state)
        .env("PMKIT_HOME", project.join("fake-home"));
    cmd
}

#[test]
fn install_writes_the_skills_and_list_reports_them_current() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let state = tmp.path().join("skills.json");

    pmkit(&project, &state)
        .args(["skill", "install", "--target", "claude-code"])
        .assert()
        .success()
        .stdout(contains("installed"));

    assert!(project
        .join(".claude/skills/pmk-feature-loop/SKILL.md")
        .exists());

    pmkit(&project, &state)
        .args(["skill", "list"])
        .assert()
        .success()
        .stdout(contains("current"));
}

#[test]
fn install_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let state = tmp.path().join("skills.json");
    for _ in 0..2 {
        pmkit(&project, &state)
            .args(["skill", "install", "--target", "codex"])
            .assert()
            .success();
    }
    pmkit(&project, &state)
        .args(["skill", "list"])
        .assert()
        .success()
        .stdout(contains("current"));
}

#[test]
fn uninstall_removes_what_was_installed() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let state = tmp.path().join("skills.json");
    pmkit(&project, &state)
        .args(["skill", "install", "--target", "codex"])
        .assert()
        .success();
    pmkit(&project, &state)
        .args(["skill", "uninstall", "--target", "codex"])
        .assert()
        .success();
    assert!(!project
        .join(".agents/skills/pmk-feature-loop/SKILL.md")
        .exists());
}

#[test]
fn an_unknown_target_fails_with_the_valid_list() {
    let tmp = tempfile::tempdir().unwrap();
    let state = tmp.path().join("skills.json");
    pmkit(tmp.path(), &state)
        .args(["skill", "install", "--target", "vscode"])
        .assert()
        .failure()
        .stderr(contains("claude-code"));
}

#[test]
fn refresh_does_not_resurrect_a_deliberately_deleted_file() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let state = tmp.path().join("skills.json");

    pmkit(&project, &state)
        .args(["skill", "install", "--target", "codex"])
        .assert()
        .success();

    let deleted = project.join(".agents/skills/pmk-feature-loop/SKILL.md");
    let survivor = project.join(".agents/skills/pmk-shape-idea/SKILL.md");
    assert!(deleted.exists());
    assert!(survivor.exists());
    std::fs::remove_file(&deleted).unwrap();

    pmkit(&project, &state)
        .args(["skill", "refresh"])
        .assert()
        .success();

    assert!(
        !deleted.exists(),
        "a deliberately deleted file must not reappear on refresh"
    );
    assert!(survivor.exists(), "other installed files must be untouched");
}

#[test]
fn uninstall_without_target_or_all_refuses_and_removes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let state = tmp.path().join("skills.json");

    pmkit(&project, &state)
        .args(["skill", "install", "--target", "codex"])
        .assert()
        .success();

    pmkit(&project, &state)
        .args(["skill", "uninstall"])
        .assert()
        .failure()
        .stderr(contains("--target"))
        .stderr(contains("--all"));

    assert!(project
        .join(".agents/skills/pmk-feature-loop/SKILL.md")
        .exists());
}

#[test]
fn uninstall_with_all_removes_every_target() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let state = tmp.path().join("skills.json");

    pmkit(&project, &state)
        .args(["skill", "install", "--target", "codex"])
        .assert()
        .success();

    pmkit(&project, &state)
        .args(["skill", "uninstall", "--all"])
        .assert()
        .success();

    assert!(!project
        .join(".agents/skills/pmk-feature-loop/SKILL.md")
        .exists());
}
