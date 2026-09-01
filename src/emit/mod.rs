pub mod chatgpt;
pub mod claude_code;
pub mod codex;
pub mod cowork;
pub mod cursor;

use crate::capabilities::Capabilities;
use crate::preamble::preamble;
use crate::skills::{Skill, SKILLS};
use crate::target::{Destination, Target};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// A skill file in the target's native skill location.
    Skill,
    /// Agent configuration — hooks, rules, permissions.
    Config,
    /// Text a human pastes or uploads by hand.
    Instructions,
}

/// One file pmkit intends to write. Planning is separate from writing so every
/// target can be verified by golden file without touching a real home
/// directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitFile {
    pub path: PathBuf,
    pub contents: String,
    pub kind: FileKind,
}

/// A skill as it lands on disk: the canonical body with the surface's preamble
/// spliced in after the frontmatter, so the frontmatter stays first (agents
/// parse it positionally) and the body itself is untouched.
pub fn skill_body(skill: &Skill, target: Target, caps: &Capabilities) -> String {
    let text = skill.content;
    let after_frontmatter = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.find("\n---").map(|i| i + "\n---".len() + 1))
        .map(|i| i + "---\n".len())
        .unwrap_or(0);
    let (head, body) = text.split_at(after_frontmatter);
    format!("{head}\n{}\n{body}", preamble(target, caps).trim_end())
}

pub fn plan_files(target: Target, caps: &Capabilities, dest: &Destination) -> Vec<EmitFile> {
    match target {
        Target::ClaudeCode => claude_code::plan(caps, dest),
        Target::Cursor => cursor::plan(caps, dest),
        Target::Codex => codex::plan(caps, dest),
        Target::Cowork => cowork::plan(caps, dest),
        Target::ChatGpt => chatgpt::plan(caps, dest),
    }
}

/// The always-loaded file for targets that read `AGENTS.md`. Restates the
/// entry point and the three gates; the detail lives in the skills.
pub(crate) fn agents_md(target: Target, caps: &Capabilities) -> String {
    format!(
        "<!-- Written by pmkit. Edit freely — pmkit detects your changes and will not overwrite them. -->\n\n\
         # Working with a product manager\n\n\
         {}\n\
         Start every piece of feature work with the `pmk-feature-loop` skill.\n\n\
         ## Never do these without an explicit yes\n\n\
         1. `git push`, force-push, merge, or open a pull request.\n\
         2. Any write to Jira.\n\
         3. Any command touching something that is not a local development URL.\n\n\
         ## Never claim what you have not seen\n\n\
         A user-visible change is unverified until a screenshot exists. Say \"I could not verify \
         this visually\" rather than implying you checked.\n",
        preamble(target, caps).trim_end()
    )
}

/// Shared helper: one skill file per skill under `root/<dir>/<name>/SKILL.md`.
pub(crate) fn skill_files(
    root: &std::path::Path,
    skills_dir: &str,
    target: Target,
    caps: &Capabilities,
) -> Vec<EmitFile> {
    SKILLS
        .iter()
        .map(|s| EmitFile {
            path: root.join(skills_dir).join(s.name).join("SKILL.md"),
            contents: skill_body(s, target, caps),
            kind: FileKind::Skill,
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::capabilities::Capabilities;
    use crate::target::{destination_for, Target};
    use std::path::PathBuf;

    fn dest(target: Target) -> Destination {
        destination_for(target, &PathBuf::from("/p"), &PathBuf::from("/h"))
    }

    #[test]
    fn claude_code_emits_one_skill_file_per_skill_plus_a_settings_file() {
        let files = plan_files(
            Target::ClaudeCode,
            &Capabilities::all_present(),
            &dest(Target::ClaudeCode),
        );
        let skills: Vec<_> = files.iter().filter(|f| f.kind == FileKind::Skill).collect();
        assert_eq!(skills.len(), crate::skills::SKILLS.len());
        assert_eq!(
            skills[0].path,
            PathBuf::from("/p/.claude/skills/pmk-feature-loop/SKILL.md")
        );
        assert_eq!(
            files.iter().filter(|f| f.kind == FileKind::Config).count(),
            1
        );
    }

    #[test]
    fn the_settings_file_blocks_push_and_merge() {
        let files = plan_files(
            Target::ClaudeCode,
            &Capabilities::all_present(),
            &dest(Target::ClaudeCode),
        );
        let cfg = files.iter().find(|f| f.kind == FileKind::Config).unwrap();
        assert_eq!(cfg.path, PathBuf::from("/p/.claude/settings.json"));
        let parsed: serde_json::Value = serde_json::from_str(&cfg.contents).unwrap();
        let hooks = parsed["hooks"]["PreToolUse"].as_array().unwrap();
        assert!(!hooks.is_empty());
        let text = cfg.contents.as_str();
        assert!(text.contains("push"));
        assert!(text.contains("merge"));
    }

    #[test]
    fn every_emitted_skill_body_starts_with_frontmatter_then_carries_the_preamble() {
        let files = plan_files(
            Target::ClaudeCode,
            &Capabilities::all_present(),
            &dest(Target::ClaudeCode),
        );
        for f in files.iter().filter(|f| f.kind == FileKind::Skill) {
            assert!(
                f.contents.starts_with("---\n"),
                "frontmatter must stay first"
            );
            assert!(f.contents.contains("## Your surface"));
        }
    }

    #[test]
    fn the_skill_body_is_byte_identical_across_targets_apart_from_the_preamble() {
        let caps = Capabilities::all_present();
        let skill = &crate::skills::SKILLS[0];
        let a = skill_body(skill, Target::ClaudeCode, &caps);
        let b = skill_body(skill, Target::Codex, &caps);
        let strip = |s: &str| -> String {
            let start = s.find("## Your surface").unwrap();
            let end = s[start..]
                .find("\n# ")
                .map(|i| start + i)
                .unwrap_or(s.len());
            format!("{}{}", &s[..start], &s[end..])
        };
        assert_eq!(strip(&a), strip(&b));
    }

    #[test]
    fn plan_files_never_escapes_its_destination_root() {
        for t in Target::all() {
            let d = dest(t);
            for f in plan_files(t, &Capabilities::all_present(), &d) {
                assert!(
                    f.path.starts_with(d.root()),
                    "{:?} escaped {:?}",
                    f.path,
                    d.root()
                );
            }
        }
    }
}
