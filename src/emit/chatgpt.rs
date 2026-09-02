use super::{EmitFile, FileKind};
use crate::capabilities::Capabilities;
use crate::preamble::preamble;
use crate::skills::SKILLS;
use crate::target::{Destination, Target};

/// ChatGPT has no skill mechanism, so every skill is flattened into one
/// document the human pastes into a project's custom instructions.
pub fn plan(caps: &Capabilities, dest: &Destination) -> Vec<EmitFile> {
    let mut body = String::from("# pmkit — paste this into your ChatGPT project instructions\n\n");
    body.push_str(preamble(Target::ChatGpt, caps).trim_end());
    body.push_str("\n\n");
    for s in SKILLS.iter() {
        body.push_str(&format!("---\n\n<!-- {} -->\n\n", s.name));
        // Strip the canonical frontmatter: it means nothing to ChatGPT and only
        // wastes the instruction budget.
        let text = s
            .content
            .strip_prefix("---\n")
            .and_then(|rest| rest.find("\n---").map(|i| &rest[i + "\n---".len()..]))
            .unwrap_or(s.content);
        body.push_str(&format!("### {}\n{}\n", s.name, text.trim_start()));
    }
    vec![EmitFile {
        path: dest.root().join("pmkit-chatgpt-instructions.md"),
        contents: body,
        kind: FileKind::Instructions,
    }]
}
