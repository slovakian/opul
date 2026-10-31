# JavaScript toolchain

Use Vite+ with the pnpm version pinned in `package.json`. Install the [global Vite+ CLI](https://viteplus.dev/guide/) before running the repository scripts. `vp install` and `pnpm install` use the same dependencies and lockfile. CI uses `vp install --frozen-lockfile`.

After switching from an npm checkout, remove the old `node_modules` directory and run `vp install --frozen-lockfile` to get a clean pnpm installation.

## Vite+ configuration

A reference project can be generated outside this repository with:

```bash
vp create vite:application --directory pnpm-reference --package-manager pnpm --no-interactive
```

The repository follows that scaffold's `pnpm-workspace.yaml` configuration:

- The catalog pins `vite-plus` and its matching `vite` alias to the same release.
- The `vite@*` override directs Vite consumers to the bundled `@voidzero-dev/vite-plus-core`.
- Vite peer rules accept the bundled core's version in plugins that declare a regular Vite peer.

Keep the catalog versions aligned when upgrading Vite+. pnpm reads these settings from `pnpm-workspace.yaml`; npm's top-level `overrides` and `legacy-peer-deps` setting do not apply. See the [Vite+ migration rules](https://viteplus.dev/guide/migrate-rules#pnpm).

Dependency build scripts require explicit approval in current pnpm. If a new dependency needs an install script, review it with `vp pm approve-builds` and commit the resulting `allowBuilds` entries. See [Vite+ dependency build scripts](https://viteplus.dev/guide/create#dependency-build-scripts).

## Running checks and the desktop app

```bash
vp check
vp test
vp run build
pnpm exec playwright install webkit
vp run test
pnpm exec tauri dev
```

`vp test` runs the frontend unit tests. `vp run test` runs the Playwright script in `package.json`. `vp run build` includes the project's TypeScript check before bundling.

Tauri's hooks invoke the Vite+ dev and build scripts. Packaging scripts use `pnpm exec tauri` to run the installed CLI. pnpm's default isolated linker works with the current React and shadcn frontend.

CI, nightly, and release workflows use the [official Vite+ setup action](https://github.com/voidzero-dev/setup-vp) to install Vite+, Node.js 24, and the pinned pnpm. Updater signing also uses the installed Tauri CLI.
