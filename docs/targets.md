# Target reference

For each target: exactly what pmkit writes, what you do by hand, whether the gates are
machine-enforced, and how to remove it. Paths below are relative to the project directory you ran
`pmkit setup` in (`--dir <path>`, or the current directory), except Cowork and ChatGPT, which have no
repo to write into and are staged under your home directory instead.

Every write is refuse-not-overwrite: if a file pmkit wants to write already exists and pmkit didn't
put it there, pmkit leaves it alone and reports it as refused rather than touching it.

## Claude Code (terminal)

**Writes:**
- `.claude/skills/pmk-feature-loop/SKILL.md`, `pmk-shape-idea/`, `pmk-build-safely/`,
  `pmk-verify-visually/`, `pmk-jira-flow/` — one `SKILL.md` per skill
- `.claude/settings.json` — a `PreToolUse` hook, matcher `Bash`, one entry each for `git push`,
  `git merge`, `gh pr create`, `gh pr merge`

**By hand:** nothing — `pmkit setup` is sufficient. If you already had a `.claude/settings.json`,
merge pmkit's `PreToolUse` hooks into it yourself, then run `pmkit setup` again so pmkit can confirm
the gate landed.

**Gates:** machine-enforced. The hook denies the matching Bash command with exit code 2 before it
runs, printing the reason to stderr. It only catches gate 1 (push/merge/PR) — Jira writes and
off-machine commands are prose, same as everywhere else.

**Remove:** `pmkit skill uninstall --target claude-code`

## Cursor

**Writes:**
- `.cursor/rules/pmkit/pmk-feature-loop/SKILL.md` and the other four skills, same layout as Claude
  Code but under `.cursor/rules/pmkit/`
- `AGENTS.md` at the project root
- `.cursor/hooks.json` — a `beforeShellExecution` hook with the same four command patterns as Claude
  Code's

**By hand:** nothing, normally. If you already had a `.cursor/hooks.json`, merge pmkit's
`beforeShellExecution` entries in by hand and re-run `pmkit setup`.

**Gates:** machine-enforced. `beforeShellExecution` denies with exit code 2, same as Claude Code,
same four patterns, same limitation (gate 1 only).

**Remove:** `pmkit skill uninstall --target cursor`

## Codex / ChatGPT Workspace Agents

**Writes:**
- `AGENTS.md` at the project root (Codex reads this the same way Cursor does)
- `.agents/skills/pmk-feature-loop/SKILL.md` and the other four skills
- `agents/openai.yaml` — display name, short description, and default prompt for the ChatGPT
  desktop app's Skills sidebar

**By hand:** nothing to wire up — Codex and the ChatGPT desktop app both pick these files up on
their own. There is no hook file for this target, so there is nothing to merge.

**Gates:** prose only. Nothing on this surface blocks a command. The agent is the only thing
standing between the human and an action they didn't ask for.

**Remove:** `pmkit skill uninstall --target codex`

## Claude Cowork

Cowork has no repo to write into, so pmkit stages a bundle under your home directory instead of the
project you ran `pmkit setup` from.

**Writes** (under `~/pmkit-cowork/`):
- `README.md` — which folder is which skill, and a note that the gates are prose only here
- `skills/pmk-feature-loop/SKILL.md` and the other four, one folder per skill

**By hand:** upload each folder under `skills/` as a skill in Cowork yourself. Nothing is
auto-registered — pmkit only stages the files.

**Gates:** prose only. The staged `README.md` says so, and points at Cursor or Claude Code if you
want them enforced.

**Remove:** `pmkit skill uninstall --target cowork` deletes the staged bundle under
`~/pmkit-cowork/`. You still have to remove whatever you uploaded into Cowork itself — pmkit has no
way to reach into Cowork's own storage.

## ChatGPT

Also staged under your home directory, since a ChatGPT project has no filesystem of its own.

**Writes** (under `~/pmkit-chatgpt/`):
- `pmkit-chatgpt-instructions.md` — every skill's body concatenated under one preamble, meant to be
  pasted whole into a ChatGPT project's custom instructions

**By hand:** paste the file's contents into your ChatGPT project's instructions field yourself.

**Gates:** prose only, same as the rest of the non-hooked targets.

**Remove:** `pmkit skill uninstall --target chatgpt` deletes the staged file under
`~/pmkit-chatgpt/`. Delete the pasted text from the ChatGPT project yourself — pmkit cannot reach it.

## Removing everything at once

```bash
pmkit skill uninstall --all
```

## Release secrets

Releases run entirely from GitHub Actions (`release-plz.yml`, `release.yml`) — nobody pastes a
formula by hand. Three repository secrets drive the pipeline, and every job that needs one is
written to skip cleanly, not fail, when that secret is absent:

| Secret | Used for |
| --- | --- |
| `RELEASE_PLZ_TOKEN` | A personal access token release-plz uses to open the release PR and push the version tag. It has to be a PAT rather than the default `GITHUB_TOKEN`: a tag pushed with `GITHUB_TOKEN` does not trigger this repo's `release` workflow (GitHub suppresses that recursion deliberately), so the release job would never start. |
| `CARGO_REGISTRY_TOKEN` | Publishing the crate to crates.io as part of the release-plz `release` command. |
| `TAP_PAT` | A PAT with write access to `biokraft/homebrew-tap`, used by the `formula` job to clone the tap, write the rendered `Formula/pmkit.rb`, and push it. |

None of these values belong in this file, in an issue, or in a commit message — only their names.
