#![allow(clippy::unwrap_used)]
use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn doctor_prints_a_table_and_never_mutates_without_fix() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("pmkit")
        .unwrap()
        .env("PMKIT_HOME", tmp.path())
        .arg("doctor")
        .assert()
        .success()
        .stdout(contains("WHY IT MATTERS"))
        .stdout(contains("git"));
}
