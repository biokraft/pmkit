---
name: pmk-verify-visually
description: Use before claiming any user-visible change works, and whenever a product manager asks whether something actually works. Drives a real browser through the change with Playwright and produces screenshots as evidence. Forbids claiming visual correctness from reading code.
license: MIT
---

## Your surface

You are running on **Claude Cowork**. pmkit wrote this section; the rules below it are the same everywhere.

The safety gates in this skill are **prose only** here: nothing on this surface blocks a command for you, so they cannot be blocked automatically. You are the only thing standing between the human and an action they did not ask for. Follow them exactly.

You have **no shell** on this surface. When a step needs a command, print it in a copyable block and ask the human to run it and paste the output. Never claim a command's result you did not see.

**You CANNOT verify anything visually** — no browser is available. Say so plainly instead of implying the change was checked.

`gh` is not installed, so you cannot open a pull request. Stop after committing and tell the human.

Jira access is through the `acli` command line tool, but you have **no shell** on this surface. Print the `acli` command in a copyable block and ask the human to run it and paste the output. Never claim a Jira read or write you did not see the human run.

# See it before you believe it

Reading the code does not tell you what the screen looks like. Neither do the types, and neither do
the tests.

## Process

1. Make sure the dev server is running. If you cannot start it, ask the human to and say why.
2. Navigate to the affected route.
3. Take a screenshot.
4. Read the browser console and report any errors, even ones you think are unrelated.
5. Walk the **golden path** — the thing the change was for — and screenshot the result.
6. Walk **one obvious edge case**: an empty state, an error state, or a narrow viewport.
7. Iterate: change, reload, screenshot, compare. Stop when it is actually right.

## The rule you cannot rationalise past

Never say a user-visible change "works", "is done", or "looks good" on the basis of code, types, or
passing tests. If no browser is available, say exactly that instead — "I could not verify this
visually" — and let the human decide.

## Evidence

The screenshots are the product manager's evidence, and the artifact they attach to the ticket.
Name them for what they show and tell the human where they are.
