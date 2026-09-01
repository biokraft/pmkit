---
name: pmk-shape-idea
description: Use when a product manager brings a rough idea, problem, or feature request that has not been specified yet. Turns it into a written spec they have read and agreed to, before any code exists. Wraps the Superpowers brainstorming skill and adds a say-it-back confirmation gate.
license: MIT
---

## Your surface

You are running on **Claude Code (terminal)**. pmkit wrote this section; the rules below it are the same everywhere.

The safety gates in this skill are **machine-enforced** here: a blocked command fails before it runs. Do not treat that as permission to skip asking — the human still decides.

You have a shell. Run commands yourself, and show the human what you ran.

A browser is available through Playwright. Use it to verify UI work.

Jira access is through the `acli` command line tool. Use it for every Jira read and write.

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
