use super::claude_code::BLOCKED;

/// Builds the shell command for one blocked pattern's `beforeShellExecution`
/// hook. Cursor's contract (https://cursor.com/docs/hooks) delivers the
/// command on stdin as `{"command": "<shell command>"}` — at the top level,
/// not nested under `tool_input` the way Claude Code's `PreToolUse` payload
/// is. Everything else about the shell idiom is unchanged from Claude Code's
/// hook for the same three load-bearing reasons: exit 2 is the only code
/// that denies (a `{"permission":"deny"}` document would also work, but
/// using one idiom for both targets means they cannot drift), a missing `jq`
/// falls back to the raw stdin blob so the gate fails *closed* rather than
/// silently allowing, and whitespace is collapsed so `git  push` and
/// `git -C dir push` still match the `( [^ ]+)*` patterns.
fn hook_command(pattern: &str, message: &str) -> String {
    format!(
        "raw=$(cat); cmd=$(printf '%s' \"$raw\" | jq -r '.command // \"\"' 2>/dev/null); \
         [ -n \"$cmd\" ] || cmd=$raw; \
         printf '%s' \"$cmd\" | tr -s '[:space:]' ' ' | grep -Eq '{pattern}' && {{ echo '{message}' >&2; exit 2; }}; exit 0"
    )
}

/// The full contents of `.cursor/hooks.json`.
pub(crate) fn hooks_json() -> String {
    let entries: Vec<serde_json::Value> = BLOCKED
        .iter()
        .map(|(pattern, message)| serde_json::json!({ "command": hook_command(pattern, message) }))
        .collect();
    let value = serde_json::json!({
        "$comment": "Written by pmkit. Edit freely — pmkit detects your changes and will not overwrite them.",
        "version": 1,
        "hooks": { "beforeShellExecution": entries }
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
    use crate::capabilities::Capabilities;
    use crate::emit::FileKind;
    use crate::target::{destination_for, Target};
    use serial_test::serial;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    #[test]
    fn the_hooks_file_declares_version_one_and_a_before_shell_execution_array() {
        let parsed: serde_json::Value = serde_json::from_str(&hooks_json()).unwrap();
        assert_eq!(parsed["version"], 1);
        assert!(parsed["hooks"]["beforeShellExecution"].as_array().is_some());
    }

    #[test]
    fn there_is_one_hook_entry_per_blocked_pattern() {
        let parsed: serde_json::Value = serde_json::from_str(&hooks_json()).unwrap();
        let entries = parsed["hooks"]["beforeShellExecution"].as_array().unwrap();
        assert_eq!(entries.len(), crate::emit::claude_code::BLOCKED.len());
    }

    #[test]
    fn every_hook_reads_the_cursor_payload_shape_not_the_claude_one() {
        // `.tool_input.command` is Claude Code's shape. A hook using it against
        // Cursor's payload matches nothing and silently never blocks.
        let json = hooks_json();
        assert!(json.contains(".command"));
        assert!(!json.contains(".tool_input"));
    }

    #[test]
    fn every_hook_denies_with_exit_code_two() {
        let parsed: serde_json::Value = serde_json::from_str(&hooks_json()).unwrap();
        for entry in parsed["hooks"]["beforeShellExecution"].as_array().unwrap() {
            let cmd = entry["command"].as_str().unwrap();
            assert!(
                cmd.contains("exit 2"),
                "a hook that does not exit 2 fails open: {cmd}"
            );
        }
    }

    /// Runs the emitted `push` hook command against a raw `.command` value,
    /// exactly as Cursor would feed it: `{"command": ...}` on stdin.
    /// `extra_path` is prepended to `PATH` so a test can simulate `jq` being
    /// unavailable by pointing it at a directory that does not contain it.
    fn run_push_hook(command_value: &str, extra_path: Option<&str>) -> i32 {
        let dest = destination_for(Target::Cursor, &PathBuf::from("/p"), &PathBuf::from("/h"));
        let files = super::super::cursor::plan(&Capabilities::all_present(), &dest);
        let cfg = files
            .iter()
            .find(|f| f.kind == FileKind::Config && f.path.ends_with("hooks.json"))
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&cfg.contents).unwrap();
        let entries = parsed["hooks"]["beforeShellExecution"].as_array().unwrap();
        let push_hook = entries
            .iter()
            .find(|h| h["command"].as_str().unwrap().contains("git( [^ ]+)* push"))
            .expect("no push hook found in emitted hooks.json");
        let shell_command = push_hook["command"].as_str().unwrap();

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

        let payload = serde_json::json!({ "command": command_value });
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
            "pmkit-cursor-no-jq-{}-{:?}",
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

    // See claude_code.rs's identical comment: these tests spawn `sh` reading
    // the inherited `PATH`, and doctor::runner's tests mutate the process
    // `PATH` under the same `#[serial(env_path)]` key — sharing that key is
    // what prevents "failed to spawn sh" flakes under concurrent test runs.
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
