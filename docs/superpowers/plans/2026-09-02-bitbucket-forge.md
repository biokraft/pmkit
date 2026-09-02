# Bitbucket Cloud Forge Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `pmkit setup` ask whether the team hosts code on GitHub, Bitbucket Cloud, or both, and make the doctor, preamble and hooks follow that answer.

**Architecture:** A new closed enum `Forge` lives in `src/forge.rs` with remote-based detection through the existing `Runner` trait. `Capabilities` carries the forge plus a `bb` flag next to `gh`; the doctor probes only the chosen host's CLI; the preamble emits one pull-request paragraph per included host; the hook `BLOCKED` table gains `bb pr create` unconditionally. Every CLI entry point gets `--forge`, resolved flag → detection → GitHub.

**Tech Stack:** Rust 1.88 (edition 2021), clap 4 derive, inquire 0.7, serde_json, assert_cmd/predicates/tempfile for integration tests. `cargo clippy -- -D warnings` with `unwrap_used`/`expect_used` denied outside `#[cfg(test)]` modules (which use `#![allow(clippy::unwrap_used)]`).

**Spec:** `docs/superpowers/specs/2026-09-02-bitbucket-forge-design.md`

## Global Constraints

- `#![forbid(unsafe_code)]` stays. No `unwrap`/`expect` outside test modules.
- No fix command anywhere uses `sudo` (test `no_fix_command_anywhere_uses_sudo` enforces).
- Probe `why` text under 160 chars.
- Hook messages must not contain `'` (test enforces).
- Golden files: regenerate with `UPDATE_GOLDEN=1 cargo test --test emit_golden`, then re-run without the env var and inspect `git diff tests/golden` before committing.
- Prose style in user-facing text: plain language, no em-dash-heavy AI cadence, matches existing README voice.
- Commit after every task with a conventional-commit subject.

---

### Task 1: `Forge` type and remote detection

**Files:**
- Create: `src/forge.rs`
- Modify: `src/lib.rs` (add `pub mod forge;`)

**Interfaces:**
- Produces:
  ```rust
  pub enum Forge { GitHub, Bitbucket, Both }
  impl Forge {
      pub fn all() -> [Forge; 3];
      pub fn as_str(self) -> &'static str;   // "github" | "bitbucket" | "both"
      pub fn label(self) -> &'static str;    // "GitHub" | "Bitbucket Cloud" | "Both"
      pub fn includes_github(self) -> bool;
      pub fn includes_bitbucket(self) -> bool;
  }
  impl std::str::FromStr for Forge { type Err = crate::error::PmError; }
  pub fn detect_forge(dir: &Path, r: &dyn Runner) -> Option<Forge>;
  ```

- [ ] **Step 1: Write the file with tests first**

