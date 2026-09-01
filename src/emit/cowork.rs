use super::{skill_files, EmitFile, FileKind};
use crate::capabilities::Capabilities;
use crate::target::{Destination, Target};

/// Cowork takes skills as uploaded folders, so the bundle is staged under the
/// user's home with a README telling them what to do with it. Nothing is
/// written into their project.
pub fn plan(caps: &Capabilities, dest: &Destination) -> Vec<EmitFile> {
    let root = dest.root();
    let mut files = skill_files(root, "skills", Target::Cowork, caps);
    files.push(EmitFile {
        path: root.join("README.md"),
        contents: format!(
            "# pmkit skills for Claude Cowork\n\n\
             Upload each folder in `skills/` as a skill in Cowork.\n\n\
             {}\n\
             The safety gates in these skills are prose only on Cowork — nothing blocks a command \
             for you there. If you want them machine-enforced, use Cursor or Claude Code.\n",
            crate::skills::SKILLS
                .iter()
                .map(|s| format!("- `{}` — {}\n", s.name, s.summary))
                .collect::<String>()
        ),
        kind: FileKind::Instructions,
    });
    files
}
