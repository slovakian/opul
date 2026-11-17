# Issue tracker: GitHub

Issues and specs live in [slovakian/opul](https://github.com/slovakian/opul/issues). Use the connected GitHub tools or `gh` with an explicit `--repo slovakian/opul`.

## Publishing and reading

- When a skill says to publish to the issue tracker, create a GitHub issue. A local Markdown draft is an intermediate artifact.
- Use `to-spec` to synthesize an agreed feature into a spec. Apply `ready-for-agent` directly, using the [triage vocabulary](triage-labels.md).
- Before creating an issue, search for an existing issue covering the same work.
- With `gh`, pass multiline bodies using `--body-file`.
- Publication is complete after reading the issue back, verifying its body and labels, and returning its URL to the user.
- Read a relevant ticket's full body and comments before implementation. For feature-map work, follow the [feature workflow](../feature-workflow.md).
- Use native GitHub sub-issues and issue dependencies for parent/child and blocking relationships when available. Otherwise record `Part of #N` and `Blocked by: #N` in the ticket.
- When closing work, record the result and validation evidence on the issue before closing it.

## Pull requests as a triage surface

**PRs as a request surface: no.**
