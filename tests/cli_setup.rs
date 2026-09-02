#![allow(clippy::unwrap_used, clippy::expect_used)]
use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
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
fn setup_does_not_claim_gates_are_enforced_when_settings_json_was_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join(".claude")).unwrap();
    // The product manager already has their own settings.json.
    std::fs::write(project.join(".claude/settings.json"), "{\"mine\": true}").unwrap();

    let assert = Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(&project)
        .env("PMKIT_HOME", tmp.path().join("home"))
        .env("PMKIT_STATE_FILE", tmp.path().join("skills.json"))
        .args(["setup", "--yes", "--target", "claude-code"])
        .assert()
        .success()
        .stdout(contains("NOT enforced"));

    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let next_steps_section = stdout
        .split("What to do next")
        .nth(1)
        .expect("no 'What to do next' section in output");
    assert!(
        !next_steps_section.contains("The safety gates are enforced here"),
        "the closing next-steps section still claims the gates are enforced, \
         even though the settings.json that carries them was refused:\n{next_steps_section}"
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

#[test]
fn setup_warns_loudly_when_a_write_fails() {
    // Fix round 1, Finding 3: a failed write is at least as serious as a
    // refused overwrite. Force one by making `.claude` read-only before the
    // skill files under it are written, so `apply`'s `write_file` call
    // returns Err and the outcome comes back `Action::Failed`.
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    let claude_dir = project.join(".claude");
    std::fs::create_dir_all(&claude_dir).unwrap();
    let mut perms = std::fs::metadata(&claude_dir).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&claude_dir, perms).unwrap();

    let assert = Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(&project)
        .env("PMKIT_HOME", tmp.path().join("home"))
        .env("PMKIT_STATE_FILE", tmp.path().join("skills.json"))
        .args(["setup", "--yes", "--target", "claude-code"])
        .assert()
        .success();

    // Restore permissions so the tempdir can be cleaned up.
    let mut perms = std::fs::metadata(&claude_dir).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    std::fs::set_permissions(&claude_dir, perms).unwrap();

    assert
        .stdout(contains("could not be written"))
        .stdout(contains("NOT active"));
}

fn git_remote(project: &std::path::Path, url: &str) {
    let run = |args: &[&str]| {
        let ok = std::process::Command::new("git")
            .args(args)
            .current_dir(project)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?} failed");
    };
    run(&["init", "-q"]);
    run(&["remote", "add", "origin", url]);
}

#[test]
fn setup_yes_with_forge_bitbucket_tells_the_agent_about_bb_and_not_gh() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(&project)
        .env("PMKIT_HOME", tmp.path().join("home"))
        .env("PMKIT_STATE_FILE", tmp.path().join("skills.json"))
        .args([
            "setup",
            "--yes",
            "--target",
            "codex",
            "--forge",
            "bitbucket",
        ])
        .assert()
        .success()
        .stdout(contains("Bitbucket Cloud CLI is"))
        .stdout(predicates::str::contains("GitHub CLI is how a").not());

    let skill =
        std::fs::read_to_string(project.join(".agents/skills/pmk-feature-loop/SKILL.md")).unwrap();
    assert!(skill.contains("Bitbucket Cloud"), "{skill}");
    assert!(!skill.contains("`gh pr create`"), "{skill}");
}

#[test]
fn setup_yes_detects_bitbucket_from_the_git_remote_when_no_flag_is_given() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    git_remote(&project, "git@bitbucket.org:acme/api.git");

    Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(&project)
        .env("PMKIT_HOME", tmp.path().join("home"))
        .env("PMKIT_STATE_FILE", tmp.path().join("skills.json"))
        .args(["setup", "--yes", "--target", "codex"])
        .assert()
        .success()
        .stdout(contains("Bitbucket Cloud CLI is"));

    let skill =
        std::fs::read_to_string(project.join(".agents/skills/pmk-feature-loop/SKILL.md")).unwrap();
    assert!(skill.contains("Bitbucket Cloud"), "{skill}");
}

#[test]
fn setup_yes_falls_back_to_github_when_nothing_is_detectable() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(&project)
        .env("PMKIT_HOME", tmp.path().join("home"))
        .env("PMKIT_STATE_FILE", tmp.path().join("skills.json"))
        .args(["setup", "--yes", "--target", "codex"])
        .assert()
        .success()
        .stdout(contains("GitHub CLI is how a"))
        .stdout(predicates::str::contains("Bitbucket Cloud CLI is").not());
}

#[test]
fn an_unknown_forge_is_rejected_with_the_valid_list() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(tmp.path())
        .args(["setup", "--yes", "--forge", "gitlab"])
        .assert()
        .failure()
        .stderr(contains("github, bitbucket, both"));
}
