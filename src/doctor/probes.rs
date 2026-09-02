use crate::capabilities::{Capabilities, JiraBackend};
use crate::doctor::runner::Runner;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeStatus {
    Ok(String),
    Missing,
    Broken(String),
}

/// One prerequisite: what it is, whether it is there, why a product manager
/// should care, and the exact command that fixes it. `fix` is never run
/// without an explicit yes.
#[derive(Debug, Clone)]
pub struct Probe {
    pub name: &'static str,
    pub status: ProbeStatus,
    pub why: &'static str,
    pub fix: Option<String>,
}

const NODE_FLOOR: u32 = 20;

pub fn probe_git(r: &dyn Runner) -> Probe {
    let why = "Git records every change, so nothing your agent does is unrecoverable.";
    if !r.exists("git") {
        return Probe {
            name: "git",
            status: ProbeStatus::Missing,
            why,
            fix: Some("brew install git".into()),
        };
    }
    let out = r.run("git", &["--version"]);
    Probe {
        name: "git",
        status: ProbeStatus::Ok(out.stdout.trim().to_string()),
        why,
        fix: None,
    }
}

pub fn probe_gh(r: &dyn Runner) -> Probe {
    let why = "The GitHub CLI is how a pull request gets opened for a developer to review.";
    if !r.exists("gh") {
        return Probe {
            name: "gh",
            status: ProbeStatus::Missing,
            why,
            fix: Some("brew install gh".into()),
        };
    }
    let out = r.run("gh", &["auth", "status"]);
    if out.ok() {
        return Probe {
            name: "gh",
            status: ProbeStatus::Ok("authenticated".into()),
            why,
            fix: None,
        };
    }
    Probe {
        name: "gh",
        status: ProbeStatus::Broken("installed but not logged in".into()),
        why,
        fix: Some("gh auth login".into()),
    }
}

pub fn probe_node(r: &dyn Runner) -> Probe {
    let why = "Node runs the browser automation that proves a screen actually works.";
    if !r.exists("node") {
        return Probe {
            name: "node",
            status: ProbeStatus::Missing,
            why,
            fix: Some("brew install node".into()),
        };
    }
    let out = r.run("node", &["-v"]);
    let major = out
        .stdout
        .trim()
        .trim_start_matches('v')
        .split('.')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);
    if major >= NODE_FLOOR {
        Probe {
            name: "node",
            status: ProbeStatus::Ok(out.stdout.trim().to_string()),
            why,
            fix: None,
        }
    } else {
        Probe {
            name: "node",
            status: ProbeStatus::Broken(format!(
                "{} is below the required {NODE_FLOOR}",
                out.stdout.trim()
            )),
            why,
            fix: Some("brew upgrade node".into()),
        }
    }
}

pub fn probe_playwright(r: &dyn Runner) -> Probe {
    let why = "Playwright drives a real browser, so a claim that something works can be checked.";
    let out = r.run("npx", &["playwright", "--version"]);
    if out.ok() && !out.stdout.trim().is_empty() {
        return Probe {
            name: "playwright",
            status: ProbeStatus::Ok(out.stdout.trim().to_string()),
            why,
            fix: None,
        };
    }
    Probe {
        name: "playwright",
        status: ProbeStatus::Missing,
        why,
        fix: Some("npx playwright install chromium".into()),
    }
}

/// `jq` is not a hard requirement -- the emitted Claude Code hook falls back to
/// grepping the raw JSON on stdin when `jq` is absent, so the gates still fire.
/// But that fallback matches against the whole JSON document rather than just
/// the command, which over-blocks. macOS ships without `jq`, so most product
/// managers land in the fallback unless the doctor says something.
pub fn probe_jq(r: &dyn Runner) -> Probe {
    let why = "Lets the safety gates read exactly the command being run, instead of guessing.";
    if !r.exists("jq") {
        return Probe {
            name: "jq",
            status: ProbeStatus::Missing,
            why,
            fix: Some("brew install jq".into()),
        };
    }
    Probe {
        name: "jq",
        status: ProbeStatus::Ok(r.run("jq", &["--version"]).stdout.trim().to_string()),
        why,
        fix: None,
    }
}

pub fn probe_superpowers(r: &dyn Runner, home: &Path) -> Probe {
    let why = "Superpowers holds the brainstorm, plan and review steps this workflow builds on.";
    let installed = home.join(".claude").join("plugins").exists() || r.exists("superpowers");
    if installed {
        return Probe {
            name: "superpowers",
            status: ProbeStatus::Ok("present".into()),
            why,
            fix: None,
        };
    }
    Probe {
        name: "superpowers",
        status: ProbeStatus::Missing,
        why,
        fix: Some(
            "in your agent, install the Superpowers plugin from the official marketplace".into(),
        ),
    }
}

/// `acli` wins when both are present: leaner token usage, and it works on every
/// surface rather than only the ones with MCP support.
pub fn jira_backend(r: &dyn Runner) -> JiraBackend {
    if r.exists("acli") {
        return JiraBackend::Acli;
    }
    let mcp = r.run("claude", &["mcp", "list"]);
    if mcp.ok() && mcp.stdout.to_lowercase().contains("atlassian") {
        return JiraBackend::Mcp;
    }
    JiraBackend::None
}

