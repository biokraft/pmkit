# pmkit

pmkit installs a safe, human-in-the-loop agentic workflow — shape an idea, build it one reviewed
step at a time, verify it in a browser, keep the Jira ticket honest — into whichever coding agent
you actually use. It is a one-shot installer, not a daemon: it writes files and gets out of the way.

## Install

```bash
brew install biokraft/tap/pmkit
```

## `pmkit setup`

Run `pmkit setup` inside the project you want it in. It asks, one question at a time:

- which of the five targets below you use (or `--yes` to install into every target, `--target
  <agent>` to install into just one, then remove any others later with `pmkit skill uninstall
  --target <agent>`)
- nothing else — there is no API key, no account, no network call beyond what `pmkit doctor`'s
  probes make to check whether your local tools are installed

It never overwrites a file you already had. If a target's config file already exists, pmkit leaves
it alone, reports it as refused, and tells you the gates are not active there until you merge
pmkit's version in by hand.

## The five targets

| Target | What it gets |
| --- | --- |
| Claude Code (terminal) | Skills under `.claude/skills/`, a `PreToolUse` hook in `.claude/settings.json` |
| Cursor | Rules under `.cursor/rules/pmkit/`, an `AGENTS.md`, a `beforeShellExecution` hook in `.cursor/hooks.json` |
| Codex / ChatGPT Workspace Agents | `AGENTS.md`, skills under `.agents/skills/`, and `agents/openai.yaml` for the ChatGPT desktop app's Skills sidebar |
| Claude Cowork | A bundle staged under `~/pmkit-cowork/` — no repo to write into, so you upload each skill folder by hand |
| ChatGPT | Instructions staged under `~/pmkit-chatgpt/` — paste `pmkit-chatgpt-instructions.md` into your project instructions |

Codex also covers ChatGPT Workspace Agents: the same Codex-powered surface, aimed at non-engineers
working from a task list rather than a terminal.

See `docs/targets.md` for the exact paths and the by-hand steps for each.

## The three hard gates

Every skill states the same three things that need an explicit yes from the human, every time, no
matter which target reads it:

1. **`git push`, force-push, merge, or opening a pull request.** Show the branch, the commits, and
   the target, then ask.
2. **Any write to Jira** — a comment, a description, a transition. Show the exact text first.
3. **Any command that reaches outside this machine** — a deploy, an API call, anything that is not
   a local development URL.

## Where the gates are enforced, and where they are not

Two of the five targets back these gates with software: **Claude Code**, via a `PreToolUse` hook in
`.claude/settings.json`, and **Cursor**, via a `beforeShellExecution` hook in `.cursor/hooks.json`.
Both deny the matching command with exit code 2 before it runs.

Even there, the hook only catches gate 1 — `git push`, `git merge`, `gh pr create`, `gh pr merge`.
Gates 2 and 3 (Jira writes, anything leaving the machine) are not something a shell hook can see
coming, so they stay prose everywhere, Claude Code and Cursor included.

**Codex / ChatGPT Workspace Agents, Claude Cowork, and ChatGPT enforce nothing.** All three gates
are written instructions there and nothing more — no hook, no block, no exit code. If you choose one
of these targets, the agent itself is the only thing standing between the human and an action they
did not ask for. Learn this now, not from an incident.

## Prerequisites

`pmkit doctor` checks these and never fixes anything without you asking — it is read-only. It prints
two lists: shell commands you can paste and run, and manual steps that happen somewhere else (in an
agent, in a browser).

- **git** — records every change, so nothing your agent does is unrecoverable.
- **gh** — the GitHub CLI is how a pull request gets opened for a developer to review.
- **Node 20+** — runs the browser automation that proves a screen actually works.
- **Playwright** (`npx playwright install chromium`) — drives a real browser, so a claim that
  something works can be checked.
- **jq** — lets the safety gates read exactly the command being run instead of guessing. Not hard
  required: the emitted hook falls back to grepping the raw JSON without it, but that fallback
  over-blocks because it matches the whole payload, not just the command.
- **Superpowers** — holds the brainstorm, plan, and review steps this workflow is built on.
- **A Jira backend** — `acli` if you have it (works everywhere, leaner), otherwise the Atlassian MCP
  server. Either way, this is how the agent keeps your ticket's status matching what is really
  happening.

## Managing what pmkit installed

```bash
pmkit doctor                                   # check prerequisites; never fixes anything itself
pmkit skill list                                # every tracked file: current, modified, or missing
pmkit skill refresh                             # re-emit already-installed targets; won't restore
                                                 # files you deliberately deleted
pmkit skill uninstall --target claude-code      # remove one target
pmkit skill uninstall --all                     # remove everything pmkit wrote
```

`uninstall` requires either `--target <agent>` or `--all` — there is no bare `pmkit skill uninstall`.

## Upgrading

```bash
brew update && brew upgrade
```

`brew upgrade` on its own does not refresh the tap, so it will not see a new pmkit release. There is
no `pmkit update` command in this version — `brew` is the upgrade path.

## Alternatives to Homebrew

No Homebrew? Install a checked binary directly:

```bash
curl -fsSL https://raw.githubusercontent.com/biokraft/pmkit/main/install.sh | sh
```

This drops `pmkit` into `~/.local/bin` (override with `PMKIT_INSTALL_DIR`) — no privilege escalation,
no Rust toolchain required. It supports macOS (arm64, x86_64) and Linux (x86_64, aarch64).

Or grab a prebuilt binary straight from the [release page](https://github.com/biokraft/pmkit/releases)
— each release ships a `.tar.gz` and a `.sha256` for every one of those four platforms.

Or, with Nix:

```bash
nix run github:biokraft/pmkit
```

`flake.nix` and `package.nix` build pmkit with `rustPlatform.buildRustPackage`, so `nix build`,
`nix profile install`, or an overlay (`overlays.default`) all work the same way.
