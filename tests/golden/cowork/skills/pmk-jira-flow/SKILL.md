---
name: pmk-jira-flow
description: Use whenever work is attached to a Jira ticket — starting it, opening a PR, finishing it, or parking it. Keeps the ticket's status matching reality and never writes to Jira without showing the exact text first.
license: MIT
---

## Your surface

You are running on **Claude Cowork**. pmkit wrote this section; the rules below it are the same everywhere.

The safety gates in this skill are **prose only** here: nothing on this surface blocks a command for you, so they cannot be blocked automatically. You are the only thing standing between the human and an action they did not ask for. Follow them exactly.

You have **no shell** on this surface. When a step needs a command, print it in a copyable block and ask the human to run it and paste the output. Never claim a command's result you did not see.

**You CANNOT verify anything visually** — no browser is available. Say so plainly instead of implying the change was checked.

You cannot open a pull request from this surface. Stop after committing and tell the human.

Jira access is through the `acli` command line tool, but you have **no shell** on this surface. Print the `acli` command in a copyable block and ask the human to run it and paste the output. Never claim a Jira read or write you did not see the human run.

# Keep the ticket honest

A ticket parked in Backlog while code is being written is a lie the whole team reads. The ticket's
state is part of the deliverable.

## Move it as the work moves

| Moment | Target state |
| --- | --- |
| Work starts — branch created, plan execution begins | In Progress |
| Pull request opened | In Code Review |
| Merged, waiting on a deploy | Ready for Release |
| Deployed, or finished and no deploy needed | Done |
| Stopped and handed back | back to To Do, with a comment saying why |

Status names differ per project. Read the real transition list for this ticket rather than guessing
an ID. Transitions are per-workflow, not global.

## Two hard rules

1. **Never write to Jira without showing the exact text first.** Comment, description, transition,
   field change: print what you are about to send, then wait for an explicit yes.
2. **Never transition to Done when acceptance is someone else's call.** A release, a sign-off, a PM
   check. Report it as ready and let the human close it.

## Style

One transition per real change of state. Do not narrate at length. A single line naming the new
state is enough. Attach the verification screenshots from `pmk-verify-visually` when you move a
ticket to review.