```rust
// src/forge.rs
use crate::doctor::runner::Runner;
use crate::error::PmError;
use std::path::Path;

/// Where the team's code is hosted. Decides which pull-request CLI the doctor
/// probes and the preamble names. Closed on purpose: a new host is a new
/// variant plus a probe, nothing more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Forge {
    GitHub,
    Bitbucket,
    Both,
}

impl Forge {
    pub fn all() -> [Forge; 3] {
        [Forge::GitHub, Forge::Bitbucket, Forge::Both]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Forge::GitHub => "github",
            Forge::Bitbucket => "bitbucket",
            Forge::Both => "both",
        }
    }

    /// Shown in the wizard's picker.
    pub fn label(self) -> &'static str {
        match self {
            Forge::GitHub => "GitHub",
            Forge::Bitbucket => "Bitbucket Cloud",
            Forge::Both => "Both",
        }
    }

    pub fn includes_github(self) -> bool {
        matches!(self, Forge::GitHub | Forge::Both)
    }

    pub fn includes_bitbucket(self) -> bool {
        matches!(self, Forge::Bitbucket | Forge::Both)
    }
}

impl std::str::FromStr for Forge {
    type Err = PmError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Forge::all()
            .into_iter()
            .find(|f| f.as_str() == s)
            .ok_or_else(|| {
                PmError::Config(format!(
                    "unknown forge `{s}` — expected one of: {}",
                    Forge::all()
                        .iter()
                        .map(|f| f.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
    }
}

/// Guesses the forge from the project's git remotes. `github.com` means
/// GitHub, `bitbucket.org` means Bitbucket Cloud, both hosts present means
/// Both. Returns `None` when there is no git, no remote, or neither host,
/// so the caller can fall back to a default rather than pmkit guessing.
pub fn detect_forge(dir: &Path, r: &dyn Runner) -> Option<Forge> {
    let dir = dir.to_string_lossy();
    let out = r.run("git", &["-C", &dir, "remote", "-v"]);
    if !out.ok() {
        return None;
    }
    let text = out.stdout.to_lowercase();
    let github = text.contains("github.com");
    let bitbucket = text.contains("bitbucket.org");
    match (github, bitbucket) {
        (true, true) => Some(Forge::Both),
        (true, false) => Some(Forge::GitHub),
        (false, true) => Some(Forge::Bitbucket),
        (false, false) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doctor::runner::FakeRunner;
    use std::path::Path;

    #[test]
    fn every_forge_round_trips_through_its_string() {
        for f in Forge::all() {
            assert_eq!(f.as_str().parse::<Forge>().ok(), Some(f), "{}", f.as_str());
        }
    }

    #[test]
    fn an_unknown_forge_is_rejected_and_lists_the_valid_ones() {
        let err = "gitlab".parse::<Forge>().unwrap_err().to_string();
        assert!(err.contains("github, bitbucket, both"), "{err}");
    }

    #[test]
    fn both_includes_each_host_and_the_singles_exclude_the_other() {
        assert!(Forge::Both.includes_github() && Forge::Both.includes_bitbucket());
        assert!(Forge::GitHub.includes_github() && !Forge::GitHub.includes_bitbucket());
        assert!(Forge::Bitbucket.includes_bitbucket() && !Forge::Bitbucket.includes_github());
    }

    #[test]
    fn a_github_remote_is_detected() {
        let r = FakeRunner::new().with(
            "git",
            0,
            "origin\tgit@github.com:biokraft/pmkit.git (fetch)\norigin\tgit@github.com:biokraft/pmkit.git (push)\n",
        );
        assert_eq!(detect_forge(Path::new("/p"), &r), Some(Forge::GitHub));
    }

    #[test]
    fn a_bitbucket_remote_is_detected() {
        let r = FakeRunner::new().with(
            "git",
            0,
            "origin\thttps://bitbucket.org/acme/api.git (fetch)\n",
        );
        assert_eq!(detect_forge(Path::new("/p"), &r), Some(Forge::Bitbucket));
    }

    #[test]
    fn mixed_remotes_are_detected_as_both() {
        let r = FakeRunner::new().with(
            "git",
            0,
            "origin\tgit@bitbucket.org:acme/api.git (fetch)\nmirror\thttps://github.com/acme/api.git (fetch)\n",
        );
        assert_eq!(detect_forge(Path::new("/p"), &r), Some(Forge::Both));
    }

    #[test]
    fn no_git_no_remote_or_an_unknown_host_is_none() {
        assert_eq!(detect_forge(Path::new("/p"), &FakeRunner::new()), None);
        let empty = FakeRunner::new().with("git", 0, "");
        assert_eq!(detect_forge(Path::new("/p"), &empty), None);
        let gitlab = FakeRunner::new().with("git", 0, "origin\tgit@gitlab.com:x/y.git (fetch)\n");
        assert_eq!(detect_forge(Path::new("/p"), &gitlab), None);
    }
}
```

Add `#![allow(clippy::unwrap_used)]` is NOT needed at file level; instead put `#[allow(clippy::unwrap_used)]` on the `mod tests` line (matches `probes.rs`).

- [ ] **Step 2: Register the module**

In `src/lib.rs` add `pub mod forge;` between `pub mod error;` and `pub mod preamble;` (alphabetical).

- [ ] **Step 3: Run tests**

