# pmkit — Design

**Date:** 2026-09-01
**Status:** Approved, pending implementation plan

## Problem

Product managers increasingly work with coding agents, but the workflows that make
agentic development trustworthy — spec before code, one task at a time, review after
every task, verify what you claim, keep the ticket honest — live in the heads and
dotfiles of engineers. A PM who opens Claude Cowork or Cursor gets a blank prompt and
no guardrails. The failure mode is not a PM who cannot code; it is a PM who cannot tell
whether the agent's confident summary is true.

`pmkit` is a single `brew install` that sets up a safe, human-in-the-loop agentic
workflow inside whichever agent the PM already uses.

## Users and scope

**Target user:** semi-technical PM. Can read a diff, sometimes runs the dev server.
Prototypes freely in their own repos; contributes to real team repos where a developer
reviews their PR.

**v1 covers the full loop:** discover → spec → build → verify → Jira.

**Agent surfaces in scope:** five emit targets — Claude Cowork, Cursor, Codex,
ChatGPT, and Claude Code. Codex and ChatGPT are separate targets despite sharing a
vendor: Codex has a shell and reads `AGENTS.md`, ChatGPT has neither.

**Non-goals for v1:** Windows support; `pmkit`-native Jira or Playwright command
wrappers; an MCP server; telemetry; a hosted install page; our own brainstorm/plan/
review skill chain (we wrap Superpowers instead); automated agent-behaviour evals;
team-shared configuration.

## Design principles

Taken from the author's published work on agentic development, and treated here as
requirements rather than preferences:

1. **Incremental beats fan-out.** Sequential subagents, one task at a time, reviewed at
   every handoff. Never a changeset too large for the human to check.
2. **The human stays the architect.** The agent synthesises; it does not decide product
   intent. When intent is unclear it must ask, not guess.
3. **The spec is the deliverable before the code is.** Plans get adversarially tested
   while they are still cheap to change.
4. **Cheap models earn their keep under supervision.** Implementation subagents run on
   Sonnet at low effort; the final whole-branch review runs on Opus.
5. **Never claim what you have not seen.** A UI change is unverified until a screenshot
   exists.
6. **Token discipline is a feature.** Skills stay short; the binary ships no runtime.

## Architecture

Two independent halves sharing one `Target` abstraction, plus a canonical skill set that
is the actual product.

```
pmkit/
├─ .agents/skills/                       # canonical skill sources
│  ├─ pmk-feature-loop/SKILL.md          # front door; routes, never implements
│  ├─ pmk-shape-idea/SKILL.md            # discover → spec
│  ├─ pmk-build-safely/SKILL.md          # plan → build
│  ├─ pmk-verify-visually/SKILL.md       # Playwright verification
│  └─ pmk-jira-flow/SKILL.md             # ticket state tracks reality
├─ src/
│  ├─ main.rs
│  ├─ lib.rs
│  ├─ skill.rs        # SKILLS[] via include_str!, sha256 state, refresh, uninstall
│  ├─ target.rs       # Target enum (5 variants) + per-target metadata
│  ├─ preamble.rs     # capability preamble generation
│  ├─ emit/           # one writer per target
│  ├─ doctor/         # one probe per prerequisite
│  └─ wizard.rs       # the interactive setup conversation
├─ install.sh
├─ package.nix
├─ flake.nix
├─ release-plz.toml
└─ docs/superpowers/specs/
```

**Language and distribution:** Rust, mirroring the `bbcloud` skeleton — Cargo,
`include_str!` skills, sha256 state tracking, brew tap, `install.sh`, Nix flake,
`release-plz`. A single static binary with no runtime, which matters because the target
user cannot troubleshoot a missing Python or Node.

### Canonical skills plus capability preamble

Skill bodies are authored once and embedded in the binary. At emit time, `pmkit`
prepends a per-target **capability preamble** and writes the result in that target's
native shape. Skill bodies are byte-identical across targets; only the preamble differs.

The preamble is the single place where truth about the surface lives. It states which
tools exist here, whether the safety gates are machine-enforced or prose-only, and what
the skill must do instead when a capability is absent.

