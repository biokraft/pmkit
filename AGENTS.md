# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`CLAUDE.md` is a symlink to `AGENTS.md`; edit `AGENTS.md`.

## What pmkit is

A Rust CLI that writes a fixed set of five skills (`pmk-*`) plus safety-gate hooks into whichever
coding agent a product manager uses. It runs once, writes files, tracks them, and exits. Nothing
stays running. Read `README.md` and `docs/targets.md` for the user-facing contract before changing
behaviour: every promise in those two files is backed by a test.

## Commands

```bash
cargo build
cargo test --all                                  # what CI runs
cargo test --test cli_setup                       # one integration test file
cargo test --test emit_golden codex               # one test by name within a file
cargo test state::tests::a_locally_edited         # one unit test (module path prefix)
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings         # CI sets RUSTFLAGS=-D warnings too
RUSTUP_TOOLCHAIN=1.88 cargo check --all-targets --locked   # MSRV job; rust-toolchain.toml pins 1.97
UPDATE_GOLDEN=1 cargo test --test emit_golden     # regenerate tests/golden/<target>/ after an intentional text change
cargo run -- setup --yes --target claude-code     # try it in a scratch dir, never in this repo (see below)
```

`PMKIT_HOME` and `PMKIT_STATE_FILE` override the home directory and the state file
(`~/.config/pmkit/skills.json`). Every CLI test sets both to a tempdir; do the same for any new one.

Lints: `unwrap_used` and `expect_used` are `deny` in `Cargo.toml`, `unsafe_code` is forbidden in
`lib.rs` and `main.rs`. Test modules opt out with `#![allow(clippy::unwrap_used)]` at the top.

## Architecture

One pipeline, each stage a module, data flows left to right:

```
doctor/probes  ->  Capabilities  ->  preamble  ->  emit::plan_files  ->  state::apply
(Runner trait)     (what works)     (per-target     (Vec<EmitFile>,      (write, hash,
                                     header text)    pure, no I/O)        refuse-not-overwrite)
```

- **`skills.rs`** embeds the five skill bodies with `include_str!` from `.agents/skills/*/SKILL.md`.
  That directory is the single source of truth for skill text and ships inside the binary. Editing
  a skill there changes every target's output and every golden file. `tests/skill_lint.rs` checks
  frontmatter, the `pmk-` prefix, and that the three gates are still spelled out.
- **`target.rs`** and **`forge.rs`** are closed enums. `Target` (claude-code, cursor, codex, cowork,
  chatgpt) knows whether it lives in the repo or is staged under `$HOME`, and whether its gates are
  hook-enforced (only Claude Code and Cursor). `Forge` (github, bitbucket, both) decides whether the
  doctor probes `gh` or `bb` and which tool the preamble names. `commands::resolve_forge` is flag,
  then `git remote -v` sniffing, then GitHub.
- **`doctor/`**: every external check goes through the `Runner` trait so probes are tested with
  `FakeRunner` and never touch the network. Probes are read-only and only ever suggest fixes; no fix
  command may use `sudo` (tested). `capabilities_from` turns probe results into `Capabilities`.
- **`preamble.rs`** is the only text that differs between targets. A `false` capability becomes an
  explicit prohibition in the skill ("You CANNOT verify anything visually"), which is how a missing
  prerequisite degrades instead of aborting. Prose-only targets must say so out loud.
- **`emit/`**: `plan_files(target, caps, dest)` is pure and returns `Vec<EmitFile>`; one submodule
  per target. `skill_body` splices the preamble after the frontmatter so frontmatter stays first.
  `tests/emit_golden.rs` diffs the plan against `tests/golden/<target>/`.
- **`state.rs`** is the safety core. `apply` writes only when the on-disk bytes match the wanted
  content or a hash pmkit itself recorded; anything else is `SkippedModified` and left alone, tracked
  or not. `created: false` entries are never written or removed. `is_pmkit_path` rejects any path the
  (user-editable) state file names that pmkit would not itself write. `MissingPolicy::Restore` is
  for explicit `skill install`; `Preserve` for `refresh`, so a deliberately deleted file stays gone.
- **`wizard.rs`** wraps the pipeline with `inquire` questions and prints next steps. The
  `gate_installed` check exists so pmkit never tells a PM the gates are enforced when the hook file
  was refused. `run_unattended` is the `--yes` path and what tests drive.

### Adding a target or a forge

A new `Target` variant forces `Target::all()`, an `emit/<name>.rs`, `destination_for`, an
`is_pmkit_path` case for any new config filename, a `tests/golden/<name>/` directory, and a section
in `docs/targets.md`. `state::tests::every_target_round_trips_every_planned_file_to_disk_with_no_failures`
will catch a planned path the guard rejects. A new `Forge` needs a probe, a `Capabilities` flag,
preamble text, and tests in `preamble.rs` asserting the other host's tool is never mentioned.

## Do not run pmkit inside this repo

This repository is itself a Codex-style target layout: `.agents/skills/` is the skill source and
`AGENTS.md` is this file. Running `pmkit setup` here would try to write both and refuse (which is
correct behaviour, but a confusing test). Use a tempdir.

## Releases

release-plz runs from GitHub Actions on merges to `main`. Commit subjects must be Conventional
Commits because `release-plz.toml` builds the changelog from them: `feat` -> Added, `fix` -> Fixed,
`docs` -> Documentation, `refactor`/`perf` -> Changed; `test`, `chore`, `ci` are skipped. Secrets
and the tap flow are described at the end of `docs/targets.md`.
