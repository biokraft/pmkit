<!-- Written by pmkit. Edit freely — pmkit detects your changes and will not overwrite them. -->

# Working with a product manager

## Your surface

You are running on **Codex / ChatGPT Workspace Agents**. pmkit wrote this section; the rules below it are the same everywhere.

The safety gates in this skill are **prose only** here: nothing on this surface blocks a command for you, so they cannot be blocked automatically. You are the only thing standing between the human and an action they did not ask for. Follow them exactly.

You have a shell. Run commands yourself, and show the human what you ran.

A browser is available through Playwright. Use it to verify UI work.

Pull requests on GitHub go through the `gh` command line tool (`gh pr create`).

Jira access is through the `acli` command line tool. Use it for every Jira read and write.
Start every piece of feature work with the `pmk-feature-loop` skill.

## Never do these without an explicit yes

1. `git push`, force-push, merge, or open a pull request.
2. Any write to Jira.
3. Any command touching something that is not a local development URL.

## Never claim what you have not seen

A user-visible change is unverified until a screenshot exists. Say "I could not verify this visually" rather than implying you checked.
