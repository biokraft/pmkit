---
name: pmk-build-safely
description: Use when a product manager has an agreed written spec and wants it built. Plans the work, then builds it one small reviewed task at a time on an isolated branch, explaining each change in language they can check. Wraps the Superpowers writing-plans and subagent-driven-development skills.
license: MIT
---

## Your surface

You are running on **Cursor**. pmkit wrote this section; the rules below it are the same everywhere.

The safety gates in this skill are **machine-enforced** here: a blocked command fails before it runs. Do not treat that as permission to skip asking — the human still decides.

You have a shell. Run commands yourself, and show the human what you ran.

A browser is available through Playwright. Use it to verify UI work.

Pull requests on GitHub go through the `gh` command line tool (`gh pr create`).

Jira access is through the `acli` command line tool. Use it for every Jira read and write.

# Build it safely

The spec is agreed. Now the only thing that matters is that nothing lands the human did not
understand.

## Before anything

1. **Never work on `main`.** Create a branch or a worktree first, named for the work.
2. **Never start in a dirty tree.** If there are uncommitted changes, stop and ask what to do
   with them.
3. Confirm you are in the repo they think you are in. Say its name back to them.

## Plan, then build

1. **Invoke `superpowers:writing-plans`** to turn the spec into a task-by-task plan. Have the
   human read it. A plan they have not read is not a plan.
2. **Invoke `superpowers:subagent-driven-development`** to execute it. Sequential — one task at a
   time, never a fan-out. A review subagent checks every task's diff before the next task starts,
   and one final review runs across the whole branch at the end.
3. Implementation subagents run on **Sonnet at low thinking effort**. The final whole-branch review
   runs on **Opus**. Pass the model explicitly on every dispatch; an omitted model silently
   inherits the most expensive one.

## The stop-and-explain gate

At the end of every task, before the next one begins, post:

> **Task N done.** <one paragraph, no jargon, that a non-engineer can check>
> **Files changed:** <list>
> **What I would look at to disbelieve me:** <the one file or screen that would show this is wrong>
> **Carry on to task N+1?**

Wait. This is the whole point of the loop: the human sees every change while it is still small
enough to check.

## When you do not know

If a task turns out to depend on a product decision the spec does not settle, stop and ask. Do not
pick the plausible option and note it later. "I don't know, and here are the two ways it could go"
is always the right answer.

## Never, without an explicit yes

`git push`. Force-push. Merge. Open a pull request. Delete anything that is not yours. Touch
anything outside this repo.

Every one of these needs its own yes, every time. Permission for one push is not permission for the
next one, and permission to open a pull request is not permission to push more commits into it.