| Target | Emitted to | Gate enforcement |
| --- | --- | --- |
| Claude Code | `.claude/skills/pmk-*/SKILL.md` + `settings.json` hooks | prose + PreToolUse hooks (hard block) |
| Cursor | `.cursor/rules/`, `.cursor/commands/`, `AGENTS.md` | prose + Cursor hooks |
| Codex | `.agents/skills/`, `AGENTS.md` | prose; shell available |
| Cowork | `~/pmkit-cowork/` folder, ready to upload; opened in Finder | prose only |
| ChatGPT | `pmkit-chatgpt-instructions.md`, copied to clipboard | prose only |

Emit is a pure function of (skills, target, destination) — the destination is the project
directory for the three in-repo targets, and a fixed location under the user's home for
Cowork and ChatGPT, which have no repo to write into. Every written path is recorded with
its sha256, so a PM's own edits are detected and never overwritten. Side effects that are
not file writes (opening Finder, copying to the clipboard) live in the wizard, not in
`emit/`, so emit stays testable by golden file.

**Rejected alternatives.** Per-target hand-written skill packs: five times the prose to
keep in sync, and gate wording drifting between surfaces is precisely the failure this
tool exists to prevent. A single lowest-common-denominator prose-only pack: discards the
one surface where guardrails are actually enforceable.

**Accepted cost.** The weakest surface sets the floor for what any skill may assume, and
one abstraction must be maintained as the surfaces evolve.

## The PM loop

`pmk-feature-loop` is the only entry point a PM must remember. It classifies the request
and routes into a stage; it never implements anything itself.

### Stage 1 — `pmk-shape-idea` (discover → spec)

Invokes `superpowers:brainstorming`. Adds two things: questions asked one at a time in
plain language without jargon, and a **say-it-back gate** — before writing the spec, the
agent restates the idea in the PM's own words along with the three assumptions it is
making, and waits for confirmation.

Output: a spec at `docs/specs/YYYY-MM-DD-<topic>.md` in the PM's own project — a shorter
path than the `docs/superpowers/specs/` this document lives in, chosen deliberately so a
PM can find their own specs without knowing what Superpowers is.

### Stage 2 — `pmk-build-safely` (plan → build)

Invokes `superpowers:writing-plans`, then `superpowers:subagent-driven-development`:
sequential execution, one task at a time, a review subagent after every task, and an
Opus pass across the whole branch at the end. Additions:

- **Branch or worktree first, always.** Never work on `main`; never start in a dirty
  tree.
- **Stop-and-explain gate** at every task boundary: what changed, in one paragraph a
  non-engineer can verify, before the next task begins.
- **Explicit "I don't know" escape hatch.** On any question of product intent the agent
  must ask rather than assume.
- Implementation subagents run on Sonnet at low thinking effort; the final whole-branch
  review runs on Opus. Model is passed explicitly on every dispatch.

### Stage 3 — `pmk-verify-visually`

Playwright: ensure the dev server is running, navigate to the affected route, screenshot,
check the console, then exercise the golden path and one obvious edge case. A UI change
is never reported as working on the basis of reading code, types, or tests. Screenshots
are the PM's evidence and the artifact they attach to the ticket. If no browser is
available, the skill must say so explicitly rather than claim success.

### Stage 4 — `pmk-jira-flow`

The ticket's status is part of the deliverable and moves as the work moves
(In Progress → In Code Review → Ready for Release → Done). Two hard rules: never
transition to `Done` when acceptance is someone else's call, and never write anything to
Jira without showing the exact text first — a PM's Jira is read by their whole
organisation.

### The safety gates

Three actions always require an explicit human yes, on every surface:

1. `git push`, force-push, merge, or opening a pull request.
2. Any write to Jira.
3. Any command touching a target that is not a local development URL.

