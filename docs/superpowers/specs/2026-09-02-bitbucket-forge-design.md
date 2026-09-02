# Bitbucket Cloud support via `bb` — Design

**Date:** 2026-09-02
**Status:** Approved (autonomous run, author-approved design), pending implementation plan

## Problem

pmkit assumes every product manager's code lives on GitHub. The doctor probes `gh`, the preamble
tells the agent to open pull requests with `gh`, and the shell hooks block `gh pr create` and
`gh pr merge`. A team on Bitbucket Cloud gets a doctor that nags about a tool they will never use,
a preamble that lies about how a pull request gets opened, and a hook that does not block the
command that actually opens one.

[`bb`](https://github.com/biokraft/bbcloud) is the Bitbucket Cloud equivalent of `gh`: one binary,
`bb pr create`, `bb auth status`, exit code 2 when not logged in. pmkit should let the human say
which host their team uses and adapt everything downstream of that answer.

## Scope

**In:**

- A **forge** concept: `github`, `bitbucket`, or `both`.
- The setup wizard asks one new question after the agents question: "Where does your team host
  code?" Single select, three options, default pre-selected from the project's git remote.
- Unattended runs (`setup --yes`, `skill install`, `skill refresh`, `doctor`) take `--forge
  <github|bitbucket|both>`; when omitted, the forge is detected from the git remote and falls back
  to `github` (today's behaviour) when nothing is detectable.
- Doctor probes `gh` only when the forge includes GitHub and `bb` only when it includes Bitbucket.
- The capability preamble names the right CLI for the chosen forge, and says plainly when that CLI
  is missing or not logged in. For `both` it names both and tells the agent to check the remote.
- The Claude Code and Cursor hooks additionally block `bb pr create`. `bb` has no merge command,
  so nothing to block there.
- README, `docs/targets.md`, CHANGELOG updated.

**Out:**

- GitLab, Azure DevOps, or any other host. The `Forge` type is a closed enum; adding a host later
  is a new variant plus a probe, not a redesign.
- Remembering the forge choice in the state file. Detection from the remote plus an explicit flag
  covers every run; persisting a global setting would change the state file's shape for one field.
- Teaching the pmkit skills the `bb` command surface. bbcloud ships its own `bitbucket-cloud` and
  `bbc-open-pr` skills for that. pmkit's job is to say which CLI applies and gate it.
- Blocking `bb pr comment` or other Bitbucket writes in the hook. Gate 1 covers push, merge and
  opening a PR; comments were never hook-blocked for `gh` either.

## Design

### `Forge` (new, `src/forge.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Forge { GitHub, Bitbucket, Both }

impl Forge {
    pub fn all() -> [Forge; 3];
    pub fn as_str(self) -> &'static str;          // "github" | "bitbucket" | "both"
    pub fn label(self) -> &'static str;           // "GitHub" | "Bitbucket Cloud" | "Both"
    pub fn includes_github(self) -> bool;
    pub fn includes_bitbucket(self) -> bool;
}
impl FromStr for Forge;                            // for clap, same error style as Target

/// Reads `git remote -v` in `dir`. `github.com` → GitHub, `bitbucket.org` → Bitbucket, both hosts
/// present → Both, no git / no remotes / neither host → None.
pub fn detect_forge(dir: &Path, r: &dyn Runner) -> Option<Forge>;
```

`detect_forge` goes through the existing `Runner` trait so it is testable with `FakeRunner`. The
`RealRunner` version runs `git -C <dir> remote -v`. Callers apply `unwrap_or(Forge::GitHub)`.

### `Capabilities`

```rust
pub struct Capabilities {
    pub shell: bool,
    pub playwright: bool,
    pub superpowers: bool,
    pub forge: Forge,
    pub gh: bool,   // gh installed and authenticated
    pub bb: bool,   // bb installed and authenticated
    pub jira: JiraBackend,
}
```

`all_present()` sets `forge: Forge::GitHub, gh: true, bb: true`, so golden files for existing
targets keep a single-host preamble and the `bb` field is exercised by tests that set `forge`
explicitly. `none()` sets `forge: GitHub, gh: false, bb: false`.

### Doctor

- New `probe_bb(r)`: missing → `Fix::Command("brew install biokraft/tap/bb")`; `bb auth status`
  exit 0 → `Ok("authenticated")`; any other exit → `Broken("installed but not logged in")` with
  `Fix::Command("bb auth login")`. Why-text: "The Bitbucket Cloud CLI is how a pull request gets
  opened for a developer to review."
- `run_all(r, home, forge)` includes `probe_gh` iff `forge.includes_github()` and `probe_bb` iff
  `forge.includes_bitbucket()`.
- `capabilities_from(probes, forge)` fills `forge`, `gh`, `bb`.

### Preamble

The pull-request paragraph replaces the current `gh` paragraph:

- Not in repo (Cowork, ChatGPT): unchanged text, "you cannot open a pull request. Stop after
  committing and tell the human." Forge irrelevant.
- In repo, forge GitHub: `gh` present → "Pull requests go through the `gh` command line tool
  (`gh pr create`)." absent → existing "`gh` is not installed…" sentence.
- In repo, forge Bitbucket: `bb` present → "This team hosts code on Bitbucket Cloud. Pull requests
  go through the `bb` command line tool (`bb pr create`). Never use `gh` here." absent → "`bb` is
  not installed or not logged in, so you cannot open a pull request on Bitbucket Cloud. Stop after
  committing and tell the human."
- In repo, forge Both: both of the above paragraphs, prefixed by "This team hosts code on both
  GitHub and Bitbucket Cloud. Run `git remote -v` to see which one this repository uses, then use
  only that host's tool."

### Hooks

`BLOCKED` gains one entry:

```rust
("bb( [^ ]+)* pr( [^ ]+)* create",
 "pmkit: opening a pull request needs an explicit yes from the human."),
```

Emitted for every forge. Blocking a command the team never runs costs nothing; conditional hooks
would make the golden files and the "gates active" claim depend on a runtime answer.

### Wizard and CLI

`pmkit setup`:

1. "Which agents do you use?" (unchanged)
2. "Where does your team host code?" — `inquire::Select` over `Forge::all()` labels, cursor
   starting on the detected forge (or GitHub). Skipped when `--forge` is given.

`pmkit setup --yes`, `pmkit skill install`, `pmkit skill refresh`, `pmkit doctor` all accept
`--forge`. Resolution order everywhere: flag → `detect_forge` → `GitHub`.

`run_unattended` gains a `forge: Forge` parameter. Its closing "one thing is missing" block also
covers the forge CLI: if the chosen forge's CLI probe is not ok, print one line naming the fix.

### Docs

- README: wizard bullet list gains the forge question; prerequisites lists `gh` *or* `bb` by
  host; hook list adds `bb pr create`; the "Managing" block shows `--forge`.
- `docs/targets.md`: hook pattern lists for Claude Code and Cursor add `bb pr create`.
- `CHANGELOG.md`: `## [Unreleased]` → `### Added` entry.
- `pmk-build-safely` skill: one sentence under "Never, without an explicit yes": the CLI that opens
  a pull request depends on the host; the surface section above names it.

## Testing

- `forge.rs` unit tests: round-trip strings, detection for github/bitbucket/both/none remotes via
  `FakeRunner`.
- `probes.rs`: `probe_bb` ok/broken/missing; `run_all` includes exactly the right forge probes per
  forge; `capabilities_from` fills `bb`.
- `preamble.rs`: each forge × present/absent CLI produces the right sentence and never the other
  host's instruction; Cowork/ChatGPT unchanged.
- `claude_code.rs` / `cursor_hooks.rs`: `bb pr create main --title x` is blocked (exit 2);
  `bb pr list` allowed.
- Golden files regenerated (`UPDATE_GOLDEN=1`) and the diff eyeballed: only the hook entry and
  the reworded gh paragraph change.
- `tests/cli_setup.rs`: `setup --yes --forge bitbucket` in a dir with a bitbucket remote emits a
  skill containing "Bitbucket Cloud" and never "`gh pr create`"; `--forge github` omits `bb` from
  the doctor table; a project whose remote is bitbucket.org and no flag picks Bitbucket.
- `cargo clippy -- -D warnings` clean, `cargo fmt --check` clean.