pub fn probe_jira(r: &dyn Runner) -> Probe {
    let why =
        "Jira access lets the agent keep your ticket's status matching what is really happening.";
    match jira_backend(r) {
        JiraBackend::Acli => Probe {
            name: "jira",
            status: ProbeStatus::Ok("acli".into()),
            why,
            fix: None,
        },
        JiraBackend::Mcp => Probe {
            name: "jira",
            status: ProbeStatus::Ok("atlassian mcp".into()),
            why,
            fix: None,
        },
        JiraBackend::None => Probe {
            name: "jira",
            status: ProbeStatus::Missing,
            why,
            fix: Some("brew install acli && acli jira auth login".into()),
        },
    }
}

pub fn run_all(r: &dyn Runner, home: &Path) -> Vec<Probe> {
    vec![
        probe_git(r),
        probe_gh(r),
        probe_node(r),
        probe_playwright(r),
        probe_jq(r),
        probe_superpowers(r, home),
        probe_jira(r),
    ]
}

fn is_ok(probes: &[Probe], name: &str) -> bool {
    probes
        .iter()
        .any(|p| p.name == name && matches!(p.status, ProbeStatus::Ok(_)))
}

/// `jq` deliberately has no `Capabilities` field: its absence changes how
/// precisely the emitted hook matches, not what the agent is allowed to do, so
/// it must never appear in the preamble.
pub fn capabilities_from(probes: &[Probe]) -> Capabilities {
    Capabilities {
        // A shell exists wherever the doctor could run at all.
        shell: true,
        playwright: is_ok(probes, "playwright") && is_ok(probes, "node"),
        superpowers: is_ok(probes, "superpowers"),
        gh: is_ok(probes, "gh"),
        jira: match probes.iter().find(|p| p.name == "jira").map(|p| &p.status) {
            Some(ProbeStatus::Ok(b)) if b == "acli" => JiraBackend::Acli,
            Some(ProbeStatus::Ok(_)) => JiraBackend::Mcp,
            _ => JiraBackend::None,
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::capabilities::JiraBackend;
    use crate::doctor::runner::FakeRunner;
    use std::path::Path;

    #[test]
    fn a_present_tool_is_reported_ok_with_no_fix() {
        let r = FakeRunner::new().with("git", 0, "git version 2.45.0");
        let p = probe_git(&r);
        assert!(matches!(p.status, ProbeStatus::Ok(_)));
        assert!(p.fix.is_none());
    }

    #[test]
    fn a_missing_tool_offers_an_exact_command_and_never_sudo() {
        let r = FakeRunner::new();
        let p = probe_git(&r);
        assert_eq!(p.status, ProbeStatus::Missing);
        let fix = p.fix.unwrap();
        assert_eq!(fix, "brew install git");
        assert!(!fix.contains("sudo"));
    }

    #[test]
    fn every_probe_explains_why_it_matters_in_plain_language() {
        let r = FakeRunner::new();
        for p in run_all(&r, Path::new("/h")) {
            assert!(!p.why.is_empty(), "{} has no explanation", p.name);
            assert!(p.why.len() < 160, "{} explanation is too long", p.name);
        }
    }

    #[test]
    fn no_fix_command_anywhere_uses_sudo() {
        let r = FakeRunner::new();
        for p in run_all(&r, Path::new("/h")) {
            if let Some(fix) = p.fix {
                assert!(!fix.contains("sudo"), "{}: {}", p.name, fix);
            }
        }
    }

    #[test]
    fn gh_present_but_unauthenticated_is_broken_not_ok() {
        let r = FakeRunner::new().with("gh", 1, "You are not logged into any GitHub hosts");
        let p = probe_gh(&r);
        assert!(matches!(p.status, ProbeStatus::Broken(_)));
        assert_eq!(p.fix.as_deref(), Some("gh auth login"));
    }

    #[test]
    fn node_below_the_floor_is_broken() {
        let r = FakeRunner::new().with("node", 0, "v18.19.0");
        assert!(matches!(probe_node(&r).status, ProbeStatus::Broken(_)));
        let r = FakeRunner::new().with("node", 0, "v20.11.0");
        assert!(matches!(probe_node(&r).status, ProbeStatus::Ok(_)));
    }

    #[test]
    fn acli_wins_when_both_jira_backends_are_present() {
        let r = FakeRunner::new().with("acli", 0, "acli 1.0.0").with(
            "claude",
            0,
            "atlassian: connected",
        );
        assert_eq!(jira_backend(&r), JiraBackend::Acli);
    }

    #[test]
    fn the_mcp_is_used_when_acli_is_absent() {
        let r = FakeRunner::new().with("claude", 0, "atlassian: connected");
        assert_eq!(jira_backend(&r), JiraBackend::Mcp);
    }

    #[test]
    fn neither_backend_present_means_no_jira() {
        assert_eq!(jira_backend(&FakeRunner::new()), JiraBackend::None);
    }

    #[test]
    fn capabilities_reflect_exactly_what_the_probes_found() {
        let empty = capabilities_from(&run_all(&FakeRunner::new(), Path::new("/h")));
        assert!(!empty.playwright);
        assert!(!empty.gh);
        assert_eq!(empty.jira, JiraBackend::None);

        let full = FakeRunner::new()
            .with("git", 0, "git version 2.45.0")
            .with("gh", 0, "Logged in to github.com")
            .with("node", 0, "v20.11.0")
            .with("npx", 0, "Version 1.50.0")
            .with("acli", 0, "acli 1.0.0");
        let caps = capabilities_from(&run_all(&full, Path::new("/h")));
        assert!(caps.playwright);
        assert!(caps.gh);
        assert_eq!(caps.jira, JiraBackend::Acli);
    }
}
