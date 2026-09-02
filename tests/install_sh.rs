#![allow(clippy::unwrap_used)]

/// The install script is the one piece of this repo a stranger pipes into a
/// shell. These checks are cheap and catch the mistakes that matter.
#[test]
fn install_sh_is_strict_verifies_a_checksum_and_never_uses_sudo() {
    let script = std::fs::read_to_string("install.sh").unwrap();
    assert!(script.contains("set -eu"), "must abort on error");
    assert!(script.contains("sha256"), "must verify a checksum");
    assert!(!script.contains("sudo"), "must never escalate");
    assert!(script.contains("biokraft/pmkit"));
}

#[test]
fn install_sh_passes_shellcheck_if_it_is_available() {
    let available = std::process::Command::new("shellcheck")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !available {
        return;
    }
    let out = std::process::Command::new("shellcheck")
        .arg("install.sh")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "shellcheck: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}