Run: `cargo test forge::`
Expected: 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/forge.rs src/lib.rs
git commit -m "feat: add Forge enum with git-remote detection"
```

---

### Task 2: Capabilities and doctor learn about the forge

**Files:**
- Modify: `src/capabilities.rs`
- Modify: `src/doctor/probes.rs` (`probe_bb`, `run_all`, `capabilities_from`, tests)
- Modify: `src/wizard.rs:89-91` and `src/main.rs` (`Doctor` arm) — only enough to compile, passing `Forge::GitHub`. Task 5 wires real values.

**Interfaces:**
- Consumes: `crate::forge::Forge` from Task 1.
- Produces:
  ```rust
  pub struct Capabilities { shell, playwright, superpowers, forge: Forge, gh: bool, bb: bool, jira }
  pub fn probe_bb(r: &dyn Runner) -> Probe;
  pub fn run_all(r: &dyn Runner, home: &Path, forge: Forge) -> Vec<Probe>;
  pub fn capabilities_from(probes: &[Probe], forge: Forge) -> Capabilities;
  ```

- [ ] **Step 1: Extend `Capabilities`**

```rust
use crate::forge::Forge;

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
```

`none()`: `forge: Forge::GitHub, gh: false, bb: false`. `all_present()`: `forge: Forge::GitHub, gh: true, bb: true`. Add a doc comment on `all_present` saying the forge defaults to GitHub so single-host golden files stay stable; callers that know the forge override it with struct-update syntax.

- [ ] **Step 2: Write failing probe tests**

Append to `mod tests` in `src/doctor/probes.rs`:

```rust
    #[test]
    fn bb_present_and_authenticated_is_ok() {
        let r = FakeRunner::new().with("bb", 0, "email  x@y.z");
        let p = probe_bb(&r);
        assert!(matches!(p.status, ProbeStatus::Ok(_)));
        assert!(p.fix.is_none());
    }

    #[test]
    fn bb_present_but_unauthenticated_is_broken_not_ok() {
        let r = FakeRunner::new().with("bb", 2, "");
        let p = probe_bb(&r);
        assert!(matches!(p.status, ProbeStatus::Broken(_)));
        assert_eq!(p.fix, Some(Fix::Command("bb auth login".into())));
    }

    #[test]
    fn bb_missing_offers_the_tap_install() {
        let p = probe_bb(&FakeRunner::new());
        assert_eq!(p.status, ProbeStatus::Missing);
        assert_eq!(p.fix, Some(Fix::Command("brew install biokraft/tap/bb".into())));
    }

    #[test]
    fn run_all_probes_only_the_chosen_forges_cli() {
        let names = |forge: Forge| -> Vec<&'static str> {
            run_all(&FakeRunner::new(), Path::new("/h"), forge)
                .iter()
                .map(|p| p.name)
                .collect()
        };
        let gh_only = names(Forge::GitHub);
        assert!(gh_only.contains(&"gh") && !gh_only.contains(&"bb"));
        let bb_only = names(Forge::Bitbucket);
        assert!(bb_only.contains(&"bb") && !bb_only.contains(&"gh"));
        let both = names(Forge::Both);
        assert!(both.contains(&"gh") && both.contains(&"bb"));
    }

    #[test]
    fn capabilities_carry_the_forge_and_the_bb_flag() {
        let r = FakeRunner::new().with("bb", 0, "ok");
        let caps = capabilities_from(&run_all(&r, Path::new("/h"), Forge::Bitbucket), Forge::Bitbucket);
        assert_eq!(caps.forge, Forge::Bitbucket);
        assert!(caps.bb);
        assert!(!caps.gh);
    }
```

Add `use crate::forge::Forge;` to the test module imports. Update every existing `run_all(&r, Path::new("/h"))` call in tests to `run_all(&r, Path::new("/h"), Forge::Both)` (Both, so the "every probe" loops keep covering gh). Update `capabilities_from(...)` calls to pass `Forge::Both` (or `Forge::GitHub` where the test asserts only on gh; `Both` is fine everywhere).

- [ ] **Step 3: Run to see failures**

Run: `cargo test doctor::probes`
Expected: compile errors (`probe_bb` missing, wrong arity).

- [ ] **Step 4: Implement**

In `src/doctor/probes.rs`:

```rust
use crate::forge::Forge;

pub fn probe_bb(r: &dyn Runner) -> Probe {
    let why = "The Bitbucket Cloud CLI is how a pull request gets opened for a developer to review.";
    if !r.exists("bb") {
        return Probe {
            name: "bb",
            status: ProbeStatus::Missing,
            why,
            fix: Some(Fix::Command("brew install biokraft/tap/bb".into())),
        };
    }
    // `bb auth status` exits 0 when logged in and 2 when not.
    let out = r.run("bb", &["auth", "status"]);
    if out.ok() {
        return Probe {
            name: "bb",
            status: ProbeStatus::Ok("authenticated".into()),
            why,
            fix: None,
        };
    }
    Probe {
        name: "bb",
        status: ProbeStatus::Broken("installed but not logged in".into()),
        why,
        fix: Some(Fix::Command("bb auth login".into())),
    }
}

pub fn run_all(r: &dyn Runner, home: &Path, forge: Forge) -> Vec<Probe> {
    let mut probes = vec![probe_git(r)];
    if forge.includes_github() {
        probes.push(probe_gh(r));
    }
    if forge.includes_bitbucket() {
        probes.push(probe_bb(r));
    }
    probes.extend([
        probe_node(r),
        probe_playwright(r),
        probe_jq(r),
        probe_superpowers(r, home),
        probe_jira(r),
    ]);
    probes
}

