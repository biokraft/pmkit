use super::{cursor_hooks, skill_files, EmitFile, FileKind};
use crate::capabilities::Capabilities;
use crate::target::{Destination, Target};

/// Cursor reads project rules from `.cursor/rules/*.mdc` and honours
/// `AGENTS.md`. Skills are emitted as rules; the gates are restated in
/// `AGENTS.md` because that is the file Cursor always loads, and enforced by
/// a `.cursor/hooks.json` `beforeShellExecution` hook so the preamble's claim
/// of machine enforcement is actually true here.
pub fn plan(caps: &Capabilities, dest: &Destination) -> Vec<EmitFile> {
    let root = dest.root();
    let mut files = skill_files(root, ".cursor/rules/pmkit", Target::Cursor, caps);
    files.push(EmitFile {
        path: root.join("AGENTS.md"),
        contents: super::agents_md(Target::Cursor, caps),
        kind: FileKind::Config,
    });
    files.push(EmitFile {
        path: root.join(".cursor").join("hooks.json"),
        contents: cursor_hooks::hooks_json(),
        kind: FileKind::Config,
    });
    files
}
