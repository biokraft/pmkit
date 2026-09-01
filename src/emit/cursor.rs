use super::{skill_files, EmitFile, FileKind};
use crate::capabilities::Capabilities;
use crate::target::{Destination, Target};

/// Cursor reads project rules from `.cursor/rules/*.mdc` and honours
/// `AGENTS.md`. Skills are emitted as rules; the gates are restated in
/// `AGENTS.md` because that is the file Cursor always loads.
pub fn plan(caps: &Capabilities, dest: &Destination) -> Vec<EmitFile> {
    let root = dest.root();
    let mut files = skill_files(root, ".cursor/rules/pmkit", Target::Cursor, caps);
    files.push(EmitFile {
        path: root.join("AGENTS.md"),
        contents: super::agents_md(Target::Cursor, caps),
        kind: FileKind::Config,
    });
    files
}