pub fn capabilities_from(probes: &[Probe], forge: Forge) -> Capabilities {
    Capabilities {
        shell: true,
        playwright: is_ok(probes, "playwright") && is_ok(probes, "node"),
        superpowers: is_ok(probes, "superpowers"),
        forge,
        gh: is_ok(probes, "gh"),
        bb: is_ok(probes, "bb"),
        jira: /* unchanged */,
    }
}
```

Compile fixes (temporary, Task 5 replaces): `src/wizard.rs` `run_unattended` → `probes::run_all(&RealRunner, home, Forge::GitHub)` and `probes::capabilities_from(&probes, Forge::GitHub)`; `src/main.rs` Doctor arm → `run_all(&RealRunner, &home_dir(), pmkit::forge::Forge::GitHub)`. Add `use crate::forge::Forge;` in wizard.rs.

- [ ] **Step 5: Run full tests and clippy**

Run: `cargo test && cargo clippy --all-targets -- -D warnings`
Expected: all pass. Golden tests still pass because `all_present` still has `forge: GitHub` and preamble is untouched.

- [ ] **Step 6: Commit**

```bash
git add src/capabilities.rs src/doctor/probes.rs src/wizard.rs src/main.rs
git commit -m "feat: probe bb and carry the forge in Capabilities"
```

---

### Task 3: Preamble names the right pull-request CLI

**Files:**
- Modify: `src/preamble.rs:58-63` (replace the `gh` paragraph) and its tests
- Modify: `tests/golden/**` (regenerate)

**Interfaces:**
- Consumes: `Capabilities { forge, gh, bb }` from Task 2.

- [ ] **Step 1: Write failing tests**

Replace the two `..._still_cannot_verify_visually_or_use_gh` assertions' expectations with the shared string `"cannot open a pull request"` (they already use it, keep them). Add:

```rust
    #[test]
    fn a_github_team_with_gh_is_told_to_use_gh_and_never_hears_about_bb() {
        let text = preamble(Target::ClaudeCode, &Capabilities::all_present());
        assert!(text.contains("`gh pr create`"), "{text}");
        assert!(!text.contains("`bb`"), "{text}");
        assert!(!text.contains("Bitbucket"), "{text}");
    }

    #[test]
    fn a_github_team_without_gh_cannot_open_a_pull_request() {
        let caps = Capabilities { gh: false, ..Capabilities::all_present() };
        let text = preamble(Target::ClaudeCode, &caps);
        assert!(text.contains("`gh` is not installed"), "{text}");
        assert!(text.contains("cannot open a pull request"), "{text}");
    }

    #[test]
    fn a_bitbucket_team_with_bb_is_told_to_use_bb_and_never_gh() {
        let caps = Capabilities { forge: Forge::Bitbucket, ..Capabilities::all_present() };
        let text = preamble(Target::ClaudeCode, &caps);
        assert!(text.contains("Bitbucket Cloud"), "{text}");
        assert!(text.contains("`bb pr create`"), "{text}");
        assert!(text.contains("Never use `gh` here"), "{text}");
        assert!(!text.contains("`gh pr create`"), "{text}");
    }

    #[test]
    fn a_bitbucket_team_without_bb_cannot_open_a_pull_request() {
        let caps = Capabilities { forge: Forge::Bitbucket, bb: false, ..Capabilities::all_present() };
        let text = preamble(Target::ClaudeCode, &caps);
        assert!(text.contains("`bb` is not installed or not logged in"), "{text}");
        assert!(text.contains("cannot open a pull request on Bitbucket Cloud"), "{text}");
    }

    #[test]
    fn a_team_on_both_hosts_is_told_to_check_the_remote_and_hears_about_both_tools() {
        let caps = Capabilities { forge: Forge::Both, ..Capabilities::all_present() };
        let text = preamble(Target::ClaudeCode, &caps);
        assert!(text.contains("git remote -v"), "{text}");
        assert!(text.contains("`gh pr create`"), "{text}");
        assert!(text.contains("`bb pr create`"), "{text}");
    }

    #[test]
    fn a_shell_less_target_never_names_a_pull_request_tool_whatever_the_forge() {
        for forge in Forge::all() {
            let caps = Capabilities { forge, ..Capabilities::all_present() };
            for t in [Target::Cowork, Target::ChatGpt] {
                let text = preamble(t, &caps);
                assert!(text.contains("cannot open a pull request"), "{text}");
                assert!(!text.contains("pr create"), "{text}");
            }
        }
    }
```

Add `use crate::forge::Forge;` to the test imports.

- [ ] **Step 2: Run to see failures**

Run: `cargo test preamble::`
Expected: the new tests fail (no `gh pr create` text yet, no Bitbucket text).

- [ ] **Step 3: Implement**

Replace lines 58-63 of `src/preamble.rs` with:

```rust
    if !target.is_in_repo() {
        out.push_str(
            "You cannot open a pull request from this surface. Stop after committing and tell \
             the human.\n\n",
        );
    } else {
        if caps.forge == Forge::Both {
            out.push_str(
                "This team hosts code on both GitHub and Bitbucket Cloud. Run `git remote -v` to \
                 see which one this repository uses, then use only that host's tool.\n\n",
            );
        }
        if caps.forge.includes_github() {
            if caps.gh {
                out.push_str(
                    "Pull requests on GitHub go through the `gh` command line tool \
                     (`gh pr create`).\n\n",
                );
            } else {
                out.push_str(
                    "`gh` is not installed, so you cannot open a pull request. Stop after \
                     committing and tell the human.\n\n",
                );
            }
        }
        if caps.forge.includes_bitbucket() {
            if caps.bb {
                out.push_str(
                    "This team hosts code on Bitbucket Cloud. Pull requests go through the `bb` \
                     command line tool (`bb pr create`). Never use `gh` here.\n\n",
                );
            } else {
                out.push_str(
                    "`bb` is not installed or not logged in, so you cannot open a pull request on \
                     Bitbucket Cloud. Stop after committing and tell the human.\n\n",
                );
            }
        }
    }
```

Add `use crate::forge::Forge;` at the top. For `Forge::Both` the Bitbucket sentence "This team hosts code on Bitbucket Cloud." repeats the header; make the Bitbucket positive sentence start with "Pull requests on Bitbucket Cloud go through the `bb` command line tool (`bb pr create`). Never use `gh` here." instead, so it reads well alone and under the Both header. Keep the test assertion `contains("Bitbucket Cloud")` satisfied.

- [ ] **Step 4: Regenerate goldens and inspect**

Run: `UPDATE_GOLDEN=1 cargo test --test emit_golden && cargo test && git diff --stat tests/golden`
Expected: every golden skill file and the chatgpt/cowork/codex/cursor bundles change only by the new `gh pr create` sentence (in-repo targets) or the reworded "cannot open a pull request from this surface" sentence (Cowork/ChatGPT). `git diff tests/golden | grep '^[-+]' | grep -v '^[-+][-+]' | sort -u` should show only those lines.

- [ ] **Step 5: Clippy, commit**

Run: `cargo clippy --all-targets -- -D warnings && cargo fmt --check`

```bash
git add src/preamble.rs tests/golden
git commit -m "feat: preamble names gh or bb according to the chosen forge"
```

---

### Task 4: Hooks block `bb pr create`; skill text acknowledges host-specific CLI

**Files:**
- Modify: `src/emit/claude_code.rs:14-31` (`BLOCKED`), tests in both `claude_code.rs` and `cursor_hooks.rs`
- Modify: `.agents/skills/pmk-build-safely/SKILL.md` (one sentence)
- Modify: `tests/golden/**` (regenerate)

- [ ] **Step 1: Write failing hook tests**

In `src/emit/claude_code.rs` tests, generalise `run_push_hook` into `run_hook(pattern_needle: &str, command_value: &str, extra_path: Option<&str>) -> i32` where `pattern_needle` is the substring used to find the hook (currently hard-coded `"git( [^ ]+)* push"`), and keep `run_push_hook` as a thin wrapper calling `run_hook("git( [^ ]+)* push", ...)`. Add:

```rust
    #[test]
    #[serial(env_path)]
    fn opening_a_bitbucket_pull_request_is_blocked() {
        assert_eq!(
            run_hook("bb( [^ ]+)* pr( [^ ]+)* create", "bb pr create main --title x", None),
            2
        );
    }

    #[test]
    #[serial(env_path)]
    fn listing_bitbucket_pull_requests_is_allowed() {
        assert_eq!(run_hook("bb( [^ ]+)* pr( [^ ]+)* create", "bb pr list --json", None), 0);
    }

    #[test]
    #[serial(env_path)]
    fn a_bb_pr_create_with_a_repo_flag_before_the_verb_is_blocked() {
        assert_eq!(
            run_hook("bb( [^ ]+)* pr( [^ ]+)* create", "bb -R acme/api pr create main", None),
            2
        );
    }
```

Do the same generalisation in `src/emit/cursor_hooks.rs` (`run_hook_full(needle, cmd, path) -> (i32, String)`, `run_push_hook*` wrappers) and add the same three tests there (Cursor payload shape).

- [ ] **Step 2: Run to see failures**

Run: `cargo test emit::`
Expected: the new tests panic with "no ... hook found" (`expect` in the finder) because there is no `bb` entry.

- [ ] **Step 3: Add the BLOCKED entry**

Append to `BLOCKED` in `src/emit/claude_code.rs`:

```rust
    (
        "bb( [^ ]+)* pr( [^ ]+)* create",
        "pmkit: opening a pull request needs an explicit yes from the human.",
    ),
```

Update the doc comment above `BLOCKED` to mention `bb` (the Bitbucket Cloud CLI) has no merge command, so only its `pr create` is listed.

- [ ] **Step 4: Skill text**

In `.agents/skills/pmk-build-safely/SKILL.md`, under `## Never, without an explicit yes`, after the paragraph ending "...permission to push more commits into it." add:

```markdown
Which command opens the pull request depends on where the team hosts code. The "Your surface"
section at the top of this skill names the tool; use that one and no other.
```

- [ ] **Step 5: Regenerate goldens, run everything**

Run: `UPDATE_GOLDEN=1 cargo test --test emit_golden && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check && git diff --stat tests/golden`
Expected: green; golden diff touches `.claude/settings.json`-equivalent (Claude Code has no golden dir but Cursor's `hooks.json` does), plus every `pmk-build-safely/SKILL.md` copy and the chatgpt instructions bundle.

- [ ] **Step 6: Commit**

```bash
git add src/emit .agents/skills/pmk-build-safely/SKILL.md tests/golden
git commit -m "feat: hooks block bb pr create; build skill defers to the surface for the PR tool"
```

---

### Task 5: Wizard question and `--forge` on every entry point

**Files:**
- Modify: `src/wizard.rs` (`run_unattended` signature, forge question in `run`, closing note)
- Modify: `src/main.rs` (`--forge` on `Setup`, `TargetArg`, `Refresh`, `Doctor`; resolution helper)
- Modify: `src/commands/mod.rs` (add `resolve_forge`)
- Test: `tests/cli_setup.rs`, `tests/cli_doctor.rs`

**Interfaces:**
- Produces:
  ```rust
  // src/commands/mod.rs
  pub fn resolve_forge(flag: Option<Forge>, project_dir: &Path) -> Forge;  // flag → detect_forge(RealRunner) → GitHub
  // src/wizard.rs
  pub fn run_unattended(targets: &[Target], project_dir: &Path, home: &Path, state_file: &Path, forge: Forge) -> Result<()>;
  pub fn run(project_dir: &Path, home: &Path, state_file: &Path, preselected: Option<Target>, forge: Option<Forge>) -> Result<()>;
  ```

- [ ] **Step 1: Write failing integration tests**

Append to `tests/cli_setup.rs`:

```rust
fn git_remote(project: &std::path::Path, url: &str) {
    let run = |args: &[&str]| {
        let ok = std::process::Command::new("git")
            .args(args)
            .current_dir(project)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?} failed");
    };
    run(&["init", "-q"]);
    run(&["remote", "add", "origin", url]);
}

#[test]
fn setup_yes_with_forge_bitbucket_tells_the_agent_about_bb_and_not_gh() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(&project)
        .env("PMKIT_HOME", tmp.path().join("home"))
        .env("PMKIT_STATE_FILE", tmp.path().join("skills.json"))
        .args(["setup", "--yes", "--target", "codex", "--forge", "bitbucket"])
        .assert()
        .success()
        .stdout(contains("│ bb "))
        .stdout(predicates::str::contains("│ gh ").not());

    let skill = std::fs::read_to_string(project.join(".agents/skills/pmk-feature-loop/SKILL.md")).unwrap();
    assert!(skill.contains("Bitbucket Cloud"), "{skill}");
    assert!(!skill.contains("`gh pr create`"), "{skill}");
}

#[test]
fn setup_yes_detects_bitbucket_from_the_git_remote_when_no_flag_is_given() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    git_remote(&project, "git@bitbucket.org:acme/api.git");

    Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(&project)
        .env("PMKIT_HOME", tmp.path().join("home"))
        .env("PMKIT_STATE_FILE", tmp.path().join("skills.json"))
        .args(["setup", "--yes", "--target", "codex"])
        .assert()
        .success()
        .stdout(contains("│ bb "));

    let skill = std::fs::read_to_string(project.join(".agents/skills/pmk-feature-loop/SKILL.md")).unwrap();
    assert!(skill.contains("Bitbucket Cloud"), "{skill}");
}

#[test]
fn setup_yes_falls_back_to_github_when_nothing_is_detectable() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("project");
    std::fs::create_dir_all(&project).unwrap();

    Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(&project)
        .env("PMKIT_HOME", tmp.path().join("home"))
        .env("PMKIT_STATE_FILE", tmp.path().join("skills.json"))
        .args(["setup", "--yes", "--target", "codex"])
        .assert()
        .success()
        .stdout(contains("│ gh "))
        .stdout(predicates::str::contains("│ bb ").not());
}

#[test]
fn an_unknown_forge_is_rejected_with_the_valid_list() {
    let tmp = tempfile::tempdir().unwrap();
    Command::cargo_bin("pmkit")
        .unwrap()
        .current_dir(tmp.path())
        .args(["setup", "--yes", "--forge", "gitlab"])
        .assert()
        .failure()
        .stderr(contains("github, bitbucket, both"));
}
```

Note: `--forge` for `setup` must be accepted alongside `--yes`. The table's first column is `TOOL`, and comfy-table renders `│ gh ` with the name left-aligned and padded, which is what the `contains` checks rely on; if the exact spacing differs when you run it, adjust the needle to whatever the real table prints (verify by printing stdout once) but keep the positive/negative pair.

In `tests/cli_doctor.rs` add one test:

```rust
#[test]
fn doctor_with_forge_both_probes_gh_and_bb() {
    Command::cargo_bin("pmkit")
        .unwrap()
        .env("PMKIT_HOME", tempfile::tempdir().unwrap().path())
        .args(["doctor", "--forge", "both"])
        .assert()
        .success()
        .stdout(contains("│ gh "))
        .stdout(contains("│ bb "));
}
```

(Read the existing tests in that file first and match their env setup.)

- [ ] **Step 2: Run to see failures**

Run: `cargo test --test cli_setup --test cli_doctor`
Expected: `--forge` unrecognised → failures.

- [ ] **Step 3: Implement `resolve_forge`**

`src/commands/mod.rs`:

```rust
use crate::doctor::runner::RealRunner;
use crate::forge::{detect_forge, Forge};
use std::path::Path;

/// Flag wins; otherwise guess from the project's git remote; otherwise
/// GitHub, which is what pmkit assumed before it knew about forges.
pub fn resolve_forge(flag: Option<Forge>, project_dir: &Path) -> Forge {
    flag.or_else(|| detect_forge(project_dir, &RealRunner))
        .unwrap_or(Forge::GitHub)
}
```

- [ ] **Step 4: Wizard**

`src/wizard.rs`:

```rust
pub fn run_unattended(
    targets: &[Target],
    project_dir: &Path,
    home: &Path,
    state_file: &Path,
    forge: Forge,
) -> Result<()> {
    let probes = probes::run_all(&RealRunner, home, forge);
    println!("{}", table(&probes));
    let caps = probes::capabilities_from(&probes, forge);
    // ... unchanged ...
    if !caps.superpowers { /* unchanged */ }
    if forge.includes_github() && !caps.gh {
        println!(
            "The GitHub CLI is not ready, so your agent cannot open pull requests on GitHub. \
             Run `brew install gh && gh auth login`, then run `pmkit setup` again."
        );
    }
    if forge.includes_bitbucket() && !caps.bb {
        println!(
            "The Bitbucket Cloud CLI is not ready, so your agent cannot open pull requests on \
             Bitbucket. Run `brew install biokraft/tap/bb && bb auth login`, then run `pmkit \
             setup` again."
        );
    }
    Ok(())
}

