---
name: pmk-jira-flow
description: Use whenever work is attached to a Jira ticket — starting it, opening a PR, finishing it, or parking it. Keeps the ticket's status matching reality and never writes to Jira without showing the exact text first.
license: MIT
---

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
an ID — transitions are per-workflow, not global.

## Two hard rules

1. **Never write to Jira without showing the exact text first.** Comment, description, transition,
   field change — print what you are about to send, then wait for an explicit yes.
2. **Never transition to Done when acceptance is someone else's call.** A release, a sign-off, a PM
   check. Report it as ready and let the human close it.

## Style

One transition per real change of state. Do not narrate at length — a single line naming the new
state is enough. Attach the verification screenshots from `pmk-verify-visually` when you move a
ticket to review.
