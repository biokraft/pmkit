use super::{skill_files, EmitFile, FileKind};
use crate::capabilities::Capabilities;
use crate::target::{Destination, Target};

/// The commands the gates forbid without an explicit human yes. Matched with
/// `grep -E` against the command line a `Bash` tool call is about to run.
///
/// Claude Code's `PreToolUse` hooks block a tool call only on exit code 2 —
/// any other non-zero exit is treated as a non-blocking warning, and exit 0
/// always allows the call through. The hook command below is built so a
/// match exits 2 (blocked, message on stderr) and a non-match exits 0
/// (allowed) — never anything in between, and never nonzero on the pass
/// path, which would otherwise block *every* Bash call rather than none.
const BLOCKED: &[(&str, &str)] = &[
    (
        "git( [^ ]+)* push",
        "pmkit: `git push` needs an explicit yes from the human. Show the branch and target, then ask.",
    ),
    (
        "git( [^ ]+)* merge",
        "pmkit: merging needs an explicit yes from the human.",
    ),
    (
        "gh( [^ ]+)* pr( [^ ]+)* create",
        "pmkit: opening a pull request needs an explicit yes from the human.",
    ),
    (
        "gh( [^ ]+)* pr( [^ ]+)* merge",
        "pmkit: merging a pull request needs an explicit yes from the human.",
    ),
];

pub fn plan(caps: &Capabilities, dest: &Destination) -> Vec<EmitFile> {
    let root = dest.root();
    let mut files = skill_files(root, ".claude/skills", Target::ClaudeCode, caps);
    files.push(EmitFile {
        path: root.join(".claude").join("settings.json"),
        contents: settings_json(),
        kind: FileKind::Config,
    });
    files
}

/// Builds the shell command for one blocked pattern's `PreToolUse` hook.
///
/// Reads the tool call's JSON from stdin (as Claude Code feeds every hook)
/// and pulls out `.tool_input.command` with `jq`. If `jq` is missing or
/// fails — notably, macOS ships no `jq` by default — `cmd` falls back to the
/// raw JSON blob itself, which still contains the command text, so the gate
/// fails *closed* (it blocks) rather than silently doing nothing. Before
/// matching, runs of whitespace are collapsed to a single space so a command
/// written with extra spaces (`git  push`) still matches. The pattern is then
/// matched with `grep -E` against that normalized text. On a match it writes
/// `message` to stderr and exits 2 (blocking); otherwise it exits 0
/// (allowing) explicitly, rather than relying on the exit status of the last
/// command run.
fn hook_command(pattern: &str, message: &str) -> String {
    format!(
        "raw=$(cat); cmd=$(printf '%s' \"$raw\" | jq -r '.tool_input.command // \"\"' 2>/dev/null); \
         [ -n \"$cmd\" ] || cmd=$raw; \
         printf '%s' \"$cmd\" | tr -s '[:space:]' ' ' | grep -Eq '{pattern}' && {{ echo '{message}' >&2; exit 2; }}; exit 0"
    )
}