pub fn run(
    project_dir: &Path,
    home: &Path,
    state_file: &Path,
    preselected: Option<Target>,
    forge: Option<Forge>,
) -> Result<()> {
    let targets = /* unchanged */;
    if targets.is_empty() { /* unchanged */ }
    let forge = match forge {
        Some(f) => f,
        None => {
            let detected = detect_forge(project_dir, &RealRunner).unwrap_or(Forge::GitHub);
            let options: Vec<&'static str> = Forge::all().iter().map(|f| f.label()).collect();
            let start = Forge::all().iter().position(|f| *f == detected).unwrap_or(0);
            let chosen = inquire::Select::new("Where does your team host code?", options)
                .with_starting_cursor(start)
                .prompt()
                .ok();
            Forge::all()
                .into_iter()
                .find(|f| Some(f.label()) == chosen)
                .unwrap_or(detected)
        }
    };
    run_unattended(&targets, project_dir, home, state_file, forge)
}
```

Imports: `use crate::forge::{detect_forge, Forge};`. Add a unit test in `wizard.rs`? The prompt is interactive; skip. The closing-note text is exercised by `setup_yes_with_forge_bitbucket_...` only if `bb` is absent on the machine, so do not assert on it in CI.

- [ ] **Step 5: CLI**

`src/main.rs`:

- `Setup { yes, target, forge: Option<Forge> }` with `#[arg(long, value_parser = parse_forge)]` and doc `/// Where your team hosts code: github, bitbucket, or both. Detected from the git remote when omitted.`
- `TargetArg` gains the same `forge` field; `SkillCmd::Refresh` becomes `Refresh(ForgeArg)` where `ForgeArg { forge: Option<Forge> }`.
- `Doctor { forge: Option<Forge> }` (turn the unit variant into a struct variant).
- `fn parse_forge(s: &str) -> Result<Forge, String> { s.parse::<Forge>().map_err(|e| e.to_string()) }`
- In `run_skill` Install: `let forge = resolve_forge(arg.forge, &dir); let caps = Capabilities { forge, ..Capabilities::all_present() };`. Refresh: same with `dir = current_dir()`.
- Setup: `let forge = ...; if yes { run_unattended(&targets, &dir, &home, &state, resolve_forge(forge, &dir)) } else { run(&dir, &home, &state, target, forge) }`.
- Doctor: `let forge = resolve_forge(forge, &std::env::current_dir()?); run_all(&RealRunner, &home_dir(), forge)`.

