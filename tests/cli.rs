#![allow(clippy::unwrap_used)]

use assert_cmd::Command;
use predicates::str::contains;

#[test]
fn version_prints_the_crate_version() {
    Command::cargo_bin("pmkit")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_names_the_tool() {
    Command::cargo_bin("pmkit")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("product managers"));
}
