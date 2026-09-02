# Changelog

All notable changes to pmkit are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and pmkit adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Bitbucket Cloud support.** `pmkit setup` now asks whether your team hosts code on GitHub,
  Bitbucket Cloud, or both, guessing from the git remote first. Choose Bitbucket and the doctor
  checks [`bb`](https://github.com/biokraft/bbcloud) instead of `gh`, the skills tell the agent to
  open pull requests with `bb pr create`, and the Claude Code and Cursor hooks block that command
  until a human says yes. `--forge github|bitbucket|both` skips the question on `setup`,
  `skill install`, `skill refresh` and `doctor`.

## [0.1.0] - 2026-09-02

First release.

### Added

- **`pmkit setup`** — a one-time guided install. Run it inside a project and it asks which coding
  agents you use, probes the machine, writes the skill files, and exits. Nothing stays running.
  `--yes` installs into every target unattended; `--target <agent>` installs into one.
- **Five skills**, authored once and emitted to five agents (Claude Code, Cursor,
  Codex / ChatGPT Workspace Agents, Claude Cowork, ChatGPT):
  - `pmk-feature-loop` — the outer loop, and the three gates that need a human's yes
  - `pmk-shape-idea` — turn a vague idea into a spec small enough to build
  - `pmk-build-safely` — build one reviewed step at a time
  - `pmk-verify-visually` — open the thing in a browser and look at it before calling it done
  - `pmk-jira-flow` — keep the ticket's status matching reality
- **Three gates the agent must ask about**: pushing, merging or opening a pull request; any write to
  Jira, a status transition included; and anything that leaves the machine. Each command needs its
  own yes, so a yes to one push is not a yes to a force-push over it.
- **Machine enforcement where the agent supports it.** Claude Code gets `PreToolUse` hooks and
  Cursor gets `beforeShellExecution` hooks, both of which deny `git push`, `git merge`,
  `gh pr create` and `gh pr merge` outright. The three prose-only agents say plainly that their
  gates are prose.
- **A capability preamble per agent**, generated from what the machine actually has. An agent with
  no shell is never told it can run commands, and an agent with no Playwright is never told it can
  open a browser.
- **`pmkit doctor`** — a read-only report on git, GitHub CLI, Node, Playwright, `jq`, Superpowers
  and Jira (`acli` or the Atlassian MCP, whichever is present). Fixes are split into commands you
  can paste and steps you have to do yourself. No probe and no fix uses `sudo`, and doctor never
  changes anything.
- **`pmkit skill`** — `install`, `refresh`, `list` and `uninstall`, for driving the files directly.
- pmkit records a checksum of every file it writes, so it never overwrites a file it did not create.
  Edit anything it installed and the next run leaves your version alone and says so.
- **Install paths**: Homebrew (`brew install biokraft/tap/pmkit`), `install.sh` (verifies the
  checksum before installing, needs no `sudo`), Nix flake, and `cargo install pmkit`.

### Notes for this first release

- Of the four gate hooks, only `git push` has been tested by executing it. The other three are
  covered by snapshot equality against the same generator.
- The Homebrew formula and the Nix flake are exercised for the first time by this release.