- [ ] **Step 6: Run everything**

Run: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: green. If `│ gh ` spacing assertions fail, print the table once and fix the needle (see Step 1 note).

- [ ] **Step 7: Commit**

```bash
git add src/wizard.rs src/main.rs src/commands/mod.rs tests/cli_setup.rs tests/cli_doctor.rs
git commit -m "feat: ask which forge the team uses and accept --forge everywhere"
```

---

### Task 6: Docs and changelog

**Files:**
- Modify: `README.md`, `docs/targets.md`, `CHANGELOG.md`

- [ ] **Step 1: README**

- In `## pmkit setup` bullets, after the targets bullet add: "whether your team hosts code on GitHub, Bitbucket Cloud, or both. pmkit guesses from the project's git remote and you confirm. Pass `--forge github|bitbucket|both` to skip the question."
- Change "nothing else" bullet to still hold (it says no API key etc.) — keep.
- `## Where the gates are enforced`: change "`gh pr create`, `gh pr merge`" to "`gh pr create`, `gh pr merge`, `bb pr create`". Add a sentence: "`bb` (the Bitbucket Cloud CLI) has no merge command, so merging on Bitbucket happens in the browser and stays prose."
- `## Prerequisites`: replace the `gh` bullet with:
  - "**gh** or **bb** — the pull-request CLI for your host. `gh` for GitHub, [`bb`](https://github.com/biokraft/bbcloud) for Bitbucket Cloud (`brew install biokraft/tap/bb`, then `bb auth login`). The doctor only checks the one you chose."