Enforcement is machine-backed where the surface allows (PreToolUse hooks on Claude Code,
Cursor's hook format on Cursor) and prose-only where it does not (Cowork, ChatGPT). The
preamble states which regime is in force, so the PM knows whether the line is actually
being held.

**Stated limits.** Prose gates can be rationalised past by a sufficiently motivated
agent, which makes Cowork and ChatGPT the weaker surfaces. Stage 2 depends on
Superpowers, so Cowork and ChatGPT need those skills uploaded as well as ours; the
wizard says so at the end of setup.

## Command surface

```
pmkit setup                  # the guided wizard; what a PM runs once
pmkit doctor [--fix]         # probe prerequisites, print a table, offer repairs
pmkit skill install [--target <t>] [--dir <path>]
pmkit skill list | refresh | uninstall
pmkit update                 # self-update via the GitHub Releases API
```

`pmkit setup` is the whole conversation: which agents do you use → doctor those → emit
for those → what to do next on each surface.

## Doctor

Each probe follows the same shape: detect, explain in one plain sentence why it matters,
then offer the exact command. It never runs a fix unasked.

| Probe | Detection | Fix offered |
| --- | --- | --- |
| git | `git --version` | `brew install git` |
| gh | `gh auth status` | `brew install gh`, then `gh auth login` run by the human |
| Node ≥ 20 | `node -v` | `brew install node` |
| Playwright | `npx playwright --version`, browsers present | `npx playwright install chromium` |
| Superpowers | plugin present, or `~/.claude/plugins` | per-harness install instruction |
| Jira backend | `acli` present, else Atlassian MCP configured, else neither | `brew install acli` + `acli jira auth login`, or the `claude mcp add` line |
| Agent surfaces | which of the four are installed | determines what gets emitted |

**Jira backend selection:** whichever is present wins. If both are present, `acli` is
preferred — leaner token usage, and it works on every surface. If neither is present,
the wizard asks. The chosen backend is recorded in the capability preamble so
`pmk-jira-flow` never has to guess which tool it has.

**Constraints.** Doctor never mutates without an explicit yes. It never uses sudo. It
never writes to a PM's `settings.json` without showing the diff first. Anything it
cannot fix is printed as a copy-pasteable line together with a suggestion of who to ask.

**Failure posture.** A missing prerequisite degrades rather than aborts: skills are
emitted anyway, and the preamble records the absent capability so the agent behaves
correctly without it — for example, with no Playwright it is forbidden from claiming
visual verification.

**State.** `~/.config/pmkit/state.json` records emitted entries (path, target, kind,
sha256, version, skill), matching `bbcloud`'s shape so that refresh detects version drift
and protects local edits. Nothing else is persisted. No credentials are ever handled:
`gh`, `acli`, and the MCP own their own authentication.

## Testing

Most of the interesting logic is pure and testable without a configured machine.

- **emit:** golden-file tests per target — (skills, target, temp dir) produces an exact
  tree and exact file bytes. Catches preamble drift and path mistakes.
- **preamble:** each capability combination produces the right claims, with particular
  attention to the negative ones (no Playwright forbids claiming visual verification).
- **skill state:** sha256 round-trip — fresh install; refresh on a version bump; a
  locally edited file is left untouched; uninstall removes exactly what was written and
  nothing more.
- **doctor:** probes take an injected command runner, so every detect / absent / broken
  branch is covered with fake outputs. No test touches the network.
- **integration:** `assert_cmd` over `pmkit skill install --dir <tmp>` for each target,
  and `pmkit doctor` against a stubbed runner.
- **skill lint:** every `SKILLS[]` entry parses, carries frontmatter `name` and
  `description`, and has a name matching its directory.

Whether an agent actually obeys a prose gate is an eval, not a unit test. Deferred to
v1.1, where `claude plugin eval` on the Claude Code surface is the natural vehicle.

## Release

The `bbcloud` pipeline, reused: `release-plz` for versioning and changelog; a CI matrix
building macOS arm64/x86_64 and Linux x86_64/aarch64; tarballs and `.sha256` files on
GitHub Releases; `install.sh` verifying the checksum and installing to `~/.local/bin`;
`package.nix` and `flake.nix` for Nix; and a formula in `biokraft/homebrew-tap`. The
repository is public from the outset, so `install.sh` can be fetched by raw URL and the
design is readable alongside the article that will describe it.

## Build order

1. Skeleton and release plumbing.
2. `skill.rs` plus one target end to end: Claude Code, hooks included.
3. The remaining three emitters and the preamble.
4. Doctor probes.
5. The wizard.
6. Documentation and the accompanying article.
