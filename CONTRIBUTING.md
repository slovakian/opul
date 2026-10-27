# Contributing to Opul

Focused pull requests and bug reports are welcome. Discuss large changes in an
issue before starting, unless the maintainer has already requested the work.

Install dependencies with `npm ci`, then run:

```bash
npx playwright install webkit
Scripts/test.sh
Scripts/package_app.sh debug
Scripts/smoke_test.sh
```

The UI uses the official shadcn skill installed in `.agents/skills/shadcn`.
Use Base UI primitives, semantic colors, Sidebar popover menus, and the existing
Rust commands. Keep persisted download state in Rust.

Describe the user-visible behavior, tradeoffs, and verification in your PR.
Contributions are licensed under the MIT License.
