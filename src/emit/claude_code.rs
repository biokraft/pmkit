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
        "git push",
        "pmkit: `git push` needs an explicit yes from the human. Show the branch and target, then ask.",
    ),
    (
        "git merge",
        "pmkit: merging needs an explicit yes from the human.",
    ),
    (
        "gh pr create",
        "pmkit: opening a pull request needs an explicit yes from the human.",
    ),
    (
        "gh pr merge",
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
/// Reads the tool call's JSON from stdin (as Claude Code feeds every hook),
/// pulls out `.tool_input.command` with `jq`, and greps it for `pattern`.
/// On a match it writes `message` to stderr and exits 2 (blocking); on no
/// match the `if` has no `else`, so the script's own exit status is 0
/// (allowing).
fn hook_command(pattern: &str, message: &str) -> String {
    format!(
        "cmd=$(jq -r '.tool_input.command // \"\"'); \
         if printf '%s' \"$cmd\" | grep -Eq '{pattern}'; then echo '{message}' >&2; exit 2; fi"
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
