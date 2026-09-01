use super::{skill_files, EmitFile, FileKind};
use crate::capabilities::Capabilities;
use crate::target::{Destination, Target};

/// Codex reads `.agents/skills/` and `AGENTS.md`. This target also serves
/// **ChatGPT Workspace Agents**, which are Codex-powered and read the same
/// `.agents/skills` folders, so one emitter covers both. `agents/openai.yaml`
/// is optional — skills appear without it — but it sets the display name and
/// invocation policy in the ChatGPT desktop app's Skills sidebar.
pub fn plan(caps: &Capabilities, dest: &Destination) -> Vec<EmitFile> {
    let root = dest.root();
    let mut files = skill_files(root, ".agents/skills", Target::Codex, caps);
    files.push(EmitFile {
        path: root.join("AGENTS.md"),
        contents: super::agents_md(Target::Codex, caps),
        kind: FileKind::Config,
    });
    files.push(EmitFile {
        path: root.join("agents").join("openai.yaml"),
        contents: openai_yaml(),
        kind: FileKind::Config,
    });
    files
}

/// Deliberately minimal: display metadata and the invocation policy, no tool
/// dependencies. pmkit ships no MCP server, so declaring one would be a lie
/// the desktop app surfaces to the user.
fn openai_yaml() -> String {
    r#"# Written by pmkit. Edit freely — pmkit detects your changes and will not overwrite them.
interface:
  display_name: "pmkit — the product manager's loop"
  short_description: "Shape an idea, build it one reviewed step at a time, verify it, keep the ticket honest."
  default_prompt: "Use the pmkit loop."

policy:
  allow_implicit_invocation: true
"#
    .to_string()
}