- `## Managing what pmkit installed` block: add `pmkit doctor --forge bitbucket  # check the Bitbucket CLI instead of gh` and mention `--forge` is accepted by `setup`, `skill install`, `skill refresh`, and `doctor`.

- [ ] **Step 2: docs/targets.md**

Claude Code "Writes" bullet: "one entry each for `git push`, `git merge`, `gh pr create`, `gh pr merge`, `bb pr create`". Cursor: "the same five command patterns as Claude Code's". Update "same four patterns" → "same five patterns".

- [ ] **Step 3: CHANGELOG**

Above `## [0.1.0]` add:

```markdown
## [Unreleased]

### Added

- **Bitbucket Cloud support.** `pmkit setup` now asks whether your team hosts code on GitHub,
  Bitbucket Cloud, or both, guessing from the git remote first. Choose Bitbucket and the doctor
  checks [`bb`](https://github.com/biokraft/bbcloud) instead of `gh`, the skills tell the agent to
  open pull requests with `bb pr create`, and the Claude Code and Cursor hooks block that command
  until a human says yes. `--forge github|bitbucket|both` skips the question on `setup`,
  `skill install`, `skill refresh` and `doctor`.
```

- [ ] **Step 4: Verify and commit**

Run: `cargo test --test install_sh` (README untouched by tests, but run the suite once more: `cargo test`).

```bash
git add README.md docs/targets.md CHANGELOG.md
git commit -m "docs: document the forge question, bb prerequisite and the new hook pattern"
```

---

## Self-review

- **Spec coverage:** Forge type + detection (T1); Capabilities/doctor (T2); preamble (T3); hooks + skill sentence (T4); wizard question, `--forge` on setup/install/refresh/doctor, closing note (T5); README/targets/CHANGELOG (T6). Out-of-scope items untouched.
- **Placeholders:** none; every code step has code.
- **Type consistency:** `run_all(r, home, forge)`, `capabilities_from(probes, forge)`, `run_unattended(..., forge: Forge)`, `run(..., forge: Option<Forge>)`, `resolve_forge(flag, dir)` used identically across T2/T5.
