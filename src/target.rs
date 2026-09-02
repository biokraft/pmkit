use crate::error::PmError;
use std::path::{Path, PathBuf};

/// One agent surface pmkit can emit into. Codex and ChatGPT are separate
/// variants despite sharing a vendor: Codex has a shell and reads `AGENTS.md`,
/// ChatGPT has neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    ClaudeCode,
    Cursor,
    Codex,
    Cowork,
    ChatGpt,
}

impl Target {
    pub fn all() -> [Target; 5] {
        [
            Target::ClaudeCode,
            Target::Cursor,
            Target::Codex,
            Target::Cowork,
            Target::ChatGpt,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Target::ClaudeCode => "claude-code",
            Target::Cursor => "cursor",
            Target::Codex => "codex",
            Target::Cowork => "cowork",
            Target::ChatGpt => "chatgpt",
        }
    }

    /// Shown in the wizard's checklist.
    pub fn label(self) -> &'static str {
        match self {
            Target::ClaudeCode => "Claude Code (terminal)",
            Target::Cursor => "Cursor",
            Target::Codex => "Codex / ChatGPT Workspace Agents",
            Target::Cowork => "Claude Cowork",
            Target::ChatGpt => "ChatGPT",
        }
    }

    /// Whether this target's files belong inside the project being worked on.
    /// Cowork and ChatGPT have no repo to write into: their bundles are staged
    /// under the user's home and then uploaded or pasted by hand.
    pub fn is_in_repo(self) -> bool {
        match self {
            Target::ClaudeCode | Target::Cursor | Target::Codex => true,
            Target::Cowork | Target::ChatGpt => false,
        }
    }

    /// Whether the safety gates are machine-enforced here. Where this is false
    /// the gates are prose only, and the preamble must say so out loud.
    pub fn enforces_gates_with_hooks(self) -> bool {
        matches!(self, Target::ClaudeCode | Target::Cursor)
    }

    /// Where this target's hook/settings file lands, relative to its
    /// destination root — the file `gate_installed` must find an outcome
    /// for to say enforcement actually landed. `None` for the three
    /// prose-only targets, which have no such file.
    pub fn gate_config_relpath(self) -> Option<&'static str> {
        match self {
            Target::ClaudeCode => Some(".claude/settings.json"),
            Target::Cursor => Some(".cursor/hooks.json"),
            Target::Codex | Target::Cowork | Target::ChatGpt => None,
        }
    }
}

impl std::str::FromStr for Target {
    type Err = PmError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Target::all()
            .into_iter()
            .find(|t| t.as_str() == s)
            .ok_or_else(|| {
                PmError::Config(format!(
                    "unknown target `{s}` — expected one of: {}",
                    Target::all()
                        .iter()
                        .map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
    }
}

/// Where a target's files go. Split as a type rather than a bare path so the
/// wizard can tell the human "these are in your project" from "these are
/// staged for you to upload".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    Repo(PathBuf),
    Home(PathBuf),
}

impl Destination {
    pub fn root(&self) -> &Path {
        match self {
            Destination::Repo(p) | Destination::Home(p) => p.as_path(),
        }
    }
}

pub fn destination_for(target: Target, project_dir: &Path, home: &Path) -> Destination {
    match target {
        Target::ClaudeCode | Target::Cursor | Target::Codex => {
            Destination::Repo(project_dir.to_path_buf())
        }
        Target::Cowork => Destination::Home(home.join("pmkit-cowork")),
        Target::ChatGpt => Destination::Home(home.join("pmkit-chatgpt")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn every_target_round_trips_through_its_string() {
        for t in Target::all() {
            assert_eq!(t.as_str().parse::<Target>().ok(), Some(t), "{}", t.as_str());
        }
    }

    #[test]
    fn an_unknown_target_is_rejected() {
        assert!("vscode".parse::<Target>().is_err());
    }

    #[test]
    fn only_claude_code_and_cursor_enforce_gates_with_hooks() {
        let hooked: Vec<&str> = Target::all()
            .into_iter()
            .filter(|t| t.enforces_gates_with_hooks())
            .map(|t| t.as_str())
            .collect();
        assert_eq!(hooked, vec!["claude-code", "cursor"]);
    }

    #[test]
    fn gate_config_relpath_matches_enforcement_and_only_hooked_targets_have_one() {
        for t in Target::all() {
            assert_eq!(
                t.gate_config_relpath().is_some(),
                t.enforces_gates_with_hooks()
            );
        }
        assert_eq!(
            Target::ClaudeCode.gate_config_relpath(),
            Some(".claude/settings.json")
        );
        assert_eq!(
            Target::Cursor.gate_config_relpath(),
            Some(".cursor/hooks.json")
        );
    }

    #[test]
    fn in_repo_targets_write_under_the_project_and_the_others_under_home() {
        let project = PathBuf::from("/tmp/project");
        let home = PathBuf::from("/tmp/home");
        assert_eq!(
            destination_for(Target::ClaudeCode, &project, &home).root(),
            project.as_path()
        );
        assert_eq!(
            destination_for(Target::Cowork, &project, &home).root(),
            home.join("pmkit-cowork").as_path()
        );
        assert_eq!(
            destination_for(Target::ChatGpt, &project, &home).root(),
            home.join("pmkit-chatgpt").as_path()
        );
    }
}
