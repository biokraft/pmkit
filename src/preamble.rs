use crate::capabilities::{Capabilities, JiraBackend};
use crate::target::Target;

/// The only text that differs between targets. Everything the skill bodies are
/// allowed to assume about this surface is stated here, including — especially —
/// what is absent.
pub fn preamble(target: Target, caps: &Capabilities) -> String {
    let mut out = String::new();
    out.push_str("## Your surface\n\n");
    out.push_str(&format!(
        "You are running on **{}**. pmkit wrote this section; the rules below it are the same everywhere.\n\n",
        target.label()
    ));

    if target.enforces_gates_with_hooks() {
        out.push_str(
            "The safety gates in this skill are **machine-enforced** here: a blocked command fails \
             before it runs. Do not treat that as permission to skip asking — the human still \
             decides.\n\n",
        );
    } else {
        out.push_str(
            "The safety gates in this skill are **prose only** here: nothing on this surface \
             blocks a command for you, so they cannot be blocked automatically. You are the only \
             thing standing between the human and an action they did not ask for. Follow them \
             exactly.\n\n",
        );
    }

    if caps.shell && target.is_in_repo() {
        out.push_str(
            "You have a shell. Run commands yourself, and show the human what you ran.\n\n",
        );
    } else {
        out.push_str(
            "You have **no shell** on this surface. When a step needs a command, print it in a \
             copyable block and ask the human to run it and paste the output. Never claim a \
             command's result you did not see.\n\n",
        );
    }

    if caps.playwright && target.is_in_repo() {
        out.push_str("A browser is available through Playwright. Use it to verify UI work.\n\n");
    } else {
        out.push_str(
            "**You CANNOT verify anything visually** — no browser is available. Say so plainly \
             instead of implying the change was checked.\n\n",
        );
    }

    if !caps.superpowers {
        out.push_str(
            "**Superpowers is NOT available** on this surface. The build stage depends on it, so \
             stop at the spec and tell the human what to install before going further.\n\n",
        );
    }

    if !caps.gh || !target.is_in_repo() {
        out.push_str(
            "`gh` is not installed, so you cannot open a pull request. Stop after committing and \
             tell the human.\n\n",
        );
    }

    match caps.jira {
        JiraBackend::Acli if target.is_in_repo() => out.push_str(
            "Jira access is through the `acli` command line tool. Use it for every Jira read and \
             write.\n\n",
        ),
        JiraBackend::Acli => out.push_str(
            "Jira access is through the `acli` command line tool, but you have **no shell** on \
             this surface. Print the `acli` command in a copyable block and ask the human to run \
             it and paste the output. Never claim a Jira read or write you did not see the human \
             run.\n\n",
        ),
        JiraBackend::Mcp => out.push_str(
            "Jira access is through the Atlassian MCP tools. Use them for every Jira read and \
             write.\n\n",
        ),
        JiraBackend::None => out.push_str(
            "You have **no Jira access** configured. Do not invent ticket state; ask the human to \
             tell you the ticket, or to set one up.\n\n",
        ),
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::{Capabilities, JiraBackend};
    use crate::target::Target;

    #[test]
    fn a_hooked_target_says_the_gates_are_enforced() {
        let text = preamble(Target::ClaudeCode, &Capabilities::all_present());
        assert!(text.contains("machine-enforced"));
        assert!(!text.contains("prose only"));
    }

    #[test]
    fn a_prose_only_target_says_so_out_loud() {
        let text = preamble(Target::Cowork, &Capabilities::all_present());
        assert!(text.contains("prose only"));
        assert!(text.contains("cannot be blocked automatically"));
    }

    #[test]
    fn a_missing_browser_forbids_claiming_visual_verification() {
        let caps = Capabilities {
            playwright: false,
            ..Capabilities::all_present()
        };
        let text = preamble(Target::ClaudeCode, &caps);
        assert!(text.contains("You CANNOT verify anything visually"));
        assert!(text.contains("Say so plainly"));
    }

    #[test]
    fn a_present_browser_never_emits_the_prohibition() {
        let text = preamble(Target::ClaudeCode, &Capabilities::all_present());
        assert!(!text.contains("You CANNOT verify anything visually"));
    }

    #[test]
    fn the_jira_backend_is_named_exactly_once_and_never_guessed() {
        let acli = preamble(
            Target::Codex,
            &Capabilities {
                jira: JiraBackend::Acli,
                ..Capabilities::all_present()
            },
        );
        assert!(acli.contains("`acli`"));
        assert!(!acli.contains("Atlassian MCP"));

        let mcp = preamble(
            Target::Codex,
            &Capabilities {
                jira: JiraBackend::Mcp,
                ..Capabilities::all_present()
            },
        );
        assert!(mcp.contains("Atlassian MCP"));
        assert!(!mcp.contains("`acli`"));

        let none = preamble(
            Target::Codex,
            &Capabilities {
                jira: JiraBackend::None,
                ..Capabilities::all_present()
            },
        );
        assert!(none.contains("no Jira access"));
    }

    #[test]
    fn a_shell_less_target_with_acli_tells_the_human_to_run_it_not_the_agent() {
        let text = preamble(
            Target::ChatGpt,
            &Capabilities {
                jira: JiraBackend::Acli,
                ..Capabilities::all_present()
            },
        );
        assert!(text.contains("ask the human to run it and paste the output"));
        assert!(!text.contains("Use it for every Jira read and write"));
    }

    #[test]
    fn a_shell_less_target_tells_the_agent_to_ask_the_human_to_run_commands() {
        let text = preamble(Target::ChatGpt, &Capabilities::all_present());
        assert!(text.contains("no shell"));
        assert!(text.contains("ask the human to run it and paste the output"));
    }

    #[test]
    fn missing_superpowers_is_reported_as_a_blocking_gap_for_the_build_stage() {
        let caps = Capabilities {
            superpowers: false,
            ..Capabilities::all_present()
        };
        let text = preamble(Target::ClaudeCode, &caps);
        assert!(text.contains("Superpowers is NOT available"));
    }

    #[test]
    fn cowork_with_all_caps_still_cannot_verify_visually_or_use_gh() {
        let text = preamble(Target::Cowork, &Capabilities::all_present());
        assert!(text.contains("You CANNOT verify anything visually"));
        assert!(!text.contains("A browser is available"));
        assert!(text.contains("cannot open a pull request"));
    }

    #[test]
    fn chatgpt_with_all_caps_still_cannot_verify_visually_or_use_gh() {
        let text = preamble(Target::ChatGpt, &Capabilities::all_present());
        assert!(text.contains("You CANNOT verify anything visually"));
        assert!(!text.contains("A browser is available"));
        assert!(text.contains("cannot open a pull request"));
    }

    #[test]
    fn every_target_and_capability_combination_produces_a_non_empty_preamble() {
        for t in Target::all() {
            for caps in [Capabilities::none(), Capabilities::all_present()] {
                assert!(!preamble(t, &caps).trim().is_empty(), "{}", t.as_str());
            }
        }
    }
}
