---
name: pmk-shape-idea
description: Use when a product manager brings a rough idea, problem, or feature request that has not been specified yet. Turns it into a written spec they have read and agreed to, before any code exists. Wraps the Superpowers brainstorming skill and adds a say-it-back confirmation gate.
license: MIT
---

## Your surface

You are running on **Claude Cowork**. pmkit wrote this section; the rules below it are the same everywhere.

The safety gates in this skill are **prose only** here: nothing on this surface blocks a command for you, so they cannot be blocked automatically. You are the only thing standing between the human and an action they did not ask for. Follow them exactly.

You have **no shell** on this surface. When a step needs a command, print it in a copyable block and ask the human to run it and paste the output. Never claim a command's result you did not see.

**You CANNOT verify anything visually** — no browser is available. Say so plainly instead of implying the change was checked.

You cannot open a pull request from this surface. Stop after committing and tell the human.

Jira access is through the `acli` command line tool, but you have **no shell** on this surface. Print the `acli` command in a copyable block and ask the human to run it and paste the output. Never claim a Jira read or write you did not see the human run.

# Shape the idea

No code. No plan. No files touched except the spec at the end.

## Process

1. **Invoke `superpowers:brainstorming`** and follow it. If it is unavailable, say so and stop —
   do not improvise a replacement.
2. Ask questions **one at a time**, in plain language. Prefer a multiple choice when the options
   are genuinely distinct.
3. Cover, at minimum: who this is for, what they do today instead, what "done" looks like, what
   must NOT change, and how you will know it worked.

## The say-it-back gate

Before you write a single line of the spec, stop and post exactly this shape:

> **What I think you want:** <two or three sentences, in their words, not yours>
> **What I am assuming:** <exactly three assumptions, each one falsifiable>
> **Have I got this right?**

Wait for their answer. If they correct anything, revise and say it back again. Do not proceed on a
silence or a "sure".

## Output

Write the spec to `docs/specs/YYYY-MM-DD-<topic>.md` in their project. Include: the problem, who it
is for, what is in scope, what is explicitly out of scope, how it will be verified, and the open
questions you could not resolve.

Then ask them to read it. The next stage does not start until they say the spec is right.
