# Feature workflow

`docs/features/` is the list of product features. One YAML file per category. [`docs/FEATURES.md`](FEATURES.md) is generated from it. [`docs/features.schema.json`](features.schema.json) is the entry schema.

Each entry has an id, title, category, source, status, priority, summary, notes, reason, issue, and the dates it was added and updated. `reason` is required when the status is `wont-do`. Priority is `p0`–`p3` or empty.

## Status

| Status | Meaning |
| --- | --- |
| `idea` | Not built. Use this when the code was not checked or the behavior is uncertain. |
| `planned` | Accepted, not started. |
| `in-progress` | Someone is building it. |
| `partial` | Some of it works. The notes say what is missing. |
| `shipped` | Available in the app. |
| `wont-do` | Will not be built. Say why in `reason`. |

## Path

1. **Idea.** Add an entry, usually as `idea`, or open a discussion if it is not ready to track.
2. **Triage.** Decide whether to build it. Set `planned`, `wont-do` (with a reason), or leave it as `idea`. Set a priority when the order matters.
3. **Issue.** Open a feature issue from the [feature template](../.github/ISSUE_TEMPLATE/feature.yml). Put the tracker id in the issue, and put the issue URL on the entry. Label it `feature` and `needs-triage`. When the acceptance criteria are clear enough to implement without guessing, add `agent-ready` and remove `needs-triage`.
4. **Build.** An agent or a person implements it on a branch. The pull request links the issue (`Fixes #123`) and updates the entry's status to `in-progress` or `partial`, and its `updated` date.
5. **Shipped.** On merge, the entry is `shipped` (or stays `partial` when the issue only finished part of it) and `FEATURES.md` matches the YAML.

Regenerate the readable list before committing:

```bash
node Scripts/check-features.mjs --write
node Scripts/check-features.mjs --check
```

CI runs the check. It fails on an invalid entry, a duplicate id, a `wont-do` without a reason, or a stale `FEATURES.md`.

## Labels

Create these repository labels before relying on the template:

- `feature` — tracked product work
- `needs-triage` — not yet accepted or specified
- `agent-ready` — acceptance criteria are clear enough to implement

The repository already has `ready-for-agent`. That is a different label. This workflow uses `agent-ready`.
