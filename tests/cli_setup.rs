#![allow(clippy::unwrap_used)]
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn setup_yes_installs_and_prints_next_steps_per_target() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(&project)
        .env("PMKIT_HOME", tmp.path().join("home"))
        .env("PMKIT_STATE_FILE", tmp.path().join("skills.json"))
        .args(["setup", "--yes", "--target", "claude-code"])
        .assert()
        .success()
        .stdout(contains("WHY IT MATTERS"))
        .stdout(contains("ready to use"));

    assert!(project
        .join(".claude/skills/pmk-feature-loop/SKILL.md")
        .exists());
}

#[test]
fn setup_warns_loudly_when_it_refuses_to_overwrite_an_existing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join(".claude")).unwrap();
    // The product manager already has their own settings.json.
    std::fs::write(project.join(".claude/settings.json"), "{\"mine\": true}").unwrap();

    Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(&project)
        .env("PMKIT_HOME", tmp.path().join("home"))
        .env("PMKIT_STATE_FILE", tmp.path().join("skills.json"))
        .args(["setup", "--yes", "--target", "claude-code"])
        .assert()
        .success()
        .stdout(contains("were left alone"))
        .stdout(contains("NOT enforced"));

    // Their file survives untouched.
    assert_eq!(
        std::fs::read_to_string(project.join(".claude/settings.json")).unwrap(),
        "{\"mine\": true}"
    );
}

#[test]
fn setup_yes_degrades_when_prerequisites_are_missing_instead_of_aborting() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    // An empty PATH means no probe can find anything.
    Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(&project)
        .env("PATH", "")
        .env("PMKIT_HOME", tmp.path().join("home"))
        .env("PMKIT_STATE_FILE", tmp.path().join("skills.json"))
        .args(["setup", "--yes", "--target", "codex"])
        .assert()
        .success();

    let skill =
        std::fs::read_to_string(project.join(".agents/skills/pmk-verify-visually/SKILL.md"))
            .unwrap();
    assert!(skill.contains("You CANNOT verify anything visually"));
}
