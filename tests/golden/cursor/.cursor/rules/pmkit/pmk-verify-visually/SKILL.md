---
name: pmk-verify-visually
description: Use before claiming any user-visible change works, and whenever a product manager asks whether something actually works. Drives a real browser through the change with Playwright and produces screenshots as evidence. Forbids claiming visual correctness from reading code.
license: MIT
---

## Your surface

You are running on **Cursor**. pmkit wrote this section; the rules below it are the same everywhere.

The safety gates in this skill are **machine-enforced** here: a blocked command fails before it runs. Do not treat that as permission to skip asking — the human still decides.

You have a shell. Run commands yourself, and show the human what you ran.

A browser is available through Playwright. Use it to verify UI work.

Jira access is through the `acli` command line tool. Use it for every Jira read and write.

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
