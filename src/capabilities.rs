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

/// What this surface can actually do. Produced by the doctor, consumed by the
/// preamble. A false here becomes an explicit prohibition in the skill text,
/// which is how a missing prerequisite degrades instead of aborting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    pub shell: bool,
    pub playwright: bool,
    pub superpowers: bool,
    pub gh: bool,
    pub jira: JiraBackend,
}

impl Capabilities {
    pub fn none() -> Self {
        Self {
            shell: false,
            playwright: false,
            superpowers: false,
            gh: false,
            jira: JiraBackend::None,
        }
    }

    pub fn all_present() -> Self {
        Self {
            shell: true,
            playwright: true,
            superpowers: true,
            gh: true,
            jira: JiraBackend::Acli,
        }
    }
}