fn settings_json() -> String {
    let hooks: Vec<serde_json::Value> = BLOCKED
        .iter()
        .map(|(pattern, message)| {
            serde_json::json!({
                "matcher": "Bash",
                "hooks": [{
                    "type": "command",
                    "command": hook_command(pattern, message)
                }]
            })
        })
        .collect();
    let value = serde_json::json!({
        "$comment": "Written by pmkit. Edit freely — pmkit detects your changes and will not overwrite them.",
        "hooks": { "PreToolUse": hooks }
    });
    format!(
        "{}\n",
        serde_json::to_string_pretty(&value).unwrap_or_default()
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::target::destination_for;
    use serial_test::serial;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    /// Runs the emitted `push` hook command against a raw `tool_input.command`
    /// value, exactly as Claude Code would feed it: the hook's JSON payload on
    /// stdin. Returns the process exit code. `extra_path` is prepended to
    /// `PATH` so a test can simulate `jq` being unavailable by pointing it at
    /// a directory that does not contain it.
    fn run_push_hook(command_value: &str, extra_path: Option<&str>) -> i32 {
        let dest = destination_for(
            Target::ClaudeCode,
            &PathBuf::from("/p"),
            &PathBuf::from("/h"),
        );
        let files = plan(&Capabilities::all_present(), &dest);
        let cfg = files.iter().find(|f| f.kind == FileKind::Config).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&cfg.contents).unwrap();
        let hooks = parsed["hooks"]["PreToolUse"].as_array().unwrap();
        let push_hook = hooks
            .iter()
            .find(|h| {
                h["hooks"][0]["command"]
                    .as_str()
                    .unwrap()
                    .contains("git( [^ ]+)* push")
            })
            .expect("no push hook found in emitted settings.json");
        let shell_command = push_hook["hooks"][0]["command"].as_str().unwrap();

        let path = match extra_path {
            Some(p) => p.to_string(),
            None => std::env::var("PATH").unwrap_or_default(),
        };

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(shell_command)
            .env_clear()
            .env("PATH", path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn sh");

        let payload = serde_json::json!({ "tool_input": { "command": command_value } });
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(payload.to_string().as_bytes())
            .unwrap();

        let status = child.wait().unwrap();
        status.code().unwrap_or(-1)
    }

    /// A PATH built from symlinks to the tools the hook needs (`sh`, `cat`,
    /// `printf`, `tr`, `grep`) but deliberately excluding `jq` — simulating a
    /// stock macOS install, which ships no `jq`.
    fn path_without_jq() -> String {
        let dir = std::env::temp_dir().join(format!(
            "pmkit-no-jq-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        for bin in ["sh", "cat", "printf", "tr", "grep"] {
            let target = which(bin).unwrap_or_else(|| panic!("{bin} not found on this system"));
            let link = dir.join(bin);
            if !link.exists() {
                std::os::unix::fs::symlink(target, &link).unwrap();
            }
        }
        dir.to_string_lossy().into_owned()
    }

    fn which(bin: &str) -> Option<PathBuf> {
        for dir in std::env::var("PATH").unwrap_or_default().split(':') {
            let candidate = PathBuf::from(dir).join(bin);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    // Every test below reads the process `PATH` (via `run_push_hook`'s
    // `None` fallback, which spawns `sh` with the inherited `PATH`) even
    // though it never mutates it. `doctor::runner`'s tests DO mutate the
    // process `PATH` under the same `#[serial(env_path)]` key, and `cargo
    // test` runs tests in the same binary concurrently by default — so
    // without sharing that key, one of these could spawn `sh` while another
    // thread has `PATH` set to a scratch dir or emptied, and fail with
    // "failed to spawn sh: No such file or directory". Serializing on the
    // same key as the mutators is what actually fixes that, rather than
    // hoping the flake stays rare.
    #[test]
    #[serial(env_path)]
    fn a_plain_push_is_blocked() {
        assert_eq!(run_push_hook("git push origin main", None), 2);
    }

    #[test]
    #[serial(env_path)]
    fn a_push_with_flags_is_blocked() {
        assert_eq!(run_push_hook("git push --force", None), 2);
    }

    #[test]
    #[serial(env_path)]
    fn a_push_chained_after_another_command_is_blocked() {
        assert_eq!(run_push_hook("echo hi && git push", None), 2);
    }

    #[test]
    #[serial(env_path)]
    fn an_unrelated_command_is_allowed() {
        assert_eq!(run_push_hook("npm install", None), 0);
    }

    #[test]
    #[serial(env_path)]
    fn extra_whitespace_between_git_and_push_is_still_blocked() {
        assert_eq!(run_push_hook("git  push origin main", None), 2);
    }

    #[test]
    #[serial(env_path)]
    fn a_push_with_git_flags_before_the_subcommand_is_still_blocked() {
        assert_eq!(run_push_hook("git -C /tmp/repo push", None), 2);
    }

    #[test]
    #[serial(env_path)]
    fn a_missing_jq_fails_closed_rather_than_silently_allowing() {
        let path = path_without_jq();
        assert_eq!(run_push_hook("git push origin main", Some(&path)), 2);
    }
}
