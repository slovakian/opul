# Website release notifications

After a successful `Release` or `Nightly` workflow, **Notify website** sends an `opul-release` repository dispatch to `slovakian/opul-website`. The website then fetches releases and assets during SSG and deploys fresh download links, versions, and release history. Waiting for workflow completion ensures installers have finished uploading, including updates to an existing release. Manual release publication, editing, and deletion also notify the site. Other tool releases are excluded. `workflow_run` is necessary because releases published by the built-in `GITHUB_TOKEN` do not trigger another workflow's `release` event.

Merge the companion website listener first. Add **OPUL_WEBSITE_DISPATCH_TOKEN** as an Actions secret in **slovakian/opul**. Use a fine-grained PAT limited to **slovakian/opul-website**, with **Contents: read and write**, owned by an account with write access to the website. The app's built-in `GITHUB_TOKEN` cannot dispatch to another repository. Rotate the PAT before its expiration date.

Run **Notify website → Run workflow** once after setup and confirm the website's **Deploy website** workflow succeeds. If a notification fails, fix the secret and rerun it. The website workflow also supports manual rebuilding from its Actions UI.

No app code is checked out or run by the notification workflow. The token is only passed to the GitHub CLI dispatch step. Cloudflare credentials stay in the website repository.

References: [workflow events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows), [dispatch permissions](https://docs.github.com/en/rest/repos/repos#create-a-repository-dispatch-event).
