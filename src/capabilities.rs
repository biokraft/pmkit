/// Which Jira surface is actually present on this machine. Recorded so the
/// Jira skill never has to guess which tool it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JiraBackend {
    Acli,
    Mcp,
    None,
}

impl JiraBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            JiraBackend::Acli => "acli",
            JiraBackend::Mcp => "mcp",
            JiraBackend::None => "none",
        }
    }
}

use crate::forge::Forge;

/// What this surface can actually do. Produced by the doctor, consumed by the
/// preamble. A false here becomes an explicit prohibition in the skill text,
/// which is how a missing prerequisite degrades instead of aborting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    pub shell: bool,
    pub playwright: bool,
    pub superpowers: bool,
    /// Which host the team chose. Decides which of `gh`/`bb` the preamble
    /// talks about; the flags below say whether that tool actually works.
    pub forge: Forge,
    /// `gh` installed and authenticated.
    pub gh: bool,
    /// `bb` (Bitbucket Cloud CLI) installed and authenticated.
    pub bb: bool,
    pub jira: JiraBackend,
}

impl Capabilities {
    pub fn none() -> Self {
        Self {
            shell: false,
            playwright: false,
            superpowers: false,
            forge: Forge::GitHub,
            gh: false,
            bb: false,
            jira: JiraBackend::None,
        }
    }

    /// `forge` defaults to GitHub so single-host golden files stay stable;
    /// callers that know the forge override it with struct-update syntax.
    pub fn all_present() -> Self {
        Self {
            shell: true,
            playwright: true,
            superpowers: true,
            forge: Forge::GitHub,
            gh: true,
            bb: true,
            jira: JiraBackend::Acli,
        }
    }
}
